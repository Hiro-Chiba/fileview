use std::path::{Path, PathBuf};

use super::node::sort_entries;
use super::TreeEntry;
use crate::core::SortMode;

/// A cached visible entry with all fields needed for display and lookup
#[derive(Debug, Clone)]
pub struct VisibleEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    /// Depth in the tree (0 = root)
    pub depth: usize,
    expanded: bool,
}

impl VisibleEntry {
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }
}

#[derive(Clone)]
pub struct TreeNavigator {
    root: TreeEntry,
    show_hidden: bool,
    /// Whether in stdin mode (read-only, no filesystem operations)
    stdin_mode: bool,
    sort_mode: SortMode,
    /// Rebuilt only when `cache_dirty` is set.
    cached_visible: Vec<VisibleEntry>,
    cache_dirty: bool,
    /// Whether the watcher needs re-syncing with expanded paths
    watcher_dirty: bool,
}

impl TreeNavigator {
    pub fn new(root_path: &Path, show_hidden: bool) -> anyhow::Result<Self> {
        let mut root = TreeEntry::new(root_path.to_path_buf(), 0);
        root.load_children(show_hidden)?;
        root.set_expanded(true);

        Ok(Self {
            root,
            show_hidden,
            stdin_mode: false,
            sort_mode: SortMode::default(),
            cached_visible: Vec::new(),
            cache_dirty: true,
            watcher_dirty: true,
        })
    }

    /// Create navigator from stdin paths
    ///
    /// Paths that don't exist in the filesystem will still be shown in the tree.
    pub fn from_paths(
        root_path: &Path,
        paths: Vec<PathBuf>,
        show_hidden: bool,
    ) -> anyhow::Result<Self> {
        let mut root = TreeEntry::new(root_path.to_path_buf(), 0);
        root.set_expanded(true);

        for path in paths {
            insert_path_into_tree(&mut root, &path, root_path);
        }

        sort_tree_children(&mut root);

        Ok(Self {
            root,
            show_hidden,
            stdin_mode: true,
            sort_mode: SortMode::default(),
            cached_visible: Vec::new(),
            cache_dirty: true,
            watcher_dirty: true,
        })
    }

    pub fn is_stdin_mode(&self) -> bool {
        self.stdin_mode
    }

