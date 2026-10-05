//! Bounded UTF-8 text windows with file-version checks and cancellation.

#[cfg(test)]
use std::fs::File;
use std::fs::{Metadata, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::UNIX_EPOCH;

use anyhow::{bail, ensure, Context, Result};
use serde::Serialize;

const CHUNK_BYTES: usize = 8192;
pub const MAX_WINDOW_BYTES: usize = 64 * 1024;
pub const WINDOW_LINES: usize = 256;

/// Observable file identity/version. This does not detect deliberately restored
/// timestamps after same-size edits, and cannot provide an atomic filesystem snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileStamp {
    len: u64,
    modified_ns: u128,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl FileStamp {
    pub fn capture(path: &Path) -> Result<Self> {
        Self::from_metadata(&std::fs::metadata(path)?)
    }

    pub fn from_metadata(metadata: &Metadata) -> Result<Self> {
        ensure!(metadata.is_file(), "viewport requires a regular file");
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            len: metadata.len(),
            modified_ns: metadata.modified()?.duration_since(UNIX_EPOCH)?.as_nanos(),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
        })
    }
}

/// A trusted line number and UTF-8 byte offset from the same file version.
/// Continuation offsets may point inside a line. Validating the line number
/// itself would require scanning the prefix.
#[derive(Clone, Debug, Serialize)]
pub struct TextAnchor {
    pub line_number: u64,
    pub byte_offset: u64,
    pub stamp: FileStamp,
}

#[derive(Debug, Serialize)]
pub struct TextWindow {
    /// Valid UTF-8 preserving original CRLF/newline bytes. Render with `.lines()`.
    pub text: String,
    pub anchor: TextAnchor,
    pub next: Option<TextAnchor>,
    pub first_line: u64,
    pub byte_offset: u64,
    /// Data bytes read; validating a nonzero line-start offset reads one extra byte.
    pub bytes_read: usize,
    /// End of displayed bytes. If partial_last_line is true this is not a line start.
    pub next_offset: u64,
    pub has_more: bool,
    pub partial_last_line: bool,
    /// True when displaying a trusted match offset rather than its line prefix.
    pub prefix_clipped: bool,
}

fn check_cancelled(cancelled: &dyn Fn() -> bool) -> Result<()> {
    ensure!(!cancelled(), "viewport cancelled");
    Ok(())
}

