//! Loads configuration from `config.toml` in [`ConfigFile::config_dir`]

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub use crate::handler::HooksConfig;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ConfigFile {
    pub general: GeneralConfig,
    pub preview: PreviewConfig,
    pub performance: PerformanceConfig,
    pub ui: UiConfig,
    pub commands: CommandsConfig,
    pub hooks: HooksConfig,
    pub budget: BudgetConfig,
}

/// Context budget bar settings (`[budget]` section in config.toml)
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct BudgetConfig {
    /// Default model used by the budget bar (e.g. "sonnet-4-6-200k").
    /// Empty string means "use the built-in default".
    pub default_model: String,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub show_hidden: bool,
    /// Enable Nerd Font icons
    pub enable_icons: bool,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PreviewConfig {
    pub hex_max_bytes: usize,
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

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct PerformanceConfig {
    pub git_poll_interval_secs: u64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            git_poll_interval_secs: 5,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub show_size: bool,
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CommandsConfig {
    /// Named commands: name -> command template
    /// Placeholders: $f (file path), $d (directory), $n (filename), $s (stem), $e (extension)
    #[serde(flatten)]
    pub commands: HashMap<String, String>,
}

impl CommandsConfig {
    pub fn get(&self, name: &str) -> Option<&String> {
        self.commands.get(name)
    }

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

fn resolve_config_dir(
    xdg_config_home: Option<PathBuf>,
    home: Option<PathBuf>,
    platform_config: Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(xdg) = xdg_config_home.filter(|p| p.is_absolute()) {
        return Some(xdg.join("fileview"));
    }
    let preferred = home.map(|h| h.join(".config").join("fileview"));
    let legacy = platform_config.map(|p| p.join("fileview"));
    match (preferred, legacy) {
        (Some(preferred), Some(legacy)) if !preferred.exists() && legacy.exists() => Some(legacy),
        (Some(preferred), _) => Some(preferred),
        (None, legacy) => legacy,
    }
}

impl ConfigFile {
    /// Directory that holds config.toml, keymap.toml, theme.toml, and plugins.
    ///
    /// `$XDG_CONFIG_HOME/fileview` wins when it is set, then
    /// `~/.config/fileview`, which the docs promise on every platform. The
    /// platform directory (Application Support on macOS, AppData on Windows)
    /// is still read when only it exists, because earlier versions used it.
    pub fn config_dir() -> Option<PathBuf> {
        resolve_config_dir(
            std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            dirs::home_dir(),
            dirs::config_dir(),
        )
    }

    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("config.toml"))
    }

    /// Returns default config if file doesn't exist or can't be parsed
    pub fn load() -> Self {
        Self::config_path()
            .and_then(|path| {
                if path.exists() {
                    crate::util::read_to_string_capped(&path, crate::util::MAX_STATE_FILE_BYTES)
                        .ok()
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
    fn xdg_config_home_wins_when_absolute() {
        // temp_dir is absolute on every platform, unlike "/xdg" on Windows.
        let xdg = std::env::temp_dir().join("xdg");
        let dir = resolve_config_dir(
            Some(xdg.clone()),
            Some(PathBuf::from("/home/u")),
            Some(PathBuf::from("/platform")),
        );
        assert_eq!(dir, Some(xdg.join("fileview")));
    }

    #[test]
    fn relative_xdg_config_home_is_ignored() {
        let dir = resolve_config_dir(
            Some(PathBuf::from("rel")),
            Some(PathBuf::from("/home/u")),
            None,
        );
        assert_eq!(dir, Some(PathBuf::from("/home/u/.config/fileview")));
    }

    #[test]
    fn dot_config_is_preferred_and_platform_dir_is_a_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let platform = temp.path().join("platform");

        let resolve = || resolve_config_dir(None, Some(home.clone()), Some(platform.clone()));
        assert_eq!(resolve(), Some(home.join(".config/fileview")));

        std::fs::create_dir_all(platform.join("fileview")).unwrap();
        assert_eq!(resolve(), Some(platform.join("fileview")));

        std::fs::create_dir_all(home.join(".config/fileview")).unwrap();
        assert_eq!(resolve(), Some(home.join(".config/fileview")));
    }

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
    #[cfg(unix)]
    fn test_expand_shell_escaped_unix_style() {
        let path = PathBuf::from("/tmp/it's tricky.txt");
        let expanded = CommandsConfig::expand_shell_escaped("echo $f", &path);
        assert!(expanded.contains("'"));
        assert!(expanded.contains("\\''"));
    }
}
