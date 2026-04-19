//! End-to-end tests that demonstrate known bugs in the AI activity reflection
//! feature. Each test asserts the *correct* behaviour, so running any of them
//! today fails; the test becomes green when the bug is fixed.
//!
//! They are marked `#[ignore]` so CI stays green. Run them locally with:
//!
//! ```sh
//! cargo test --test ai_activity_known_bugs -- --ignored --nocapture
//! ```
//!
//! Each test's body links to the matching bullet in the PR #179 self-review.

use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use fileview::ai_activity::{
    ActivityEmitter, ActivityEvent, ActivityWatcher, AiActivityState, SessionRegistry,
};
use fileview::core::{AppState, ViewMode};
use fileview::mcp::server::extract_primary_paths;
use fileview::mcp::types::ToolCallParams;
use fileview::render::render_ai_activity_popup;
use ratatui::{backend::TestBackend, style::Color, Terminal};
use tempfile::TempDir;

// -------------------------------------------------------------------------
// Bug 1: popup cannot scroll; selection beyond visible window is never drawn.
// -------------------------------------------------------------------------

/// Repro:
/// 1. Fill the ring buffer with enough events to overflow the popup.
/// 2. Move the cursor past the popup's visible height.
/// 3. Render the popup.
/// 4. Expect the highlighted row (cyan background) to be visible in the buffer.
///
/// Current behaviour: `render_ai_activity_popup` does `take(max_items)` without
/// adjusting a scroll offset, so cells with `bg == Cyan` never appear when the
/// cursor is past `max_items`.
#[test]
fn bug_popup_scrolls_to_keep_selection_visible() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut state = AppState::new(PathBuf::from("/tmp"));
    state.mode = ViewMode::AiActivityLog;
    for i in 0..50 {
        state
            .ai_activity
            .record(ActivityEvent::now("test", format!("tool_{:03}", i), None));
    }
    // Cursor well past the popup's max visible rows.
    state.ai_activity.log_selected = 40;

    terminal
        .draw(|frame| render_ai_activity_popup(frame, &state))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let mut has_cyan_bg = false;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer[(x, y)].bg == Color::Cyan {
                has_cyan_bg = true;
                break;
            }
        }
        if has_cyan_bg {
            break;
        }
    }
    assert!(
        has_cyan_bg,
        "expected the selected row (cyan bg) to be visible when log_selected=40, \
         but no cell has Color::Cyan background — popup is truncating before the cursor"
    );
}

// -------------------------------------------------------------------------
// Bug 3: activity.jsonl is world-readable; other local users can see which
// files the AI touched. Should be 0600 on unix.
// -------------------------------------------------------------------------

/// Repro (unix only):
/// 1. Register a session in a temp dir.
/// 2. Inspect the file mode of `activity.jsonl`.
/// 3. Expect owner-only permissions (0o600).
///
/// Current behaviour: `OpenOptions::new().create(true).append(true).open(..)`
/// relies on umask; on most systems this ends up 0o644 or 0o664.
#[cfg(unix)]
#[test]
fn bug_activity_log_file_permissions_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().unwrap();
    let reg = SessionRegistry::at(tmp.path().to_path_buf()).unwrap();
    let session = reg.register_current(tmp.path()).unwrap();

    let meta = fs::metadata(&session.activity_log).unwrap();
    let mode = meta.permissions().mode() & 0o777;
    reg.unregister(&session);

    assert_eq!(
        mode, 0o600,
        "activity.jsonl should be created with owner-only permissions (0o600), got 0o{:o}",
        mode
    );
}

// -------------------------------------------------------------------------
// Bug 8: popup cursor silently points to a different event when new events
// arrive and shift the ring buffer (push_front semantics).
// -------------------------------------------------------------------------

/// Repro:
/// 1. Record events "a", "b", "c". Ring buffer (newest first): [c, b, a].
/// 2. User selects "a" — log_selected = 2.
/// 3. Record "d". Ring buffer is now [d, c, b, a].
/// 4. log_selected still 2, but `selected_event()` now returns "b", not "a".
///
/// The expected UX: after a new event arrives, the cursor should either
/// compensate (shift by one) or the popup should lock event identity somehow
/// while it is open, so the user's intended selection doesn't silently change.
///
/// Current behaviour: the cursor is a plain index into the ring buffer, and
/// new events push_front, so indexed identity is not stable.
#[test]
fn bug_popup_cursor_follows_intended_event_across_inserts() {
    let mut state = AiActivityState::default();
    state.record(ActivityEvent::now("test", "a", None));
    state.record(ActivityEvent::now("test", "b", None));
    state.record(ActivityEvent::now("test", "c", None));
    state.log_selected = 2;

    let initial = state
        .selected_event()
        .map(|e| e.tool.clone())
        .expect("selected event present");
    assert_eq!(initial, "a", "sanity: cursor should start on 'a'");

    // A new AI event arrives while the user is viewing the popup.
    state.record(ActivityEvent::now("test", "d", None));

    let after_shift = state
        .selected_event()
        .map(|e| e.tool.clone())
        .expect("selected event present");
    assert_eq!(
        after_shift, "a",
        "expected cursor to still point to 'a' after new event; got '{}' instead \
         (cursor shifted silently when ring buffer push_front'd a new event)",
        after_shift
    );
}

// -------------------------------------------------------------------------
// Bug 7: `read_files` takes N paths but the emitter surfaces only the first
// one. Users watching in the TUI miss the other paths in a batch read.
// -------------------------------------------------------------------------

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

/// Repro:
/// 1. Register a session and start a watcher.
/// 2. Ask `extract_primary_paths` (the real MCP dispatcher helper) for the
///    paths a `read_files` call would act on.
/// 3. Emit once per returned path, mirroring what the dispatcher does.
/// 4. Expect the watcher to observe 3 events — one per path.
///
/// Before the fix `extract_primary_paths` returned a single path (only the
/// first element of `paths`), so a multi-file read surfaced as a single UI
/// event. After the fix the helper returns every path and the dispatcher
/// loops `emit` over them.
#[test]
fn bug_read_files_emits_one_event_per_path() {
    let tmp = TempDir::new().unwrap();
    let registry_dir = tmp.path().join("sessions");
    let reg_tui = SessionRegistry::at(registry_dir.clone()).unwrap();
    let root = tmp.path().to_path_buf();
    let session = reg_tui.register_current(&root).unwrap();
    let watcher = ActivityWatcher::start(session.activity_log.clone()).unwrap();

    let reg_mcp = SessionRegistry::at(registry_dir).unwrap();
    let emitter = ActivityEmitter::with_registry(reg_mcp, "test");

    let paths = [root.join("a.rs"), root.join("b.rs"), root.join("c.rs")];
    for p in &paths {
        fs::write(p, "").unwrap();
    }

    // Mirror the dispatcher's behaviour: ask the real helper what paths
    // the tool acts on, then emit once per path.
    let params = ToolCallParams {
        name: "read_files".to_string(),
        arguments: serde_json::json!({
            "paths": [
                paths[0].to_string_lossy(),
                paths[1].to_string_lossy(),
                paths[2].to_string_lossy(),
            ]
        }),
    };
    let extracted = extract_primary_paths(&params, &root);
    assert_eq!(extracted.len(), 3, "extract should return all paths");
    for p in &extracted {
        emitter.emit(&params.name, Some(p));
    }

    let received = wait_for_events(&watcher, 3, Duration::from_secs(2));
    reg_tui.unregister(&session);

    assert_eq!(
        received.len(),
        3,
        "expected one event per path (3 total), got {}: {:?}",
        received.len(),
        received
    );
}
