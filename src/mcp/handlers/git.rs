//! Git operation handlers
//!
//! Implements git_status, git_diff, git_log, stage_files, create_commit.

use std::path::Path;
use std::process::Command;

use super::{error_result, success_result, ToolCallResult};
use crate::git::{get_diff, DiffLine, GitStatus};
use crate::mcp::security::{validate_new_path, validate_path};

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
    // Validate existing paths and safely allow new-file paths.
    let canonical = match validate_path(root, path).or_else(|_| validate_new_path(root, path)) {
        Ok(p) => p,
        Err(e) => return error_result(&format!("Invalid path '{}': {}", path, e)),
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
    // Security: Limit to reasonable number of commits
    let limit = limit.unwrap_or(10).min(1000);

    let mut args = vec![
        "log".to_string(),
        format!("-{}", limit),
        "--pretty=format:%h|%an|%ar|%s".to_string(),
    ];

    // Security: Validate path if specified
    if let Some(p) = path {
        if let Err(e) = validate_path(root, p) {
            return error_result(&format!("Invalid path: {}", e));
        }
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
    // Security: Validate all paths before staging
    for p in paths {
        if let Err(e) = validate_path(root, p).or_else(|_| validate_new_path(root, p)) {
            return error_result(&format!("Invalid path '{}': {}", p, e));
        }
    }

    let args: Vec<&str> = if paths.is_empty() {
        vec!["add", "-A"]
    } else {
        let mut a = vec!["add", "--"];
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn init_repo(root: &Path) {
        let status = Command::new("git")
            .arg("init")
            .current_dir(root)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn stage_files_treats_option_like_name_as_path() {
        if !git_available() {
            return;
        }

        let temp = tempdir().unwrap();
        init_repo(temp.path());
        fs::write(temp.path().join("-A"), "only this file").unwrap();
        fs::write(temp.path().join("other.txt"), "leave untracked").unwrap();

        let result = stage_files(temp.path(), &["-A"]);
        assert_eq!(result.is_error, None);
        let output = Command::new("git")
            .args(["diff", "--cached", "--name-only", "-z"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"-A\0");
    }

    #[test]
    fn stage_files_accepts_deleted_path() {
        if !git_available() {
            return;
        }

        let temp = tempdir().unwrap();
        init_repo(temp.path());
        fs::write(temp.path().join("deleted.txt"), "tracked").unwrap();
        assert!(Command::new("git")
            .args(["add", "--", "deleted.txt"])
            .current_dir(temp.path())
            .status()
            .unwrap()
            .success());
        assert!(Command::new("git")
            .args([
                "-c",
                "user.name=FileView Test",
                "-c",
                "user.email=fileview@example.invalid",
                "commit",
                "-m",
                "initial",
            ])
            .current_dir(temp.path())
            .status()
            .unwrap()
            .success());
        fs::remove_file(temp.path().join("deleted.txt")).unwrap();

        let result = stage_files(temp.path(), &["deleted.txt"]);
        assert_eq!(result.is_error, None);
        let output = Command::new("git")
            .args(["diff", "--cached", "--name-status"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout), "D\tdeleted.txt\n");
    }
}
