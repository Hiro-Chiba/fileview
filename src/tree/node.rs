//! Tree entry (node) definition

use std::path::PathBuf;

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
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let is_dir = path.is_dir();

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
    pub fn load_children(&mut self, show_hidden: bool) -> anyhow::Result<()> {
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
            .map(|e| TreeEntry::new(e.path(), self.depth + 1))
            .collect();

        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        self.children = entries;
        Ok(())
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
