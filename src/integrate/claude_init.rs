//! Claude configuration bootstrap helpers (`fv init claude`).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

fn default_claude_config_path() -> anyhow::Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home).join(".claude.json"))
}

fn desired_fileview_entry(project_root: &Path) -> Value {
    json!({
        "command": "fv",
        "args": ["--mcp-server", project_root.display().to_string()]
    })
}

/// Initialize or update Claude Code config with a fileview MCP server entry.
///
/// Returns `(path, changed)`.
pub fn claude_init(
    project_root: &Path,
    config_path: Option<&Path>,
    force_overwrite: bool,
) -> anyhow::Result<(PathBuf, bool)> {
    let path = match config_path {
        Some(p) => p.to_path_buf(),
        None => default_claude_config_path()?,
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let desired = desired_fileview_entry(project_root);

    let mut root_value = if path.exists() {
        let text = fs::read_to_string(&path)?;
        serde_json::from_str::<Value>(&text)
            .map_err(|e| anyhow::anyhow!("failed to parse {}: {}", path.display(), e))?
    } else {
        json!({})
    };

    if !root_value.is_object() {
        return Err(anyhow::anyhow!(
            "expected top-level JSON object in {}",
            path.display()
        ));
    }

    let obj = root_value.as_object_mut().expect("checked object");
    let mcp_servers = obj
        .entry("mcpServers".to_string())
        .or_insert_with(|| Value::Object(Map::new()));

    if !mcp_servers.is_object() {
        return Err(anyhow::anyhow!(
            "'mcpServers' must be a JSON object in {}",
            path.display()
        ));
    }

    let mcp_obj = mcp_servers
        .as_object_mut()
        .expect("checked object for mcpServers");

    let changed = match mcp_obj.get("fileview") {
        None => {
            mcp_obj.insert("fileview".to_string(), desired);
            true
        }
        Some(current) if current == &desired => false,
        Some(_) if force_overwrite => {
            mcp_obj.insert("fileview".to_string(), desired);
            true
        }
        Some(_) => false,
    };

    if changed || !path.exists() {
        let formatted = serde_json::to_string_pretty(&root_value)?;
        fs::write(&path, formatted)?;
    }

    Ok((path, changed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_claude_init_creates_file() {
        let dir = tempdir().unwrap();
        let cfg = dir.path().join(".claude.json");
        let root = dir.path();

        let (path, changed) = claude_init(root, Some(&cfg), false).unwrap();
        assert_eq!(path, cfg);
        assert!(changed);

        let text = fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&text).unwrap();
        assert!(v["mcpServers"]["fileview"].is_object());
    }

    #[test]
    fn test_claude_init_is_idempotent() {
        let dir = tempdir().unwrap();
        let cfg = dir.path().join(".claude.json");
        let root = dir.path();

        let (_, changed1) = claude_init(root, Some(&cfg), false).unwrap();
        let (_, changed2) = claude_init(root, Some(&cfg), false).unwrap();
        assert!(changed1);
        assert!(!changed2);
    }
}
