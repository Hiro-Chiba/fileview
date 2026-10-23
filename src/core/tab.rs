use std::collections::HashSet;
use std::path::PathBuf;

use crate::tree::TreeNavigator;
use crate::util::utf8_prefix;

use super::{FocusTarget, SortMode, ViewMode, BOOKMARK_SLOTS};

pub struct Tab {
    pub root: PathBuf,
    /// Usually the directory name.
    pub name: String,
    pub navigator: TreeNavigator,
    pub focus_index: usize,
    pub viewport_top: usize,
    pub selected_paths: HashSet<PathBuf>,
    pub mode: ViewMode,
    pub focus_target: FocusTarget,
    pub show_hidden: bool,
    pub bookmarks: [Option<PathBuf>; BOOKMARK_SLOTS],
    pub filter_pattern: Option<String>,
    pub sort_mode: SortMode,
}

impl Tab {
    pub fn new(root: PathBuf, show_hidden: bool) -> anyhow::Result<Self> {
        let navigator = TreeNavigator::new(&root, show_hidden)?;
        Ok(Self::with_navigator(root, show_hidden, navigator))
    }

    fn with_navigator(root: PathBuf, show_hidden: bool, navigator: TreeNavigator) -> Self {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root.display().to_string());

        Self {
            root,
            name,
            navigator,
            focus_index: 0,
            viewport_top: 0,
            selected_paths: HashSet::new(),
            mode: ViewMode::Browse,
            focus_target: FocusTarget::Tree,
            show_hidden,
            bookmarks: [const { None }; BOOKMARK_SLOTS],
            filter_pattern: None,
            sort_mode: SortMode::default(),
        }
    }

    pub fn short_name(&self, max_len: usize) -> String {
        if self.name.len() <= max_len {
            self.name.clone()
        } else {
            format!("{}...", utf8_prefix(&self.name, max_len.saturating_sub(3)))
        }
    }
}

pub struct TabManager {
    pub tabs: Vec<Tab>,
    pub active_index: usize,
}

impl TabManager {
    pub fn new(root: PathBuf, show_hidden: bool) -> anyhow::Result<Self> {
        let initial_tab = Tab::new(root, show_hidden)?;
        Ok(Self {
            tabs: vec![initial_tab],
            active_index: 0,
        })
    }

    pub(crate) fn with_navigator(
        root: PathBuf,
        show_hidden: bool,
        navigator: TreeNavigator,
    ) -> Self {
        Self {
            tabs: vec![Tab::with_navigator(root, show_hidden, navigator)],
            active_index: 0,
        }
    }

    pub fn active(&self) -> &Tab {
        &self.tabs[self.active_index]
    }

