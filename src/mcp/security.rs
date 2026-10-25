//! Path validation and security checks for MCP operations.

use std::path::{Path, PathBuf};

use crate::error::{FileviewError, Result};
use crate::util::utf8_prefix;

/// Maximum length for entry names (prevent DoS from malicious input)
pub const MAX_ENTRY_NAME_LEN: usize = 4096;

pub const MAX_RECURSION_DEPTH: usize = 100;

pub const MAX_BATCH_SIZE: usize = 1000;

/// Validate and resolve a path, ensuring it's within the root directory.
///
/// `path` may be relative or absolute. Returns the canonicalized path, or an
/// error if the path is invalid or outside root.
pub fn validate_path(root: &Path, path: &str) -> Result<PathBuf> {
    let (root_canonical, target) = contained_target(root, path)?;

    match target.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(&root_canonical) {
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

/// Validate that a user-supplied name is a single path component, so create and
/// rename cannot escape the current directory. Rejects empty, `.`/`..`, and any
/// path separator.
pub fn validate_component(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(FileviewError::path(name, "name cannot be empty"));
    }
    if name == "." || name == ".." {
        return Err(FileviewError::path(name, "name cannot be '.' or '..'"));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(FileviewError::path(
            name,
            "name cannot contain a path separator",
        ));
    }
    Ok(())
}

/// Reject a value that an external tool would parse as an option flag (begins
/// with `-`). Defence-in-depth against argument injection like `rg --pre`.
pub fn reject_option_like(value: &str) -> Result<()> {
    if value.starts_with('-') {
        return Err(FileviewError::mcp(format!(
            "argument '{}' may not begin with '-'",
            value
        )));
    }
    Ok(())
}

/// Validate a path for a new file that doesn't exist yet.
///
/// `path` is relative to root. Returns the target path if the parent directory
/// is valid, or an error if the parent is invalid or outside root.
pub fn validate_new_path(root: &Path, path: &str) -> Result<PathBuf> {
    let (root_canonical, target) = contained_target(root, path)?;

    if let Ok(metadata) = std::fs::symlink_metadata(&target) {
        if metadata.file_type().is_symlink() {
            return Err(FileviewError::path(
                target,
                "refusing to write through a symbolic link",
            ));
        }

        let canonical = target
            .canonicalize()
            .map_err(|e| FileviewError::path(&target, format!("invalid path: {}", e)))?;
        if !canonical.starts_with(&root_canonical) {
            return Err(FileviewError::path(
                canonical,
                "path is outside root directory",
            ));
        }
        return Ok(canonical);
    }

    validate_parent(&root_canonical, &target)?;
    Ok(target)
}

/// Validate an existing path for deletion without following a symlink in the
/// final path component. Removing an in-root symlink must remove the link, not
/// the file or directory it points to.
pub fn validate_delete_path(root: &Path, path: &str) -> Result<PathBuf> {
    let (root_canonical, target) = contained_target(root, path)?;
    let metadata = std::fs::symlink_metadata(&target)
        .map_err(|e| FileviewError::path(&target, format!("invalid path: {}", e)))?;

    if metadata.file_type().is_symlink() {
        validate_parent(&root_canonical, &target)?;
        return Ok(target);
    }

    let canonical = target
        .canonicalize()
        .map_err(|e| FileviewError::path(&target, format!("invalid path: {}", e)))?;
    if !canonical.starts_with(&root_canonical) {
        return Err(FileviewError::path(
            canonical,
            "path is outside root directory",
        ));
    }
    Ok(canonical)
}

fn contained_target(root: &Path, path: &str) -> Result<(PathBuf, PathBuf)> {
    let root_canonical = root
        .canonicalize()
        .map_err(|e| FileviewError::path(root, format!("invalid root: {}", e)))?;
    let root_absolute = if root.is_absolute() {
        normalize_path(root)
    } else {
        let current = std::env::current_dir()
            .map_err(|e| FileviewError::path(root, format!("invalid root: {}", e)))?;
        normalize_path(&current.join(root))
    };
    let input = Path::new(path);
    let target = if input.is_absolute() {
        normalize_path(input)
    } else {
        normalize_path(&root_absolute.join(input))
    };

    if !target.starts_with(&root_absolute) && !target.starts_with(&root_canonical) {
        return Err(FileviewError::path(
            target,
            "path is outside root directory",
        ));
    }
    Ok((root_canonical, target))
}

fn validate_parent(root_canonical: &Path, target: &Path) -> Result<()> {
    let mut ancestor = target.parent().unwrap_or(root_canonical);
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or_else(|| {
            FileviewError::path(target, "cannot resolve an existing parent directory")
        })?;
    }

    let canonical = ancestor
        .canonicalize()
        .map_err(|e| FileviewError::path(ancestor, format!("invalid parent path: {}", e)))?;
    if !canonical.starts_with(root_canonical) {
        return Err(FileviewError::path(
            target,
            "parent directory is outside root",
        ));
    }
    Ok(())
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

pub fn is_root(root: &Path, path: &Path) -> bool {
    match (root.canonicalize(), path.canonicalize()) {
        (Ok(r), Ok(p)) => r == p,
        _ => false,
    }
}

/// Truncate a string to `max_len` bytes, slicing on a UTF-8 char boundary so a
/// multibyte name cannot trigger a panic.
pub fn truncate_string(s: String, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", utf8_prefix(&s, max_len.saturating_sub(3)))
    } else {
        s
    }
}

