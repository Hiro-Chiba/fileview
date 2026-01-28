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

    /// Reveal a path by expanding all parent directories
    ///
    /// This makes the target path visible in the tree by expanding
    /// all ancestor directories from the root to the target.
    pub fn reveal_path(&mut self, target: &Path) -> anyhow::Result<()> {
        // Collect ancestors from root to target
        let root_path = self.root.path.clone();
        let mut ancestors = Vec::new();

        // Build list of ancestors that need to be expanded
        if let Ok(relative) = target.strip_prefix(&root_path) {
            let mut current = root_path.clone();
            for component in relative.components() {
                current = current.join(component);
                if current != *target {
                    // Only expand directories, not the target itself
                    ancestors.push(current.clone());
                }
            }
        }

        // Expand each ancestor in order
        for ancestor in ancestors {
            self.expand(&ancestor)?;
        }

        Ok(())
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

    #[test]
    fn test_reveal_path_nested() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        // nested.txt is in dir_a, which needs to be expanded
        let target = temp.path().join("dir_a/nested.txt");

        // Initially, nested.txt should not be visible
        let before = nav.visible_entries();
        assert!(
            !before.iter().any(|e| e.path == target),
            "Target should not be visible initially"
        );

        // Reveal the path
        nav.reveal_path(&target).unwrap();

        // Now it should be visible
        let after = nav.visible_entries();
        assert!(
            after.iter().any(|e| e.path == target),
            "Target should be visible after reveal"
        );
    }

    #[test]
    fn test_reveal_path_deeply_nested() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("a/b/c")).unwrap();
        fs::write(temp.path().join("a/b/c/deep.txt"), "content").unwrap();

        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();
        let target = temp.path().join("a/b/c/deep.txt");

        nav.reveal_path(&target).unwrap();

        let entries = nav.visible_entries();
        assert!(
            entries.iter().any(|e| e.path == target),
            "Deep target should be visible after reveal"
        );
    }

    #[test]
    fn test_reveal_path_root_level() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        // file.txt is at root level, already visible
        let target = temp.path().join("file.txt");
        let before_count = nav.visible_count();

        nav.reveal_path(&target).unwrap();

        let after_count = nav.visible_count();
        // Count should be the same since it's already visible
        assert_eq!(before_count, after_count);
    }

    #[test]
    fn test_reveal_path_directory() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        // Reveal a directory (subdir) inside dir_a
        let target = temp.path().join("dir_a/subdir");

        nav.reveal_path(&target).unwrap();

        let entries = nav.visible_entries();
        assert!(
            entries.iter().any(|e| e.path == target),
            "Directory should be visible after reveal"
        );
    }

    #[test]
    fn test_reveal_path_nonexistent_graceful() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        let target = temp.path().join("nonexistent/path/file.txt");

        // Should not panic, should complete successfully
        let result = nav.reveal_path(&target);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reveal_path_outside_root() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        // Try to reveal a path outside the root
        let outside = PathBuf::from("/some/other/path");

        // Should not panic
        let result = nav.reveal_path(&outside);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reveal_path_idempotent() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        let target = temp.path().join("dir_a/nested.txt");

        // Reveal twice
        nav.reveal_path(&target).unwrap();
        let count1 = nav.visible_count();

        nav.reveal_path(&target).unwrap();
        let count2 = nav.visible_count();

        // Should be the same
        assert_eq!(count1, count2);
    }
}
