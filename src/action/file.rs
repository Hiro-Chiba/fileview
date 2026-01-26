//! File operations (create, rename, delete, copy)

use std::path::{Path, PathBuf};

/// Create a new file
pub fn create_file(parent: &Path, name: &str) -> anyhow::Result<PathBuf> {
    let path = parent.join(name);
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    Ok(path)
}

/// Create a new directory
pub fn create_dir(parent: &Path, name: &str) -> anyhow::Result<PathBuf> {
    let path = parent.join(name);
    std::fs::create_dir(&path)?;
    Ok(path)
}

/// Rename a file or directory
pub fn rename(path: &Path, new_name: &str) -> anyhow::Result<PathBuf> {
    let new_path = path.parent().unwrap_or(Path::new("")).join(new_name);
    std::fs::rename(path, &new_path)?;
    Ok(new_path)
}

/// Delete a file or directory
pub fn delete(path: &Path) -> anyhow::Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Copy a file to a destination directory
pub fn copy_to(src: &Path, dest_dir: &Path) -> anyhow::Result<PathBuf> {
    let file_name = src.file_name().ok_or_else(|| anyhow::anyhow!("Invalid source path"))?;
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
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let parent = path.parent().unwrap_or(Path::new(""));

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
