//! Block-until-changed file watcher for AI workflows.
//!
//! Lets a script (or an AI agent via shell) wait for one or more files to be
//! modified externally, then return the changed paths. Used to detect stale
//! reads in long-running sessions: the AI takes a snapshot, runs analysis,
//! then asks `fv --watch <file> --timeout-secs 1` right before applying a
//! patch. If the file moved underneath, the watch returns and the agent
//! refreshes its read instead of overwriting fresh edits.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;

use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};

/// Outcome of a watch call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchOutcome {
    /// At least one watched path changed before the timeout.
    Changed(Vec<PathBuf>),
    /// Timeout elapsed with no change observed.
    Timeout,
}

/// Block until any of `paths` is modified, or until `timeout` elapses.
///
/// `None` timeout means wait forever. The debouncer collapses bursts of
/// filesystem events so a single editor write does not produce multiple
/// notifications.
pub fn watch_until_change(
    paths: &[PathBuf],
    timeout: Option<Duration>,
) -> io::Result<WatchOutcome> {
    if paths.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "watch requires at least one path",
        ));
    }
    for p in paths {
        if !p.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("watch target not found: {}", p.display()),
            ));
        }
    }

    let (tx, rx) = channel();
    let mut debouncer = new_debouncer(Duration::from_millis(100), move |res| {
        let _ = tx.send(res);
    })
    .map_err(io::Error::other)?;

    for p in paths {
        let mode = if p.is_dir() {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        debouncer
            .watcher()
            .watch(p, mode)
            .map_err(io::Error::other)?;
    }

    // Drain any events delivered for activity that pre-dates the watch
    // (notably macOS FSEvents replays of the file's own creation moments
    // before this call). 250 ms is comfortably larger than the debouncer's
    // 100 ms window and stays well under any practical user timeout.
    let drain_until = std::time::Instant::now() + Duration::from_millis(250);
    while let Some(remaining) = drain_until.checked_duration_since(std::time::Instant::now()) {
        match rx.recv_timeout(remaining) {
            Ok(_) => continue,
            Err(_) => break,
        }
    }

    let recv_result = match timeout {
        Some(d) => rx.recv_timeout(d),
        None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
    };

    match recv_result {
        Ok(Ok(events)) => {
            let mut changed: Vec<PathBuf> = events.into_iter().map(|e| e.path).collect();
            changed.sort();
            changed.dedup();
            Ok(WatchOutcome::Changed(changed))
        }
        Ok(Err(err)) => Err(io::Error::other(err.to_string())),
        Err(RecvTimeoutError::Timeout) => Ok(WatchOutcome::Timeout),
        Err(RecvTimeoutError::Disconnected) => Err(io::Error::other("watcher channel closed")),
    }
}

/// Convenience wrapper for the common single-path case.
pub fn watch_one(path: &Path, timeout: Option<Duration>) -> io::Result<WatchOutcome> {
    watch_until_change(&[path.to_path_buf()], timeout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::mpsc;
    use std::thread;
    use tempfile::TempDir;

    #[test]
    fn timeout_with_no_change_returns_timeout() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("idle.txt");
        fs::write(&path, "initial\n").unwrap();

        // Past the 250 ms drain window with margin so the timeout itself,
        // not setup latency, is what the test is exercising.
        let outcome = watch_one(&path, Some(Duration::from_millis(800))).unwrap();
        assert_eq!(outcome, WatchOutcome::Timeout);
    }

    #[test]
    fn modification_during_watch_returns_changed() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("active.txt");
        fs::write(&path, "initial\n").unwrap();

        let writer_path = path.clone();
        let (ready_tx, ready_rx) = mpsc::channel::<()>();
        let writer = thread::spawn(move || {
            // Wait until the main thread signals the watcher is armed,
            // then repeatedly write so the watcher has multiple chances
            // to observe an event even if backend setup is slow.
            let _ = ready_rx.recv();
            for i in 0..20 {
                thread::sleep(Duration::from_millis(150));
                let _ = fs::write(&writer_path, format!("write {}\n", i));
            }
        });

        ready_tx.send(()).unwrap();
        let outcome = watch_one(&path, Some(Duration::from_secs(10))).unwrap();
        drop(writer); // detach; writer will finish on its own

        match outcome {
            WatchOutcome::Changed(paths) => {
                assert!(!paths.is_empty(), "expected at least one changed path");
            }
            WatchOutcome::Timeout => panic!("expected Changed, got Timeout"),
        }
    }

    #[test]
    fn empty_paths_is_rejected() {
        let err = watch_until_change(&[], Some(Duration::from_millis(100))).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn missing_path_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("nope.txt");
        let err = watch_one(&missing, Some(Duration::from_millis(100))).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
