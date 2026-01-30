//! Git status detection and caching

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Git file status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileStatus {
    /// File has been modified
    Modified,
    /// File has been staged for addition
    Added,
    /// File is not tracked by git
    Untracked,
    /// File has been deleted
    Deleted,
    /// File has been renamed
    Renamed,
    /// File is ignored by .gitignore
    Ignored,
    /// File has merge conflicts
    Conflict,
    /// File is clean (no changes)
    #[default]
    Clean,
}

/// Git repository status information
#[derive(Debug)]
pub struct GitStatus {
    /// Root directory of the git repository
    repo_root: PathBuf,
    /// Cached file statuses
    statuses: HashMap<PathBuf, FileStatus>,
    /// Directory statuses (propagated from children)
    dir_statuses: HashMap<PathBuf, FileStatus>,
    /// Current branch name
    branch: Option<String>,
}

impl GitStatus {
    /// Detect git repository and load status
    pub fn detect(path: &Path) -> Option<Self> {
        let repo_root = find_git_root(path)?;
        let branch = get_current_branch(&repo_root);
        let (statuses, dir_statuses) = load_git_status(&repo_root);

        Some(Self {
            repo_root,
            statuses,
            dir_statuses,
            branch,
        })
    }

    /// Get the status of a specific file or directory
    pub fn get_status(&self, path: &Path) -> FileStatus {
        // First check file statuses
        if let Some(status) = self.statuses.get(path) {
            return *status;
        }

        // Then check directory statuses
        if let Some(status) = self.dir_statuses.get(path) {
            return *status;
        }

        // Check if path is relative to repo root
        if let Ok(relative) = path.strip_prefix(&self.repo_root) {
            if let Some(status) = self.statuses.get(relative) {
                return *status;
            }
            if let Some(status) = self.dir_statuses.get(relative) {
                return *status;
            }
        }

        FileStatus::Clean
    }

    /// Get the current branch name
    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    /// Get the repository root path
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// Refresh git status (call after file operations)
    pub fn refresh(&mut self) {
        self.branch = get_current_branch(&self.repo_root);
        let (statuses, dir_statuses) = load_git_status(&self.repo_root);
        self.statuses = statuses;
        self.dir_statuses = dir_statuses;
    }
}

/// Find the root of the git repository containing the given path
fn find_git_root(path: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .ok()?;

    if output.status.success() {
        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(PathBuf::from(root))
    } else {
        None
    }
}

/// Get the current branch name
fn get_current_branch(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch == "HEAD" {
            // Detached HEAD state - try to get commit hash
            let hash_output = Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .current_dir(repo_root)
                .output()
                .ok()?;
            if hash_output.status.success() {
                return Some(format!(
                    "detached@{}",
                    String::from_utf8_lossy(&hash_output.stdout).trim()
                ));
            }
        }
        Some(branch)
    } else {
        None
    }
}

/// Load git status for all files in the repository
fn load_git_status(
    repo_root: &Path,
) -> (HashMap<PathBuf, FileStatus>, HashMap<PathBuf, FileStatus>) {
    let mut statuses = HashMap::new();
    let mut dir_statuses: HashMap<PathBuf, FileStatus> = HashMap::new();

    // Get status with porcelain format for machine parsing
    // -uall shows all untracked files (required for per-file status display)
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "-uall", "--ignored"])
        .current_dir(repo_root)
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return (statuses, dir_statuses),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if line.len() < 4 {
            continue;
        }

        let index_status = line.chars().next().unwrap_or(' ');
        let worktree_status = line.chars().nth(1).unwrap_or(' ');
        let path_str = &line[3..];

        // Handle renamed files (format: "R  old -> new")
        let file_path = if path_str.contains(" -> ") {
            path_str.split(" -> ").last().unwrap_or(path_str)
        } else {
            path_str
        };

        let path = PathBuf::from(file_path);
        let status = parse_status(index_status, worktree_status);

        if status != FileStatus::Clean {
            statuses.insert(path.clone(), status);

            // Propagate status to parent directories
            let mut parent = path.parent();
            while let Some(dir) = parent {
                if dir.as_os_str().is_empty() {
                    break;
                }
                let current = dir_statuses
                    .entry(dir.to_path_buf())
                    .or_insert(FileStatus::Clean);
                *current = merge_status(*current, status);
                parent = dir.parent();
            }
        }
    }

    (statuses, dir_statuses)
}

