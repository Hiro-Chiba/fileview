//! Theme configuration and color management
//!
//! Loads theme from `~/.config/fileview/theme.toml`

use ratatui::style::Color;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::app::ConfigFile;

/// Global theme instance
static THEME: OnceLock<Theme> = OnceLock::new();

/// Get the global theme instance
pub fn theme() -> &'static Theme {
    THEME.get_or_init(Theme::load)
}

/// Theme configuration file structure
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ThemeFile {
    /// Base colors
    pub colors: BaseColors,
    /// File type colors
    pub file_colors: FileColors,
    /// Git status colors
    pub git_colors: GitColors,
}

/// Base UI colors
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct BaseColors {
    /// Background color
    pub background: String,
    /// Foreground (text) color
    pub foreground: String,
    /// Selection/focus background color
    pub selection: String,
    /// Border color
    pub border: String,
    /// Active border color (focused panel)
    pub border_active: String,
    /// Status bar background
    pub status_bg: String,
    /// Status bar foreground
    pub status_fg: String,
    /// Error message color
    pub error: String,
    /// Warning message color
    pub warning: String,
    /// Info message color
    pub info: String,
}

impl Default for BaseColors {
    fn default() -> Self {
        Self {
            background: "default".to_string(),
            foreground: "white".to_string(),
            selection: "darkgray".to_string(),
            border: "default".to_string(),
            border_active: "cyan".to_string(),
            status_bg: "default".to_string(),
            status_fg: "white".to_string(),
            error: "red".to_string(),
            warning: "yellow".to_string(),
            info: "blue".to_string(),
        }
    }
}

/// File type colors
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct FileColors {
    /// Directory color
    pub directory: String,
    /// Executable file color
    pub executable: String,
    /// Symbolic link color
    pub symlink: String,
    /// Archive file color
    pub archive: String,
    /// Image file color
    pub image: String,
    /// Video file color
    pub video: String,
    /// Audio file color
    pub audio: String,
    /// Document file color
    pub document: String,
    /// Source code file color
    pub source: String,
    /// Markup file color (md, html, etc.)
    pub markup: String,
}

impl Default for FileColors {
    fn default() -> Self {
        Self {
            directory: "blue".to_string(),
            executable: "green".to_string(),
            symlink: "cyan".to_string(),
            archive: "red".to_string(),
            image: "magenta".to_string(),
            video: "magenta".to_string(),
            audio: "magenta".to_string(),
            document: "yellow".to_string(),
            source: "green".to_string(),
            markup: "yellow".to_string(),
        }
    }
}

/// Git status colors
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct GitColors {
    /// Modified file color
    pub modified: String,
    /// Staged file color
    pub staged: String,
    /// Added/untracked file color
    pub untracked: String,
    /// Deleted file color
    pub deleted: String,
    /// Renamed file color
    pub renamed: String,
    /// Conflicted file color
    pub conflict: String,
    /// Ignored file color
    pub ignored: String,
}

impl Default for GitColors {
    fn default() -> Self {
        Self {
            modified: "yellow".to_string(),
            staged: "green".to_string(),
            untracked: "green".to_string(),
            deleted: "red".to_string(),
            renamed: "cyan".to_string(),
            conflict: "magenta".to_string(),
            ignored: "darkgray".to_string(),
        }
    }
}

impl ThemeFile {
    /// Get the theme file path (~/.config/fileview/theme.toml)
    pub fn theme_path() -> Option<PathBuf> {
        ConfigFile::config_dir().map(|p| p.join("theme.toml"))
    }

    /// Load theme from file
    pub fn load() -> Self {
        Self::theme_path()
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
}

/// Parsed theme with ratatui Color values
#[derive(Debug)]
pub struct Theme {
    // Base colors
    pub background: Color,
    pub foreground: Color,
    pub selection: Color,
    pub border: Color,
    pub border_active: Color,
    pub status_bg: Color,
    pub status_fg: Color,
    pub error: Color,
    pub warning: Color,
    pub info: Color,

