//! Experimental bounded-memory literal search. Not a production FileView API.
//!
//! Text policy: skip files with NUL in the first chunk. If a later chunk contains
//! NUL, stop that file and report it; earlier streamed hits cannot be retracted.
//! Queries are nonempty single-line UTF-8 literals, at most 4096 bytes. Snippets
//! contain bounded preceding context through the match, not complete long lines.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Context, Result};
use ignore::WalkBuilder;
use regex::bytes::Regex;
use serde::Serialize;

use super::viewport_probe::FileStamp;

/// A match with independently usable source-line and match byte offsets.
#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub path: PathBuf,
    pub line_number: u64,
    pub line_start: u64,
    pub match_offset: u64,
    pub snippet: String,
    pub snippet_start: u64,
    pub truncated_before: bool,
    pub stamp: FileStamp,
}

/// Explicit bounds for the experimental search.
#[derive(Debug, Clone)]
pub struct Options {
    pub show_hidden: bool,
    pub max_hits: usize,
    pub chunk_bytes: usize,
    pub snippet_bytes: usize,
    pub max_file_bytes: Option<u64>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            show_hidden: false,
            max_hits: 100,
            chunk_bytes: 64 * 1024,
            snippet_bytes: 512,
            max_file_bytes: None,
        }
    }
}

/// Search measurements and explicit incompleteness signals.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Stats {
    pub files_seen: usize,
    pub files_searched: usize,
    pub bytes_read: u64,
    pub hits: usize,
    pub binary_skipped: usize,
    pub binary_stopped_after_prefix: usize,
    pub changed_files: usize,
    pub cancelled: bool,
    pub hit_limit_reached: bool,
    pub byte_limit_reached: bool,
    pub callback_stopped: bool,
    pub first_hit_ms: Option<f64>,
    pub total_ms: f64,
}

/// Search an explicit regular file or an ignore-aware workspace directory.
/// Callback false ends the search. Cancellation is checked between bounded
/// reads and hits; a blocked OS read itself is not forcibly interruptible.
pub fn search(
    root: &Path,
    needle: &str,
    options: &Options,
    cancelled: &dyn Fn() -> bool,
    on_hit: &mut dyn FnMut(&Hit) -> bool,
) -> Result<Stats> {
    if needle.is_empty() || needle.len() > 4096 || needle.contains(['\n', '\r', '\0']) {
        bail!("query must be a nonempty single-line UTF-8 literal up to 4096 bytes");
    }
    if options.max_hits == 0
        || options.max_hits > 100_000
        || !(1..=1024 * 1024).contains(&options.chunk_bytes)
        || !(1..=64 * 1024).contains(&options.snippet_bytes)
    {
        bail!("invalid search bounds");
    }
    let start = Instant::now();
    let regex = Regex::new(&regex::escape(needle))?;
    let mut stats = Stats::default();
    let metadata = root.symlink_metadata()?;
    if metadata.is_file() {
        scan_file(
            root,
            &regex,
            needle.len(),
            options,
            cancelled,
            on_hit,
            &mut stats,
            start,
        )?;
    } else if metadata.is_dir() {
        let mut builder = WalkBuilder::new(root);
        builder
            .hidden(!options.show_hidden)
            .git_global(false)
            .parents(false)
            .follow_links(false)
            .require_git(false)
            .filter_entry(|entry| entry.file_name() != ".git");
        for entry in builder.build() {
            if stopped(&mut stats, cancelled) {
                break;
            }
            let entry = entry?;
            if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                continue;
            }
            scan_file(
                entry.path(),
                &regex,
                needle.len(),
                options,
                cancelled,
                on_hit,
                &mut stats,
                start,
            )?;
        }
    } else {
        bail!("search root must be a regular file or directory, not a symlink");
    }
    stats.total_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(stats)
}

fn stopped(stats: &mut Stats, cancelled: &dyn Fn() -> bool) -> bool {
    if cancelled() {
        stats.cancelled = true;
    }
    stats.cancelled || stats.hit_limit_reached || stats.callback_stopped
}

