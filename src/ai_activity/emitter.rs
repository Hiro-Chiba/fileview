//! Activity emitter used by `fv --mcp-server`.
//!
//! Appends an event line to each interactive session whose configured root
//! contains the event's path. Errors are swallowed on purpose: the MCP server
//! must never fail a tool call just because the UI side could not receive a
//! notification.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

use super::{ActivityEvent, SessionInfo, SessionRegistry};

/// Emits activity events to matching interactive sessions.
pub struct ActivityEmitter {
    registry: Option<SessionRegistry>,
    source: String,
}

impl ActivityEmitter {
    /// Create an emitter. Source identifier is taken from the environment
    /// variable `FILEVIEW_AI_SOURCE` if set, otherwise defaults to
    /// `"mcp-pid-<pid>"`.
    pub fn new() -> Self {
        let source = std::env::var("FILEVIEW_AI_SOURCE")
            .unwrap_or_else(|_| format!("mcp-pid-{}", process::id()));
        let registry = SessionRegistry::new().ok();
        Self { registry, source }
    }

    /// Build an emitter backed by an explicit registry. Primarily intended for
    /// tests that need to redirect IPC to a temp directory.
    pub fn with_registry(registry: SessionRegistry, source: impl Into<String>) -> Self {
        Self {
            registry: Some(registry),
            source: source.into(),
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Emit an event for the given tool call to every alive session whose
    /// root contains `path` (or every session, if `path` is `None`).
    pub fn emit(&self, tool: &str, path: Option<&Path>) {
        let Some(registry) = &self.registry else {
            return;
        };
        let event = ActivityEvent::now(
            self.source.clone(),
            tool.to_string(),
            path.map(Path::to_path_buf),
        );
        let target_abs = path.map(absolutize);
        for session in registry.list_alive() {
            if !session_matches(&session, target_abs.as_deref()) {
                continue;
            }
            let _ = append_event(&session.activity_log, &event);
        }
    }
}

impl Default for ActivityEmitter {
    fn default() -> Self {
        Self::new()
    }
}

fn session_matches(session: &SessionInfo, target_abs: Option<&Path>) -> bool {
    match target_abs {
        Some(p) => {
            let root_abs = absolutize(&session.meta.root);
            p.starts_with(&root_abs)
        }
        None => true,
    }
}

fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        let joined = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path);
        joined.canonicalize().unwrap_or(joined)
    }
}

fn append_event(activity_log: &Path, event: &ActivityEvent) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(activity_log)?;
    let line = serde_json::to_string(event)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    writeln!(file, "{}", line)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn emit_appends_only_when_path_is_inside_session_root() {
        let tmp = TempDir::new().unwrap();
        let reg = SessionRegistry::at(tmp.path().to_path_buf()).unwrap();
        let root = tmp.path().to_path_buf();
        let session = reg.register_current(&root).unwrap();

        let in_root = root.join("x.rs");
        fs::write(&in_root, "").unwrap();
        let outside = std::env::temp_dir().join("definitely-not-in-our-root.rs");

        let emitter = ActivityEmitter {
            registry: Some(SessionRegistry::at(tmp.path().to_path_buf()).unwrap()),
            source: "test".to_string(),
        };

        emitter.emit("read_file", Some(&in_root));
        emitter.emit("read_file", Some(&outside));

        let content = fs::read_to_string(&session.activity_log).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(
            lines.len(),
            1,
            "only the event inside the session root should be recorded (got: {:?})",
            lines
        );
        let event: ActivityEvent = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(event.tool, "read_file");
        assert_eq!(event.source, "test");

        reg.unregister(&session);
    }
}
