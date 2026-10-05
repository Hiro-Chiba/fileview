//! Bounded, cancellable literal content search using optional ripgrep.
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use crate::render::preview::window::FileStamp;
use anyhow::{bail, ensure, Context, Result};

const MAX_PATH_BYTES: usize = 64 * 1024;
const BINARY_PREFIX_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug)]
pub struct ContentHit {
    pub path: PathBuf,
    pub line_number: u64,
    pub line_start: u64,
    pub match_offset: u64,
    pub snippet: String,
    pub stamp: FileStamp,
}

#[derive(Default, Debug)]
pub struct SearchSummary {
    pub hits: usize,
    pub cancelled: bool,
    pub limit_reached: bool,
    pub changed_results: usize,
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct RawHit {
    path: PathBuf,
    line_number: u64,
    line_start: u64,
    match_offset: u64,
}

struct LastFile {
    path: PathBuf,
    stamp: Option<FileStamp>,
    binary: bool,
}

enum Verification {
    Text(String),
    Binary,
    Changed,
}

/// Literal, case-sensitive search. Ripgrep is required only for this operation.
/// Files containing NUL within their first 64 KiB are skipped. This bounded
/// heuristic does not detect every binary file. Metadata stamps reject observable
/// edits after the first result; they are not filesystem snapshots.
/// Cancellation polls every 10 ms, excluding filesystem operations and callbacks.
/// Parent-side output and snippet buffers are bounded independently of line size.
/// Ripgrep's own line buffering and memory maps are outside that guarantee.
pub fn search(
    root: &Path,
    query: &str,
    hidden: bool,
    limit: usize,
    cancelled: &dyn Fn() -> bool,
    on_hit: &mut dyn FnMut(ContentHit),
) -> Result<SearchSummary> {
    search_with_program(
        Path::new("rg"),
        root,
        query,
        hidden,
        limit,
        cancelled,
        on_hit,
    )
}

#[allow(clippy::too_many_arguments)]
fn search_with_program(
    program: &Path,
    root: &Path,
    query: &str,
    hidden: bool,
    limit: usize,
    cancelled: &dyn Fn() -> bool,
    on_hit: &mut dyn FnMut(ContentHit),
) -> Result<SearchSummary> {
    ensure!(
        !query.is_empty() && query.len() <= 4096,
        "content query must contain 1 to 4096 bytes"
    );
    ensure!(
        !query.contains(['\n', '\r', '\0']),
        "content query must be a single line without NUL"
    );
    ensure!(
        (1..=1000).contains(&limit),
        "content result limit must be between 1 and 1000"
    );
    if cancelled() {
        return Ok(SearchSummary {
            cancelled: true,
            ..Default::default()
        });
    }
    let root = root.canonicalize().context("open content search root")?;
    let mut command = Command::new(program);
    command.args([
        "--no-config",
        "--line-buffered",
        "--case-sensitive",
        "--color=never",
        "--threads=1",
        "--encoding=none",
        "--text",
        "--no-heading",
        "--only-matching",
        "--null",
        "--with-filename",
        "--line-number",
        "--column",
        "--byte-offset",
        "--replace",
        "$1",
    ]);
    if hidden {
        command.arg("--hidden");
    }
    // Consume each matching line, but print only the captured literal. This
    // produces one bounded result per line even for millions of repeated matches.
    // --text prevents unframed binary warnings; verify_hit applies our NUL policy.
    let pattern = format!("({})(?-u:.*)", regex::escape(query));
    command
        .arg("--")
        .arg(pattern)
        .arg(&root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let child = command
        .spawn()
        .context("content search requires ripgrep (rg) on PATH")?;
    let mut process = Process(child);
    let stdout = process.0.stdout.take().context("open ripgrep output")?;
    let (tx, rx) = mpsc::sync_channel(8);
    let expected = query.as_bytes().to_vec();
    let reader = thread::spawn(move || {
        let mut input = BufReader::new(stdout);
        loop {
            match read_hit(&mut input, &expected) {
                Ok(Some(record)) => {
                    if tx.send(Ok(record)).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    let _ = tx.send(Err(error));
                    break;
                }
            }
        }
    });
    let result = (|| {
        let mut summary = SearchSummary::default();
        // One rg worker emits a file's results contiguously. Keep only that file.
        let mut last: Option<LastFile> = None;
        loop {
            if cancelled() {
                summary.cancelled = true;
                break;
            }
            let hit = match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(record) => record?,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    let status = process.0.wait()?;
                    ensure!(
                        status.success() || status.code() == Some(1),
                        "ripgrep could not complete content search (exit {status})"
                    );
                    break;
                }
            };
            let first_hit = last.as_ref().is_none_or(|file| file.path != hit.path);
            if first_hit {
                last = Some(LastFile {
                    stamp: FileStamp::capture(&hit.path).ok(),
                    path: hit.path.clone(),
                    binary: false,
                });
            }
            let file = last
                .as_mut()
                .expect("last file initialized for each result");
            if file.binary {
                continue;
            }
            let Some(stamp) = file.stamp.as_ref() else {
                summary.changed_results += 1;
                continue;
            };
            let verification = verify_hit(&hit.path, stamp, hit.match_offset, query, first_hit)
                .unwrap_or(Verification::Changed);
            let snippet = match verification {
                Verification::Text(snippet) => snippet,
                Verification::Binary => {
                    file.binary = true;
                    continue;
                }
                Verification::Changed => {
                    summary.changed_results += 1;
                    continue;
                }
            };
            on_hit(ContentHit {
                path: hit.path,
                line_number: hit.line_number,
                line_start: hit.line_start,
                match_offset: hit.match_offset,
                snippet,
                stamp: stamp.clone(),
            });
            summary.hits += 1;
            if summary.hits >= limit {
                summary.limit_reached = true;
                break;
            }
        }
        Ok(summary)
    })();
    // Kill first to unblock pipe reads, then drop the channel to unblock sends.
    drop(process);
    drop(rx);
    let _ = reader.join();
    result
}

