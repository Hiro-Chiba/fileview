//! Pick mode (--pick option)
//!
//! Allows external tools to use fileview as a file picker.
//! Selected path(s) are output to stdout when user confirms selection.

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::str::FromStr;

/// Exit codes for the application
///
/// These codes are stable and can be relied upon for scripting:
/// - `SUCCESS` (0): Normal exit or file selected in pick mode
/// - `CANCELLED` (1): User cancelled selection in pick mode (Esc/q)
/// - `ERROR` (2): Runtime error (I/O error, terminal error, etc.)
/// - `INVALID` (3): Invalid command-line arguments or option values
pub mod exit_code {
    /// User selected file(s) successfully or normal exit
    pub const SUCCESS: i32 = 0;
    /// User cancelled selection (pick mode only)
    pub const CANCELLED: i32 = 1;
    /// Runtime error occurred
    pub const ERROR: i32 = 2;
    /// Invalid arguments or options (e.g., unknown flag, invalid format)
    pub const INVALID: i32 = 3;
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

impl FromStr for OutputFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "lines" | "line" => Ok(Self::Lines),
            "null" | "nul" | "0" => Ok(Self::NullSeparated),
            "json" => Ok(Self::Json),
            _ => Err(()),
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
            let json_paths: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
            writeln!(handle, "{}", serde_json_mini(&json_paths))?;
        }
    }

    handle.flush()?;
    Ok(())
}

/// Output selected paths with their file contents
///
/// Format:
/// ```text
/// --- path/to/file.rs ---
/// <file content>
///
/// --- path/to/other.rs ---
/// <file content>
/// ```
pub fn output_paths_with_content(paths: &[PathBuf]) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for (i, path) in paths.iter().enumerate() {
        if i > 0 {
            writeln!(handle)?;
        }

        writeln!(handle, "--- {} ---", path.display())?;

        // Read and output file content
        match fs::read_to_string(path) {
            Ok(content) => {
                write!(handle, "{}", content)?;
                // Ensure trailing newline
                if !content.ends_with('\n') {
                    writeln!(handle)?;
                }
            }
            Err(e) => {
                // For binary files or read errors, show error message
                writeln!(handle, "[Error reading file: {}]", e)?;
            }
        }
    }

    handle.flush()?;
    Ok(())
}

/// Output paths with content in Claude-friendly markdown format
///
/// Format:
/// ```markdown
/// ### File: path/to/file.rs
/// ```rs
/// <file content>
/// ```
/// ```
pub fn output_paths_claude_format(paths: &[PathBuf]) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for (i, path) in paths.iter().enumerate() {
        if i > 0 {
            writeln!(handle)?;
        }

        writeln!(handle, "### File: {}", path.display())?;

        // Detect file extension for syntax highlighting
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        writeln!(handle, "```{}", ext)?;

        // Read and output file content
        match fs::read_to_string(path) {
            Ok(content) => {
                write!(handle, "{}", content)?;
                // Ensure trailing newline before closing fence
                if !content.ends_with('\n') {
                    writeln!(handle)?;
                }
            }
            Err(e) => {
                writeln!(handle, "[Error reading file: {}]", e)?;
            }
        }

        writeln!(handle, "```")?;
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
            Ok(OutputFormat::Lines)
        ));
        assert!(matches!(
            OutputFormat::from_str("null"),
            Ok(OutputFormat::NullSeparated)
        ));
        assert!(matches!(
            OutputFormat::from_str("json"),
            Ok(OutputFormat::Json)
        ));
        assert!(OutputFormat::from_str("invalid").is_err());
    }
}
