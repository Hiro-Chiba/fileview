//! Read past `activity.jsonl` files for the replay UI.
//!
//! Activity logs are append-only JSON-Lines written by the MCP server. The
//! reader is forgiving: a partial trailing line, a corrupt JSON object, or
//! an unreadable file degrades to "skip and continue" rather than failing
//! the whole replay.

use std::path::Path;

use anyhow::Result;

use super::ActivityEvent;

/// Parse every well-formed JSON line in `activity_log` into an `ActivityEvent`.
///
/// Lines that are empty, partial, or fail to decode are silently skipped so
/// a crashed MCP server does not poison the entire replay.
pub fn read_session_events(activity_log: &Path) -> Result<Vec<ActivityEvent>> {
    if !activity_log.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(activity_log)?;
    let mut out = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(ev) = serde_json::from_str::<ActivityEvent>(trimmed) {
            out.push(ev);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn write_jsonl(dir: &Path, name: &str, lines: &[&str]) -> PathBuf {
        let path = dir.join(name);
        let body = lines.join("\n") + "\n";
        fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn returns_empty_for_missing_file() {
        let events = read_session_events(Path::new("/no/such/file")).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn skips_empty_and_corrupt_lines() {
        let dir = tempdir().unwrap();
        let path = write_jsonl(
            dir.path(),
            "activity.jsonl",
            &[
                "",
                r#"{"ts":1700000000000,"source":"claude","tool":"read_file","path":"/x/a.rs","summary":null}"#,
                "this is not json",
                r#"{"ts":1700000005000,"source":"claude-pid-1234","tool":"write_file","path":"/x/b.rs","summary":"ok"}"#,
                "   ",
            ],
        );
        let events = read_session_events(&path).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].tool, "read_file");
        assert_eq!(events[1].tool, "write_file");
    }

    #[test]
    fn handles_partial_trailing_line() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("activity.jsonl");
        // No newline after the partial JSON to simulate a crashed write.
        fs::write(
            &path,
            "{\"ts\":1700000000000,\"source\":\"claude\",\"tool\":\"read_file\",\"path\":null,\"summary\":null}\n{\"ts\":17000",
        )
        .unwrap();
        let events = read_session_events(&path).unwrap();
        assert_eq!(events.len(), 1);
    }
}
