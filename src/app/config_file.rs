//! Loads configuration from `config.toml` in [`ConfigFile::config_dir`]

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ConfigFile {
    pub general: GeneralConfig,
    pub preview: PreviewConfig,
    pub performance: PerformanceConfig,
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PreviewConfig {
    /// Custom preview scripts: extension -> command
    /// The command can use $f for the file path
    pub custom: HashMap<String, String>,
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

/// The directory new config files go to, ignoring the legacy location.
fn preferred_config_dir(
    xdg_config_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    match xdg_config_home.filter(|p| p.is_absolute()) {
        Some(xdg) => Some(xdg.join("fileview")),
        None => home.map(|h| h.join(".config").join("fileview")),
    }
}

/// Pick the file to read for `relative`, one file at a time.
///
/// The check is per file rather than per directory, because `fv plugin init`
/// used to create `~/.config/fileview/plugins` on every platform, and a
/// directory check would then hide a config.toml kept in the old location.
fn resolve_config_file(
    xdg_config_home: Option<PathBuf>,
    home: Option<PathBuf>,
    platform_config: Option<PathBuf>,
    relative: &str,
) -> Option<PathBuf> {
    let preferred = preferred_config_dir(xdg_config_home, home).map(|d| d.join(relative));
    let legacy = platform_config.map(|p| p.join("fileview").join(relative));
    match (preferred, legacy) {
        (Some(preferred), Some(legacy)) if !preferred.exists() && legacy.exists() => Some(legacy),
        (Some(preferred), _) => Some(preferred),
        (None, legacy) => legacy,
    }
}

impl ConfigFile {
    /// Directory where new config files are created.
    ///
    /// `$XDG_CONFIG_HOME/fileview` when it is set, otherwise
    /// `~/.config/fileview`, which the docs promise on every platform.
    pub fn config_dir() -> Option<PathBuf> {
        preferred_config_dir(
            std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            dirs::home_dir(),
        )
    }

    /// The file to read for a path relative to the config directory.
    ///
    /// A file in [`Self::config_dir`] wins. When it is missing, the same file
    /// in the platform directory (Application Support on macOS, AppData on
    /// Windows) is used, because versions before 3.1.2 read only that one.
    pub fn config_file(relative: &str) -> Option<PathBuf> {
        resolve_config_file(
            std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            dirs::home_dir(),
            dirs::config_dir(),
            relative,
        )
    }

    pub fn config_path() -> Option<PathBuf> {
        Self::config_file("config.toml")
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
        let dir = preferred_config_dir(Some(xdg.clone()), Some(PathBuf::from("/home/u")));
        assert_eq!(dir, Some(xdg.join("fileview")));
    }

    #[test]
    fn relative_xdg_config_home_is_ignored() {
        let dir = preferred_config_dir(Some(PathBuf::from("rel")), Some(PathBuf::from("/home/u")));
        assert_eq!(dir, Some(PathBuf::from("/home/u/.config/fileview")));
    }

    #[test]
    fn each_file_prefers_dot_config_and_falls_back_to_the_platform_dir() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let platform = temp.path().join("platform");
        let resolve = |name: &str| {
            resolve_config_file(None, Some(home.clone()), Some(platform.clone()), name)
        };

        assert_eq!(
            resolve("config.toml"),
            Some(home.join(".config/fileview/config.toml"))
        );

        std::fs::create_dir_all(platform.join("fileview")).unwrap();
        std::fs::write(platform.join("fileview/config.toml"), "").unwrap();
        // A plugins folder made by an older `fv plugin init` must not hide it.
        std::fs::create_dir_all(home.join(".config/fileview/plugins")).unwrap();
        assert_eq!(
            resolve("config.toml"),
            Some(platform.join("fileview/config.toml"))
        );
        assert_eq!(
            resolve("theme.toml"),
            Some(home.join(".config/fileview/theme.toml"))
        );

        std::fs::write(home.join(".config/fileview/config.toml"), "").unwrap();
        assert_eq!(
            resolve("config.toml"),
            Some(home.join(".config/fileview/config.toml"))
        );
    }

    #[test]
    fn test_default_config() {
        let config = ConfigFile::default();
        assert!(!config.general.show_hidden);
        assert!(config.general.enable_icons);
        assert!(config.general.mouse_enabled);
        assert_eq!(config.performance.git_poll_interval_secs, 5);
    }

    #[test]
    fn test_parse_partial_config() {
        let toml_content = r#"
[general]
show_hidden = true

[performance]
git_poll_interval_secs = 9
"#;
        let config: ConfigFile = toml::from_str(toml_content).unwrap();
        assert!(config.general.show_hidden);
        assert!(config.general.enable_icons); // default
        assert!(config.general.mouse_enabled); // default
        assert_eq!(config.performance.git_poll_interval_secs, 9);
    }

    #[test]
    fn retired_keys_in_an_old_config_still_parse() {
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

[hooks]
on_create = "echo created"
"#;
        let config: ConfigFile = toml::from_str(toml_content).unwrap();
        assert!(config.general.show_hidden);
        assert!(!config.general.enable_icons);
        assert!(!config.general.mouse_enabled);
        assert_eq!(config.performance.git_poll_interval_secs, 10);
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
}
