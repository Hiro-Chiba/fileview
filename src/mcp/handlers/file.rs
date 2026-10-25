//! File operation handlers

use std::fs;
use std::path::Path;
use std::process::Command;

use super::{error_result, success_result, ToolCallResult, ToolContent};
use crate::mcp::security::{
    is_sensitive_path, is_sensitive_target, truncate_entry_name, validate_delete_path,
    validate_new_path, validate_path,
};

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

/// Maximum tree depth (prevent DoS from unbounded recursion)
const MAX_TREE_DEPTH: usize = 20;

pub fn get_tree(root: &Path, path: Option<&str>, depth: Option<usize>) -> ToolCallResult {
    let target = match path {
        Some(p) => match validate_path(root, p) {
            Ok(path) => path,
            Err(e) => return error_result(&e.to_string()),
        },
        None => root.to_path_buf(),
    };

    // Security: Limit depth to prevent DoS
    let safe_depth = depth
        .map(|d| d.min(MAX_TREE_DEPTH))
        .or(Some(MAX_TREE_DEPTH));

    let mut output = Vec::new();
    if let Err(e) = write_tree(&mut output, &target, safe_depth) {
        return error_result(&format!("Failed to generate tree: {}", e));
    }

    success_result(String::from_utf8_lossy(&output).to_string())
}

fn write_tree<W: std::io::Write>(
    out: &mut W,
    path: &Path,
    depth: Option<usize>,
) -> std::io::Result<()> {
    writeln!(out, "{}", path.display())?;
    crate::integrate::tree::print_tree_recursive_pub(out, path, "", depth, 0, false)
}

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

pub fn write_file(root: &Path, path: &str, content: &str, create_dirs: bool) -> ToolCallResult {
    let target = match validate_new_path(root, path) {
        Ok(p) => p,
        Err(e) => return error_result(&e.to_string()),
    };

    // Refuse sensitive files (a planted git hook would run on the next git
    // command). The raw argument and the resolved target are both checked, and
    // both relative to the root to avoid false positives on the root itself.
    if is_sensitive_path(Path::new(path)) || is_sensitive_target(root, &target) {
        return error_result(&format!("Refusing to write to sensitive path: {}", path));
    }

    if create_dirs {
        if let Some(parent) = target.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return error_result(&format!("Failed to create directories: {}", e));
            }
        }
    }

    match fs::write(&target, content) {
        Ok(_) => {
            let bytes = content.len();
            success_result(format!("Successfully wrote {} bytes to {}", bytes, path))
        }
        Err(e) => error_result(&format!("Failed to write file: {}", e)),
    }
}

