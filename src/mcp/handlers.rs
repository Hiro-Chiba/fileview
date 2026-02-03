//! MCP tool handlers
//!
//! Implements the actual functionality for each MCP tool.

use std::fs;
use std::path::Path;
use std::process::Command;

use super::types::{ToolCallResult, ToolContent};
use crate::git::{get_diff, DiffLine, GitStatus};

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

/// Get git status for the repository
///
/// Returns a list of changed/staged files with their status.
pub fn get_git_status(root: &Path) -> ToolCallResult {
    let git_status = match GitStatus::detect(root) {
        Some(status) => status,
        None => return error_result("Not a git repository"),
    };

    let mut output = String::new();

    // Add branch info
    if let Some(branch) = git_status.branch() {
        output.push_str(&format!("Branch: {}\n\n", branch));
    }

    // Get status output using git command
    let git_output = Command::new("git")
        .args(["status", "--porcelain=v1", "-uall"])
        .current_dir(root)
        .output();

    match git_output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.trim().is_empty() {
                output.push_str("Working tree clean");
            } else {
                output.push_str("Changes:\n");
                for line in stdout.lines() {
                    if line.len() >= 3 {
                        let index_status = line.chars().next().unwrap_or(' ');
                        let worktree_status = line.chars().nth(1).unwrap_or(' ');
                        let path = &line[3..];

                        let status_str = match (index_status, worktree_status) {
                            // Conflicts (check first due to overlap)
                            ('U', _) | (_, 'U') => "conflict",
                            ('M', _) | (_, 'M') => "modified",
                            ('A', _) => "added",
                            ('D', _) | (_, 'D') => "deleted",
                            ('R', _) => "renamed",
                            ('?', '?') => "untracked",
                            ('!', _) => "ignored",
                            _ => "unknown",
                        };

                        let staged = matches!(index_status, 'M' | 'A' | 'D' | 'R' | 'C');
                        let staged_marker = if staged { " [staged]" } else { "" };

                        output.push_str(&format!("  {} {}{}\n", status_str, path, staged_marker));
                    }
                }
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            return error_result(&format!("git status failed: {}", stderr));
        }
        Err(e) => {
            return error_result(&format!("Failed to run git: {}", e));
        }
    }

    ToolCallResult {
        content: vec![ToolContent::text(output)],
        is_error: None,
    }
}

/// Get git diff for a file
///
/// # Arguments
/// * `root` - The repository root directory
/// * `path` - Relative path to the file
/// * `staged` - If true, show staged changes (--cached)
pub fn get_git_diff(root: &Path, path: &str, staged: bool) -> ToolCallResult {
    let target = root.join(path);

    // Security: ensure path is within root
    match target.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(root) {
                return error_result("Path is outside root directory");
            }

            match get_diff(root, &canonical, staged) {
                Some(diff) => {
                    let mut output = String::new();
                    output.push_str(&format!("File: {}\n", path));
                    output.push_str(&format!(
                        "Changes: +{} -{}\n\n",
                        diff.additions, diff.deletions
                    ));

                    for line in &diff.lines {
                        match line {
                            DiffLine::Added(content) => {
                                output.push_str(&format!("+{}\n", content));
                            }
                            DiffLine::Removed(content) => {
                                output.push_str(&format!("-{}\n", content));
                            }
                            DiffLine::Context(content) => {
                                output.push_str(&format!(" {}\n", content));
                            }
                            DiffLine::HunkHeader(header) => {
                                output.push_str(&format!("{}\n", header));
                            }
                            DiffLine::Other(other) => {
                                output.push_str(&format!("{}\n", other));
                            }
                        }
                    }

                    ToolCallResult {
                        content: vec![ToolContent::text(output)],
                        is_error: None,
                    }
                }
                None => ToolCallResult {
                    content: vec![ToolContent::text(format!(
                        "No {} changes for: {}",
                        if staged { "staged" } else { "unstaged" },
                        path
                    ))],
                    is_error: None,
                },
            }
        }
        Err(_) => {
            // File might not exist yet (new file), try getting diff anyway
            match get_diff(root, &target, staged) {
                Some(diff) => {
                    let mut output = String::new();
                    output.push_str(&format!("File: {}\n", path));
                    output.push_str(&format!(
                        "Changes: +{} -{}\n\n",
                        diff.additions, diff.deletions
                    ));

                    for line in &diff.lines {
                        match line {
                            DiffLine::Added(content) => output.push_str(&format!("+{}\n", content)),
                            DiffLine::Removed(content) => {
                                output.push_str(&format!("-{}\n", content))
                            }
                            DiffLine::Context(content) => {
                                output.push_str(&format!(" {}\n", content))
                            }
                            DiffLine::HunkHeader(header) => {
                                output.push_str(&format!("{}\n", header))
                            }
                            DiffLine::Other(other) => output.push_str(&format!("{}\n", other)),
                        }
                    }

                    ToolCallResult {
                        content: vec![ToolContent::text(output)],
                        is_error: None,
                    }
                }
                None => error_result(&format!("File not found or no changes: {}", path)),
            }
        }
    }
}

/// Search code in the repository
///
/// # Arguments
/// * `root` - The repository root directory
/// * `pattern` - The search pattern (regex)
/// * `path` - Optional path to limit search scope
pub fn search_code(root: &Path, pattern: &str, path: Option<&str>) -> ToolCallResult {
    // Try ripgrep first, fall back to grep
    let (cmd, args) = if Command::new("rg").arg("--version").output().is_ok() {
        ("rg", vec!["-n", "--no-heading", pattern])
    } else {
        ("grep", vec!["-rn", pattern])
    };

    let search_path = match path {
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
                return ToolCallResult {
                    content: vec![ToolContent::text(format!(
                        "No matches found for pattern: {}",
                        pattern
                    ))],
                    is_error: None,
                };
            }

            // Limit output to first 100 matches to avoid overwhelming the response
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

            ToolCallResult {
                content: vec![ToolContent::text(result)],
                is_error: None,
            }
        }
        Err(e) => error_result(&format!("Failed to run search: {}", e)),
    }
}
