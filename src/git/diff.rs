//! Git diff functionality
//!
//! This module provides functions to get diff output for files in a Git repository.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::operations::find_git_executable;

/// A line in a diff output
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
    /// Added line (prefixed with +)
    Added(String),
    /// Removed line (prefixed with -)
    Removed(String),
    /// Context line (prefixed with space)
    Context(String),
    /// Hunk header (@@...@@)
    HunkHeader(String),
    /// Other line (file header, etc.)
    Other(String),
}

/// A diff hunk (section of changes)
#[derive(Debug, Clone)]
pub struct DiffHunk {
    /// Starting line number in old file
    pub old_start: usize,
    /// Number of lines in old file
    pub old_count: usize,
    /// Starting line number in new file
    pub new_start: usize,
    /// Number of lines in new file
    pub new_count: usize,
    /// Lines in this hunk
    pub lines: Vec<DiffLine>,
}

/// Complete diff output for a file
#[derive(Debug, Clone)]
pub struct FileDiff {
    /// Path to the file
    pub path: PathBuf,
    /// All hunks in the diff
    pub hunks: Vec<DiffHunk>,
    /// Raw lines for display
    pub lines: Vec<DiffLine>,
    /// Total lines added
    pub additions: usize,
    /// Total lines removed
    pub deletions: usize,
}

impl FileDiff {
    /// Check if the diff is empty
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

/// Get the diff for a file
///
/// # Arguments
/// * `repo_root` - The root directory of the git repository
/// * `file` - The absolute path to the file
/// * `staged` - If true, show staged changes (--cached), otherwise working tree changes
///
/// # Returns
/// * `Some(FileDiff)` if the file has changes
/// * `None` if there are no changes or an error occurred
pub fn get_diff(repo_root: &Path, file: &Path, staged: bool) -> Option<FileDiff> {
    let git = find_git_executable()?;

    // Get relative path from repo root
    let relative = file.strip_prefix(repo_root).unwrap_or(file);

    let mut cmd = Command::new(git);
    cmd.arg("diff");

    if staged {
        cmd.arg("--cached");
    }

    cmd.arg("--").arg(relative).current_dir(repo_root);

    let output = cmd.output().ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return None;
    }

    Some(parse_diff(&stdout, file.to_path_buf()))
}

/// Parse diff output into a FileDiff structure
fn parse_diff(diff_output: &str, path: PathBuf) -> FileDiff {
    let mut lines = Vec::new();
    let mut hunks = Vec::new();
    let mut current_hunk: Option<DiffHunk> = None;
    let mut additions = 0;
    let mut deletions = 0;

    for line in diff_output.lines() {
        let diff_line = if line.starts_with('+') && !line.starts_with("+++") {
            additions += 1;
            DiffLine::Added(line[1..].to_string())
        } else if line.starts_with('-') && !line.starts_with("---") {
            deletions += 1;
            DiffLine::Removed(line[1..].to_string())
        } else if line.starts_with("@@") {
            // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@
            if let Some(hunk) = current_hunk.take() {
                hunks.push(hunk);
            }

            let (old_start, old_count, new_start, new_count) = parse_hunk_header(line);
            current_hunk = Some(DiffHunk {
                old_start,
                old_count,
                new_start,
                new_count,
                lines: Vec::new(),
            });

            DiffLine::HunkHeader(line.to_string())
        } else if let Some(stripped) = line.strip_prefix(' ') {
            DiffLine::Context(stripped.to_string())
        } else {
            DiffLine::Other(line.to_string())
        };

        // Add to current hunk if we have one
        if let Some(ref mut hunk) = current_hunk {
            hunk.lines.push(diff_line.clone());
        }

        lines.push(diff_line);
    }

    // Don't forget the last hunk
    if let Some(hunk) = current_hunk {
        hunks.push(hunk);
    }

    FileDiff {
        path,
        hunks,
        lines,
        additions,
        deletions,
    }
}

/// Parse a hunk header to extract line numbers
fn parse_hunk_header(header: &str) -> (usize, usize, usize, usize) {
    // Format: @@ -old_start,old_count +new_start,new_count @@ optional context
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 3 {
        return (1, 0, 1, 0);
    }

    let old_part = parts[1].trim_start_matches('-');
    let new_part = parts[2].trim_start_matches('+');

    let (old_start, old_count) = parse_range(old_part);
    let (new_start, new_count) = parse_range(new_part);

    (old_start, old_count, new_start, new_count)
}

