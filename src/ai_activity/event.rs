//! Activity event type and serialization.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// A single AI activity event, emitted by `fv --mcp-server` and consumed by
/// the interactive `fv` process that registered an interest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    /// Unix timestamp in milliseconds.
    pub ts: u64,
    /// Source identifier for the emitter (e.g. `"claude-pid-1234"`). Lets the
    /// UI distinguish between multiple concurrent AI sessions.
    pub source: String,
    /// MCP tool name (e.g. `"read_file"`, `"get_file_symbols"`).
    pub tool: String,
    /// Primary path the tool acted on, if any.
    pub path: Option<PathBuf>,
    /// Optional short human-readable summary.
    pub summary: Option<String>,
}

impl ActivityEvent {
    /// Build an event with a `now()` timestamp.
    pub fn now(source: impl Into<String>, tool: impl Into<String>, path: Option<PathBuf>) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            ts,
            source: source.into(),
            tool: tool.into(),
            path,
            summary: None,
        }
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// File name only of `path`, or a placeholder if the event has no path.
    pub fn short_path(&self, root: Option<&Path>) -> String {
        let Some(p) = &self.path else {
            return "-".to_string();
        };
        if let Some(root) = root {
            if let Ok(rel) = p.strip_prefix(root) {
                let s = rel.display().to_string();
                if !s.is_empty() {
                    return s;
                }
            }
        }
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| p.display().to_string())
    }

    /// Whether this event happened within the last `window_ms` milliseconds.
    pub fn is_recent(&self, window_ms: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        now.saturating_sub(self.ts) <= window_ms
    }

    /// Short display label for the source, e.g. `"claude"` from `"claude-pid-1234"`.
    pub fn short_source(&self) -> &str {
        self.source
            .split('-')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_json() {
        let event = ActivityEvent::now("claude", "read_file", Some(PathBuf::from("/a/b.rs")))
            .with_summary("hi");
        let json = serde_json::to_string(&event).unwrap();
        let decoded: ActivityEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.tool, "read_file");
        assert_eq!(decoded.source, "claude");
        assert_eq!(decoded.path, Some(PathBuf::from("/a/b.rs")));
        assert_eq!(decoded.summary.as_deref(), Some("hi"));
    }

    #[test]
    fn short_path_relativizes_when_possible() {
        let event = ActivityEvent::now("x", "read_file", Some(PathBuf::from("/root/a/b.rs")));
        assert_eq!(event.short_path(Some(Path::new("/root"))), "a/b.rs");
        assert_eq!(event.short_path(None), "b.rs");
    }

    #[test]
    fn short_source_strips_pid_suffix() {
        let e = ActivityEvent::now("claude-pid-1234", "x", None);
        assert_eq!(e.short_source(), "claude");
    }

    #[test]
    fn is_recent_within_window() {
        let e = ActivityEvent::now("x", "read_file", None);
        assert!(e.is_recent(60_000));
    }
}
