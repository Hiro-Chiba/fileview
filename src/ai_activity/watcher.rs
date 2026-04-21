//! Watcher used by the interactive `fv` to consume activity events appended
//! by the MCP server process.
//!
//! The watcher owns a background thread that tails the session's
//! `activity.jsonl` file. `notify` wakes the thread when the file changes;
//! a periodic fallback poll (every 2 seconds) handles edge cases where the
//! OS-level watcher drops events. Parsed events are delivered through an
//! `mpsc` channel and drained by the main event loop each frame.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::ActivityEvent;

/// Watches an `activity.jsonl` and exposes a non-blocking drain API.
pub struct ActivityWatcher {
    receiver: Receiver<ActivityEvent>,
    _handle: Handle, // keeps watcher/reader alive until the field drops
}

struct Handle {
    _watcher: RecommendedWatcher,
    _reader: JoinHandle<()>,
}

impl ActivityWatcher {
    /// Start watching `activity_log`. Pre-existing content is NOT replayed
    /// (the cursor starts at EOF on construction).
    pub fn start(activity_log: PathBuf) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::channel::<ActivityEvent>();
        let (tick_tx, tick_rx) = mpsc::channel::<()>();

        // Position cursor at end-of-file so only future appends are delivered.
        let start_cursor = File::open(&activity_log)
            .and_then(|mut f| f.seek(SeekFrom::End(0)))
            .unwrap_or(0);

        let reader_path = activity_log.clone();
        let reader_tx = event_tx.clone();
        let reader = thread::spawn(move || {
            reader_loop(reader_path, start_cursor, tick_rx, reader_tx);
        });

        let notify_tx = tick_tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(ev) = res {
                if matches!(ev.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    let _ = notify_tx.send(());
                }
            }
        })
        .context("creating activity log watcher")?;

        let watch_parent = activity_log
            .parent()
            .ok_or_else(|| anyhow::anyhow!("activity log has no parent directory"))?;
        watcher
            .watch(watch_parent, RecursiveMode::NonRecursive)
            .with_context(|| format!("watching {}", watch_parent.display()))?;

        // Prime a tick so the reader syncs once on startup.
        let _ = tick_tx.send(());

        Ok(Self {
            receiver: event_rx,
            _handle: Handle {
                _watcher: watcher,
                _reader: reader,
            },
        })
    }

    /// Drain all events currently pending in the channel. Non-blocking.
    pub fn drain(&self) -> Vec<ActivityEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self.receiver.try_recv() {
            out.push(ev);
        }
        out
    }
}