#[allow(clippy::too_many_arguments)]
fn scan_file(
    path: &Path,
    regex: &Regex,
    needle_len: usize,
    options: &Options,
    cancelled: &dyn Fn() -> bool,
    on_hit: &mut dyn FnMut(&Hit) -> bool,
    stats: &mut Stats,
    start: Instant,
) -> Result<()> {
    if stopped(stats, cancelled) {
        return Ok(());
    }
    stats.files_seen += 1;
    // Do not intentionally follow leaf symlinks discovered after walking.
    if !path.symlink_metadata()?.is_file() {
        return Ok(());
    }
    let mut file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let stamp = FileStamp::from_metadata(&file.metadata()?)?;
    if FileStamp::capture(path)? != stamp {
        stats.changed_files += 1;
        return Ok(());
    }
    stats.files_searched += 1;
    let carry_limit = needle_len.saturating_sub(1).max(options.snippet_bytes);
    let mut data = Vec::with_capacity(options.chunk_bytes + carry_limit);
    let mut chunk = vec![0; options.chunk_bytes];
    let mut processed = 0_u64;
    let mut next_match_start = 0_u64;
    let mut line_number = 1_u64;
    let mut line_start = 0_u64;
    loop {
        if stopped(stats, cancelled) {
            break;
        }
        let remaining = options
            .max_file_bytes
            .map(|limit| limit.saturating_sub(processed));
        if remaining == Some(0) {
            if file.metadata()?.len() > processed {
                stats.byte_limit_reached = true;
            }
            break;
        }
        let allowed = remaining.map_or(chunk.len(), |remaining| {
            remaining.min(chunk.len() as u64) as usize
        });
        let read = file.read(&mut chunk[..allowed])?;
        stats.bytes_read += read as u64;
        if read == 0 {
            break;
        }
        if chunk[..read].contains(&0) {
            if processed == 0 {
                stats.binary_skipped += 1;
            } else {
                stats.binary_stopped_after_prefix += 1;
            }
            break;
        }
        let carry = data.len();
        let base = processed - carry as u64;
        data.extend_from_slice(&chunk[..read]);
        let mut position = next_match_start.saturating_sub(base) as usize;
        let mut line_cursor = carry;
        while position <= data.len() {
            if stopped(stats, cancelled) {
                break;
            }
            let Some(found) = regex.find_at(&data, position) else {
                break;
            };
            position = found.end();
            let absolute_end = base + found.end() as u64;
            if absolute_end <= processed {
                continue;
            }
            if found.start() >= line_cursor {
                advance_lines(
                    &data,
                    line_cursor,
                    found.start(),
                    base,
                    &mut line_number,
                    &mut line_start,
                );
                line_cursor = found.start();
            }
            let mut snippet_start = found
                .end()
                .saturating_sub(options.snippet_bytes)
                .max(line_start.saturating_sub(base) as usize);
            while snippet_start < found.end() && data[snippet_start] & 0xc0 == 0x80 {
                snippet_start += 1;
            }
            let hit = Hit {
                path: path.to_path_buf(),
                line_number,
                line_start,
                match_offset: base + found.start() as u64,
                snippet: String::from_utf8_lossy(&data[snippet_start..found.end()]).into_owned(),
                snippet_start: base + snippet_start as u64,
                truncated_before: base + snippet_start as u64 > line_start,
                stamp: stamp.clone(),
            };
            stats.hits += 1;
            stats
                .first_hit_ms
                .get_or_insert_with(|| start.elapsed().as_secs_f64() * 1000.0);
            if !on_hit(&hit) {
                stats.callback_stopped = true;
            }
            if stats.hits >= options.max_hits {
                stats.hit_limit_reached = true;
            }
            next_match_start = absolute_end;
        }
        advance_lines(
            &data,
            line_cursor,
            data.len(),
            base,
            &mut line_number,
            &mut line_start,
        );
        processed += read as u64;
        let keep = data.len().min(carry_limit);
        let keep_start = data.len() - keep;
        data.copy_within(keep_start.., 0);
        data.truncate(keep);
    }
    if FileStamp::from_metadata(&file.metadata()?)? != stamp
        || FileStamp::capture(path).ok().as_ref() != Some(&stamp)
    {
        stats.changed_files += 1;
    }
    Ok(())
}