    pub fn active_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_index]
    }

    pub fn new_tab(&mut self, root: PathBuf, show_hidden: bool) -> anyhow::Result<()> {
        let tab = Tab::new(root, show_hidden)?;
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        Ok(())
    }

    /// Returns true if a tab was closed, false if this is the last tab
    pub fn close_tab(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }

        self.tabs.remove(self.active_index);

        if self.active_index >= self.tabs.len() {
            self.active_index = self.tabs.len() - 1;
        }

        true
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_index = (self.active_index + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_index = if self.active_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_index - 1
            };
        }
    }

    pub fn switch_to(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_index = index;
        }
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Check if there are no tabs (should never happen)
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_temp_tab() -> (TempDir, Tab) {
        let temp = TempDir::new().unwrap();
        let tab = Tab::new(temp.path().to_path_buf(), false).unwrap();
        (temp, tab)
    }

    #[test]
    fn test_tab_new() {
        let (_temp, tab) = create_temp_tab();
        assert_eq!(tab.focus_index, 0);
        assert_eq!(tab.viewport_top, 0);
        assert!(tab.selected_paths.is_empty());
        assert_eq!(tab.mode, ViewMode::Browse);
    }

    #[test]
    fn test_tab_short_name() {
        let (_temp, mut tab) = create_temp_tab();
        tab.name = "very_long_directory_name".to_string();

        assert_eq!(tab.short_name(10), "very_lo...");
        assert_eq!(tab.short_name(30), "very_long_directory_name");
    }

    #[test]
    fn test_tab_manager_new_tab() {
        let temp = TempDir::new().unwrap();
        let mut manager = TabManager::new(temp.path().to_path_buf(), false).unwrap();

        assert_eq!(manager.len(), 1);
        assert_eq!(manager.active_index, 0);

        let temp2 = TempDir::new().unwrap();
        manager.new_tab(temp2.path().to_path_buf(), false).unwrap();

        assert_eq!(manager.len(), 2);
        assert_eq!(manager.active_index, 1);
    }

    #[test]
    fn test_tab_manager_uses_existing_navigator() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let navigator = TreeNavigator::new(&root, false).unwrap();

        let manager = TabManager::with_navigator(root.clone(), false, navigator);
        assert_eq!(manager.active().root, root);
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_tab_manager_close_tab() {
        let temp = TempDir::new().unwrap();
        let mut manager = TabManager::new(temp.path().to_path_buf(), false).unwrap();

        assert!(!manager.close_tab());

        let temp2 = TempDir::new().unwrap();
        manager.new_tab(temp2.path().to_path_buf(), false).unwrap();
        assert_eq!(manager.len(), 2);

        assert!(manager.close_tab());
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_tab_manager_navigation() {
        let temp = TempDir::new().unwrap();
        let mut manager = TabManager::new(temp.path().to_path_buf(), false).unwrap();

        let temp2 = TempDir::new().unwrap();
        let temp3 = TempDir::new().unwrap();
        manager.new_tab(temp2.path().to_path_buf(), false).unwrap();
        manager.new_tab(temp3.path().to_path_buf(), false).unwrap();

        assert_eq!(manager.active_index, 2);

        manager.next_tab();
        assert_eq!(manager.active_index, 0);

        manager.prev_tab();
        assert_eq!(manager.active_index, 2);

        manager.switch_to(1);
        assert_eq!(manager.active_index, 1);
    }

    #[test]
    fn test_tab_short_name_exact_length() {
        let (_temp, mut tab) = create_temp_tab();
        tab.name = "exactly10!".to_string();

        assert_eq!(tab.short_name(10), "exactly10!");
    }

    #[test]
    fn test_tab_short_name_shorter() {
        let (_temp, mut tab) = create_temp_tab();
        tab.name = "short".to_string();

        assert_eq!(tab.short_name(10), "short");
    }

    #[test]
    fn test_tab_short_name_multibyte() {
        let (_temp, mut tab) = create_temp_tab();
        tab.name = "日本語fileview".to_string();

        assert_eq!(tab.short_name(10), "日本...");
    }

    #[test]
    fn test_tab_manager_active_mut() {
        let temp = TempDir::new().unwrap();
        let mut manager = TabManager::new(temp.path().to_path_buf(), false).unwrap();

        manager.active_mut().focus_index = 5;
        assert_eq!(manager.active().focus_index, 5);
    }

    #[test]
    fn test_tab_manager_switch_to_invalid() {
        let temp = TempDir::new().unwrap();
        let mut manager = TabManager::new(temp.path().to_path_buf(), false).unwrap();

        manager.switch_to(100);
        assert_eq!(manager.active_index, 0);
    }

    #[test]
    fn test_tab_manager_close_active_middle() {
        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();
        let temp3 = TempDir::new().unwrap();

        let mut manager = TabManager::new(temp1.path().to_path_buf(), false).unwrap();
        manager.new_tab(temp2.path().to_path_buf(), false).unwrap();
        manager.new_tab(temp3.path().to_path_buf(), false).unwrap();

        manager.switch_to(1);
        assert!(manager.close_tab());
        assert_eq!(manager.len(), 2);
        assert_eq!(manager.active_index, 1); // Should stay at 1 (now the last tab)
    }

    #[test]
    fn test_tab_manager_close_last_position() {
        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();

        let mut manager = TabManager::new(temp1.path().to_path_buf(), false).unwrap();
        manager.new_tab(temp2.path().to_path_buf(), false).unwrap();

        assert!(manager.close_tab());
        assert_eq!(manager.active_index, 0);
    }

    #[test]
    fn test_tab_manager_is_empty() {
        let temp = TempDir::new().unwrap();
        let manager = TabManager::new(temp.path().to_path_buf(), false).unwrap();

        assert!(!manager.is_empty());
    }

    #[test]
    fn test_tab_selection() {
        let (_temp, mut tab) = create_temp_tab();

        tab.selected_paths.insert(PathBuf::from("/test/file1.txt"));
        tab.selected_paths.insert(PathBuf::from("/test/file2.txt"));

        assert_eq!(tab.selected_paths.len(), 2);
        assert!(tab
            .selected_paths
            .contains(&PathBuf::from("/test/file1.txt")));
    }

    #[test]
    fn test_tab_bookmarks() {
        let (_temp, mut tab) = create_temp_tab();

        tab.bookmarks[0] = Some(PathBuf::from("/bookmark/1"));
        tab.bookmarks[8] = Some(PathBuf::from("/bookmark/9"));

        assert_eq!(tab.bookmarks[0], Some(PathBuf::from("/bookmark/1")));
        assert_eq!(tab.bookmarks[8], Some(PathBuf::from("/bookmark/9")));
        assert_eq!(tab.bookmarks[1], None);
    }

    #[test]
    fn test_tab_filter_pattern() {
        let (_temp, mut tab) = create_temp_tab();

        assert!(tab.filter_pattern.is_none());

        tab.filter_pattern = Some("*.rs".to_string());
        assert_eq!(tab.filter_pattern, Some("*.rs".to_string()));
    }
}
