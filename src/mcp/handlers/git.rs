//! Git operation handlers
//!
//! Implements git_status, git_diff, git_log, stage_files, create_commit.

use std::path::Path;
use std::process::Command;

use super::{error_result, success_result, ToolCallResult};
use crate::git::{get_diff, DiffLine, GitStatus};
use crate::mcp::security::validate_path;

/// Get git status for the repository
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

    success_result(output)
}

/// Get git diff for a file
pub fn get_git_diff(root: &Path, path: &str, staged: bool) -> ToolCallResult {
    let target = root.join(path);

    // Try to validate path (might fail for new files)
    let canonical = match validate_path(root, path) {
        Ok(p) => p,
        Err(_) => target.clone(), // Use unvalidated path for new files
    };

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
                    DiffLine::Added(content) => output.push_str(&format!("+{}\n", content)),
                    DiffLine::Removed(content) => output.push_str(&format!("-{}\n", content)),
                    DiffLine::Context(content) => output.push_str(&format!(" {}\n", content)),
                    DiffLine::HunkHeader(header) => output.push_str(&format!("{}\n", header)),
                    DiffLine::Other(other) => output.push_str(&format!("{}\n", other)),
                }
            }

            success_result(output)
        }
        None => success_result(format!(
            "No {} changes for: {}",
            if staged { "staged" } else { "unstaged" },
            path
        )),
    }
}

/// Get git commit log
pub fn git_log(root: &Path, limit: Option<usize>, path: Option<&str>) -> ToolCallResult {
    let limit = limit.unwrap_or(10);

    let mut args = vec![
        "log".to_string(),
        format!("-{}", limit),
        "--pretty=format:%h|%an|%ar|%s".to_string(),
    ];

    if let Some(p) = path {
        args.push("--".to_string());
        args.push(p.to_string());
    }

    let output = Command::new("git").args(&args).current_dir(root).output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.trim().is_empty() {
                return success_result("No commits found".to_string());
            }

            let mut result = String::new();
            result.push_str("Commit History:\n\n");

            for line in stdout.lines() {
                let parts: Vec<&str> = line.splitn(4, '|').collect();
                if parts.len() == 4 {
                    result.push_str(&format!(
                        "{} - {} ({}) - {}\n",
                        parts[0], parts[1], parts[2], parts[3]
                    ));
                }
            }

            success_result(result)
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            error_result(&format!("git log failed: {}", stderr))
        }
        Err(e) => error_result(&format!("Failed to run git: {}", e)),
    }
}

/// Stage files for git commit
pub fn stage_files(root: &Path, paths: &[&str]) -> ToolCallResult {
    let args: Vec<&str> = if paths.is_empty() {
        vec!["add", "-A"]
    } else {
        let mut a = vec!["add"];
        a.extend(paths);
        a
    };

    let output = Command::new("git").args(&args).current_dir(root).output();

    match output {
        Ok(o) if o.status.success() => {
            let count = if paths.is_empty() {
                "all changes".to_string()
            } else {
                format!("{} file(s)", paths.len())
            };
            success_result(format!("Staged {}", count))
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            error_result(&format!("git add failed: {}", stderr))
        }
        Err(e) => error_result(&format!("Failed to run git: {}", e)),
    }
}

/// Create a git commit
pub fn create_commit(root: &Path, message: &str) -> ToolCallResult {
    if message.trim().is_empty() {
        return error_result("Commit message cannot be empty");
    }

    let output = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(root)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let commit_info = stdout.lines().next().unwrap_or("Commit created");
            success_result(commit_info.to_string())
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.contains("nothing to commit") || stderr.contains("nothing to commit") {
                error_result("Nothing to commit (no staged changes)")
            } else {
                error_result(&format!("git commit failed: {}", stderr))
            }
        }
        Err(e) => error_result(&format!("Failed to run git: {}", e)),
    }
}