    /// Collects files only. Used by the fuzzy finder in stdin mode.
    pub fn collect_all_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        collect_paths_recursive(&self.root, &mut paths);
        paths
    }

    pub fn root(&self) -> &TreeEntry {
        &self.root
    }

    /// Rebuild the visible entry cache if dirty. Call once per frame before
    /// accessing `visible_entries()`.
    pub fn ensure_cache(&mut self) {
        if self.cache_dirty {
            self.cached_visible.clear();
            Self::collect_visible_cached(&self.root, &mut self.cached_visible);
            self.cache_dirty = false;
        }
    }

    fn collect_visible_cached(entry: &TreeEntry, out: &mut Vec<VisibleEntry>) {
        out.push(VisibleEntry {
            path: entry.path.clone(),
            name: entry.name.clone(),
            is_dir: entry.is_dir,
            depth: entry.depth,
            expanded: entry.expanded,
        });
        if entry.is_expanded() {
            for child in entry.children() {
                Self::collect_visible_cached(child, out);
            }
        }
    }

    /// Stale until `ensure_cache` is called.
    pub fn visible_entries(&self) -> &[VisibleEntry] {
        &self.cached_visible
    }

    /// Stale until `ensure_cache` is called.
    pub fn visible_count(&self) -> usize {
        self.cached_visible.len()
    }

    pub fn is_watcher_dirty(&self) -> bool {
        self.watcher_dirty
    }

    pub fn clear_watcher_dirty(&mut self) {
        self.watcher_dirty = false;
    }

    pub fn toggle_expand(&mut self, path: &Path) -> anyhow::Result<()> {
        let show_hidden = self.show_hidden;
        let sort_mode = self.sort_mode;
        if let Some(entry) = self.find_entry_mut(path) {
            if entry.is_dir && !entry.is_expanded() && entry.children().is_empty() {
                entry.load_children_with_sort(show_hidden, sort_mode)?;
            }
            entry.toggle_expanded();
            self.cache_dirty = true;
            self.watcher_dirty = true;
        }
        Ok(())
    }

    pub fn expand(&mut self, path: &Path) -> anyhow::Result<()> {
        let show_hidden = self.show_hidden;
        let sort_mode = self.sort_mode;
        if let Some(entry) = self.find_entry_mut(path) {
            if entry.is_dir && entry.children().is_empty() {
                entry.load_children_with_sort(show_hidden, sort_mode)?;
            }
            entry.set_expanded(true);
            self.cache_dirty = true;
            self.watcher_dirty = true;
        }
        Ok(())
    }

    pub fn collapse(&mut self, path: &Path) {
        if let Some(entry) = self.find_entry_mut(path) {
            entry.set_expanded(false);
            self.cache_dirty = true;
            self.watcher_dirty = true;
        }
    }

    pub fn reload(&mut self) -> anyhow::Result<()> {
        let expanded_paths = self.expanded_paths();
        self.root
            .load_children_with_sort(self.show_hidden, self.sort_mode)?;
        self.restore_expanded(&expanded_paths)?;
        self.cache_dirty = true;
        self.watcher_dirty = true;
        Ok(())
    }

    pub fn set_show_hidden(&mut self, show: bool) -> anyhow::Result<()> {
        self.show_hidden = show;
        self.reload()
    }

    pub fn set_sort_mode(&mut self, mode: SortMode) -> anyhow::Result<()> {
        self.sort_mode = mode;
        resort_entry_children(&mut self.root, mode);
        self.cache_dirty = true;
        self.watcher_dirty = true;
        Ok(())
    }

    /// Reveal a path by expanding all parent directories
    pub fn reveal_path(&mut self, target: &Path) -> anyhow::Result<()> {
        let root_path = self.root.path.clone();
        let mut ancestors = Vec::new();

        if let Ok(relative) = target.strip_prefix(&root_path) {
            let mut current = root_path.clone();
            for component in relative.components() {
                current = current.join(component);
                if current != *target {
                    ancestors.push(current.clone());
                }
            }
        }

        for ancestor in ancestors {
            self.expand(&ancestor)?;
        }

        Ok(())
    }

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

    /// Used for syncing file watcher with expanded directories.
    pub fn expanded_paths(&self) -> Vec<PathBuf> {
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

    fn restore_expanded(&mut self, paths: &[PathBuf]) -> anyhow::Result<()> {
        for path in paths {
            self.expand(path)?;
        }
        Ok(())
    }
}

/// Insert a path into the tree, creating intermediate directories as needed
fn insert_path_into_tree(root: &mut TreeEntry, path: &Path, root_path: &Path) {
    let relative = match path.strip_prefix(root_path) {
        Ok(rel) => rel,
        Err(_) => return,
    };

    let components: Vec<_> = relative.components().collect();
    if components.is_empty() {
        return;
    }

    let mut current = root;
    let mut current_path = root_path.to_path_buf();

    for (i, component) in components.iter().enumerate() {
        current_path = current_path.join(component);
        let is_last = i == components.len() - 1;
        let is_dir = if is_last {
            path.is_dir()
        } else {
            true // Intermediate components are directories
        };

        let child_name = component.as_os_str().to_string_lossy().to_string();
        let child_idx = current.children().iter().position(|c| c.name == child_name);

        match child_idx {
            Some(idx) => {
                current = &mut current.children_mut()[idx];
            }
            None => {
                let depth = current.depth + 1;
                let mut child = TreeEntry::new_with_type(current_path.clone(), depth, is_dir);
                if is_dir {
                    child.set_expanded(true);
                }
                current.children_mut().push(child);
                let new_idx = current.children().len() - 1;
                current = &mut current.children_mut()[new_idx];
            }
        }
    }
}

fn sort_tree_children(entry: &mut TreeEntry) {
    sort_entries(entry.children_mut(), SortMode::Name);

    for child in entry.children_mut() {
        sort_tree_children(child);
    }
}