    // File type colors
    pub directory: Color,
    pub executable: Color,
    pub symlink: Color,
    pub archive: Color,
    pub image: Color,
    pub video: Color,
    pub audio: Color,
    pub document: Color,
    pub source: Color,
    pub markup: Color,

    // Git colors
    pub git_modified: Color,
    pub git_staged: Color,
    pub git_untracked: Color,
    pub git_deleted: Color,
    pub git_renamed: Color,
    pub git_conflict: Color,
    pub git_ignored: Color,

    // Cached mark color
    pub mark: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_file(&ThemeFile::default())
    }
}

impl Theme {
    /// Load theme from config file
    pub fn load() -> Self {
        let file = ThemeFile::load();
        Self::from_file(&file)
    }

    /// Create theme from ThemeFile
    fn from_file(file: &ThemeFile) -> Self {
        Self {
            // Base colors
            background: parse_color(&file.colors.background),
            foreground: parse_color(&file.colors.foreground),
            selection: parse_color(&file.colors.selection),
            border: parse_color(&file.colors.border),
            border_active: parse_color(&file.colors.border_active),
            status_bg: parse_color(&file.colors.status_bg),
            status_fg: parse_color(&file.colors.status_fg),
            error: parse_color(&file.colors.error),
            warning: parse_color(&file.colors.warning),
            info: parse_color(&file.colors.info),

            // File type colors
            directory: parse_color(&file.file_colors.directory),
            executable: parse_color(&file.file_colors.executable),
            symlink: parse_color(&file.file_colors.symlink),
            archive: parse_color(&file.file_colors.archive),
            image: parse_color(&file.file_colors.image),
            video: parse_color(&file.file_colors.video),
            audio: parse_color(&file.file_colors.audio),
            document: parse_color(&file.file_colors.document),
            source: parse_color(&file.file_colors.source),
            markup: parse_color(&file.file_colors.markup),

            // Git colors
            git_modified: parse_color(&file.git_colors.modified),
            git_staged: parse_color(&file.git_colors.staged),
            git_untracked: parse_color(&file.git_colors.untracked),
            git_deleted: parse_color(&file.git_colors.deleted),
            git_renamed: parse_color(&file.git_colors.renamed),
            git_conflict: parse_color(&file.git_colors.conflict),
            git_ignored: parse_color(&file.git_colors.ignored),

            // Cached colors
            mark: Color::Yellow,
        }
    }
}

/// Parse color string to ratatui Color
///
/// Supported formats:
/// - Named colors: "red", "blue", "green", etc.
/// - Hex colors: "#ff0000", "#f00"
/// - RGB: "rgb(255, 0, 0)"
/// - 256 colors: "color123" or "123"
pub fn parse_color(s: &str) -> Color {
    let s = s.trim().to_lowercase();

    // Handle "default" or "reset"
    if s == "default" || s == "reset" || s.is_empty() {
        return Color::Reset;
    }

    // Handle hex colors
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    // Handle rgb(r, g, b)
    if let Some(rgb) = s.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        return parse_rgb_color(rgb);
    }

    // Handle "colorN" or just "N" for 256 colors
    if let Some(n) = s.strip_prefix("color") {
        if let Ok(n) = n.parse::<u8>() {
            return Color::Indexed(n);
        }
    }
    if let Ok(n) = s.parse::<u8>() {
        return Color::Indexed(n);
    }

    // Named colors
    match s.as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" | "purple" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        _ => Color::Reset,
    }
}

/// Parse hex color (#rgb or #rrggbb)
fn parse_hex_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');

    match hex.len() {
        3 => {
            // #rgb -> #rrggbb
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).unwrap_or(0);
            Color::Rgb(r, g, b)
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            Color::Rgb(r, g, b)
        }
        _ => Color::Reset,
    }
}

