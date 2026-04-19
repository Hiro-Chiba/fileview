//! End-to-end round-trip between `ActivityEmitter` and `ActivityWatcher`.
//!
//! Simulates the MCP server (emitter) and interactive TUI (watcher) living in
//! the same process, but each using the same file-based rendezvous directory.
//! Closer to runtime behaviour than module-level unit tests.

use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use fileview::ai_activity::{ActivityEmitter, ActivityEvent, ActivityWatcher, SessionRegistry};
use tempfile::TempDir;

fn wait_for_events(
    watcher: &ActivityWatcher,
    at_least: usize,
    timeout: Duration,
) -> Vec<ActivityEvent> {
    let start = Instant::now();
    let mut out = Vec::new();
    while out.len() < at_least && start.elapsed() < timeout {
        out.extend(watcher.drain());
        thread::sleep(Duration::from_millis(50));
    }
    out
}

fn canonical_if_exists(p: &std::path::Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

#[test]
fn emitter_delivers_only_in_root_events_to_watcher() {
    let tmp = TempDir::new().unwrap();
    let registry_dir = tmp.path().join("sessions");

    let reg_tui = SessionRegistry::at(registry_dir.clone()).unwrap();
    let root = tmp.path().to_path_buf();
    let session = reg_tui.register_current(&root).unwrap();
    let watcher = ActivityWatcher::start(session.activity_log.clone()).unwrap();

    let reg_mcp = SessionRegistry::at(registry_dir).unwrap();
    let emitter = ActivityEmitter::with_registry(reg_mcp, "test");

    let in_root = root.join("src/auth.rs");
    std::fs::create_dir_all(in_root.parent().unwrap()).unwrap();
    std::fs::write(&in_root, "").unwrap();

    emitter.emit("read_file", Some(&in_root));
    let outside = std::env::temp_dir().join("definitely-outside-our-root.rs");
    emitter.emit("read_file", Some(&outside));

    let received = wait_for_events(&watcher, 1, Duration::from_secs(3));
    assert_eq!(received.len(), 1, "received: {:?}", received);
    assert_eq!(received[0].tool, "read_file");
    assert_eq!(received[0].source, "test");
    // Emitter preserves the original path (canonicalization happens only for
    // match-against-session-root, not for the stored event path).
    let got = received[0].path.as_ref().expect("path present");
    assert_eq!(canonical_if_exists(got), canonical_if_exists(&in_root));

    reg_tui.unregister(&session);
    assert!(!session.dir.exists());
}