/// Parse a range like "10,5" or "10" into (start, count)
fn parse_range(range: &str) -> (usize, usize) {
    if let Some((start, count)) = range.split_once(',') {
        (start.parse().unwrap_or(1), count.parse().unwrap_or(1))
    } else {
        (range.parse().unwrap_or(1), 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hunk_header() {
        let (old_start, old_count, new_start, new_count) =
            parse_hunk_header("@@ -10,5 +12,7 @@ some context");
        assert_eq!(old_start, 10);
        assert_eq!(old_count, 5);
        assert_eq!(new_start, 12);
        assert_eq!(new_count, 7);
    }

    #[test]
    fn test_parse_hunk_header_single_line() {
        let (old_start, old_count, new_start, new_count) = parse_hunk_header("@@ -5 +7 @@");
        assert_eq!(old_start, 5);
        assert_eq!(old_count, 1);
        assert_eq!(new_start, 7);
        assert_eq!(new_count, 1);
    }

    #[test]
    fn test_parse_range() {
        assert_eq!(parse_range("10,5"), (10, 5));
        assert_eq!(parse_range("10"), (10, 1));
        assert_eq!(parse_range("invalid"), (1, 1));
    }

    #[test]
    fn test_parse_diff() {
        let diff = r#"diff --git a/test.txt b/test.txt
index 1234567..abcdefg 100644
--- a/test.txt
+++ b/test.txt
@@ -1,3 +1,4 @@
 line 1
-old line
+new line
+added line
 line 3
"#;
        let file_diff = parse_diff(diff, PathBuf::from("test.txt"));
        assert_eq!(file_diff.additions, 2);
        assert_eq!(file_diff.deletions, 1);
        assert_eq!(file_diff.hunks.len(), 1);
    }

    #[test]
    fn test_diff_line_types() {
        let diff = r#"@@ -1,2 +1,2 @@
 context
-removed
+added
"#;
        let file_diff = parse_diff(diff, PathBuf::from("test.txt"));

        assert!(matches!(file_diff.lines[0], DiffLine::HunkHeader(_)));
        assert!(matches!(file_diff.lines[1], DiffLine::Context(_)));
        assert!(matches!(file_diff.lines[2], DiffLine::Removed(_)));
        assert!(matches!(file_diff.lines[3], DiffLine::Added(_)));
    }

    #[test]
    fn test_parse_diff_multiple_hunks() {
        let diff = r#"diff --git a/test.txt b/test.txt
--- a/test.txt
+++ b/test.txt
@@ -1,3 +1,3 @@
 line 1
-old 1
+new 1
 line 3
@@ -10,3 +10,3 @@
 line 10
-old 2
+new 2
 line 12
"#;
        let file_diff = parse_diff(diff, PathBuf::from("test.txt"));
        assert_eq!(file_diff.hunks.len(), 2);
        assert_eq!(file_diff.additions, 2);
        assert_eq!(file_diff.deletions, 2);
    }

    #[test]
    fn test_parse_diff_only_additions() {
        let diff = r#"@@ -1,2 +1,4 @@
 existing
+new line 1
+new line 2
 more existing
"#;
        let file_diff = parse_diff(diff, PathBuf::from("test.txt"));
        assert_eq!(file_diff.additions, 2);
        assert_eq!(file_diff.deletions, 0);
    }

    #[test]
    fn test_parse_diff_only_deletions() {
        let diff = r#"@@ -1,4 +1,2 @@
 existing
-removed 1
-removed 2
 more existing
"#;
        let file_diff = parse_diff(diff, PathBuf::from("test.txt"));
        assert_eq!(file_diff.additions, 0);
        assert_eq!(file_diff.deletions, 2);
    }

    #[test]
    fn test_parse_diff_empty() {
        let file_diff = parse_diff("", PathBuf::from("test.txt"));
        assert_eq!(file_diff.lines.len(), 0);
        assert_eq!(file_diff.hunks.len(), 0);
    }

    #[test]
    fn test_parse_hunk_header_no_context() {
        let (old_start, old_count, new_start, new_count) = parse_hunk_header("@@ -1,5 +1,7 @@");
        assert_eq!(old_start, 1);
        assert_eq!(old_count, 5);
        assert_eq!(new_start, 1);
        assert_eq!(new_count, 7);
    }

    #[test]
    fn test_parse_hunk_header_malformed() {
        // Too few parts
        let (old_start, old_count, new_start, new_count) = parse_hunk_header("@@");
        assert_eq!(old_start, 1);
        assert_eq!(old_count, 0);
        assert_eq!(new_start, 1);
        assert_eq!(new_count, 0);
    }

    #[test]
    fn test_parse_range_zero_count() {
        assert_eq!(parse_range("5,0"), (5, 0));
    }

    #[test]
    fn test_diff_hunk_structure() {
        let diff = r#"@@ -5,3 +5,4 @@ function foo()
 context before
-deleted
+added
+another added
 context after
"#;
        let file_diff = parse_diff(diff, PathBuf::from("test.txt"));
        assert_eq!(file_diff.hunks.len(), 1);

        let hunk = &file_diff.hunks[0];
        assert_eq!(hunk.old_start, 5);
        assert_eq!(hunk.old_count, 3);
        assert_eq!(hunk.new_start, 5);
        assert_eq!(hunk.new_count, 4);
    }

    #[test]
    fn test_diff_line_content() {
        let diff = r#"@@ -1,1 +1,1 @@
-hello world
+goodbye world
"#;
        let file_diff = parse_diff(diff, PathBuf::from("test.txt"));

        // Check the content of lines
        if let DiffLine::Removed(content) = &file_diff.lines[1] {
            assert_eq!(content, "hello world");
        } else {
            panic!("Expected Removed line");
        }

        if let DiffLine::Added(content) = &file_diff.lines[2] {
            assert_eq!(content, "goodbye world");
        } else {
            panic!("Expected Added line");
        }
    }

    #[test]
    fn test_diff_context_line() {
        let diff = r#"@@ -1,3 +1,3 @@
 unchanged line
-old
+new
"#;
        let file_diff = parse_diff(diff, PathBuf::from("test.txt"));

        if let DiffLine::Context(content) = &file_diff.lines[1] {
            assert_eq!(content, "unchanged line");
        } else {
            panic!("Expected Context line");
        }
    }
}
