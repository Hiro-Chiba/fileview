//! Git status detection and caching

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::operations::find_git_executable;

/// Create a git Command using the validated executable path
fn git_command() -> Option<Command> {
    find_git_executable().map(|executable| {
        let mut command = Command::new(executable);
        command.env("GIT_OPTIONAL_LOCKS", "0");
        command
    })
}

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
#[derive(Debug, PartialEq, Eq)]
pub struct GitStatus {
    /// Root directory of the git repository
    repo_root: PathBuf,
    /// Cached file statuses
    statuses: HashMap<PathBuf, FileStatus>,
    /// Directory statuses (propagated from children)
    dir_statuses: HashMap<PathBuf, FileStatus>,
    /// Current branch name
    branch: Option<String>,
    /// Files that are staged (have changes in the index)
    staged_files: std::collections::HashSet<PathBuf>,
    include_ignored: bool,
}

impl GitStatus {
    /// Detect git repository and load status
    pub fn detect(path: &Path) -> Option<Self> {
        Self::detect_with_ignored(path, true)
    }

    /// Load workspace predicates without enumerating ignored files.
    pub fn detect_for_workspace(path: &Path) -> Option<Self> {
        Self::detect_with_ignored(path, false)
    }

    fn detect_with_ignored(path: &Path, include_ignored: bool) -> Option<Self> {
        let repo_root = find_git_root(path)?;
        let branch = get_current_branch(&repo_root);
        let (statuses, dir_statuses, staged_files) = load_git_status(&repo_root, include_ignored);

        Some(Self {
            repo_root,
            statuses,
            dir_statuses,
            branch,
            staged_files,
            include_ignored,
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

    /// Refresh git status and report whether anything changed.
    pub fn refresh(&mut self) -> bool {
        let branch = get_current_branch(&self.repo_root);
        let (statuses, dir_statuses, staged_files) =
            load_git_status(&self.repo_root, self.include_ignored);
        let changed = self.branch != branch
            || self.statuses != statuses
            || self.dir_statuses != dir_statuses
            || self.staged_files != staged_files;
        self.branch = branch;
        self.statuses = statuses;
        self.dir_statuses = dir_statuses;
        self.staged_files = staged_files;
        changed
    }

    /// Check if a file is staged (has changes in the index)
    pub fn is_staged(&self, path: &Path) -> bool {
        // Check if the file is in the staged files set
        if self.staged_files.contains(path) {
            return true;
        }

        // Also check relative path
        if let Ok(relative) = path.strip_prefix(&self.repo_root) {
            if self.staged_files.contains(relative) {
                return true;
            }
        }

        false
    }

    /// Create a GitStatus with a specific repo root (for testing)
    #[cfg(test)]
    pub fn default_with_root(repo_root: PathBuf) -> Self {
        Self {
            repo_root,
            statuses: std::collections::HashMap::new(),
            dir_statuses: std::collections::HashMap::new(),
            branch: None,
            staged_files: std::collections::HashSet::new(),
            include_ignored: true,
        }
    }
}

/// Find the root of the git repository containing the given path
fn find_git_root(path: &Path) -> Option<PathBuf> {
    let output = git_command()?
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .ok()?;

    if output.status.success() {
        let bytes = output.stdout.strip_suffix(b"\n").unwrap_or(&output.stdout);
        status_path(bytes)
    } else {
        None
    }
}

/// Get the current branch name
fn get_current_branch(repo_root: &Path) -> Option<String> {
    let output = git_command()?
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch == "HEAD" {
            // Detached HEAD state - try to get commit hash
            let hash_output = git_command()?
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
    include_ignored: bool,
) -> (
    HashMap<PathBuf, FileStatus>,
    HashMap<PathBuf, FileStatus>,
    std::collections::HashSet<PathBuf>,
) {
    use std::collections::HashSet;

    let mut statuses = HashMap::new();
    let mut dir_statuses: HashMap<PathBuf, FileStatus> = HashMap::new();
    let mut staged_files: HashSet<PathBuf> = HashSet::new();

    // Get status with porcelain format for machine parsing
    // -uall shows all untracked files (required for per-file status display)
    let Some(mut cmd) = git_command() else {
        return (statuses, dir_statuses, staged_files);
    };
    cmd.args(["status", "--porcelain=v1", "-z", "-uall"]);
    if include_ignored {
        cmd.arg("--ignored");
    }
    let output = cmd.current_dir(repo_root).output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return (statuses, dir_statuses, staged_files),
    };

    let mut records = output.stdout.split(|byte| *byte == 0);
    while let Some(record) = records.next() {
        if record.len() < 4 {
            continue;
        }
        let index_status = record[0] as char;
        let worktree_status = record[1] as char;
        // With -z, paths are raw bytes and rename destinations precede sources.
        let Some(path) = status_path(&record[3..]) else {
            continue;
        };
        if matches!(index_status, 'R' | 'C') || matches!(worktree_status, 'R' | 'C') {
            let _ = records.next();
        }
        let status = parse_status(index_status, worktree_status);

        // Track staged files (index has changes: M, A, D, R, C)
        if matches!(index_status, 'M' | 'A' | 'D' | 'R' | 'C') {
            staged_files.insert(path.clone());
        }

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

    (statuses, dir_statuses, staged_files)
}

// Git emits native path bytes on Unix and UTF-8 paths on Windows.
fn status_path(bytes: &[u8]) -> Option<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Some(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
    }
    #[cfg(not(unix))]
    {
        std::str::from_utf8(bytes).ok().map(PathBuf::from)
    }
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
    use std::fs;
    use std::process::Command as StdCommand;
    use tempfile::tempdir;

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

    #[test]
    fn refresh_reports_only_actual_changes() {
        let temp = tempdir().unwrap();
        let initialized = StdCommand::new("git")
            .args(["init", "--quiet"])
            .current_dir(temp.path())
            .status()
            .is_ok_and(|status| status.success());
        if !initialized {
            return;
        }

        let mut status = GitStatus::detect(temp.path()).unwrap();
        assert!(!status.refresh());

        let changed = temp.path().join("changed.txt");
        fs::write(&changed, "changed").unwrap();
        assert!(status.refresh());
        assert_eq!(
            status.get_status(Path::new("changed.txt")),
            FileStatus::Untracked
        );
        assert!(!status.refresh());
    }
    #[test]
    fn workspace_status_excludes_ignored_entries_and_preserves_visible_changes() {
        let temp = tempdir().unwrap();
        if !StdCommand::new("git")
            .args(["init", "--quiet"])
            .current_dir(temp.path())
            .status()
            .is_ok_and(|status| status.success())
        {
            return;
        }
        fs::write(temp.path().join(".gitignore"), "ignored.txt\n").unwrap();
        fs::write(temp.path().join("ignored.txt"), "ignored").unwrap();
        fs::write(temp.path().join("visible.txt"), "visible").unwrap();
        let full = GitStatus::detect(temp.path()).unwrap();
        assert_eq!(
            full.get_status(Path::new("ignored.txt")),
            FileStatus::Ignored
        );
        let mut workspace = GitStatus::detect_for_workspace(temp.path()).unwrap();
        assert_eq!(
            workspace.get_status(Path::new("ignored.txt")),
            FileStatus::Clean
        );
        assert_eq!(
            workspace.get_status(Path::new("visible.txt")),
            FileStatus::Untracked
        );
        assert!(!workspace.refresh());
        assert_eq!(
            workspace.get_status(Path::new("ignored.txt")),
            FileStatus::Clean
        );
    }

    #[cfg(unix)]
    #[test]
    fn nul_status_handles_unicode_quotes_newlines_and_rename_destinations() {
        let temp = tempdir().unwrap();
        let git = |args: &[&str]| {
            StdCommand::new("git")
                .args(args)
                .current_dir(temp.path())
                .output()
                .unwrap()
        };
        if !git(&["init", "--quiet"]).status.success() {
            return;
        }
        for name in [
            "日本語.txt",
            "quote\"file.txt",
            "line\nfile.txt",
            "space name.txt",
            "arrow -> name.txt",
        ] {
            fs::write(temp.path().join(name), "contents").unwrap();
        }
        let status = GitStatus::detect(temp.path()).unwrap();
        for name in [
            "日本語.txt",
            "quote\"file.txt",
            "line\nfile.txt",
            "space name.txt",
            "arrow -> name.txt",
        ] {
            assert_eq!(
                status.get_status(Path::new(name)),
                FileStatus::Untracked,
                "{name:?}"
            );
        }
        assert!(git(&["add", "."]).status.success());
        assert!(git(&[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "fixture"
        ])
        .status
        .success());
        fs::rename(
            temp.path().join("日本語.txt"),
            temp.path().join("renamed -> 日本語.txt"),
        )
        .unwrap();
        assert!(git(&["add", "-A"]).status.success());
        let status = GitStatus::detect(temp.path()).unwrap();
        assert_eq!(
            status.get_status(Path::new("renamed -> 日本語.txt")),
            FileStatus::Renamed
        );
        fs::write(temp.path().join("space name.txt"), "modified").unwrap();
        let status = GitStatus::detect(temp.path()).unwrap();
        assert_eq!(
            status.get_status(Path::new("space name.txt")),
            FileStatus::Modified
        );
    }
}
