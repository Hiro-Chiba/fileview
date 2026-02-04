//! Plugin command helpers (`fv plugin init|test`).

use std::fs;
use std::path::{Path, PathBuf};

use crate::plugin::PluginManager;

const DEFAULT_PLUGIN_TEMPLATE: &str = r#"-- FileView plugin template
fv.notify("FileView plugin loaded")

fv.on("start", function()
  fv.notify("Ready for AI-driven workflows")
end)
"#;

fn default_plugin_path() -> anyhow::Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("fileview")
        .join("plugins")
        .join("init.lua"))
}

/// Initialize plugin file with a starter template.
pub fn plugin_init(path: Option<&Path>) -> anyhow::Result<PathBuf> {
    let plugin_path = match path {
        Some(p) => p.to_path_buf(),
        None => default_plugin_path()?,
    };

    if let Some(parent) = plugin_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if !plugin_path.exists() {
        fs::write(&plugin_path, DEFAULT_PLUGIN_TEMPLATE)?;
    }

    Ok(plugin_path)
}

/// Execute a plugin file in sandboxed Lua runtime and return notifications.
pub fn plugin_test(path: &Path) -> anyhow::Result<Vec<String>> {
    let code = fs::read_to_string(path)?;
    let mut manager = PluginManager::new().map_err(|e| anyhow::anyhow!("{}", e))?;
    manager.exec(&code).map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(manager.take_notifications())
}