fn reader_loop(
    activity_log: PathBuf,
    mut cursor: u64,
    tick_rx: Receiver<()>,
    event_tx: Sender<ActivityEvent>,
) {
    loop {
        match tick_rx.recv_timeout(Duration::from_secs(2)) {
            Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {
                // Proceed to drain new lines.
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }

        let mut file = match File::open(&activity_log) {
            Ok(f) => f,
            Err(_) => continue,
        };
        // Detect external truncation / recreation. We always advance cursor
        // past a '\n', so byte at cursor-1 must still be '\n' if the file is
        // the same one we last read. Anything else (file shorter than
        // cursor, or sentinel byte changed) means the content was rewritten
        // and we have to start over from byte 0; otherwise we silently sit
        // past the end of a fresh file and never see new events.
        if cursor > 0 {
            let still_same_file = file.seek(SeekFrom::Start(cursor - 1)).ok().and_then(|_| {
                let mut sentinel = [0u8; 1];
                file.read_exact(&mut sentinel).ok().map(|()| sentinel[0])
            }) == Some(b'\n');
            if !still_same_file {
                cursor = 0;
            }
        }
        let mut reader = BufReader::new(file);
        if reader.seek(SeekFrom::Start(cursor)).is_err() {
            continue;
        }
        let mut line = String::new();
        loop {
            line.clear();
            let read = reader.read_line(&mut line);
            match read {
                Ok(0) => break, // real EOF
                Ok(_n) if !line.ends_with('\n') => {
                    // Partial line — leave cursor where it was so we re-read
                    // this line once it is completed by the emitter.
                    break;
                }
                Ok(n) => {
                    cursor += n as u64;
                    let trimmed = line.trim_end();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if let Ok(event) = serde_json::from_str::<ActivityEvent>(trimmed) {
                        if event_tx.send(event).is_err() {
                            return; // receiver dropped
                        }
                    }
                    // Unparseable lines are skipped silently (forward-compat).
                }
                Err(_) => break,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::path::Path;
    use std::time::Instant;
    use tempfile::TempDir;

    fn wait_for_events(w: &ActivityWatcher, want: usize, timeout_ms: u64) -> Vec<ActivityEvent> {
        let start = Instant::now();
        let mut all = Vec::new();
        while all.len() < want && start.elapsed() < Duration::from_millis(timeout_ms) {
            all.extend(w.drain());
            thread::sleep(Duration::from_millis(50));
        }
        all
    }

    fn append(path: &Path, text: &str) {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        f.write_all(text.as_bytes()).unwrap();
        f.flush().unwrap();
    }

    #[test]
    fn receives_events_appended_after_start() {
        let tmp = TempDir::new().unwrap();
        let log = tmp.path().join("activity.jsonl");
        // Pre-create empty file so notify has something to watch.
        std::fs::write(&log, "").unwrap();

        let watcher = ActivityWatcher::start(log.clone()).unwrap();

        let ev = ActivityEvent::now("src", "read_file", Some(PathBuf::from("/x")));
        let line = serde_json::to_string(&ev).unwrap();
        append(&log, &format!("{}\n", line));

        let received = wait_for_events(&watcher, 1, 2_000);
        assert!(!received.is_empty(), "expected at least one event");
        assert_eq!(received[0].tool, "read_file");
    }

    #[test]
    fn ignores_history_before_start() {
        let tmp = TempDir::new().unwrap();
        let log = tmp.path().join("activity.jsonl");
        let old_ev = ActivityEvent::now("src", "old", None);
        std::fs::write(
            &log,
            format!("{}\n", serde_json::to_string(&old_ev).unwrap()),
        )
        .unwrap();

        let watcher = ActivityWatcher::start(log.clone()).unwrap();

        // Nothing new appended — we should not see the historical event.
        let received = wait_for_events(&watcher, 1, 500);
        assert!(
            received.is_empty(),
            "historical events should not be replayed, got {:?}",
            received
        );
    }

    #[test]
    fn recovers_after_external_truncation() {
        // Simulate a user wiping the activity log mid-session
        // (e.g., `> ~/.cache/fileview/sessions/$pid/activity.jsonl` to clean up).
        // After truncation the reader's cursor is past EOF, so without a
        // recovery mechanism every subsequent append is silently lost.
        let tmp = TempDir::new().unwrap();
        let log = tmp.path().join("activity.jsonl");
        std::fs::write(&log, "").unwrap();

        let watcher = ActivityWatcher::start(log.clone()).unwrap();

        // First event lands as expected.
        let first = ActivityEvent::now("src", "first", None);
        append(
            &log,
            &format!("{}\n", serde_json::to_string(&first).unwrap()),
        );
        let initial = wait_for_events(&watcher, 1, 2_000);
        assert_eq!(initial.len(), 1, "first event should arrive normally");

        // Externally truncate the file. The reader's cursor is now past EOF.
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&log)
            .unwrap();

        // A new event is appended starting from byte 0 of the freshly truncated file.
        let second = ActivityEvent::now("src", "after_truncation", None);
        append(
            &log,
            &format!("{}\n", serde_json::to_string(&second).unwrap()),
        );

        let received = wait_for_events(&watcher, 1, 3_000);
        assert_eq!(
            received.len(),
            1,
            "expected the post-truncation event to be delivered, got {:?}",
            received
        );
        assert_eq!(received[0].tool, "after_truncation");
    }

    #[test]
    fn handles_partial_lines() {
        let tmp = TempDir::new().unwrap();
        let log = tmp.path().join("activity.jsonl");
        std::fs::write(&log, "").unwrap();

        let watcher = ActivityWatcher::start(log.clone()).unwrap();
        let ev = ActivityEvent::now("src", "read_file", None);
        let line = serde_json::to_string(&ev).unwrap();

        // Write without trailing newline first.
        append(&log, &line);
        thread::sleep(Duration::from_millis(200));
        assert!(watcher.drain().is_empty(), "partial line must not deliver");

        // Complete the line.
        append(&log, "\n");
        let received = wait_for_events(&watcher, 1, 2_000);
        assert_eq!(received.len(), 1);
    }
}
