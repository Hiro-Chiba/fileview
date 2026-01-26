//! Tree navigator - handles tree traversal and flattening

use std::path::{Path, PathBuf};

use super::TreeEntry;

/// Manages file tree navigation
pub struct TreeNavigator {
    /// Root entry
    root: TreeEntry,
    /// Whether to show hidden files
    show_hidden: bool,
}

impl TreeNavigator {
    /// Create a new navigator for the given root path
    pub fn new(root_path: &Path, show_hidden: bool) -> anyhow::Result<Self> {
        let mut root = TreeEntry::new(root_path.to_path_buf(), 0);
        root.load_children(show_hidden)?;
        root.set_expanded(true);

        Ok(Self { root, show_hidden })
    }

    /// Get root entry
    pub fn root(&self) -> &TreeEntry {
        &self.root
    }

    /// Flatten the tree into a list of visible entries
    pub fn visible_entries(&self) -> Vec<&TreeEntry> {
        let mut entries = Vec::new();
        self.collect_visible(&self.root, &mut entries);
        entries
    }

    /// Recursively collect visible entries
    fn collect_visible<'a>(&'a self, entry: &'a TreeEntry, out: &mut Vec<&'a TreeEntry>) {
        out.push(entry);
        if entry.is_expanded() {
            for child in entry.children() {
                self.collect_visible(child, out);
            }
        }
    }

    /// Get total count of visible entries
    pub fn visible_count(&self) -> usize {
        self.visible_entries().len()
    }

    /// Toggle expand/collapse for entry at path
    pub fn toggle_expand(&mut self, path: &Path) -> anyhow::Result<()> {
        let show_hidden = self.show_hidden;
        if let Some(entry) = self.find_entry_mut(path) {
            if entry.is_dir && !entry.is_expanded() && entry.children().is_empty() {
                entry.load_children(show_hidden)?;
            }
            entry.toggle_expanded();
        }
        Ok(())
    }

    /// Expand entry at path
    pub fn expand(&mut self, path: &Path) -> anyhow::Result<()> {
        let show_hidden = self.show_hidden;
        if let Some(entry) = self.find_entry_mut(path) {
            if entry.is_dir && entry.children().is_empty() {
                entry.load_children(show_hidden)?;
            }
            entry.set_expanded(true);
        }
        Ok(())
    }

    /// Collapse entry at path
    pub fn collapse(&mut self, path: &Path) {
        if let Some(entry) = self.find_entry_mut(path) {
            entry.set_expanded(false);
        }
    }

    /// Reload tree from filesystem
    pub fn reload(&mut self) -> anyhow::Result<()> {
        let expanded_paths = self.collect_expanded_paths();
        self.root.load_children(self.show_hidden)?;
        self.restore_expanded(&expanded_paths)?;
        Ok(())
    }

    /// Set show_hidden and reload
    pub fn set_show_hidden(&mut self, show: bool) -> anyhow::Result<()> {
        self.show_hidden = show;
        self.reload()
    }

    /// Find entry by path (mutable)
    fn find_entry_mut(&mut self, path: &Path) -> Option<&mut TreeEntry> {
        Self::find_in_entry_mut(&mut self.root, path)
    }

    fn find_in_entry_mut<'a>(entry: &'a mut TreeEntry, path: &Path) -> Option<&'a mut TreeEntry> {
        if entry.path == path {
            return Some(entry);
        }
        for child in entry.children_mut() {
            if let Some(found) = Self::find_in_entry_mut(child, path) {
                return Some(found);
            }
        }
        None
    }

    /// Collect paths of all expanded entries
    fn collect_expanded_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        self.collect_expanded_in(&self.root, &mut paths);
        paths
    }

    fn collect_expanded_in(&self, entry: &TreeEntry, paths: &mut Vec<PathBuf>) {
        if entry.is_expanded() {
            paths.push(entry.path.clone());
            for child in entry.children() {
                self.collect_expanded_in(child, paths);
            }
        }
    }

    /// Restore expanded state from paths
    fn restore_expanded(&mut self, paths: &[PathBuf]) -> anyhow::Result<()> {
        for path in paths {
            self.expand(path)?;
        }
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
        fs::create_dir(temp.path().join("dir_a")).unwrap();
        fs::create_dir(temp.path().join("dir_b")).unwrap();
        fs::write(temp.path().join("file.txt"), "test").unwrap();
        fs::write(temp.path().join("dir_a/nested.txt"), "nested").unwrap();
        fs::create_dir(temp.path().join("dir_a/subdir")).unwrap();
        temp
    }

    #[test]
    fn test_navigator_new() {
        let temp = setup_test_dir();
        let nav = TreeNavigator::new(temp.path(), false).unwrap();

        // Root should be expanded
        assert!(nav.root().is_expanded());
        // Should have 3 children (dir_a, dir_b, file.txt)
        assert_eq!(nav.root().children().len(), 3);
    }

    #[test]
    fn test_visible_entries() {
        let temp = setup_test_dir();
        let nav = TreeNavigator::new(temp.path(), false).unwrap();

        let visible = nav.visible_entries();
        // Root + 3 children = 4 entries (children not expanded yet)
        assert_eq!(visible.len(), 4);
    }

    #[test]
    fn test_expand_collapse() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        let dir_a_path = temp.path().join("dir_a");

        // Initially collapsed
        let count_before = nav.visible_count();

        // Expand dir_a
        nav.expand(&dir_a_path).unwrap();
        let count_after = nav.visible_count();

        // Should have more visible entries now
        assert!(count_after > count_before);

        // Collapse dir_a
        nav.collapse(&dir_a_path);
        assert_eq!(nav.visible_count(), count_before);
    }

    #[test]
    fn test_toggle_expand() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        let dir_a_path = temp.path().join("dir_a");
        let count_collapsed = nav.visible_count();

        nav.toggle_expand(&dir_a_path).unwrap();
        let count_expanded = nav.visible_count();
        assert!(count_expanded > count_collapsed);

        nav.toggle_expand(&dir_a_path).unwrap();
        assert_eq!(nav.visible_count(), count_collapsed);
    }

    #[test]
    fn test_set_show_hidden() {
        let temp = setup_test_dir();
        // Create a hidden file
        fs::write(temp.path().join(".hidden"), "hidden").unwrap();

        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();
        let count_without_hidden = nav.visible_count();

        nav.set_show_hidden(true).unwrap();
        let count_with_hidden = nav.visible_count();

        assert!(count_with_hidden > count_without_hidden);
    }

    #[test]
    fn test_reload() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        let count_before = nav.visible_count();

        // Add a new file
        fs::write(temp.path().join("new_file.txt"), "new").unwrap();

        // Reload should pick up the new file
        nav.reload().unwrap();
        let count_after = nav.visible_count();

        assert_eq!(count_after, count_before + 1);
    }
}
