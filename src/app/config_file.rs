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
    pub commands: CommandsConfig,
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
    /// Image protocol: "auto", "sixel", "kitty", "iterm2", "halfblocks"
    pub image_protocol: String,
    /// Custom preview scripts: extension -> command
    /// The command can use $f for the file path
    pub custom: HashMap<String, String>,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
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

    /// Replace placeholders with parts of `file_path`.
    ///
    /// `$f` path, `$d` directory, `$n` file name, `$s` stem, `$e` extension.
    pub fn expand(template: &str, file_path: &std::path::Path) -> String {
        expand_placeholders(template, file_path, &[], false)
    }

    /// Like [`Self::expand`], but shell-escapes every substituted value, for
    /// templates that a shell will run.
    pub fn expand_shell_escaped(template: &str, file_path: &std::path::Path) -> String {
        expand_placeholders(template, file_path, &[], true)
    }

    /// Expand a `[commands]` template for execution. Values are shell-escaped
    /// and `$S` becomes the selected paths separated by spaces.
    pub fn expand_command(
        template: &str,
        file_path: &std::path::Path,
        selected: &[PathBuf],
    ) -> String {
        expand_placeholders(template, file_path, selected, true)
    }
}

/// Expand all placeholders in one pass, so a value that contains `$S` or
/// `$f` is never expanded again. A placeholder followed by a letter, digit, or
/// underscore is left alone, so `$SHELL` and `$filename` stay as written.
fn expand_placeholders(
    template: &str,
    file_path: &std::path::Path,
    selected: &[PathBuf],
    escape: bool,
) -> String {
    let quote = |value: String| if escape { shell_escape(&value) } else { value };
    let lossy = |s: Option<&std::ffi::OsStr>| s.map(|s| s.to_string_lossy().into_owned());

    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        let Some(&key) = chars.peek().filter(|_| c == '$') else {
            out.push(c);
            continue;
        };
        let mut lookahead = chars.clone();
        lookahead.next();
        let continues_word = lookahead
            .peek()
            .is_some_and(|n| n.is_alphanumeric() || *n == '_');
        let value = match key {
            _ if continues_word => None,
            'f' => Some(quote(file_path.display().to_string())),
            'd' => Some(quote(
                file_path
                    .parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            )),
            'n' => Some(quote(lossy(file_path.file_name()).unwrap_or_default())),
            's' => Some(quote(lossy(file_path.file_stem()).unwrap_or_default())),
            'e' => Some(quote(lossy(file_path.extension()).unwrap_or_default())),
            'S' => Some(
                selected
                    .iter()
                    .map(|p| quote(p.display().to_string()))
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            _ => None,
        };
        match value {
            Some(value) => {
                out.push_str(&value);
                chars.next();
            }
            None => out.push(c),
        }
    }
    out
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
        assert_eq!(config.preview.image_protocol, "auto");
        assert_eq!(config.performance.git_poll_interval_secs, 5);
    }

    #[test]
    fn test_parse_partial_config() {
        let toml_content = r#"
[general]
show_hidden = true

[preview]
image_protocol = "sixel"
"#;
        let config: ConfigFile = toml::from_str(toml_content).unwrap();
        assert!(config.general.show_hidden);
        assert!(config.general.enable_icons); // default
        assert_eq!(config.preview.image_protocol, "sixel");
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
        assert_eq!(config.preview.image_protocol, "kitty");
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

    #[test]
    fn placeholders_followed_by_a_word_character_are_left_alone() {
        let path = PathBuf::from("/p/a.rs");
        assert_eq!(
            CommandsConfig::expand("cd $d && $SHELL $filename $s.$e", &path),
            "cd /p && $SHELL $filename a.rs"
        );
    }

    #[test]
    #[cfg(unix)]
    fn selected_paths_replace_capital_s_once() {
        let path = PathBuf::from("/p/x$S.txt");
        let selected = [PathBuf::from("/p/a b"), PathBuf::from("/p/c")];
        assert_eq!(
            CommandsConfig::expand_command("tar cf out.tar $S; echo $f", &path, &selected),
            "tar cf out.tar '/p/a b' '/p/c'; echo '/p/x$S.txt'"
        );
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
