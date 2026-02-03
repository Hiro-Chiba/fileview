//! MCP security utilities
//!
//! Path validation and security checks for MCP operations.

use std::path::{Path, PathBuf};

use crate::error::{FileviewError, Result};

/// Maximum length for entry names (prevent DoS from malicious input)
pub const MAX_ENTRY_NAME_LEN: usize = 4096;

/// Maximum depth for recursive operations
pub const MAX_RECURSION_DEPTH: usize = 100;

/// Maximum number of files to process in a single operation
pub const MAX_BATCH_SIZE: usize = 1000;

/// Validate and resolve a path, ensuring it's within the root directory.
///
/// # Arguments
/// * `root` - The root directory that all paths must be within
/// * `path` - The path to validate (can be relative or absolute)
///
/// # Returns
/// * `Ok(PathBuf)` - The canonicalized path if valid
/// * `Err(FileviewError)` - If the path is invalid or outside root
pub fn validate_path(root: &Path, path: &str) -> Result<PathBuf> {
    let target = root.join(path);

    match target.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(root) {
                return Err(FileviewError::path(
                    canonical,
                    "path is outside root directory",
                ));
            }
            Ok(canonical)
        }
        Err(e) => Err(FileviewError::path(target, format!("invalid path: {}", e))),
    }
}

/// Validate a path for a new file that doesn't exist yet.
///
/// # Arguments
/// * `root` - The root directory that all paths must be within
/// * `path` - The path to validate (relative to root)
///
/// # Returns
/// * `Ok(PathBuf)` - The target path if the parent directory is valid
/// * `Err(FileviewError)` - If the parent is invalid or outside root
pub fn validate_new_path(root: &Path, path: &str) -> Result<PathBuf> {
    let target = root.join(path);
    let parent = target.parent().unwrap_or(root);

    // For new files, check if parent exists and is within root
    if parent.exists() {
        if let Ok(canonical_parent) = parent.canonicalize() {
            if !canonical_parent.starts_with(root) {
                return Err(FileviewError::path(
                    target,
                    "parent directory is outside root",
                ));
            }
        }
    } else {
        // Parent doesn't exist - check if path traverses outside root
        // by checking if normalized path stays within root
        let normalized = normalize_path(&target);
        let root_normalized = normalize_path(root);
        if !normalized.starts_with(&root_normalized) {
            return Err(FileviewError::path(
                target,
                "path would be outside root directory",
            ));
        }
    }

    Ok(target)
}

/// Normalize a path without requiring it to exist.
/// Removes `.` and `..` components.
fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            _ => normalized.push(component),
        }
    }
    normalized
}

/// Check if a path is the root directory itself.
pub fn is_root(root: &Path, path: &Path) -> bool {
    match (root.canonicalize(), path.canonicalize()) {
        (Ok(r), Ok(p)) => r == p,
        _ => false,
    }
}

/// Truncate a string if it exceeds the maximum length.
pub fn truncate_string(s: String, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    } else {
        s
    }
}

/// Truncate entry name if too long (security measure).
pub fn truncate_entry_name(name: String) -> String {
    truncate_string(name, MAX_ENTRY_NAME_LEN)
}

/// Validate batch operation size.
pub fn validate_batch_size(count: usize) -> Result<()> {
    if count > MAX_BATCH_SIZE {
        return Err(FileviewError::mcp(format!(
            "batch size {} exceeds maximum {}",
            count, MAX_BATCH_SIZE
        )));
    }
    Ok(())
}

/// Check if a path is a sensitive file that shouldn't be modified.
pub fn is_sensitive_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();

    // Check for sensitive file patterns
    let sensitive_patterns = [
        ".git/config",
        ".git/hooks",
        ".ssh",
        ".gnupg",
        ".env",
        "id_rsa",
        "id_ed25519",
        ".npmrc",
        ".pypirc",
        "credentials",
        "secrets",
        ".aws/credentials",
    ];

    for pattern in &sensitive_patterns {
        if path_str.contains(pattern) {
            return true;
        }
    }

    false
}

/// Sanitize a filename to remove potentially dangerous characters.
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_' || *c == ' ')
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_validate_path_within_root() {
        let temp = tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::create_dir(root.join("subdir")).unwrap();
        fs::write(root.join("subdir/file.txt"), "test").unwrap();

        let result = validate_path(&root, "subdir/file.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_outside_root() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        let result = validate_path(root, "../outside");
        assert!(result.is_err());
    }

    #[test]
    fn test_truncate_entry_name() {
        let short = "short.txt".to_string();
        assert_eq!(truncate_entry_name(short.clone()), short);

        let long = "a".repeat(5000);
        let truncated = truncate_entry_name(long);
        assert!(truncated.len() <= MAX_ENTRY_NAME_LEN);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_is_sensitive_path() {
        assert!(is_sensitive_path(Path::new("/home/user/.ssh/id_rsa")));
        assert!(is_sensitive_path(Path::new("/project/.env")));
        assert!(is_sensitive_path(Path::new("/repo/.git/config")));
        assert!(!is_sensitive_path(Path::new("/project/src/main.rs")));
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("normal.txt"), "normal.txt");
        assert_eq!(sanitize_filename("file<>name.txt"), "filename.txt");
        // Dots are kept, slashes are removed
        assert_eq!(sanitize_filename("../../etc/passwd"), "....etcpasswd");
    }

    #[test]
    fn test_validate_batch_size() {
        assert!(validate_batch_size(100).is_ok());
        assert!(validate_batch_size(1000).is_ok());
        assert!(validate_batch_size(1001).is_err());
    }
}
