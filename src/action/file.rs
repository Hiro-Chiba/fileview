//! File operations (create, rename, delete, copy)

use std::path::{Path, PathBuf};

/// Create a new file
pub fn create_file(parent: &Path, name: &str) -> anyhow::Result<PathBuf> {
    let path = parent.join(name);
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| anyhow::anyhow!("Failed to create file '{}': {}", path.display(), e))?;
    Ok(path)
}

/// Create a new directory
pub fn create_dir(parent: &Path, name: &str) -> anyhow::Result<PathBuf> {
    let path = parent.join(name);
    std::fs::create_dir(&path)
        .map_err(|e| anyhow::anyhow!("Failed to create directory '{}': {}", path.display(), e))?;
    Ok(path)
}

/// Rename a file or directory
pub fn rename(path: &Path, new_name: &str) -> anyhow::Result<PathBuf> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("Cannot determine parent directory for '{}'", path.display())
        })?;
    let new_path = parent.join(new_name);
    std::fs::rename(path, &new_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to rename '{}' to '{}': {}",
            path.display(),
            new_name,
            e
        )
    })?;
    Ok(new_path)
}

/// Delete a file or directory
pub fn delete(path: &Path) -> anyhow::Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)
            .map_err(|e| anyhow::anyhow!("Failed to delete '{}': {}", path.display(), e))?;
    } else {
        std::fs::remove_file(path)
            .map_err(|e| anyhow::anyhow!("Failed to delete '{}': {}", path.display(), e))?;
    }
    Ok(())
}

/// Copy a file to a destination directory
pub fn copy_to(src: &Path, dest_dir: &Path) -> anyhow::Result<PathBuf> {
    let file_name = src
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Cannot copy '{}': no filename", src.display()))?;
    let dest = get_unique_path(&dest_dir.join(file_name));

    if src.is_dir() {
        copy_dir_recursive(src, &dest)?;
    } else {
        std::fs::copy(src, &dest)?;
    }
    Ok(dest)
}

/// Get a unique path by appending _1, _2, etc. if needed
fn get_unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    let mut counter = 1;
    loop {
        let new_name = format!("{}_{}{}", stem, counter, ext);
        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
        counter += 1;
    }
}

/// Copy directory recursively
fn copy_dir_recursive(src: &Path, dest: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_create_file() {
        let temp = TempDir::new().unwrap();
        let result = create_file(temp.path(), "test.txt").unwrap();

        assert!(result.exists());
        assert!(result.is_file());
        assert_eq!(result.file_name().unwrap(), "test.txt");
    }

    #[test]
    fn test_create_file_already_exists() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("test.txt"), "existing").unwrap();

        let result = create_file(temp.path(), "test.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_dir() {
        let temp = TempDir::new().unwrap();
        let result = create_dir(temp.path(), "subdir").unwrap();

        assert!(result.exists());
        assert!(result.is_dir());
        assert_eq!(result.file_name().unwrap(), "subdir");
    }

    #[test]
    fn test_rename_file() {
        let temp = TempDir::new().unwrap();
        let original = temp.path().join("original.txt");
        fs::write(&original, "content").unwrap();

        let result = rename(&original, "renamed.txt").unwrap();

        assert!(!original.exists());
        assert!(result.exists());
        assert_eq!(result.file_name().unwrap(), "renamed.txt");
        assert_eq!(fs::read_to_string(&result).unwrap(), "content");
    }

    #[test]
    fn test_rename_dir() {
        let temp = TempDir::new().unwrap();
        let original = temp.path().join("original_dir");
        fs::create_dir(&original).unwrap();
        fs::write(original.join("file.txt"), "content").unwrap();

        let result = rename(&original, "renamed_dir").unwrap();

        assert!(!original.exists());
        assert!(result.exists());
        assert!(result.is_dir());
        assert!(result.join("file.txt").exists());
    }

    #[test]
    fn test_delete_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("to_delete.txt");
        fs::write(&file, "content").unwrap();

        delete(&file).unwrap();
        assert!(!file.exists());
    }

    #[test]
    fn test_delete_dir() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("to_delete");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("file.txt"), "content").unwrap();

        delete(&dir).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn test_copy_to_file() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("source.txt");
        fs::write(&src, "content").unwrap();

        let dest_dir = temp.path().join("dest");
        fs::create_dir(&dest_dir).unwrap();

        let result = copy_to(&src, &dest_dir).unwrap();

        assert!(src.exists()); // Original still exists
        assert!(result.exists());
        assert_eq!(fs::read_to_string(&result).unwrap(), "content");
    }

    #[test]
    fn test_copy_to_dir() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("source_dir");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("file.txt"), "content").unwrap();

        let dest_dir = temp.path().join("dest");
        fs::create_dir(&dest_dir).unwrap();

        let result = copy_to(&src, &dest_dir).unwrap();

        assert!(src.exists()); // Original still exists
        assert!(result.exists());
        assert!(result.is_dir());
        assert!(result.join("file.txt").exists());
    }

    #[test]
    fn test_copy_to_unique_name() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("file.txt");
        fs::write(&src, "content").unwrap();

        // Create existing file with same name in dest
        fs::write(temp.path().join("file.txt"), "existing").unwrap();

        let result = copy_to(&src, temp.path()).unwrap();

        // Should create file_1.txt
        assert_eq!(result.file_name().unwrap(), "file_1.txt");
        assert!(result.exists());
    }
}