/// Truncate entry name if too long (security measure).
pub fn truncate_entry_name(name: String) -> String {
    truncate_string(name, MAX_ENTRY_NAME_LEN)
}

pub fn validate_batch_size(count: usize) -> Result<()> {
    if count > MAX_BATCH_SIZE {
        return Err(FileviewError::mcp(format!(
            "batch size {} exceeds maximum {}",
            count, MAX_BATCH_SIZE
        )));
    }
    Ok(())
}

/// Check a validated target the way [`is_sensitive_path`] checks a relative
/// path, after resolving symlinks in its existing ancestors.
///
/// A raw argument can look harmless while a symlinked parent points into
/// `.git`, so this looks at where the write or delete would really land. The
/// last component is not resolved, because deleting an in-root symlink removes
/// only the link. A target that cannot be placed under the root is treated as
/// sensitive.
pub fn is_sensitive_target(root: &Path, target: &Path) -> bool {
    let Ok(root_canonical) = root.canonicalize() else {
        return true;
    };
    let (Some(parent), Some(name)) = (target.parent(), target.file_name()) else {
        return true;
    };
    let mut existing = parent;
    let mut rest = vec![name.to_owned()];
    while !existing.exists() {
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_owned());
                existing = parent;
            }
            _ => return true,
        }
    }
    let Ok(mut resolved) = existing.canonicalize() else {
        return true;
    };
    resolved.extend(rest.iter().rev());
    match resolved.strip_prefix(&root_canonical) {
        Ok(relative) => is_sensitive_path(relative),
        Err(_) => true,
    }
}

/// Check if a path is a sensitive file that shouldn't be modified.
///
/// Anything inside a `.git` directory counts, because a planted hook, config,
/// or attributes file runs code on the next git command. The path is rebuilt
/// from its components first, so spellings such as `.git//hooks` or
/// `.git/./config` match like the plain form.
///
/// Each component is also reduced the way Windows resolves names. Windows drops
/// trailing dots and spaces and treats `name:stream` as `name`, so `.git.` and
/// `.git::$INDEX_ALLOCATION` open `.git`, and `GIT~1` is its 8.3 short name.
pub fn is_sensitive_path(path: &Path) -> bool {
    let components: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().to_lowercase()),
            _ => None,
        })
        .flat_map(|s| s.split('\\').map(str::to_owned).collect::<Vec<_>>())
        .map(|s| {
            let name = s.split(':').next().unwrap_or_default();
            name.trim_end_matches(['.', ' ']).to_owned()
        })
        .filter(|s| !s.is_empty())
        .collect();
    if components
        .iter()
        .any(|c| c == ".git" || is_git_short_name(c))
    {
        return true;
    }
    let path_str = components.join("/");

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

