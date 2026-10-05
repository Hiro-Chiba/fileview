//! Experimental bounded plain-text windows. This is not a production preview.

use std::fs::{File, Metadata};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::UNIX_EPOCH;

use anyhow::{bail, ensure, Context, Result};
use serde::Serialize;

const CHUNK_BYTES: usize = 8192;
pub const MAX_WINDOW_BYTES: usize = 1024 * 1024;

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

/// A trusted line number and line-start offset obtained from the same file
/// version. Validating the line number itself would require scanning the prefix.
#[derive(Clone, Debug, Serialize)]
pub struct ViewportAnchor {
    pub line_number: u64,
    pub byte_offset: u64,
    pub stamp: FileStamp,
}

#[derive(Debug, Serialize)]
pub struct ViewportPage {
    /// Valid UTF-8 preserving original CRLF/newline bytes. Render with `.lines()`.
    pub text: String,
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
/// Arbitrary distant offsets must be line starts from a scanner, not guessed offsets.
pub fn read_viewport(
    path: &Path,
    anchor: &ViewportAnchor,
    lines: usize,
    max_bytes: usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<ViewportPage> {
    read_window(path, anchor, lines, max_bytes, false, cancelled)
}

/// Display a scanner hit without reading an arbitrarily long line prefix.
/// A distant match starts the displayed window at its exact byte offset. The
/// caller supplies a trusted line number and UTF-8 match offset from the stamped
/// scanner result. No attempt is made to infer arbitrary byte-offset line numbers.
pub fn read_match_viewport(
    path: &Path,
    line_anchor: &ViewportAnchor,
    match_offset: u64,
    lines: usize,
    max_bytes: usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<ViewportPage> {
    ensure!(
        match_offset >= line_anchor.byte_offset,
        "match precedes line start"
    );
    ensure!(
        match_offset < line_anchor.stamp.len,
        "match offset exceeds file length"
    );
    let clipped = match_offset - line_anchor.byte_offset >= (max_bytes / 2) as u64
        && match_offset != line_anchor.byte_offset;
    let mut anchor = line_anchor.clone();
    if clipped {
        anchor.byte_offset = match_offset;
    }
    read_window(path, &anchor, lines, max_bytes, clipped, cancelled)
}

fn read_window(
    path: &Path,
    anchor: &ViewportAnchor,
    lines: usize,
    max_bytes: usize,
    prefix_clipped: bool,
    cancelled: &dyn Fn() -> bool,
) -> Result<ViewportPage> {
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
    let mut file = File::open(path).context("open viewport file")?;
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
        ensure!(previous[0] == b'\n', "anchor must point to a line start");
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
    let next_offset = anchor.byte_offset + bytes.len() as u64;
    let has_more = next_offset < anchor.stamp.len;
    let partial_last_line = has_more && bytes.last().is_some_and(|byte| *byte != b'\n');
    Ok(ViewportPage {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor(path: &Path, line_number: u64, byte_offset: u64) -> ViewportAnchor {
        ViewportAnchor {
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
}
