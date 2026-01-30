//! Application state management

use std::collections::HashSet;
use std::path::PathBuf;

use super::{FocusTarget, ViewMode};
use crate::action::Clipboard;
use crate::git::GitStatus;

/// Number of bookmark slots (1-9)
pub const BOOKMARK_SLOTS: usize = 9;

/// Sort mode for file entries
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortMode {
    /// Sort by name (alphabetically, case-insensitive)
    #[default]
    Name,
    /// Sort by size (descending, largest first)
    Size,
    /// Sort by modification date (descending, newest first)
    Date,
}

impl SortMode {
    /// Cycle to the next sort mode
    pub fn next(self) -> Self {
        match self {
            SortMode::Name => SortMode::Size,
            SortMode::Size => SortMode::Date,
            SortMode::Date => SortMode::Name,
        }
    }

    /// Get display name for status bar
    pub fn display_name(&self) -> &'static str {
        match self {
            SortMode::Name => "name",
            SortMode::Size => "size",
            SortMode::Date => "date",
        }
    }
}

/// Main application state
pub struct AppState {
    /// Root directory path
    pub root: PathBuf,
    /// Current focus index in visible entries
    pub focus_index: usize,
    /// Top of viewport (scroll position)
    pub viewport_top: usize,
    /// Selected paths (multi-select)
    pub selected_paths: HashSet<PathBuf>,
    /// Current view mode
    pub mode: ViewMode,
    /// Status message
    pub message: Option<String>,
    /// Preview panel visibility
    pub preview_visible: bool,
    /// Focus target for split view (Tree or Preview)
    pub focus_target: FocusTarget,
    /// Whether to show hidden files
    pub show_hidden: bool,
    /// Exit flag
    pub should_quit: bool,
    /// Pick mode (--pick option)
    pub pick_mode: bool,
    /// Clipboard for copy/cut/paste
    pub clipboard: Option<Clipboard>,
    /// Git repository status
    pub git_status: Option<GitStatus>,
    /// Whether to show Nerd Fonts icons
    pub icons_enabled: bool,
    /// Directory path to cd on exit (shell integration)
    pub choosedir_path: Option<PathBuf>,
    /// Target path to jump to from fuzzy finder
    pub fuzzy_jump_target: Option<PathBuf>,
    /// Whether in stdin mode (file operations disabled)
    pub stdin_mode: bool,
    /// Whether file watching is enabled
    pub watch_enabled: bool,
    /// Bookmarks (slots 0-8 for keys 1-9)
    pub bookmarks: [Option<PathBuf>; BOOKMARK_SLOTS],
    /// File filter pattern (glob-like, e.g., "*.rs", "test*")
    pub filter_pattern: Option<String>,
    /// Current sort mode
    pub sort_mode: SortMode,
    /// Search match info (current_index, total_count)
    pub search_matches: Option<(usize, usize)>,
}

impl AppState {
    /// Create new application state
    ///
    /// Note: Git status is NOT initialized at startup for performance.
    /// Call `init_git_status()` after the first frame is rendered.
    pub fn new(root: PathBuf) -> Self {
        // Check environment variable for icons setting (default: enabled)
        let icons_enabled = std::env::var("FILEVIEW_ICONS")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(true);

        Self {
            root,
            focus_index: 0,
            viewport_top: 0,
            selected_paths: HashSet::new(),
            mode: ViewMode::Browse,
            message: None,
            preview_visible: false,
            focus_target: FocusTarget::Tree,
            show_hidden: false,
            should_quit: false,
            pick_mode: false,
            clipboard: None,
            git_status: None, // Lazy-initialized for faster startup
            icons_enabled,
            choosedir_path: None,
            fuzzy_jump_target: None,
            stdin_mode: false,
            watch_enabled: false,
            bookmarks: [const { None }; BOOKMARK_SLOTS],
            filter_pattern: None,
            sort_mode: SortMode::default(),
            search_matches: None,
        }
    }

    /// Initialize git status (call after first frame render for faster startup)
    pub fn init_git_status(&mut self) {
        if self.git_status.is_none() {
            self.git_status = GitStatus::detect(&self.root);
        }
    }

    /// Refresh git status (call after file operations)
    pub fn refresh_git_status(&mut self) {
        if let Some(ref mut git) = self.git_status {
            git.refresh();
        }
    }

    /// Adjust viewport to keep focus visible
    pub fn adjust_viewport(&mut self, visible_height: usize) {
        if self.focus_index < self.viewport_top {
            self.viewport_top = self.focus_index;
        } else if self.focus_index >= self.viewport_top + visible_height {
            self.viewport_top = self.focus_index.saturating_sub(visible_height) + 1;
        }
    }

    /// Set status message
    pub fn set_message(&mut self, msg: impl Into<String>) {
        self.message = Some(msg.into());
    }

    /// Clear status message
    pub fn clear_message(&mut self) {
        self.message = None;
    }

    /// Toggle focus between Tree and Preview (only effective when preview is visible)
    pub fn toggle_focus(&mut self) {
        if self.preview_visible {
            self.focus_target = match self.focus_target {
                FocusTarget::Tree => FocusTarget::Preview,
                FocusTarget::Preview => FocusTarget::Tree,
            };
        }
    }

    /// Set focus target (automatically resets to Tree if preview is not visible)
    pub fn set_focus(&mut self, target: FocusTarget) {
        if self.preview_visible || target == FocusTarget::Tree {
            self.focus_target = target;
        }
    }

    /// Reset focus to Tree (call when closing preview)
    pub fn reset_focus(&mut self) {
        self.focus_target = FocusTarget::Tree;
    }
}
