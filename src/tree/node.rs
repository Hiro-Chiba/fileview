//! Tree entry (node) definition

use std::path::PathBuf;
use std::time::SystemTime;

use crate::core::SortMode;

/// A single entry in the file tree
#[derive(Debug, Clone)]
pub struct TreeEntry {
    /// Full path to the entry
    pub path: PathBuf,
    /// Display name
    pub name: String,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Depth in the tree (0 = root)
    pub depth: usize,
    /// Whether directory is expanded
    pub expanded: bool,
    /// Child entries (directories only)
    children: Vec<TreeEntry>,
}

impl TreeEntry {
    /// Create a new tree entry
    pub fn new(path: PathBuf, depth: usize) -> Self {
        let is_dir = path.is_dir();
        Self::new_with_type(path, depth, is_dir)
    }

    /// Create a new tree entry with pre-computed is_dir value
    ///
    /// This avoids an extra stat() call when is_dir is already known
    /// (e.g., from DirEntry::file_type()).
    pub fn new_with_type(path: PathBuf, depth: usize, is_dir: bool) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        Self {
            path,
            name,
            is_dir,
            depth,
            expanded: false,
            children: Vec::new(),
        }
    }

    /// Check if this entry is expanded
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Get children (immutable)
    pub fn children(&self) -> &[TreeEntry] {
        &self.children
    }

    /// Get children (mutable)
    pub fn children_mut(&mut self) -> &mut Vec<TreeEntry> {
        &mut self.children
    }

    /// Toggle expanded state
    pub fn toggle_expanded(&mut self) {
        if self.is_dir {
            self.expanded = !self.expanded;
        }
    }

    /// Set expanded state
    pub fn set_expanded(&mut self, expanded: bool) {
        if self.is_dir {
            self.expanded = expanded;
        }
    }

    /// Load children from filesystem
    ///
    /// Uses `DirEntry::file_type()` to avoid extra stat() calls for better performance.
    /// For symlinks, falls back to `path.is_dir()` to follow the link.
    pub fn load_children(&mut self, show_hidden: bool) -> anyhow::Result<()> {
        self.load_children_with_sort(show_hidden, SortMode::Name)
    }

    /// Load children from filesystem with specified sort mode
    ///
    /// Uses `DirEntry::file_type()` to avoid extra stat() calls for better performance.
    /// For symlinks, falls back to `path.is_dir()` to follow the link.
    pub fn load_children_with_sort(
        &mut self,
        show_hidden: bool,
        sort_mode: SortMode,
    ) -> anyhow::Result<()> {
        if !self.is_dir {
            return Ok(());
        }

        self.children.clear();
        let mut entries: Vec<_> = std::fs::read_dir(&self.path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                if show_hidden {
                    true
                } else {
                    !e.file_name().to_string_lossy().starts_with('.')
                }
            })
            .map(|e| {
                // Use file_type() from DirEntry to avoid extra stat() call
                // For symlinks, follow the link to determine if it points to a directory
                let is_dir = e
                    .file_type()
                    .map(|t| {
                        if t.is_symlink() {
                            // Follow symlink to check if target is directory
                            e.path().is_dir()
                        } else {
                            t.is_dir()
                        }
                    })
                    .unwrap_or(false);
                TreeEntry::new_with_type(e.path(), self.depth + 1, is_dir)
            })
            .collect();

        // Sort: directories first, then by sort mode
        sort_entries(&mut entries, sort_mode);

        self.children = entries;
        Ok(())
    }
}

/// Sort entries with directories first, then by sort mode
///
/// For Size and Date modes, sort keys are precomputed once (O(N) stat calls)
/// instead of being fetched on every comparison (O(N log N) stat calls).
pub fn sort_entries(entries: &mut [TreeEntry], sort_mode: SortMode) {
    match sort_mode {
        SortMode::Name => {
            entries.sort_by_cached_key(|entry| (!entry.is_dir, entry.name.to_lowercase()));
        }
        SortMode::Size => {
            let keys: Vec<u64> = entries
                .iter()
                .map(|e| {
                    if e.is_dir {
                        0
                    } else {
                        e.path.metadata().map(|m| m.len()).unwrap_or(0)
                    }
                })
                .collect();
            let mut idx: Vec<usize> = (0..entries.len()).collect();
            idx.sort_by(|&i, &j| {
                match (entries[i].is_dir, entries[j].is_dir) {
                    (true, false) => return std::cmp::Ordering::Less,
                    (false, true) => return std::cmp::Ordering::Greater,
                    (true, true) => {
                        return entries[i]
                            .name
                            .to_lowercase()
                            .cmp(&entries[j].name.to_lowercase());
                    }
                    _ => {}
                }
                keys[j].cmp(&keys[i]) // Descending (largest first)
            });
            apply_index_permutation(entries, idx);
        }
        SortMode::Date => {
            let keys: Vec<Option<SystemTime>> = entries
                .iter()
                .map(|e| e.path.metadata().and_then(|m| m.modified()).ok())
                .collect();
            let mut idx: Vec<usize> = (0..entries.len()).collect();
            idx.sort_by(|&i, &j| {
                match (entries[i].is_dir, entries[j].is_dir) {
                    (true, false) => return std::cmp::Ordering::Less,
                    (false, true) => return std::cmp::Ordering::Greater,
                    _ => {}
                }
                keys[j].cmp(&keys[i]) // Descending (newest first)
            });
            apply_index_permutation(entries, idx);
        }
    }
}

