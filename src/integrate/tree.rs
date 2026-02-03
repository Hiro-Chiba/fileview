//! Tree output mode for CLI integration
//!
//! Outputs directory tree structure to stdout in a format suitable for AI tools.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Output a directory tree to stdout
///
/// # Arguments
/// * `root` - Root directory path
/// * `max_depth` - Maximum depth to traverse (None = unlimited)
/// * `show_hidden` - Whether to show hidden files
pub fn output_tree(root: &Path, max_depth: Option<usize>, show_hidden: bool) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Print root
    writeln!(handle, "{}", root.display())?;

    // Print children
    print_tree_recursive(&mut handle, root, "", max_depth, 0, show_hidden)?;

    handle.flush()
}

/// Recursively print tree structure (public for MCP)
pub fn print_tree_recursive_pub<W: Write>(
    out: &mut W,
    path: &Path,
    prefix: &str,
    max_depth: Option<usize>,
    current_depth: usize,
    show_hidden: bool,
) -> io::Result<()> {
    print_tree_recursive(out, path, prefix, max_depth, current_depth, show_hidden)
}

/// Recursively print tree structure
fn print_tree_recursive<W: Write>(
    out: &mut W,
    path: &Path,
    prefix: &str,
    max_depth: Option<usize>,
    current_depth: usize,
    show_hidden: bool,
) -> io::Result<()> {
    // Check depth limit
    if let Some(max) = max_depth {
        if current_depth >= max {
            return Ok(());
        }
    }

    // Read directory entries
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return Ok(()), // Skip unreadable directories
    };

    // Collect and sort entries
    let mut entries: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            if show_hidden {
                true
            } else {
                !e.file_name().to_string_lossy().starts_with('.')
            }
        })
        .collect();

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    let count = entries.len();
    for (i, entry) in entries.into_iter().enumerate() {
        let is_last = i == count - 1;
        let name = entry.file_name();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        // Choose connector
        let connector = if is_last { "└── " } else { "├── " };

        // Print entry with trailing / for directories
        if is_dir {
            writeln!(out, "{}{}{}/", prefix, connector, name.to_string_lossy())?;
        } else {
            writeln!(out, "{}{}{}", prefix, connector, name.to_string_lossy())?;
        }

        // Recurse into directories
        if is_dir {
            let new_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            print_tree_recursive(
                out,
                &entry.path(),
                &new_prefix,
                max_depth,
                current_depth + 1,
                show_hidden,
            )?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(temp.path().join("src/lib.rs"), "pub mod utils;").unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(temp.path().join(".hidden"), "hidden").unwrap();
        temp
    }

    #[test]
    fn test_tree_output() {
        let temp = setup_test_dir();
        let mut output = Vec::new();
        print_tree_recursive(&mut output, temp.path(), "", None, 0, false).unwrap();
        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("src/"));
        assert!(output.contains("Cargo.toml"));
        assert!(!output.contains(".hidden")); // Hidden file not shown
    }

    #[test]
    fn test_tree_with_hidden() {
        let temp = setup_test_dir();
        let mut output = Vec::new();
        print_tree_recursive(&mut output, temp.path(), "", None, 0, true).unwrap();
        let output = String::from_utf8(output).unwrap();

        assert!(output.contains(".hidden"));
    }

    #[test]
    fn test_tree_depth_limit() {
        let temp = setup_test_dir();
        let mut output = Vec::new();
        print_tree_recursive(&mut output, temp.path(), "", Some(1), 0, false).unwrap();
        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("src/"));
        assert!(!output.contains("main.rs")); // Not shown due to depth limit
    }

    #[test]
    fn test_tree_connectors() {
        let temp = setup_test_dir();
        let mut output = Vec::new();
        print_tree_recursive(&mut output, temp.path(), "", None, 0, false).unwrap();
        let output = String::from_utf8(output).unwrap();

        // Should contain tree connectors
        assert!(output.contains("├── ") || output.contains("└── "));
    }
}
