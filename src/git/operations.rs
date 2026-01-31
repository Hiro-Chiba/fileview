//! Git operations (stage, unstage)
//!
//! This module provides functions to stage and unstage files in a Git repository.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Cached git executable path (shared with status.rs)
static GIT_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Find git executable path using standard locations or which command
pub fn find_git_executable() -> Option<&'static PathBuf> {
    GIT_PATH
        .get_or_init(|| {
            let candidates = [
                "/usr/bin/git",
                "/usr/local/bin/git",
                "/opt/homebrew/bin/git",
            ];

            for path in candidates {
                let p = PathBuf::from(path);
                if p.exists() {
                    return Some(p);
                }
            }

            // Fallback: which git
            std::process::Command::new("which")
                .arg("git")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| PathBuf::from(s.trim()))
                .filter(|p| p.exists())
        })
        .as_ref()
}

/// Stage a file (git add)
///
/// # Arguments
/// * `repo_root` - The root directory of the git repository
/// * `file` - The absolute path to the file to stage
///
/// # Returns
/// * `Ok(())` if the file was successfully staged
/// * `Err` with error message if staging failed
pub fn stage(repo_root: &Path, file: &Path) -> anyhow::Result<()> {
    let git = find_git_executable().ok_or_else(|| anyhow::anyhow!("git not found"))?;

    // Get relative path from repo root
    let relative = file.strip_prefix(repo_root).unwrap_or(file);

    let output = Command::new(git)
        .args(["add", "--"])
        .arg(relative)
        .current_dir(repo_root)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git add failed: {}", stderr.trim())
    }
}

/// Unstage a file (git reset HEAD)
///
/// # Arguments
/// * `repo_root` - The root directory of the git repository
/// * `file` - The absolute path to the file to unstage
///
/// # Returns
/// * `Ok(())` if the file was successfully unstaged
/// * `Err` with error message if unstaging failed
pub fn unstage(repo_root: &Path, file: &Path) -> anyhow::Result<()> {
    let git = find_git_executable().ok_or_else(|| anyhow::anyhow!("git not found"))?;

    // Get relative path from repo root
    let relative = file.strip_prefix(repo_root).unwrap_or(file);

    let output = Command::new(git)
        .args(["reset", "HEAD", "--"])
        .arg(relative)
        .current_dir(repo_root)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git reset failed: {}", stderr.trim())
    }
}

/// Check if a file is staged (has changes in the index)
///
/// # Arguments
/// * `repo_root` - The root directory of the git repository
/// * `file` - The absolute path to the file to check
///
/// # Returns
/// * `true` if the file has staged changes
/// * `false` if the file is not staged or not in a git repo
pub fn is_staged(repo_root: &Path, file: &Path) -> bool {
    let git = match find_git_executable() {
        Some(g) => g,
        None => return false,
    };

    // Get relative path from repo root
    let relative = file.strip_prefix(repo_root).unwrap_or(file);

    // Use git diff --cached to check if file has staged changes
    let output = Command::new(git)
        .args(["diff", "--cached", "--name-only", "--"])
        .arg(relative)
        .current_dir(repo_root)
        .output();

    match output {
        Ok(o) => {
            if o.status.success() {
                // If the file name appears in the output, it's staged
                let stdout = String::from_utf8_lossy(&o.stdout);
                !stdout.trim().is_empty()
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_git_executable() {
        // This test may fail on systems without git installed
        let git = find_git_executable();
        if let Some(path) = git {
            assert!(path.exists());
            assert!(path.to_string_lossy().contains("git"));
        }
    }

    #[test]
    fn test_stage_nonexistent_repo() {
        let result = stage(
            Path::new("/nonexistent/repo"),
            Path::new("/nonexistent/file.txt"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_unstage_nonexistent_repo() {
        let result = unstage(
            Path::new("/nonexistent/repo"),
            Path::new("/nonexistent/file.txt"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_is_staged_nonexistent_repo() {
        let result = is_staged(
            Path::new("/nonexistent/repo"),
            Path::new("/nonexistent/file.txt"),
        );
        assert!(!result);
    }
}