fn advance_lines(
    data: &[u8],
    start: usize,
    end: usize,
    base: u64,
    number: &mut u64,
    line_start: &mut u64,
) {
    for (offset, byte) in data[start..end].iter().enumerate() {
        if *byte == b'\n' {
            *number += 1;
            *line_start = base + (start + offset + 1) as u64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn hits(path: &Path, needle: &str, options: &Options) -> (Vec<Hit>, Stats) {
        let mut hits = Vec::new();
        let stats = search(path, needle, options, &|| false, &mut |hit| {
            hits.push(hit.clone());
            true
        })
        .unwrap();
        (hits, stats)
    }

    #[test]
    fn chunk_boundaries_unicode_crlf_and_overlapping_literal_semantics() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("sample.txt");
        let text = "αβ\r\n日本語needle\r\n終needle\nneedle";
        fs::write(&path, text).unwrap();
        for chunk_bytes in 1..=17 {
            let options = Options {
                chunk_bytes,
                ..Default::default()
            };
            let (found, _) = hits(&path, "needle", &options);
            assert_eq!(
                found
                    .iter()
                    .map(|hit| (hit.line_number, hit.line_start, hit.match_offset))
                    .collect::<Vec<_>>(),
                vec![(2, 6, 15), (3, 23, 26), (4, 33, 33)]
            );
            let (found, _) = hits(&path, "日本語", &options);
            assert_eq!(found[0].match_offset, 6);
            assert_eq!(found[0].line_start, 6);
        }
        fs::write(&path, "aaaaaaa").unwrap();
        for chunk_bytes in 1..=8 {
            let (found, _) = hits(
                &path,
                "aaa",
                &Options {
                    chunk_bytes,
                    ..Default::default()
                },
            );
            assert_eq!(
                found.iter().map(|hit| hit.match_offset).collect::<Vec<_>>(),
                [0, 3]
            );
        }
    }

    #[test]
    fn long_line_keeps_bounded_snippet_and_exact_line_start() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("long.txt");
        let mut bytes = vec![b'x'; 1024 * 1024];
        bytes.extend_from_slice(b"needle\n");
        fs::write(&path, bytes).unwrap();
        let (found, stats) = hits(
            &path,
            "needle",
            &Options {
                chunk_bytes: 257,
                snippet_bytes: 32,
                ..Default::default()
            },
        );
        assert_eq!(found[0].line_start, 0);
        assert_eq!(found[0].line_number, 1);
        assert_eq!(found[0].match_offset, 1024 * 1024);
        assert!(found[0].snippet.len() <= 32);
        assert!(found[0].truncated_before);
        assert_eq!(stats.hits, 1);
    }

    #[test]
    fn ignores_hidden_binary_and_symlink_paths_and_reports_late_nul() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join(".gitignore"), "ignored.txt\n").unwrap();
        fs::write(temp.path().join("ignored.txt"), "needle").unwrap();
        fs::write(temp.path().join(".hidden.txt"), "needle").unwrap();
        fs::write(temp.path().join("binary.bin"), b"needle\0").unwrap();
        fs::write(temp.path().join("visible.txt"), "needle").unwrap();
        let (found, stats) = hits(temp.path(), "needle", &Options::default());
        assert_eq!(found.len(), 1);
        assert!(found[0].path.ends_with("visible.txt"));
        assert_eq!(stats.binary_skipped, 1);
        assert_eq!(
            hits(
                temp.path(),
                "needle",
                &Options {
                    show_hidden: true,
                    ..Default::default()
                }
            )
            .0
            .len(),
            2
        );
        let late = temp.path().join("late.bin");
        fs::write(&late, b"needleXX\0").unwrap();
        let (found, stats) = hits(
            &late,
            "needle",
            &Options {
                chunk_bytes: 8,
                ..Default::default()
            },
        );
        assert_eq!(found.len(), 1);
        assert_eq!(stats.binary_stopped_after_prefix, 1);
        #[cfg(unix)]
        {
            let outside = tempfile::tempdir().unwrap();
            fs::write(outside.path().join("outside.txt"), "needle").unwrap();
            std::os::unix::fs::symlink(outside.path(), temp.path().join("link")).unwrap();
            assert!(!hits(temp.path(), "needle", &Options::default())
                .0
                .iter()
                .any(|hit| hit.path.ends_with("outside.txt")));
        }
    }

    #[test]
    fn cancellation_limits_and_changed_file_are_explicit() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("sample.txt");
        fs::write(&path, "needle\n".repeat(1000)).unwrap();
        let (_, capped) = hits(
            &path,
            "needle",
            &Options {
                max_hits: 2,
                ..Default::default()
            },
        );
        assert_eq!(capped.hits, 2);
        assert!(capped.hit_limit_reached);
        let mut count = 0;
        let cancelled = std::cell::Cell::new(false);
        let stats = search(
            &path,
            "needle",
            &Options::default(),
            &|| cancelled.get(),
            &mut |_| {
                count += 1;
                cancelled.set(true);
                true
            },
        )
        .unwrap();
        assert_eq!(count, 1);
        assert!(stats.cancelled);
        let (_, capped) = hits(
            &path,
            "missing",
            &Options {
                max_file_bytes: Some(10),
                ..Default::default()
            },
        );
        assert_eq!(capped.bytes_read, 10);
        assert!(capped.byte_limit_reached);
        let stats = search(&path, "needle", &Options::default(), &|| false, &mut |_| {
            fs::write(&path, "changed").unwrap();
            false
        })
        .unwrap();
        assert_eq!(stats.changed_files, 1);
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_paths_remain_exact() {
        use std::os::unix::ffi::OsStringExt;
        let temp = tempfile::tempdir().unwrap();
        let raw = std::ffi::OsString::from_vec(vec![b'x', 0xff]);
        let path = temp.path().join(&raw);
        if fs::write(&path, "needle").is_err() {
            return;
        }
        assert_eq!(
            hits(temp.path(), "needle", &Options::default()).0[0].path,
            path
        );
    }
}
