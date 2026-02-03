//! MCP tool handlers
//!
//! Implements the actual functionality for each MCP tool.

use std::fs;
use std::path::Path;

use super::types::{ToolCallResult, ToolContent};

/// List directory contents
pub fn list_directory(root: &Path, path: Option<&str>) -> ToolCallResult {
    let target = match path {
        Some(p) => {
            let target = root.join(p);
            // Security: ensure path is within root
            match target.canonicalize() {
                Ok(canonical) => {
                    if !canonical.starts_with(root) {
                        return error_result("Path is outside root directory");
                    }
                    canonical
                }
                Err(e) => return error_result(&format!("Invalid path: {}", e)),
            }
        }
        None => root.to_path_buf(),
    };

    match fs::read_dir(&target) {
        Ok(entries) => {
            let mut items: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| {
                    let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    let name = e.file_name().to_string_lossy().to_string();
                    if is_dir {
                        format!("{}/", name)
                    } else {
                        name
                    }
                })
                .collect();
            items.sort();

            ToolCallResult {
                content: vec![ToolContent::text(items.join("\n"))],
                is_error: None,
            }
        }
        Err(e) => error_result(&format!("Failed to read directory: {}", e)),
    }
}

/// Get directory tree
pub fn get_tree(root: &Path, path: Option<&str>, depth: Option<usize>) -> ToolCallResult {
    let target = match path {
        Some(p) => {
            let target = root.join(p);
            match target.canonicalize() {
                Ok(canonical) => {
                    if !canonical.starts_with(root) {
                        return error_result("Path is outside root directory");
                    }
                    canonical
                }
                Err(e) => return error_result(&format!("Invalid path: {}", e)),
            }
        }
        None => root.to_path_buf(),
    };

    let mut output = Vec::new();
    if let Err(e) = write_tree(&mut output, &target, depth) {
        return error_result(&format!("Failed to generate tree: {}", e));
    }

    ToolCallResult {
        content: vec![ToolContent::text(
            String::from_utf8_lossy(&output).to_string(),
        )],
        is_error: None,
    }
}

/// Write tree to output
fn write_tree<W: std::io::Write>(
    out: &mut W,
    path: &Path,
    depth: Option<usize>,
) -> std::io::Result<()> {
    writeln!(out, "{}", path.display())?;
    crate::integrate::tree::print_tree_recursive_pub(out, path, "", depth, 0, false)
}

/// Read file content
pub fn read_file(root: &Path, path: &str) -> ToolCallResult {
    let target = root.join(path);

    // Security: ensure path is within root
    match target.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(root) {
                return error_result("Path is outside root directory");
            }

            if canonical.is_dir() {
                return error_result("Path is a directory, not a file");
            }

            match fs::read_to_string(&canonical) {
                Ok(content) => ToolCallResult {
                    content: vec![ToolContent::text(content)],
                    is_error: None,
                },
                Err(e) => error_result(&format!("Failed to read file: {}", e)),
            }
        }
        Err(e) => error_result(&format!("Invalid path: {}", e)),
    }
}

/// Create error result
fn error_result(message: &str) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent::text(message.to_string())],
        is_error: Some(true),
    }
}
