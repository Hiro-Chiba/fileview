//! Application state management

use std::collections::HashSet;
use std::path::PathBuf;

use super::ViewMode;
use crate::action::Clipboard;

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
    /// Whether to show hidden files
    pub show_hidden: bool,
    /// Exit flag
    pub should_quit: bool,
    /// Pick mode (--pick option)
    pub pick_mode: bool,
    /// Clipboard for copy/cut/paste
    pub clipboard: Option<Clipboard>,
}

impl AppState {
    /// Create new application state
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            focus_index: 0,
            viewport_top: 0,
            selected_paths: HashSet::new(),
            mode: ViewMode::Browse,
            message: None,
            preview_visible: false,
            show_hidden: false,
            should_quit: false,
            pick_mode: false,
            clipboard: None,
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
}