fn verify_hit(
    path: &Path,
    stamp: &FileStamp,
    offset: u64,
    query: &str,
    check_binary: bool,
) -> Result<Verification> {
    if FileStamp::capture(path)? != *stamp {
        return Ok(Verification::Changed);
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // A replaced FIFO must not stall the worker; symlinks are not followed.
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    let mut file = options.open(path)?;
    if FileStamp::from_metadata(&file.metadata()?)? != *stamp {
        return Ok(Verification::Changed);
    }
    if check_binary {
        let mut prefix = Vec::with_capacity(BINARY_PREFIX_BYTES);
        (&mut file)
            .take(BINARY_PREFIX_BYTES as u64)
            .read_to_end(&mut prefix)?;
        if prefix.contains(&0) {
            return Ok(Verification::Binary);
        }
    }
    file.seek(SeekFrom::Start(offset))?;
    let capacity = query.len().max(512);
    let mut bytes = Vec::with_capacity(capacity);
    (&mut file).take(capacity as u64).read_to_end(&mut bytes)?;
    if !bytes.starts_with(query.as_bytes())
        || FileStamp::from_metadata(&file.metadata()?)? != *stamp
        || FileStamp::capture(path)? != *stamp
    {
        return Ok(Verification::Changed);
    }
    let end = bytes
        .iter()
        .position(|&byte| byte == b'\n')
        .unwrap_or(bytes.len());
    Ok(Verification::Text(
        String::from_utf8_lossy(&bytes[..end])
            .trim_end_matches('\r')
            .to_string(),
    ))
}

/// NUL terminates the filename, so newlines, colons and non-UTF-8 names remain
/// unambiguous. The following fields are decimal line, byte-column, byte-offset,
/// then the exact literal query terminated by LF.
fn read_hit(input: &mut impl BufRead, query: &[u8]) -> Result<Option<RawHit>> {
    let Some(path) = read_field(input, 0, MAX_PATH_BYTES)? else {
        return Ok(None);
    };
    ensure!(!path.is_empty(), "empty ripgrep result path");
    let line_number = read_number(input)?;
    let column = read_number(input)?;
    let match_offset = read_number(input)?;
    ensure!(
        line_number > 0 && column > 0,
        "invalid ripgrep line or column"
    );
    let line_start = match_offset
        .checked_sub(column - 1)
        .context("invalid ripgrep byte offset")?;
    let literal = read_field(input, b'\n', query.len())?.context("incomplete ripgrep match")?;
    ensure!(literal == query, "unexpected ripgrep match payload");
    Ok(Some(RawHit {
        path: decode_path(path)?,
        line_number,
        line_start,
        match_offset,
    }))
}

fn read_number(input: &mut impl BufRead) -> Result<u64> {
    let bytes = read_field(input, b':', 20)?.context("incomplete ripgrep numeric field")?;
    ensure!(
        !bytes.is_empty() && bytes.iter().all(u8::is_ascii_digit),
        "invalid ripgrep numeric field"
    );
    std::str::from_utf8(&bytes)?
        .parse()
        .context("ripgrep numeric field exceeds u64")
}

fn read_field(input: &mut impl BufRead, delimiter: u8, limit: usize) -> Result<Option<Vec<u8>>> {
    let mut field = Vec::new();
    loop {
        let available = input.fill_buf()?;
        if available.is_empty() {
            return if field.is_empty() {
                Ok(None)
            } else {
                bail!("incomplete ripgrep result field")
            };
        }
        let position = available.iter().position(|&byte| byte == delimiter);
        let count = position.unwrap_or(available.len());
        ensure!(
            field.len() + count <= limit,
            "ripgrep result field exceeds its bounded limit"
        );
        field.extend_from_slice(&available[..count]);
        input.consume(count + usize::from(position.is_some()));
        if position.is_some() {
            return Ok(Some(field));
        }
    }
}

fn decode_path(bytes: Vec<u8>) -> Result<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(std::ffi::OsString::from_vec(bytes).into())
    }
    #[cfg(not(unix))]
    {
        Ok(String::from_utf8(bytes)
            .context("non-UTF-8 content path is unsupported on this platform")?
            .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_result_fields() {
        assert!(read_hit(&mut &vec![b'x'; MAX_PATH_BYTES + 1][..], b"x").is_err());
        assert!(read_hit(&mut &b"partial"[..], b"x").is_err());
        assert!(read_hit(&mut &b"path\x001:1:0:wrong\n"[..], b"x").is_err());
        assert!(read_hit(&mut &b"path\x000:1:0:x\n"[..], b"x").is_err());
        let hit = read_hit(&mut &b"C:\\a:new\nline\x002:10:15:x\n"[..], b"x")
            .unwrap()
            .unwrap();
        assert_eq!(hit.line_number, 2);
        assert_eq!(hit.match_offset, 15);
        assert_eq!(hit.line_start, 6);
        assert_eq!(hit.path, PathBuf::from("C:\\a:new\nline"));
    }
    #[test]
    fn validates_before_launch() {
        for query in ["", "a\nb", "a\0b"] {
            assert!(search(Path::new("."), query, false, 10, &|| false, &mut |_| {}).is_err());
        }
        let temp = tempfile::tempdir().unwrap();
        let error = search_with_program(
            &temp.path().join("missing-rg"),
            temp.path(),
            "x",
            false,
            10,
            &|| false,
            &mut |_| {},
        )
        .unwrap_err();
        assert!(error.to_string().contains("requires ripgrep"));
    }
    #[test]
    fn literal_unicode_offsets_and_ignore() {
        if Command::new("rg").arg("--version").output().is_err() {
            eprintln!("ripgrep unavailable; skipping real-process integration test");
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("visible"), "first\r\n日本語TARGET\n").unwrap();
        std::fs::write(temp.path().join(".hidden"), "TARGET").unwrap();
        std::fs::write(temp.path().join(".ignore"), "ignored\n").unwrap();
        std::fs::write(temp.path().join("ignored"), "TARGET").unwrap();
        let mut hits = Vec::new();
        let summary = search(temp.path(), "TARGET", false, 10, &|| false, &mut |hit| {
            hits.push(hit)
        })
        .unwrap();
        assert_eq!(summary.hits, 1);
        assert_eq!(hits[0].line_number, 2);
        assert_eq!(hits[0].line_start, 7);
        assert_eq!(hits[0].match_offset, 16);
        assert_eq!(hits[0].snippet, "TARGET");
        assert_eq!(
            search(temp.path(), "TARGET", true, 10, &|| false, &mut |_| {})
                .unwrap()
                .hits,
            2
        );
    }
    #[test]
    fn early_cancel_and_hit_limit() {
        if Command::new("rg").arg("--version").output().is_err() {
            eprintln!("ripgrep unavailable; skipping real-process integration test");
            return;
        }
        assert!(
            search(Path::new("."), "x", false, 1, &|| true, &mut |_| {})
                .unwrap()
                .cancelled
        );
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("hits"), "x\nx\nx\n").unwrap();
        let summary = search(temp.path(), "x", false, 1, &|| false, &mut |_| {}).unwrap();
        assert_eq!(summary.hits, 1);
        assert!(summary.limit_reached);
    }
    #[cfg(unix)]
    #[test]
    fn decodes_non_utf8_paths() {
        use std::os::unix::ffi::OsStrExt;
        let path = decode_path(vec![b'a', b'/', 255]).unwrap();
        assert_eq!(path.as_os_str().as_bytes(), &[b'a', b'/', 255]);
    }
    #[test]
    fn rejects_results_after_observable_edit() {
        if Command::new("rg").arg("--version").output().is_err() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("hits");
        std::fs::write(&path, "x+y\nx+y\n").unwrap();
        let result = search(temp.path(), "x+y", false, 10, &|| false, &mut |_| {
            std::fs::write(&path, "changed and larger content\n").unwrap();
        })
        .unwrap();
        assert_eq!(result.hits, 1);
        assert_eq!(result.changed_results, 1);
    }

    #[cfg(unix)]
    #[test]
    fn searches_non_utf8_filename_without_following_symlinks() {
        use std::os::unix::ffi::OsStringExt;
        if Command::new("rg").arg("--version").output().is_err() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let mut path = temp
            .path()
            .join(std::ffi::OsString::from_vec(vec![b'f', 255]));
        if let Err(error) = std::fs::write(&path, "TARGET") {
            // APFS rejects invalid UTF-8 names. Keep the no-follow assertion there.
            eprintln!("filesystem rejected non-UTF-8 name: {error}");
            path = temp.path().join("日本語");
            std::fs::write(&path, "TARGET").unwrap();
        }
        std::fs::write(outside.path().join("outside"), "TARGET").unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("linked")).unwrap();
        let mut found = Vec::new();
        search(temp.path(), "TARGET", false, 10, &|| false, &mut |hit| {
            found.push(hit.path)
        })
        .unwrap();
        assert_eq!(found, vec![path.canonicalize().unwrap()]);
    }

    #[test]
    fn frequent_matches_emit_one_bounded_result_per_line() {
        if Command::new("rg").arg("--version").output().is_err() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("dense");
        std::fs::write(&path, "x".repeat(30_000)).unwrap();
        let mut hits = Vec::new();
        let summary = search(&path, "x", false, 10, &|| false, &mut |hit| hits.push(hit)).unwrap();
        assert_eq!(summary.hits, 1);
        assert_eq!(hits[0].match_offset, 0);
        assert_eq!(hits[0].snippet, "x".repeat(512));
    }

    #[test]
    fn literal_regex_characters_and_invalid_utf8_tail() {
        if Command::new("rg").arg("--version").output().is_err() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("literal");
        let query = "日本語.*+[x]($)^?\\";
        let mut input = format!("nonmatching 日本語xxxxx\nprefix {query}").into_bytes();
        input.extend_from_slice(&[255, 254]);
        input.extend_from_slice(query.as_bytes());
        input.extend_from_slice(b"\n");
        std::fs::write(&path, &input).unwrap();
        let mut hits = Vec::new();
        let summary = search(&path, query, false, 10, &|| false, &mut |hit| {
            hits.push(hit)
        })
        .unwrap();
        assert_eq!(summary.hits, 1);
        assert_eq!(hits[0].line_number, 2);
        assert_eq!(hits[0].line_start, "nonmatching 日本語xxxxx\n".len() as u64);
        assert_eq!(
            hits[0].match_offset,
            "nonmatching 日本語xxxxx\nprefix ".len() as u64
        );
        assert!(hits[0].snippet.starts_with(query));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_replaced_file_and_symlink_for_verification() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("source");
        let target = temp.path().join("target");
        std::fs::write(&path, "TARGET").unwrap();
        let stamp = FileStamp::capture(&path).unwrap();
        assert!(matches!(
            verify_hit(&path, &stamp, 0, "TARGET", true).unwrap(),
            Verification::Text(_)
        ));
        std::fs::rename(&path, &target).unwrap();
        std::os::unix::fs::symlink(&target, &path).unwrap();
        assert!(!matches!(
            verify_hit(&path, &stamp, 0, "TARGET", true),
            Ok(Verification::Text(_))
        ));
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, "TARGET").unwrap();
        assert!(matches!(
            verify_hit(&path, &stamp, 0, "TARGET", true).unwrap(),
            Verification::Changed
        ));
    }

    #[test]
    fn searches_megabyte_lines_with_far_japanese_match() {
        use std::io::Write;
        if Command::new("rg").arg("--version").output().is_err() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("huge-line");
        let query = "日本語TARGET($1)";
        let chunk = vec![b'x'; 64 * 1024];
        for size in [1024 * 1024, 16 * 1024 * 1024] {
            let mut file = std::fs::File::create(&path).unwrap();
            file.write_all(b"header\n").unwrap();
            for _ in 0..size / chunk.len() {
                file.write_all(&chunk).unwrap();
            }
            file.write_all(query.as_bytes()).unwrap();
            file.write_all(b" trailing context").unwrap();
            file.flush().unwrap();
            let mut hits = Vec::new();
            let summary = search(&path, query, false, 10, &|| false, &mut |hit| {
                hits.push(hit)
            })
            .unwrap();
            assert_eq!(summary.hits, 1);
            assert_eq!(hits[0].line_number, 2);
            assert_eq!(hits[0].line_start, 7);
            assert_eq!(hits[0].match_offset, 7 + size as u64);
            assert_eq!(hits[0].snippet, format!("{query} trailing context"));
            // A dense huge line must also emit only the first occurrence.
            let mut dense = Vec::new();
            search(&path, "x", false, 10, &|| false, &mut |hit| dense.push(hit)).unwrap();
            assert_eq!(dense.len(), 1);
            assert_eq!(dense[0].match_offset, 7);
            assert_eq!(dense[0].snippet.len(), 512);
        }
    }

    #[test]
    fn binary_prefix_policy_keeps_output_framed() {
        if Command::new("rg").arg("--version").output().is_err() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("binary");
        std::fs::write(&path, b"TARGET\0data").unwrap();
        assert_eq!(
            search(&path, "TARGET", false, 10, &|| false, &mut |_| {})
                .unwrap()
                .hits,
            0
        );
        // A NUL beyond the documented prefix does not suppress earlier results
        // and must never inject an unframed binary-warning message into stdout.
        let mut bytes = vec![b'x'; BINARY_PREFIX_BYTES + 10];
        bytes[..7].copy_from_slice(b"TARGET\n");
        bytes.push(0);
        bytes.extend_from_slice(b"TARGET");
        std::fs::write(&path, bytes).unwrap();
        assert_eq!(
            search(&path, "TARGET", false, 10, &|| false, &mut |_| {})
                .unwrap()
                .hits,
            2
        );
    }

    #[cfg(unix)]
    #[test]
    fn searches_paths_with_colons_and_newlines() {
        if Command::new("rg").arg("--version").output().is_err() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("colon:new\nline 日本語");
        std::fs::write(&path, "prefixTARGET context").unwrap();
        let mut found = Vec::new();
        search(temp.path(), "TARGET", false, 10, &|| false, &mut |hit| {
            found.push(hit)
        })
        .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, path.canonicalize().unwrap());
        assert_eq!(found[0].match_offset, 6);
        assert_eq!(found[0].snippet, "TARGET context");
    }

    #[cfg(unix)]
    #[test]
    fn cancels_silent_process_and_reaps() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let program = temp.path().join("fake-rg");
        // exec ensures no descendant retains the pipe after the owned process dies.
        std::fs::write(&program, "#!/bin/sh\nexec sleep 30\n").unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let started = std::time::Instant::now();
        let result = search_with_program(
            &program,
            temp.path(),
            "x",
            false,
            10,
            &|| started.elapsed() > Duration::from_millis(30),
            &mut |_| {},
        )
        .unwrap();
        assert!(result.cancelled);
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
