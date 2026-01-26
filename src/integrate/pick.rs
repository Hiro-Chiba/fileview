//! Pick mode (--pick option)
//!
//! Allows external tools to use fileview as a file picker.
//! Selected path(s) are output to stdout when user confirms selection.

use std::io::{self, Write};
use std::path::PathBuf;

/// Exit codes for pick mode
pub mod exit_code {
    /// User selected file(s) successfully
    pub const SUCCESS: i32 = 0;
    /// User cancelled selection
    pub const CANCELLED: i32 = 1;
    /// Error occurred
    pub const ERROR: i32 = 2;
}

/// Output format for picked paths
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    /// One path per line (default)
    #[default]
    Lines,
    /// Null-separated paths (for xargs -0)
    NullSeparated,
    /// JSON array
    Json,
}

impl OutputFormat {
    /// Parse from string argument
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "lines" | "line" => Some(Self::Lines),
            "null" | "nul" | "0" => Some(Self::NullSeparated),
            "json" => Some(Self::Json),
            _ => None,
        }
    }
}

/// Output selected paths to stdout
pub fn output_paths(paths: &[PathBuf], format: OutputFormat) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    match format {
        OutputFormat::Lines => {
            for path in paths {
                writeln!(handle, "{}", path.display())?;
            }
        }
        OutputFormat::NullSeparated => {
            for (i, path) in paths.iter().enumerate() {
                if i > 0 {
                    write!(handle, "\0")?;
                }
                write!(handle, "{}", path.display())?;
            }
            // Final null for xargs compatibility
            if !paths.is_empty() {
                write!(handle, "\0")?;
            }
        }
        OutputFormat::Json => {
            let json_paths: Vec<String> =
                paths.iter().map(|p| p.display().to_string()).collect();
            writeln!(handle, "{}", serde_json_mini(&json_paths))?;
        }
    }

    handle.flush()?;
    Ok(())
}

/// Minimal JSON array serialization (no serde dependency)
fn serde_json_mini(paths: &[String]) -> String {
    let escaped: Vec<String> = paths
        .iter()
        .map(|p| {
            let escaped = p
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            format!("\"{}\"", escaped)
        })
        .collect();

    format!("[{}]", escaped.join(","))
}

/// Pick mode result
#[derive(Debug)]
pub enum PickResult {
    /// User selected paths
    Selected(Vec<PathBuf>),
    /// User cancelled
    Cancelled,
}

impl PickResult {
    /// Get exit code for this result
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Selected(_) => exit_code::SUCCESS,
            Self::Cancelled => exit_code::CANCELLED,
        }
    }

    /// Output result to stdout if paths were selected
    pub fn output(&self, format: OutputFormat) -> io::Result<i32> {
        match self {
            Self::Selected(paths) => {
                output_paths(paths, format)?;
                Ok(exit_code::SUCCESS)
            }
            Self::Cancelled => Ok(exit_code::CANCELLED),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_escape() {
        let paths = vec![
            String::from("/path/to/file"),
            String::from("/path/with\"quote"),
            String::from("/path/with\\backslash"),
        ];
        let json = serde_json_mini(&paths);
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(json.contains("\\\""));
        assert!(json.contains("\\\\"));
    }

    #[test]
    fn test_output_format_parse() {
        assert!(matches!(
            OutputFormat::from_str("lines"),
            Some(OutputFormat::Lines)
        ));
        assert!(matches!(
            OutputFormat::from_str("null"),
            Some(OutputFormat::NullSeparated)
        ));
        assert!(matches!(
            OutputFormat::from_str("json"),
            Some(OutputFormat::Json)
        ));
        assert!(OutputFormat::from_str("invalid").is_none());
    }
}
