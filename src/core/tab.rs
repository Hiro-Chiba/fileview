//! Tab state management
//!
//! Each tab maintains its own file tree state, including the current directory,
//! focus position, selection, and scroll position.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::tree::TreeNavigator;

use super::{FocusTarget, SortMode, ViewMode, BOOKMARK_SLOTS};

/// Represents a single tab in the application
pub struct Tab {
    /// Root directory for this tab
    pub root: PathBuf,
    /// Display name (usually the directory name)
    pub name: String,
    /// Tree navigator for this tab
    pub navigator: TreeNavigator,
    /// Current focus index
    pub focus_index: usize,
    /// Viewport top (scroll position)
    pub viewport_top: usize,
    /// Selected paths (multi-select)
    pub selected_paths: HashSet<PathBuf>,
    /// Current view mode
    pub mode: ViewMode,
    /// Focus target (Tree or Preview)
    pub focus_target: FocusTarget,
    /// Show hidden files
    pub show_hidden: bool,
    /// Bookmarks for this tab
    pub bookmarks: [Option<PathBuf>; BOOKMARK_SLOTS],
    /// Filter pattern
    pub filter_pattern: Option<String>,
    /// Sort mode
    pub sort_mode: SortMode,
}

impl Tab {
    /// Create a new tab for the given directory
    pub fn new(root: PathBuf, show_hidden: bool) -> anyhow::Result<Self> {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root.display().to_string());

        let navigator = TreeNavigator::new(&root, show_hidden)?;

        Ok(Self {
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
        })
    }

    /// Get a short display name for the tab bar
    pub fn short_name(&self, max_len: usize) -> String {
        if self.name.len() <= max_len {
            self.name.clone()
        } else {
            format!("{}...", &self.name[..max_len.saturating_sub(3)])
        }
    }
}

/// Manager for multiple tabs
pub struct TabManager {
    /// All open tabs
    pub tabs: Vec<Tab>,
    /// Index of the currently active tab
    pub active_index: usize,
}

impl TabManager {
    /// Create a new tab manager with an initial tab
    pub fn new(root: PathBuf, show_hidden: bool) -> anyhow::Result<Self> {
        let initial_tab = Tab::new(root, show_hidden)?;
        Ok(Self {
            tabs: vec![initial_tab],
            active_index: 0,
        })
    }

    /// Get the currently active tab
    pub fn active(&self) -> &Tab {
        &self.tabs[self.active_index]
    }

    /// Get mutable reference to the currently active tab
    pub fn active_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_index]
    }

    /// Open a new tab for the given directory
    pub fn new_tab(&mut self, root: PathBuf, show_hidden: bool) -> anyhow::Result<()> {
        let tab = Tab::new(root, show_hidden)?;
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        Ok(())
    }

    /// Close the currently active tab
    ///
    /// Returns true if a tab was closed, false if this is the last tab
    pub fn close_tab(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }

        self.tabs.remove(self.active_index);

        // Adjust active index if needed
        if self.active_index >= self.tabs.len() {
            self.active_index = self.tabs.len() - 1;
        }

        true
    }

    /// Switch to the next tab
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_index = (self.active_index + 1) % self.tabs.len();
        }
    }

    /// Switch to the previous tab
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_index = if self.active_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_index - 1
            };
        }
    }

    /// Switch to a specific tab by index
    pub fn switch_to(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_index = index;
        }
    }

    /// Get the number of tabs
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
    fn test_tab_manager_close_tab() {
        let temp = TempDir::new().unwrap();
        let mut manager = TabManager::new(temp.path().to_path_buf(), false).unwrap();

        // Can't close last tab
        assert!(!manager.close_tab());

        // Add and close a tab
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
}
