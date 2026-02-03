//! File operation handlers
//!
//! Implements read_file, write_file, delete_file, read_files, list_directory, get_tree, search_code.

use std::fs;
use std::path::Path;
use std::process::Command;

use super::{error_result, success_result, ToolCallResult, ToolContent};
use crate::mcp::security::{truncate_entry_name, validate_new_path, validate_path};

/// List directory contents
pub fn list_directory(root: &Path, path: Option<&str>) -> ToolCallResult {
    let target = match path {
        Some(p) => match validate_path(root, p) {
            Ok(path) => path,
            Err(e) => return error_result(&e.to_string()),
        },
        None => root.to_path_buf(),
    };

    match fs::read_dir(&target) {
        Ok(entries) => {
            let mut items: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| {
                    let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    let name = truncate_entry_name(e.file_name().to_string_lossy().to_string());
                    if is_dir {
                        format!("{}/", name)
                    } else {
                        name
                    }
                })
                .collect();
            items.sort();

            success_result(items.join("\n"))
        }
        Err(e) => error_result(&format!("Failed to read directory: {}", e)),
    }
}

/// Get directory tree
pub fn get_tree(root: &Path, path: Option<&str>, depth: Option<usize>) -> ToolCallResult {
    let target = match path {
        Some(p) => match validate_path(root, p) {
            Ok(path) => path,
            Err(e) => return error_result(&e.to_string()),
        },
        None => root.to_path_buf(),
    };

    let mut output = Vec::new();
    if let Err(e) = write_tree(&mut output, &target, depth) {
        return error_result(&format!("Failed to generate tree: {}", e));
    }

    success_result(String::from_utf8_lossy(&output).to_string())
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
    let canonical = match validate_path(root, path) {
        Ok(p) => p,
        Err(e) => return error_result(&e.to_string()),
    };

    if canonical.is_dir() {
        return error_result("Path is a directory, not a file");
    }

    match fs::read_to_string(&canonical) {
        Ok(content) => success_result(content),
        Err(e) => error_result(&format!("Failed to read file: {}", e)),
    }
}

/// Read multiple files at once
pub fn read_files(root: &Path, paths: &[&str]) -> ToolCallResult {
    let mut results = Vec::new();
    let mut success_count = 0;
    let mut error_count = 0;

    for path in paths {
        match validate_path(root, path) {
            Ok(canonical) => {
                if canonical.is_dir() {
                    results.push(format!("--- {} ---\nError: Path is a directory\n", path));
                    error_count += 1;
                    continue;
                }

                match fs::read_to_string(&canonical) {
                    Ok(content) => {
                        results.push(format!("--- {} ---\n{}\n", path, content));
                        success_count += 1;
                    }
                    Err(e) => {
                        results.push(format!("--- {} ---\nError: {}\n", path, e));
                        error_count += 1;
                    }
                }
            }
            Err(e) => {
                results.push(format!("--- {} ---\nError: {}\n", path, e));
                error_count += 1;
            }
        }
    }

    let summary = format!(
        "Read {} file(s) successfully, {} error(s)\n\n",
        success_count, error_count
    );

    ToolCallResult {
        content: vec![ToolContent::text(format!(
            "{}{}",
            summary,
            results.join("\n")
        ))],
        is_error: if error_count > 0 && success_count == 0 {
            Some(true)
        } else {
            None
        },
    }
}

/// Write content to a file
pub fn write_file(root: &Path, path: &str, content: &str, create_dirs: bool) -> ToolCallResult {
    let target = match validate_new_path(root, path) {
        Ok(p) => p,
        Err(e) => return error_result(&e.to_string()),
    };

    // Create parent directories if requested
    if create_dirs {
        if let Some(parent) = target.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return error_result(&format!("Failed to create directories: {}", e));
            }
        }
    }

    // Write the file
    match fs::write(&target, content) {
        Ok(_) => {
            let bytes = content.len();
            success_result(format!("Successfully wrote {} bytes to {}", bytes, path))
        }
        Err(e) => error_result(&format!("Failed to write file: {}", e)),
    }
}

/// Delete a file or directory
pub fn delete_file(root: &Path, path: &str, recursive: bool, use_trash: bool) -> ToolCallResult {
    let canonical = match validate_path(root, path) {
        Ok(p) => p,
        Err(e) => return error_result(&e.to_string()),
    };

    // Don't allow deleting the root itself
    if let Ok(root_canonical) = root.canonicalize() {
        if canonical == root_canonical {
            return error_result("Cannot delete root directory");
        }
    }

    let is_dir = canonical.is_dir();

    if use_trash {
        // Use trash crate for safe deletion
        match trash::delete(&canonical) {
            Ok(_) => success_result(format!("Moved to trash: {}", path)),
            Err(e) => error_result(&format!("Failed to move to trash: {}", e)),
        }
    } else if is_dir {
        if recursive {
            match fs::remove_dir_all(&canonical) {
                Ok(_) => success_result(format!("Deleted directory: {}", path)),
                Err(e) => error_result(&format!("Failed to delete directory: {}", e)),
            }
        } else {
            match fs::remove_dir(&canonical) {
                Ok(_) => success_result(format!("Deleted empty directory: {}", path)),
                Err(e) => error_result(&format!("Failed to delete directory (not empty?): {}", e)),
            }
        }
    } else {
        match fs::remove_file(&canonical) {
            Ok(_) => success_result(format!("Deleted file: {}", path)),
            Err(e) => error_result(&format!("Failed to delete file: {}", e)),
        }
    }
}

/// Search code in the repository
pub fn search_code(root: &Path, pattern: &str, path: Option<&str>) -> ToolCallResult {
    // Try ripgrep first, fall back to grep
    let (cmd, args) = if Command::new("rg").arg("--version").output().is_ok() {
        ("rg", vec!["-n", "--no-heading", pattern])
    } else {
        ("grep", vec!["-rn", pattern])
    };

    let search_path = match path {
        Some(p) => match validate_path(root, p) {
            Ok(path) => path,
            Err(e) => return error_result(&e.to_string()),
        },
        None => root.to_path_buf(),
    };

    let output = Command::new(cmd)
        .args(&args)
        .arg(&search_path)
        .current_dir(root)
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);

            if !o.status.success() && stdout.is_empty() {
                if stderr.contains("No such file") || stderr.contains("not found") {
                    return error_result(&format!("Search error: {}", stderr));
                }
                // No matches found is not an error
                return success_result(format!("No matches found for pattern: {}", pattern));
            }

            // Limit output to first 100 matches
            let lines: Vec<&str> = stdout.lines().take(100).collect();
            let total_matches = stdout.lines().count();

            let mut result = String::new();
            result.push_str(&format!(
                "Search results for '{}' ({} matches):\n\n",
                pattern,
                total_matches.min(100)
            ));

            for line in &lines {
                result.push_str(line);
                result.push('\n');
            }

            if total_matches > 100 {
                result.push_str(&format!(
                    "\n... and {} more matches (showing first 100)",
                    total_matches - 100
                ));
            }

            success_result(result)
        }
        Err(e) => error_result(&format!("Failed to run search: {}", e)),
    }
}