/// Parse rgb(r, g, b) color
fn parse_rgb_color(rgb: &str) -> Color {
    let parts: Vec<&str> = rgb.split(',').map(|s| s.trim()).collect();
    if parts.len() == 3 {
        let r = parts[0].parse::<u8>().unwrap_or(0);
        let g = parts[1].parse::<u8>().unwrap_or(0);
        let b = parts[2].parse::<u8>().unwrap_or(0);
        Color::Rgb(r, g, b)
    } else {
        Color::Reset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_named_color() {
        assert_eq!(parse_color("red"), Color::Red);
        assert_eq!(parse_color("Blue"), Color::Blue);
        assert_eq!(parse_color("GREEN"), Color::Green);
        assert_eq!(parse_color("darkgray"), Color::DarkGray);
        assert_eq!(parse_color("magenta"), Color::Magenta);
        assert_eq!(parse_color("purple"), Color::Magenta);
    }

    #[test]
    fn test_parse_default_color() {
        assert_eq!(parse_color("default"), Color::Reset);
        assert_eq!(parse_color("reset"), Color::Reset);
        assert_eq!(parse_color(""), Color::Reset);
    }

    #[test]
    fn test_parse_hex_color_short() {
        assert_eq!(parse_color("#f00"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("#0f0"), Color::Rgb(0, 255, 0));
        assert_eq!(parse_color("#00f"), Color::Rgb(0, 0, 255));
    }

    #[test]
    fn test_parse_hex_color_long() {
        assert_eq!(parse_color("#ff0000"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("#00ff00"), Color::Rgb(0, 255, 0));
        assert_eq!(parse_color("#0000ff"), Color::Rgb(0, 0, 255));
        assert_eq!(parse_color("#808080"), Color::Rgb(128, 128, 128));
    }

    #[test]
    fn test_parse_rgb_color() {
        assert_eq!(parse_color("rgb(255, 0, 0)"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("rgb(0, 255, 0)"), Color::Rgb(0, 255, 0));
        assert_eq!(parse_color("rgb(128, 128, 128)"), Color::Rgb(128, 128, 128));
    }

    #[test]
    fn test_parse_indexed_color() {
        assert_eq!(parse_color("color196"), Color::Indexed(196));
        assert_eq!(parse_color("123"), Color::Indexed(123));
        assert_eq!(parse_color("0"), Color::Indexed(0));
    }

    #[test]
    fn test_parse_unknown_color() {
        assert_eq!(parse_color("invalid"), Color::Reset);
        assert_eq!(parse_color("foobar"), Color::Reset);
    }

    #[test]
    fn test_theme_file_parse() {
        let toml_content = r##"
[colors]
background = "#1e1e1e"
foreground = "#d4d4d4"
selection = "blue"

[file_colors]
directory = "#569cd6"

[git_colors]
modified = "yellow"
"##;
        let theme: ThemeFile = toml::from_str(toml_content).unwrap();
        assert_eq!(theme.colors.background, "#1e1e1e");
        assert_eq!(theme.colors.foreground, "#d4d4d4");
        assert_eq!(theme.colors.selection, "blue");
        assert_eq!(theme.file_colors.directory, "#569cd6");
        assert_eq!(theme.git_colors.modified, "yellow");
    }

    #[test]
    fn test_default_theme() {
        let theme = Theme::default();
        assert_eq!(theme.directory, Color::Blue);
        assert_eq!(theme.git_modified, Color::Yellow);
        assert_eq!(theme.git_staged, Color::Green);
    }

    #[test]
    fn test_theme_from_file() {
        let file = ThemeFile {
            colors: BaseColors {
                background: "#000000".to_string(),
                foreground: "#ffffff".to_string(),
                ..Default::default()
            },
            file_colors: FileColors {
                directory: "cyan".to_string(),
                ..Default::default()
            },
            git_colors: GitColors::default(),
        };
        let theme = Theme::from_file(&file);
        assert_eq!(theme.background, Color::Rgb(0, 0, 0));
        assert_eq!(theme.foreground, Color::Rgb(255, 255, 255));
        assert_eq!(theme.directory, Color::Cyan);
    }
}