/// Parse git status characters into FileStatus
fn parse_status(index: char, worktree: char) -> FileStatus {
    // Check for conflicts first
    if index == 'U'
        || worktree == 'U'
        || (index == 'A' && worktree == 'A')
        || (index == 'D' && worktree == 'D')
    {
        return FileStatus::Conflict;
    }

    // Check for ignored
    if index == '!' {
        return FileStatus::Ignored;
    }

    // Check for untracked
    if index == '?' {
        return FileStatus::Untracked;
    }

    // Check for renamed
    if index == 'R' || worktree == 'R' {
        return FileStatus::Renamed;
    }

    // Check for added
    if index == 'A' {
        return FileStatus::Added;
    }

    // Check for deleted
    if index == 'D' || worktree == 'D' {
        return FileStatus::Deleted;
    }

    // Check for modified
    if index == 'M' || worktree == 'M' {
        return FileStatus::Modified;
    }

    FileStatus::Clean
}

/// Merge two statuses, preferring the more "severe" one
fn merge_status(a: FileStatus, b: FileStatus) -> FileStatus {
    use FileStatus::*;

    match (a, b) {
        // Conflict is highest priority
        (Conflict, _) | (_, Conflict) => Conflict,
        // Then Deleted
        (Deleted, _) | (_, Deleted) => Deleted,
        // Then Modified
        (Modified, _) | (_, Modified) => Modified,
        // Then Renamed
        (Renamed, _) | (_, Renamed) => Renamed,
        // Then Added
        (Added, _) | (_, Added) => Added,
        // Then Untracked
        (Untracked, _) | (_, Untracked) => Untracked,
        // Ignored doesn't propagate
        (Ignored, other) | (other, Ignored) => other,
        // Default to Clean
        (Clean, Clean) => Clean,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status_modified() {
        assert_eq!(parse_status('M', ' '), FileStatus::Modified);
        assert_eq!(parse_status(' ', 'M'), FileStatus::Modified);
        assert_eq!(parse_status('M', 'M'), FileStatus::Modified);
    }

    #[test]
    fn test_parse_status_added() {
        assert_eq!(parse_status('A', ' '), FileStatus::Added);
    }

    #[test]
    fn test_parse_status_deleted() {
        assert_eq!(parse_status('D', ' '), FileStatus::Deleted);
        assert_eq!(parse_status(' ', 'D'), FileStatus::Deleted);
    }

    #[test]
    fn test_parse_status_untracked() {
        assert_eq!(parse_status('?', '?'), FileStatus::Untracked);
    }

    #[test]
    fn test_parse_status_ignored() {
        assert_eq!(parse_status('!', '!'), FileStatus::Ignored);
    }

    #[test]
    fn test_parse_status_conflict() {
        assert_eq!(parse_status('U', 'U'), FileStatus::Conflict);
        assert_eq!(parse_status('A', 'A'), FileStatus::Conflict);
    }

    #[test]
    fn test_parse_status_renamed() {
        assert_eq!(parse_status('R', ' '), FileStatus::Renamed);
    }

    #[test]
    fn test_merge_status() {
        assert_eq!(
            merge_status(FileStatus::Clean, FileStatus::Modified),
            FileStatus::Modified
        );
        assert_eq!(
            merge_status(FileStatus::Modified, FileStatus::Conflict),
            FileStatus::Conflict
        );
        assert_eq!(
            merge_status(FileStatus::Untracked, FileStatus::Added),
            FileStatus::Added
        );
    }
}
