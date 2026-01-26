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
        if let Some(entry) = self.find_entry_mut(path) {
            if entry.is_dir && !entry.is_expanded() && entry.children().is_empty() {
                entry.load_children(self.show_hidden)?;
            }
            entry.toggle_expanded();
        }
        Ok(())
    }

    /// Expand entry at path
    pub fn expand(&mut self, path: &Path) -> anyhow::Result<()> {
        if let Some(entry) = self.find_entry_mut(path) {
            if entry.is_dir && entry.children().is_empty() {
                entry.load_children(self.show_hidden)?;
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
