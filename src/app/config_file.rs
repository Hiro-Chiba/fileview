//! Configuration file loading and parsing
//!
//! Loads configuration from `~/.config/fileview/config.toml`

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub use crate::handler::HooksConfig;

/// Main configuration file structure
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ConfigFile {
    /// General settings
    pub general: GeneralConfig,
    /// Preview settings
    pub preview: PreviewConfig,
    /// Performance settings
    pub performance: PerformanceConfig,
    /// UI display settings
    pub ui: UiConfig,
    /// Custom commands
    pub commands: CommandsConfig,
    /// Event hooks
    pub hooks: HooksConfig,
}

/// General application settings
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Show hidden files by default
    pub show_hidden: bool,
    /// Enable Nerd Font icons
    pub enable_icons: bool,
    /// Enable mouse support
    pub mouse_enabled: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            enable_icons: true,
            mouse_enabled: true,
        }
    }
}

/// Preview-related settings
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PreviewConfig {
    /// Maximum bytes to show in hex preview
    pub hex_max_bytes: usize,
    /// Maximum entries to show in archive preview
    pub max_archive_entries: usize,
    /// Image protocol: "auto", "sixel", "kitty", "iterm2", "halfblocks"
    pub image_protocol: String,
    /// Custom preview scripts: extension -> command
    /// The command can use $f for the file path
    pub custom: HashMap<String, String>,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            hex_max_bytes: 4096,
            max_archive_entries: 500,
            image_protocol: "auto".to_string(),
            custom: HashMap::new(),
        }
    }
}

/// Performance-related settings
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct PerformanceConfig {
    /// Git status polling interval in seconds
    pub git_poll_interval_secs: u64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            git_poll_interval_secs: 5,
        }
    }
}

/// UI display settings
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Show file sizes in tree view
    pub show_size: bool,
    /// Show file permissions in tree view
    pub show_permissions: bool,
    /// Date format string (strftime-style)
    pub date_format: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_size: true,
            show_permissions: false,
            date_format: "%Y-%m-%d %H:%M".to_string(),
        }
    }
}

/// Custom commands configuration
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CommandsConfig {
    /// Named commands: name -> command template
    /// Placeholders: $f (file path), $d (directory), $n (filename), $s (stem), $e (extension)
    #[serde(flatten)]
    pub commands: HashMap<String, String>,
}

impl CommandsConfig {
    /// Get a command by name
    pub fn get(&self, name: &str) -> Option<&String> {
        self.commands.get(name)
    }

    /// Expand placeholders in a command template
    pub fn expand(template: &str, file_path: &std::path::Path) -> String {
        let path_str = file_path.display().to_string();
        let dir = file_path
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let stem = file_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = file_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        template
            .replace("$f", &path_str)
            .replace("$d", &dir)
            .replace("$n", &name)
            .replace("$s", &stem)
            .replace("$e", &ext)
    }

    /// Expand placeholders and shell-escape substituted values.
    ///
    /// This is intended for command execution paths where placeholder content
    /// can include spaces or shell metacharacters.
    pub fn expand_shell_escaped(template: &str, file_path: &std::path::Path) -> String {
        let path_str = file_path.display().to_string();
        let dir = file_path
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let stem = file_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = file_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        template
            .replace("$f", &shell_escape(&path_str))
            .replace("$d", &shell_escape(&dir))
            .replace("$n", &shell_escape(&name))
            .replace("$s", &shell_escape(&stem))
            .replace("$e", &shell_escape(&ext))
    }
}

fn shell_escape(value: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else if value.contains('\'') {
        format!("'{}'", value.replace('\'', "'\\''"))
    } else {
        format!("'{}'", value)
    }
}

impl ConfigFile {
    /// Get the config directory path (~/.config/fileview)
    pub fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("fileview"))
    }

    /// Get the config file path (~/.config/fileview/config.toml)
    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("config.toml"))
    }

    /// Load configuration from file
    ///
    /// Returns default config if file doesn't exist or can't be parsed
    pub fn load() -> Self {
        Self::config_path()
            .and_then(|path| {
                if path.exists() {
                    fs::read_to_string(&path).ok()
                } else {
                    None
                }
            })
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Load configuration from a specific path (for testing)
    pub fn load_from(path: &PathBuf) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: ConfigFile = toml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = ConfigFile::default();
        assert!(!config.general.show_hidden);
        assert!(config.general.enable_icons);
        assert!(config.general.mouse_enabled);
        assert_eq!(config.preview.hex_max_bytes, 4096);
        assert_eq!(config.preview.max_archive_entries, 500);
        assert_eq!(config.preview.image_protocol, "auto");
        assert_eq!(config.performance.git_poll_interval_secs, 5);
        assert!(config.ui.show_size);
        assert!(!config.ui.show_permissions);
        assert_eq!(config.ui.date_format, "%Y-%m-%d %H:%M");
    }

    #[test]
    fn test_parse_partial_config() {
        let toml_content = r#"
[general]
show_hidden = true

[preview]
hex_max_bytes = 8192
"#;
        let config: ConfigFile = toml::from_str(toml_content).unwrap();
        assert!(config.general.show_hidden);
        assert!(config.general.enable_icons); // default
        assert_eq!(config.preview.hex_max_bytes, 8192);
        assert_eq!(config.preview.max_archive_entries, 500); // default
    }

    #[test]
    fn test_parse_full_config() {
        let toml_content = r#"
[general]
show_hidden = true
enable_icons = false
mouse_enabled = false

[preview]
hex_max_bytes = 8192
max_archive_entries = 1000
image_protocol = "kitty"

[performance]
git_poll_interval_secs = 10

[ui]
show_size = false
show_permissions = true
date_format = "%d/%m/%Y"
"#;
        let config: ConfigFile = toml::from_str(toml_content).unwrap();
        assert!(config.general.show_hidden);
        assert!(!config.general.enable_icons);
        assert!(!config.general.mouse_enabled);
        assert_eq!(config.preview.hex_max_bytes, 8192);
        assert_eq!(config.preview.max_archive_entries, 1000);
        assert_eq!(config.preview.image_protocol, "kitty");
        assert_eq!(config.performance.git_poll_interval_secs, 10);
        assert!(!config.ui.show_size);
        assert!(config.ui.show_permissions);
        assert_eq!(config.ui.date_format, "%d/%m/%Y");
    }

    #[test]
    fn test_load_from_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
[general]
show_hidden = true
"#
        )
        .unwrap();

        let config = ConfigFile::load_from(&file.path().to_path_buf()).unwrap();
        assert!(config.general.show_hidden);
    }

    #[test]
    fn test_invalid_toml_returns_error() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "invalid toml {{{{").unwrap();

        let result = ConfigFile::load_from(&file.path().to_path_buf());
        assert!(result.is_err());
    }

    #[test]
    fn test_expand_shell_escaped_unix_style() {
        let path = PathBuf::from("/tmp/it's tricky.txt");
        let expanded = CommandsConfig::expand_shell_escaped("echo $f", &path);
        assert!(expanded.contains("'"));
        assert!(expanded.contains("\\''"));
    }
}