/// Read at most max_bytes plus one validation byte, stopping after the requested
/// number of newline-delimited lines. Long lines are clipped, not fully allocated.
/// Cancellation runs before open and between bounded reads; an individual kernel
/// read or metadata operation is not interruptible by this callback.
/// Distant offsets must come from a scanner or an earlier page, not guessed offsets.
fn read_viewport(
    path: &Path,
    anchor: &TextAnchor,
    lines: usize,
    max_bytes: usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<TextWindow> {
    read_bounded_window(path, anchor, lines, max_bytes, false, cancelled)
}

/// Display a scanner hit without reading an arbitrarily long line prefix.
/// A distant match starts the displayed window at its exact byte offset. The
/// caller supplies a trusted line number and UTF-8 match offset from the stamped
/// scanner result. No attempt is made to infer arbitrary byte-offset line numbers.
fn read_match_viewport(
    path: &Path,
    line_anchor: &TextAnchor,
    match_offset: u64,
    lines: usize,
    max_bytes: usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<TextWindow> {
    ensure!(
        match_offset >= line_anchor.byte_offset,
        "match precedes line start"
    );
    ensure!(
        match_offset < line_anchor.stamp.len,
        "match offset exceeds file length"
    );
    let clipped = match_offset != line_anchor.byte_offset;
    let mut anchor = line_anchor.clone();
    if clipped {
        anchor.byte_offset = match_offset;
    }
    read_bounded_window(path, &anchor, lines, max_bytes, clipped, cancelled)
}

fn read_bounded_window(
    path: &Path,
    anchor: &TextAnchor,
    lines: usize,
    max_bytes: usize,
    mut prefix_clipped: bool,
    cancelled: &dyn Fn() -> bool,
) -> Result<TextWindow> {
    ensure!(
        (1..=1000).contains(&lines),
        "lines must be between 1 and 1000"
    );
    ensure!(
        (1..=MAX_WINDOW_BYTES).contains(&max_bytes),
        "invalid viewport byte budget"
    );
    ensure!(anchor.line_number > 0, "line numbers start at one");
    check_cancelled(cancelled)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let mut file = options.open(path).context("open preview file")?;
    ensure!(
        FileStamp::from_metadata(&file.metadata()?)? == anchor.stamp,
        "file changed since anchor was captured"
    );
    ensure!(
        anchor.byte_offset <= anchor.stamp.len,
        "anchor offset exceeds file length"
    );
    if !prefix_clipped && anchor.byte_offset > 0 && anchor.byte_offset < anchor.stamp.len {
        file.seek(SeekFrom::Start(anchor.byte_offset - 1))?;
        let mut previous = [0];
        file.read_exact(&mut previous)?;
        prefix_clipped = previous[0] != b'\n';
    }
    file.seek(SeekFrom::Start(anchor.byte_offset))?;
    let mut bytes = Vec::with_capacity(max_bytes);
    let mut chunk = [0; CHUNK_BYTES];
    let mut newline_count = 0;
    let mut bytes_read = 0;
    while bytes.len() < max_bytes && newline_count < lines {
        check_cancelled(cancelled)?;
        let capacity = chunk.len().min(max_bytes - bytes.len());
        let count = file.read(&mut chunk[..capacity])?;
        if count == 0 {
            break;
        }
        bytes_read += count;
        let mut used = count;
        for (position, byte) in chunk[..count].iter().enumerate() {
            if *byte == b'\n' {
                newline_count += 1;
                if newline_count == lines {
                    used = position + 1;
                    break;
                }
            }
        }
        bytes.extend_from_slice(&chunk[..used]);
    }
    check_cancelled(cancelled)?;
    ensure!(
        FileStamp::from_metadata(&file.metadata()?)? == anchor.stamp,
        "file changed while reading viewport"
    );
    ensure!(
        FileStamp::capture(path)? == anchor.stamp,
        "file was replaced while reading viewport"
    );
    match std::str::from_utf8(&bytes) {
        Ok(_) => {}
        Err(error)
            if error.error_len().is_none()
                && anchor.byte_offset + (bytes.len() as u64) < anchor.stamp.len =>
        {
            bytes.truncate(error.valid_up_to());
            ensure!(
                !bytes.is_empty(),
                "byte budget too small for first UTF-8 character"
            );
        }
        Err(error) => bail!("viewport is not valid UTF-8: {error}"),
    }
    ensure!(
        !bytes.contains(&0),
        "binary file cannot be displayed as text"
    );
    let next_offset = anchor.byte_offset + bytes.len() as u64;
    let has_more = next_offset < anchor.stamp.len;
    let partial_last_line = has_more && bytes.last().is_some_and(|byte| *byte != b'\n');
    let next = has_more.then(|| TextAnchor {
        line_number: anchor.line_number
            + bytes.iter().filter(|byte| **byte == b'\n').count() as u64,
        byte_offset: next_offset,
        stamp: anchor.stamp.clone(),
    });
    Ok(TextWindow {
        anchor: anchor.clone(),
        next,
        text: String::from_utf8(bytes)?,
        first_line: anchor.line_number,
        byte_offset: anchor.byte_offset,
        bytes_read,
        next_offset,
        has_more,
        partial_last_line,
        prefix_clipped,
    })
}

/// Read a bounded page. Subsequent anchors come from `TextWindow::next`.
/// Stamps detect ordinary edits and replacements, not an atomic filesystem snapshot.
pub fn read_window(
    path: &Path,
    anchor: Option<&TextAnchor>,
    cancelled: &dyn Fn() -> bool,
) -> Result<TextWindow> {
    check_cancelled(cancelled)?;
    let initial;
    let anchor = match anchor {
        Some(anchor) => anchor,
        None => {
            initial = TextAnchor {
                line_number: 1,
                byte_offset: 0,
                stamp: FileStamp::capture(path)?,
            };
            &initial
        }
    };
    read_viewport(path, anchor, WINDOW_LINES, MAX_WINDOW_BYTES, cancelled)
}

/// Open a known search hit at its match byte, including hits on huge single lines.
pub fn read_match_window(
    path: &Path,
    line_anchor: &TextAnchor,
    match_offset: u64,
    cancelled: &dyn Fn() -> bool,
) -> Result<TextWindow> {
    read_match_viewport(
        path,
        line_anchor,
        match_offset,
        WINDOW_LINES,
        MAX_WINDOW_BYTES,
        cancelled,
    )
}

/// Locate the end with bounded memory. Line numbering requires scanning the
/// prefix, so cancellation is checked between every fixed-size read.
pub fn read_bottom_window(path: &Path, cancelled: &dyn Fn() -> bool) -> Result<TextWindow> {
    check_cancelled(cancelled)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let mut file = options.open(path)?;
    let stamp = FileStamp::from_metadata(&file.metadata()?)?;
    let mut chunk = [0; CHUNK_BYTES];
    let mut line_number = 1u64;
    let mut offset = 0u64;
    // Only scan the captured size. An actively appended log cannot keep this
    // request running forever; its changed stamp causes a retry instead.
    while offset < stamp.len {
        check_cancelled(cancelled)?;
        let capacity = CHUNK_BYTES.min((stamp.len - offset) as usize);
        let count = file.read(&mut chunk[..capacity])?;
        ensure!(count > 0, "file changed while locating final page");
        line_number += chunk[..count].iter().filter(|byte| **byte == b'\n').count() as u64;
        offset += count as u64;
    }
    check_cancelled(cancelled)?;
    ensure!(
        FileStamp::from_metadata(&file.metadata()?)? == stamp,
        "file changed while locating final page"
    );
    read_previous_window(
        path,
        &TextAnchor {
            line_number,
            byte_offset: offset,
            stamp,
        },
        cancelled,
    )
}

/// Read the preceding bounded window without retaining a growing page history.
pub fn read_previous_window(
    path: &Path,
    anchor: &TextAnchor,
    cancelled: &dyn Fn() -> bool,
) -> Result<TextWindow> {
    check_cancelled(cancelled)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let mut file = options.open(path)?;
    ensure!(
        FileStamp::from_metadata(&file.metadata()?)? == anchor.stamp,
        "file changed since anchor was captured"
    );
    let start = anchor.byte_offset.saturating_sub(MAX_WINDOW_BYTES as u64);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = vec![0; (anchor.byte_offset - start) as usize];
    for chunk in bytes.chunks_mut(CHUNK_BYTES) {
        check_cancelled(cancelled)?;
        file.read_exact(chunk)?;
    }
    ensure!(
        FileStamp::from_metadata(&file.metadata()?)? == anchor.stamp,
        "file changed while reading preview"
    );
    ensure!(
        FileStamp::capture(path)? == anchor.stamp,
        "file was replaced while reading preview"
    );
    // Keep the last page of lines. At an arbitrary byte boundary discard the
    // leading UTF-8 continuation bytes, preserving progress through huge lines.
    let mut skip = 0;
    if start > 0 {
        while skip < bytes.len() && bytes[skip] & 0xc0 == 0x80 {
            skip += 1;
        }
    }
    let newlines: Vec<usize> = bytes
        .iter()
        .enumerate()
        .filter_map(|(i, b)| (*b == b'\n').then_some(i))
        .collect();
    let keep_newlines = WINDOW_LINES - usize::from(bytes.last().is_some_and(|byte| *byte != b'\n'));
    if newlines.len() > keep_newlines {
        skip = newlines[newlines.len() - keep_newlines - 1] + 1;
    } else if start > 0 && newlines.len() > 1 {
        skip = newlines[0] + 1;
    }
    let preceding_lines = bytes[skip..].iter().filter(|byte| **byte == b'\n').count() as u64;
    let previous = TextAnchor {
        line_number: anchor.line_number.saturating_sub(preceding_lines).max(1),
        byte_offset: start + skip as u64,
        stamp: anchor.stamp.clone(),
    };
    read_window(path, Some(&previous), cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor(path: &Path, line_number: u64, byte_offset: u64) -> TextAnchor {
        TextAnchor {
            line_number,
            byte_offset,
            stamp: FileStamp::capture(path).unwrap(),
        }
    }

    #[test]
    fn pages_match_whole_small_fixture_and_near_eof() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("lines.txt");
        let content = (0..100)
            .map(|n| format!("line {n}\r\n"))
            .collect::<String>();
        std::fs::write(&path, &content).unwrap();
        let page = read_viewport(&path, &anchor(&path, 1, 0), 40, 65536, &|| false).unwrap();
        assert_eq!(
            page.text.lines().collect::<Vec<_>>(),
            content.lines().take(40).collect::<Vec<_>>()
        );
        let offset = content.find("line 98\r\n").unwrap() as u64;
        let page = read_viewport(&path, &anchor(&path, 99, offset), 40, 65536, &|| false).unwrap();
        assert_eq!(page.text, "line 98\r\nline 99\r\n");
        assert!(!page.has_more);
        assert_eq!(page.first_line, 99);
        let page = read_viewport(
            &path,
            &anchor(&path, 101, content.len() as u64),
            40,
            65536,
            &|| false,
        )
        .unwrap();
        assert!(page.text.is_empty());
    }

    #[test]
    fn huge_single_line_and_utf8_cap_remain_bounded() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("long.txt");
        std::fs::write(&path, "あ".repeat(100_000)).unwrap();
        let page = read_viewport(&path, &anchor(&path, 1, 0), 40, 65536, &|| false).unwrap();
        assert_eq!(page.bytes_read, 65536);
        assert_eq!(page.text.len(), 65535);
        assert!(page.partial_last_line && page.has_more);
        assert!(read_viewport(&path, &anchor(&path, 1, 1), 40, 65536, &|| false).is_err());
        assert!(read_viewport(&path, &anchor(&path, 1, 0), 40, 2, &|| false).is_err());
    }

    #[test]
    fn match_near_end_of_long_line_is_visible_without_reading_prefix() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("long.txt");
        let prefix = "あ".repeat(400_000);
        let content = format!("{prefix}TARGET日本語\nnext\n");
        std::fs::write(&path, content).unwrap();
        let original = anchor(&path, 1, 0);
        let page = read_match_viewport(&path, &original, prefix.len() as u64, 40, 65536, &|| false)
            .unwrap();
        assert_eq!(page.text, "TARGET日本語\nnext\n");
        assert!(page.prefix_clipped);
        assert_eq!(page.first_line, 1);
        assert!(page.bytes_read <= 65536);
        assert!(read_match_viewport(&path, &original, 1_000_001, 40, 65536, &|| false).is_err());
    }

    #[test]
    fn same_length_edit_invalidates_anchor_by_modification_time() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("file.txt");
        std::fs::write(&path, "first\nsecond\n").unwrap();
        let original = anchor(&path, 2, 6);
        let later = std::fs::metadata(&path).unwrap().modified().unwrap()
            + std::time::Duration::from_secs(2);
        std::fs::write(&path, "other\nsecond\n").unwrap();
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(later))
            .unwrap();
        assert!(read_viewport(&path, &original, 40, 65536, &|| false).is_err());
    }

    #[test]
    fn rejects_changed_truncated_and_replaced_files() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("file.txt");
        std::fs::write(&path, "first\nsecond\n").unwrap();
        let original = anchor(&path, 2, 6);
        std::fs::write(&path, "short").unwrap();
        assert!(read_viewport(&path, &original, 40, 65536, &|| false)
            .unwrap_err()
            .to_string()
            .contains("changed"));
        let changed = anchor(&path, 1, 0);
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, "replacement").unwrap();
        assert!(read_viewport(&path, &changed, 40, 65536, &|| false).is_err());
    }

    #[test]
    fn cancellation_and_mid_read_mutation_are_detected() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("file.txt");
        std::fs::write(&path, "a".repeat(200_000)).unwrap();
        let original = anchor(&path, 1, 0);
        let calls = std::cell::Cell::new(0);
        assert!(read_viewport(&path, &original, 40, 65536, &|| {
            calls.set(calls.get() + 1);
            calls.get() >= 3
        })
        .unwrap_err()
        .to_string()
        .contains("cancelled"));
        let calls = std::cell::Cell::new(0);
        assert!(read_viewport(&path, &original, 40, 65536, &|| {
            calls.set(calls.get() + 1);
            if calls.get() == 3 {
                std::fs::write(&path, "changed").unwrap();
            }
            false
        })
        .is_err());
    }
    #[test]
    fn production_pages_preserve_unicode_and_line_numbers_in_both_directions() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pages.log");
        let content = (0..700)
            .map(|n| format!("日本語 {n}\r\n"))
            .collect::<String>();
        std::fs::write(&path, &content).unwrap();
        let first = read_window(&path, None, &|| false).unwrap();
        assert_eq!(first.text.lines().count(), 256);
        let second = read_window(&path, first.next.as_ref(), &|| false).unwrap();
        assert_eq!(second.first_line, 257);
        let previous = read_previous_window(&path, &second.anchor, &|| false).unwrap();
        assert_eq!(previous.text, first.text);
        let bottom = read_bottom_window(&path, &|| false).unwrap();
        assert_eq!(bottom.first_line, 445);
        assert!(bottom.text.ends_with("日本語 699\r\n"));
        assert!(bottom.next.is_none());
    }

    #[test]
    fn huge_line_continuations_reconstruct_without_unbounded_memory() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("huge.log");
        let content = format!("{}\nlast", "あ".repeat(100_000));
        std::fs::write(&path, &content).unwrap();
        let mut page = read_window(&path, None, &|| false).unwrap();
        let mut reconstructed = page.text.clone();
        while let Some(next) = page.next {
            page = read_window(&path, Some(&next), &|| false).unwrap();
            assert!(page.text.len() <= MAX_WINDOW_BYTES);
            assert_eq!(page.first_line, 1);
            assert!(page.prefix_clipped);
            reconstructed.push_str(&page.text);
        }
        assert_eq!(reconstructed, content);
        let bottom = read_bottom_window(&path, &|| false).unwrap();
        assert!(bottom.text.ends_with("\nlast"));
        assert_eq!(bottom.first_line, 1);
    }

    #[test]
    fn binary_invalid_utf8_and_cancelled_bottom_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("binary.log");
        std::fs::write(&path, b"a\0b").unwrap();
        assert!(read_window(&path, None, &|| false)
            .unwrap_err()
            .to_string()
            .contains("binary"));
        std::fs::write(&path, [0xff]).unwrap();
        assert!(read_window(&path, None, &|| false).is_err());
        assert!(read_bottom_window(&path, &|| true)
            .unwrap_err()
            .to_string()
            .contains("cancelled"));
    }

    #[cfg(unix)]
    #[test]
    fn fifo_is_rejected_without_waiting_for_a_writer() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pipe.txt");
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: name is NUL terminated and remains alive for the call.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(read_window(&path, None, &|| false).is_err());
        assert!(read_bottom_window(&path, &|| false).is_err());
    }
    #[test]
    fn bottom_includes_last_line_without_trailing_newline() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("tail.log");
        std::fs::write(
            &path,
            (1..=700)
                .map(|n| format!("line {n}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
        let bottom = read_bottom_window(&path, &|| false).unwrap();
        assert_eq!(bottom.first_line, 445);
        assert_eq!(bottom.text.lines().count(), 256);
        assert!(bottom.text.ends_with("line 700"));
        assert!(bottom.next.is_none());
    }
}