pub fn delete_file(root: &Path, path: &str, recursive: bool, use_trash: bool) -> ToolCallResult {
    let canonical = match validate_delete_path(root, path) {
        Ok(p) => p,
        Err(e) => return error_result(&e.to_string()),
    };

    if let Ok(root_canonical) = root.canonicalize() {
        if canonical == root_canonical {
            return error_result("Cannot delete root directory");
        }
    }

    // Refuse sensitive files. Check the relative path to avoid false positives.
    if is_sensitive_path(Path::new(path)) || is_sensitive_target(root, &canonical) {
        return error_result(&format!("Refusing to delete sensitive path: {}", path));
    }

    let is_dir = canonical.is_dir();

    if use_trash {
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

/// Maximum search pattern length (prevent ReDoS)
const MAX_SEARCH_PATTERN_LEN: usize = 500;

pub fn search_code(root: &Path, pattern: &str, path: Option<&str>) -> ToolCallResult {
    // Security: Validate pattern length to prevent ReDoS
    if pattern.len() > MAX_SEARCH_PATTERN_LEN {
        return error_result(&format!(
            "Search pattern too long (max {} chars)",
            MAX_SEARCH_PATTERN_LEN
        ));
    }

    // Security: Reject patterns with null bytes
    if pattern.contains('\0') {
        return error_result("Search pattern contains invalid characters");
    }

    // `--` stops option parsing so a `-`-prefixed pattern (e.g. `rg --pre`)
    // cannot be smuggled in as a flag.
    let (cmd, args) = if Command::new("rg").arg("--version").output().is_ok() {
        ("rg", vec!["-n", "--no-heading", "--", pattern])
    } else {
        ("grep", vec!["-rn", "--", pattern])
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_write_file_refuses_sensitive_path() {
        let temp = tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();

        let result = write_file(&root, ".env", "SECRET=1", false);
        assert_eq!(result.is_error, Some(true));
        assert!(!root.join(".env").exists());
    }

    #[test]
    fn test_write_file_refuses_git_hook() {
        let temp = tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::create_dir_all(root.join(".git/hooks")).unwrap();

        let result = write_file(&root, ".git/hooks/pre-commit", "#!/bin/sh\nevil", true);
        assert_eq!(result.is_error, Some(true));
        assert!(!root.join(".git/hooks/pre-commit").exists());
    }

    #[test]
    fn write_file_refuses_git_paths_however_they_are_spelled() {
        let temp = tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::create_dir_all(root.join(".git/hooks")).unwrap();

        for path in [
            ".git//hooks/pre-commit",
            ".git/./hooks/pre-commit",
            "./.git/hooks/pre-commit",
            "src/../.git/hooks/pre-commit",
            ".GIT/hooks/pre-commit",
            ".git/./config",
            ".git/info/attributes",
        ] {
            let result = write_file(&root, path, "#!/bin/sh\nevil", true);
            assert_eq!(result.is_error, Some(true), "{path} was written");
        }
        assert!(!root.join(".git/hooks/pre-commit").exists());
        assert!(!root.join(".git/config").exists());
        assert!(!root.join(".git/info").exists());
    }

    #[cfg(unix)]
    #[test]
    fn write_file_refuses_a_symlinked_directory_that_points_into_git() {
        let temp = tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::create_dir_all(root.join(".git/hooks")).unwrap();
        std::os::unix::fs::symlink(root.join(".git/hooks"), root.join("tools")).unwrap();

        let result = write_file(&root, "tools/pre-commit", "#!/bin/sh\nevil", false);
        assert_eq!(result.is_error, Some(true));
        assert!(!root.join(".git/hooks/pre-commit").exists());
    }

    #[test]
    fn delete_file_refuses_the_git_directory_itself() {
        let temp = tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::create_dir_all(root.join(".git/hooks")).unwrap();

        for path in [".git", ".git/", "./.git", ".git//hooks"] {
            let result = delete_file(&root, path, true, false);
            assert_eq!(result.is_error, Some(true), "{path} was deleted");
        }
        assert!(root.join(".git/hooks").exists());
    }

    #[test]
    fn test_write_file_allows_normal_path() {
        let temp = tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();

        let result = write_file(&root, "src/main.rs", "fn main() {}", true);
        assert_eq!(result.is_error, None);
        assert!(root.join("src/main.rs").exists());
    }

    #[test]
    fn test_write_file_allows_normal_path_under_sensitive_named_root() {
        // A project directory whose own name contains a trigger word (e.g.
        // "credentials") must not block writes to ordinary files inside it.
        let temp = tempdir().unwrap();
        let root = temp.path().join("credentials-app");
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();

        let result = write_file(&root, "src/main.rs", "fn main() {}", true);
        assert_eq!(result.is_error, None);
        assert!(root.join("src/main.rs").exists());
    }

    #[test]
    fn test_delete_file_refuses_sensitive_path() {
        let temp = tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/config"), "[core]").unwrap();

        let result = delete_file(&root, ".git/config", false, false);
        assert_eq!(result.is_error, Some(true));
        assert!(root.join(".git/config").exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_write_file_refuses_symlink_outside_root() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let target = outside.path().join("target.txt");
        fs::write(&target, "original").unwrap();
        symlink(&target, root.join("alias.txt")).unwrap();

        let result = write_file(&root, "alias.txt", "changed", false);
        assert_eq!(result.is_error, Some(true));
        assert_eq!(fs::read_to_string(target).unwrap(), "original");
    }

    #[cfg(unix)]
    #[test]
    fn test_delete_file_removes_symlink_not_target() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let target = outside.path().join("target.txt");
        fs::write(&target, "keep").unwrap();
        let alias = root.join("alias.txt");
        symlink(&target, &alias).unwrap();

        let result = delete_file(&root, "alias.txt", false, false);
        assert_eq!(result.is_error, None);
        assert!(!alias.exists());
        assert_eq!(fs::read_to_string(target).unwrap(), "keep");
    }
}