fn resort_entry_children(entry: &mut TreeEntry, mode: SortMode) {
    sort_entries(entry.children_mut(), mode);
    for child in entry.children_mut() {
        if child.is_dir && !child.children().is_empty() {
            resort_entry_children(child, mode);
        }
    }
}

fn collect_paths_recursive(entry: &TreeEntry, paths: &mut Vec<PathBuf>) {
    if !entry.is_dir {
        paths.push(entry.path.clone());
    }
    for child in entry.children() {
        collect_paths_recursive(child, paths);
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

        assert!(nav.root().is_expanded());
        assert_eq!(nav.root().children().len(), 3);
    }

    #[test]
    fn test_visible_entries() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        nav.ensure_cache();
        let visible = nav.visible_entries();
        // Root + 3 children = 4 entries (children not expanded yet)
        assert_eq!(visible.len(), 4);
    }

    #[test]
    fn test_expand_collapse() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        let dir_a_path = temp.path().join("dir_a");

        nav.ensure_cache();
        let count_before = nav.visible_count();

        nav.expand(&dir_a_path).unwrap();
        nav.ensure_cache();
        let count_after = nav.visible_count();

        assert!(count_after > count_before);

        nav.collapse(&dir_a_path);
        nav.ensure_cache();
        assert_eq!(nav.visible_count(), count_before);
    }

    #[test]
    fn test_toggle_expand() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        let dir_a_path = temp.path().join("dir_a");
        nav.ensure_cache();
        let count_collapsed = nav.visible_count();

        nav.toggle_expand(&dir_a_path).unwrap();
        nav.ensure_cache();
        let count_expanded = nav.visible_count();
        assert!(count_expanded > count_collapsed);

        nav.toggle_expand(&dir_a_path).unwrap();
        nav.ensure_cache();
        assert_eq!(nav.visible_count(), count_collapsed);
    }

    #[test]
    fn test_set_show_hidden() {
        let temp = setup_test_dir();
        fs::write(temp.path().join(".hidden"), "hidden").unwrap();

        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();
        nav.ensure_cache();
        let count_without_hidden = nav.visible_count();

        nav.set_show_hidden(true).unwrap();
        nav.ensure_cache();
        let count_with_hidden = nav.visible_count();

        assert!(count_with_hidden > count_without_hidden);
    }

    #[test]
    fn test_reload() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        nav.ensure_cache();
        let count_before = nav.visible_count();

        fs::write(temp.path().join("new_file.txt"), "new").unwrap();

        nav.reload().unwrap();
        nav.ensure_cache();
        let count_after = nav.visible_count();

        assert_eq!(count_after, count_before + 1);
    }

    #[test]
    fn test_reveal_path_nested() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        let target = temp.path().join("dir_a/nested.txt");

        nav.ensure_cache();
        let before = nav.visible_entries();
        assert!(
            !before.iter().any(|e| e.path == target),
            "Target should not be visible initially"
        );

        nav.reveal_path(&target).unwrap();

        nav.ensure_cache();
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

        nav.ensure_cache();
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

        let target = temp.path().join("file.txt");
        nav.ensure_cache();
        let before_count = nav.visible_count();

        nav.reveal_path(&target).unwrap();

        nav.ensure_cache();
        let after_count = nav.visible_count();
        assert_eq!(before_count, after_count);
    }

    #[test]
    fn test_reveal_path_directory() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        let target = temp.path().join("dir_a/subdir");

        nav.reveal_path(&target).unwrap();

        nav.ensure_cache();
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

        let result = nav.reveal_path(&target);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reveal_path_outside_root() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        let outside = PathBuf::from("/some/other/path");

        let result = nav.reveal_path(&outside);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reveal_path_idempotent() {
        let temp = setup_test_dir();
        let mut nav = TreeNavigator::new(temp.path(), false).unwrap();

        let target = temp.path().join("dir_a/nested.txt");

        nav.reveal_path(&target).unwrap();
        nav.ensure_cache();
        let count1 = nav.visible_count();

        nav.reveal_path(&target).unwrap();
        nav.ensure_cache();
        let count2 = nav.visible_count();

        assert_eq!(count1, count2);
    }
}
