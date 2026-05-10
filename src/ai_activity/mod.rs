//! AI activity reflection.
//!
//! Bridges `fv --mcp-server` (called by an AI agent such as Claude Code) and
//! the interactive `fv` TUI so that the TUI can surface live AI activity.
//!
//! ## Architecture overview
//!
//! The MCP server and the interactive TUI run as independent OS processes.
//! They rendezvous through a file-based protocol in the user's cache
//! directory: each interactive TUI registers a `sessions/<pid>/` directory
//! containing `session.json` (metadata) and `activity.jsonl` (append-only
//! event log). On every tool dispatch, the MCP server appends an event to
//! each matching session's log. The interactive TUI uses `notify` to watch
//! its own log and surface events in the status bar, optionally following
//! the AI by auto-focusing its last accessed file.
//!
//! ## Key components
//!
//! - [`ActivityEvent`] — wire format for a single event.
//! - [`SessionRegistry`] / [`SessionInfo`] — discover and manage sessions.
//! - [`ActivityEmitter`] — MCP-server side; writes events.
//! - [`ActivityWatcher`] — TUI side; watches, parses, and delivers events.
//! - [`AiActivityState`] — in-memory UI state attached to `AppState`.

pub mod emitter;
pub mod event;
pub mod replay;
pub mod session;
pub mod watcher;

use std::collections::VecDeque;

pub use emitter::ActivityEmitter;
pub use event::ActivityEvent;
pub use replay::read_session_events;
pub use session::{SessionInfo, SessionMeta, SessionRegistry};
pub use watcher::ActivityWatcher;

/// How many recent events to retain in memory for the activity log view.
pub const MAX_RECENT_EVENTS: usize = 100;

/// How long (ms) the status-bar indicator keeps showing a single event before
/// fading to a passive state.
pub const STATUS_FRESH_WINDOW_MS: u64 = 5_000;

/// UI-facing AI activity state. Attached to `AppState`.
#[derive(Debug, Default)]
pub struct AiActivityState {
    /// When enabled, the TUI auto-focuses the AI's most recent file whenever
    /// it is safe to do so (see `can_follow` in the event loop).
    pub follow_mode: bool,
    /// The most recent event, used for the status-bar indicator.
    pub last_event: Option<ActivityEvent>,
    /// Bounded ring buffer of recent events for the activity log view.
    /// Newest events at the front.
    pub recent_events: VecDeque<ActivityEvent>,
    /// Cursor for the activity log view.
    pub log_selected: usize,
}

impl AiActivityState {
    /// Record a freshly observed event, bounding the ring buffer.
    ///
    /// When events arrive while a cursor is already set (e.g. the activity
    /// log popup is open), `log_selected` is compensated so that it continues
    /// to point at the same event after the new one is pushed to the front.
    /// This prevents the silently-shifting-selection bug where the cursor
    /// changes meaning without the user moving it.
    pub fn record(&mut self, event: ActivityEvent) {
        self.last_event = Some(event.clone());
        self.recent_events.push_front(event);
        let truncated = self.recent_events.len() > MAX_RECENT_EVENTS;
        if truncated {
            self.recent_events.truncate(MAX_RECENT_EVENTS);
        }
        // Shift the cursor by one to keep it on the same event. If the ring
        // buffer was capped and truncated, the shifted cursor is bounded to
        // the new length so it never points past the end.
        let max_idx = self.recent_events.len().saturating_sub(1);
        self.log_selected = (self.log_selected + 1).min(max_idx);
    }

    /// Toggle follow-mode, returning the new state.
    pub fn toggle_follow(&mut self) -> bool {
        self.follow_mode = !self.follow_mode;
        self.follow_mode
    }

    /// Most recent event if it happened within the given window (ms),
    /// otherwise `None` so the status bar can fall back to passive state.
    pub fn fresh_event(&self) -> Option<&ActivityEvent> {
        self.last_event
            .as_ref()
            .filter(|e| e.is_recent(STATUS_FRESH_WINDOW_MS))
    }

    /// Reset the log selection cursor when opening the view.
    pub fn reset_log_cursor(&mut self) {
        self.log_selected = 0;
    }

    /// Event currently under the log cursor, if any.
    pub fn selected_event(&self) -> Option<&ActivityEvent> {
        self.recent_events.get(self.log_selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn record_caps_ring_buffer() {
        let mut state = AiActivityState::default();
        for i in 0..(MAX_RECENT_EVENTS + 5) {
            state.record(ActivityEvent::now("x", format!("t{}", i), None));
        }
        assert_eq!(state.recent_events.len(), MAX_RECENT_EVENTS);
        assert_eq!(
            state.recent_events[0].tool,
            format!("t{}", MAX_RECENT_EVENTS + 4)
        );
    }

    #[test]
    fn toggle_follow_round_trip() {
        let mut state = AiActivityState::default();
        assert!(!state.follow_mode);
        assert!(state.toggle_follow());
        assert!(state.follow_mode);
        assert!(!state.toggle_follow());
    }

    #[test]
    fn selected_event_returns_entry() {
        let mut state = AiActivityState::default();
        state.record(ActivityEvent::now("x", "a", Some(PathBuf::from("/a"))));
        state.record(ActivityEvent::now("x", "b", Some(PathBuf::from("/b"))));
        state.log_selected = 1;
        assert_eq!(state.selected_event().unwrap().tool, "a");
    }
}