/// Reorder elements in-place according to the given index permutation.
fn apply_index_permutation<T>(entries: &mut [T], mut indices: Vec<usize>) {
    let mut target_positions = vec![0; indices.len()];
    for (new_index, &old_index) in indices.iter().enumerate() {
        target_positions[old_index] = new_index;
    }
    indices = target_positions;

    for i in 0..entries.len() {
        while indices[i] != i {
            let j = indices[i];
            entries.swap(i, j);
            indices.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("subdir")).unwrap();
        fs::write(temp.path().join("file.txt"), "test").unwrap();
        fs::write(temp.path().join(".hidden"), "hidden").unwrap();
        fs::write(temp.path().join("subdir/nested.txt"), "nested").unwrap();
        temp
    }

    #[test]
    fn test_tree_entry_new_file() {
        let temp = setup_test_dir();
        let file_path = temp.path().join("file.txt");
        let entry = TreeEntry::new(file_path.clone(), 0);

        assert_eq!(entry.name, "file.txt");
        assert!(!entry.is_dir);
        assert_eq!(entry.depth, 0);
        assert!(!entry.expanded);
        assert!(entry.children().is_empty());
    }

    #[test]
    fn test_tree_entry_new_dir() {
        let temp = setup_test_dir();
        let dir_path = temp.path().join("subdir");
        let entry = TreeEntry::new(dir_path.clone(), 1);

        assert_eq!(entry.name, "subdir");
        assert!(entry.is_dir);
        assert_eq!(entry.depth, 1);
        assert!(!entry.expanded);
    }

    #[test]
    fn test_toggle_expanded_dir() {
        let temp = setup_test_dir();
        let dir_path = temp.path().join("subdir");
        let mut entry = TreeEntry::new(dir_path, 0);

        assert!(!entry.is_expanded());
        entry.toggle_expanded();
        assert!(entry.is_expanded());
        entry.toggle_expanded();
        assert!(!entry.is_expanded());
    }

    #[test]
    fn test_toggle_expanded_file() {
        let temp = setup_test_dir();
        let file_path = temp.path().join("file.txt");
        let mut entry = TreeEntry::new(file_path, 0);

        assert!(!entry.is_expanded());
        entry.toggle_expanded(); // Should have no effect on files
        assert!(!entry.is_expanded());
    }

    #[test]
    fn test_load_children() {
        let temp = setup_test_dir();
        let mut entry = TreeEntry::new(temp.path().to_path_buf(), 0);

        entry.load_children(false).unwrap();

        // Should have 2 children (subdir and file.txt, not .hidden)
        assert_eq!(entry.children().len(), 2);

        // Directories should come first
        assert!(entry.children()[0].is_dir);
        assert_eq!(entry.children()[0].name, "subdir");
        assert!(!entry.children()[1].is_dir);
        assert_eq!(entry.children()[1].name, "file.txt");
    }

    #[test]
    fn test_load_children_show_hidden() {
        let temp = setup_test_dir();
        let mut entry = TreeEntry::new(temp.path().to_path_buf(), 0);

        entry.load_children(true).unwrap();

        // Should have 3 children (subdir, file.txt, and .hidden)
        assert_eq!(entry.children().len(), 3);
    }

    #[test]
    fn test_size_sort_orders_directories_then_largest_files() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("small.txt"), "1").unwrap();
        fs::write(temp.path().join("large.txt"), "12345").unwrap();
        fs::write(temp.path().join("medium.txt"), "123").unwrap();
        fs::create_dir(temp.path().join("dir")).unwrap();
        let mut entry = TreeEntry::new(temp.path().to_path_buf(), 0);

        entry
            .load_children_with_sort(false, SortMode::Size)
            .unwrap();
        let names: Vec<&str> = entry.children().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["dir", "large.txt", "medium.txt", "small.txt"]);
    }

    #[test]
    fn test_index_permutation_handles_three_cycle() {
        let mut values = ["first", "second", "third"];
        apply_index_permutation(&mut values, vec![2, 0, 1]);
        assert_eq!(values, ["third", "first", "second"]);
    }

    #[test]
    fn test_set_expanded() {
        let temp = setup_test_dir();
        let dir_path = temp.path().join("subdir");
        let mut entry = TreeEntry::new(dir_path, 0);

        entry.set_expanded(true);
        assert!(entry.is_expanded());
        entry.set_expanded(false);
        assert!(!entry.is_expanded());
    }
}