fn is_git_short_name(name: &str) -> bool {
    name.strip_prefix("git~")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
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
    fn test_validate_new_absolute_path_within_root() {
        let temp = tempdir().unwrap();
        let target = temp.path().join("new.txt");

        assert!(validate_new_path(temp.path(), target.to_str().unwrap()).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn test_validate_new_path_rejects_symlink_leaf_and_ancestor() {
        use std::os::unix::fs::symlink;

        let root_dir = tempdir().unwrap();
        let outside_dir = tempdir().unwrap();
        let root = root_dir.path().canonicalize().unwrap();
        let outside = outside_dir.path().canonicalize().unwrap();
        fs::write(outside.join("target.txt"), "outside").unwrap();
        symlink(outside.join("target.txt"), root.join("file-link")).unwrap();
        symlink(&outside, root.join("dir-link")).unwrap();

        assert!(validate_new_path(&root, "file-link").is_err());
        assert!(validate_new_path(&root, "dir-link/new.txt").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_validate_delete_path_preserves_symlink_leaf() {
        use std::os::unix::fs::symlink;

        let root_dir = tempdir().unwrap();
        let outside_dir = tempdir().unwrap();
        let root = root_dir.path().canonicalize().unwrap();
        let outside_file = outside_dir.path().join("target.txt");
        fs::write(&outside_file, "outside").unwrap();
        let link = root.join("link");
        symlink(&outside_file, &link).unwrap();

        assert_eq!(validate_delete_path(&root, "link").unwrap(), link);
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
        assert!(is_sensitive_path(Path::new(
            r"project\.git\hooks\pre-commit"
        )));
    }

    #[test]
    fn sensitive_path_ignores_spelling_tricks() {
        for path in [
            ".git//hooks/pre-commit",
            ".git/./config",
            "a/../.git/config",
            ".Git/HOOKS/x",
            ".git",
            ".git/objects/ab/cd",
            "./.env",
        ] {
            assert!(is_sensitive_path(Path::new(path)), "{path}");
        }
        for path in [
            ".git./hooks/pre-commit",
            ".git /config",
            ".git. . ./hooks/x",
            ".git::$INDEX_ALLOCATION/hooks/x",
            "GIT~1/hooks/pre-commit",
            ".env.",
        ] {
            assert!(is_sensitive_path(Path::new(path)), "{path}");
        }
        assert!(!is_sensitive_path(Path::new("notes/git~backup.md")));
        assert!(!is_sensitive_path(Path::new("docs/git-guide.md")));
        assert!(!is_sensitive_path(Path::new(".github/workflows/ci.yml")));
    }

    #[test]
    fn sensitive_target_uses_the_path_inside_the_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("credentials-app");
        fs::create_dir_all(root.join(".git")).unwrap();

        assert!(!is_sensitive_target(&root, &root.join("src/main.rs")));
        assert!(is_sensitive_target(&root, &root.join(".git/new/file")));
        assert!(is_sensitive_target(
            &root,
            &temp.path().join("elsewhere.txt")
        ));
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

    #[test]
    fn test_truncate_string_multibyte_no_panic() {
        // A long string of 3-byte characters must not panic when the cut point
        // lands inside a multibyte codepoint.
        let s = "あ".repeat(3000); // 9000 bytes
        let truncated = truncate_string(s, 100);
        assert!(truncated.len() <= 100);
        assert!(truncated.ends_with("..."));
        // Result must still be valid UTF-8 (guaranteed by String, but assert
        // it round-trips through chars without loss of the boundary).
        assert!(truncated.chars().count() > 0);
    }

    #[test]
    fn test_validate_component_ok() {
        assert!(validate_component("file.txt").is_ok());
        assert!(validate_component("my-dir_2").is_ok());
    }

    #[test]
    fn test_validate_component_rejects_traversal() {
        assert!(validate_component("").is_err());
        assert!(validate_component(".").is_err());
        assert!(validate_component("..").is_err());
        assert!(validate_component("../evil").is_err());
        assert!(validate_component("sub/dir").is_err());
        assert!(validate_component("/abs/path").is_err());
        assert!(validate_component("a\\b").is_err());
    }

    #[test]
    fn test_reject_option_like() {
        assert!(reject_option_like("pattern").is_ok());
        assert!(reject_option_like("foo_bar").is_ok());
        assert!(reject_option_like("--pre").is_err());
        assert!(reject_option_like("-rf").is_err());
    }
}
