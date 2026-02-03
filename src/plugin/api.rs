//! Plugin API context
//!
//! This module provides the context that plugins can access to interact
//! with FileView's current state.

use std::path::PathBuf;

/// Plugin events that can be listened to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginEvent {
    /// Triggered when a file is focused/selected
    FileSelected,
    /// Triggered when navigating to a new directory
    DirectoryChanged,
    /// Triggered when the selection set changes
    SelectionChanged,
    /// Triggered on application start (after plugins load)
    Start,
    /// Triggered before quitting
    BeforeQuit,
}

impl PluginEvent {
    /// Get the event name as used in Lua
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginEvent::FileSelected => "file_selected",
            PluginEvent::DirectoryChanged => "directory_changed",
            PluginEvent::SelectionChanged => "selection_changed",
            PluginEvent::Start => "start",
            PluginEvent::BeforeQuit => "before_quit",
        }
    }

    /// Parse event name from string
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "file_selected" => Some(PluginEvent::FileSelected),
            "directory_changed" => Some(PluginEvent::DirectoryChanged),
            "selection_changed" => Some(PluginEvent::SelectionChanged),
            "start" => Some(PluginEvent::Start),
            "before_quit" => Some(PluginEvent::BeforeQuit),
            _ => None,
        }
    }
}

/// Actions that can be triggered from plugins
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginAction {
    /// Navigate to a directory
    Navigate(PathBuf),
    /// Add a file to selection
    Select(PathBuf),
    /// Remove a file from selection
    Deselect(PathBuf),
    /// Clear all selections
    ClearSelection,
    /// Refresh the tree view
    Refresh,
    /// Set clipboard text
    SetClipboard(String),
    /// Focus on a specific file (reveal and select)
    Focus(PathBuf),
}

/// Context shared between FileView and Lua plugins
///
/// This structure holds the current state that plugins can read and
/// provides a way for plugins to communicate back to FileView.
#[derive(Debug, Default)]
pub struct PluginContext {
    /// Currently focused file path (None if directory or no focus)
    current_file: Option<PathBuf>,
    /// Current directory path
    current_dir: PathBuf,
    /// Currently selected files (multi-select)
    selected_files: Vec<PathBuf>,
    /// Pending notifications from plugins
    notifications: Vec<String>,
    /// Pending actions from plugins
    actions: Vec<PluginAction>,
}

impl PluginContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            current_file: None,
            current_dir: PathBuf::new(),
            selected_files: Vec::new(),
            notifications: Vec::new(),
            actions: Vec::new(),
        }
    }

    /// Get the currently focused file path
    pub fn current_file(&self) -> Option<&PathBuf> {
        self.current_file.as_ref()
    }

    /// Set the currently focused file
    pub fn set_current_file(&mut self, path: Option<PathBuf>) {
        self.current_file = path;
    }

    /// Get the current directory
    pub fn current_dir(&self) -> &PathBuf {
        &self.current_dir
    }

    /// Set the current directory
    pub fn set_current_dir(&mut self, path: PathBuf) {
        self.current_dir = path;
    }

    /// Get the selected files
    pub fn selected_files(&self) -> &[PathBuf] {
        &self.selected_files
    }

    /// Set the selected files
    pub fn set_selected_files(&mut self, paths: Vec<PathBuf>) {
        self.selected_files = paths;
    }

    /// Add a notification message
    pub fn add_notification(&mut self, msg: String) {
        self.notifications.push(msg);
    }

    /// Take all pending notifications
    pub fn take_notifications(&mut self) -> Vec<String> {
        std::mem::take(&mut self.notifications)
    }

    /// Check if there are pending notifications
    pub fn has_notifications(&self) -> bool {
        !self.notifications.is_empty()
    }

    /// Queue an action to be executed
    pub fn queue_action(&mut self, action: PluginAction) {
        self.actions.push(action);
    }

    /// Take all pending actions
    pub fn take_actions(&mut self) -> Vec<PluginAction> {
        std::mem::take(&mut self.actions)
    }

    /// Check if there are pending actions
    pub fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_new() {
        let ctx = PluginContext::new();
        assert!(ctx.current_file().is_none());
        assert!(ctx.current_dir().as_os_str().is_empty());
        assert!(ctx.selected_files().is_empty());
        assert!(!ctx.has_notifications());
    }

    #[test]
    fn test_set_current_file() {
        let mut ctx = PluginContext::new();
        ctx.set_current_file(Some(PathBuf::from("/test/file.txt")));
        assert_eq!(ctx.current_file(), Some(&PathBuf::from("/test/file.txt")));

        ctx.set_current_file(None);
        assert!(ctx.current_file().is_none());
    }

    #[test]
    fn test_set_current_dir() {
        let mut ctx = PluginContext::new();
        ctx.set_current_dir(PathBuf::from("/test/dir"));
        assert_eq!(ctx.current_dir(), &PathBuf::from("/test/dir"));
    }

    #[test]
    fn test_selected_files() {
        let mut ctx = PluginContext::new();
        let files = vec![PathBuf::from("/a.txt"), PathBuf::from("/b.txt")];
        ctx.set_selected_files(files.clone());
        assert_eq!(ctx.selected_files(), files.as_slice());
    }

    #[test]
    fn test_notifications() {
        let mut ctx = PluginContext::new();
        assert!(!ctx.has_notifications());

        ctx.add_notification("Hello".to_string());
        assert!(ctx.has_notifications());

        ctx.add_notification("World".to_string());

        let notes = ctx.take_notifications();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0], "Hello");
        assert_eq!(notes[1], "World");

        // After take, should be empty
        assert!(!ctx.has_notifications());
        assert!(ctx.take_notifications().is_empty());
    }

    #[test]
    fn test_actions() {
        let mut ctx = PluginContext::new();
        assert!(!ctx.has_actions());

        ctx.queue_action(PluginAction::Navigate(PathBuf::from("/test")));
        assert!(ctx.has_actions());

        ctx.queue_action(PluginAction::Refresh);
        ctx.queue_action(PluginAction::SetClipboard("hello".to_string()));

        let actions = ctx.take_actions();
        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0], PluginAction::Navigate(PathBuf::from("/test")));
        assert_eq!(actions[1], PluginAction::Refresh);
        assert_eq!(actions[2], PluginAction::SetClipboard("hello".to_string()));

        // After take, should be empty
        assert!(!ctx.has_actions());
        assert!(ctx.take_actions().is_empty());
    }

    #[test]
    fn test_selection_actions() {
        let mut ctx = PluginContext::new();

        ctx.queue_action(PluginAction::Select(PathBuf::from("/a.txt")));
        ctx.queue_action(PluginAction::Select(PathBuf::from("/b.txt")));
        ctx.queue_action(PluginAction::Deselect(PathBuf::from("/a.txt")));
        ctx.queue_action(PluginAction::ClearSelection);

        let actions = ctx.take_actions();
        assert_eq!(actions.len(), 4);
        assert_eq!(actions[0], PluginAction::Select(PathBuf::from("/a.txt")));
        assert_eq!(actions[3], PluginAction::ClearSelection);
    }

    #[test]
    fn test_focus_action() {
        let mut ctx = PluginContext::new();
        ctx.queue_action(PluginAction::Focus(PathBuf::from("/test/file.txt")));

        let actions = ctx.take_actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            PluginAction::Focus(PathBuf::from("/test/file.txt"))
        );
    }

    // === PluginEvent tests ===

    #[test]
    fn test_plugin_event_as_str() {
        assert_eq!(PluginEvent::FileSelected.as_str(), "file_selected");
        assert_eq!(PluginEvent::DirectoryChanged.as_str(), "directory_changed");
        assert_eq!(PluginEvent::SelectionChanged.as_str(), "selection_changed");
        assert_eq!(PluginEvent::Start.as_str(), "start");
        assert_eq!(PluginEvent::BeforeQuit.as_str(), "before_quit");
    }

    #[test]
    fn test_plugin_event_parse() {
        assert_eq!(
            PluginEvent::parse("file_selected"),
            Some(PluginEvent::FileSelected)
        );
        assert_eq!(
            PluginEvent::parse("directory_changed"),
            Some(PluginEvent::DirectoryChanged)
        );
        assert_eq!(
            PluginEvent::parse("selection_changed"),
            Some(PluginEvent::SelectionChanged)
        );
        assert_eq!(PluginEvent::parse("start"), Some(PluginEvent::Start));
        assert_eq!(
            PluginEvent::parse("before_quit"),
            Some(PluginEvent::BeforeQuit)
        );
        assert_eq!(PluginEvent::parse("invalid"), None);
        assert_eq!(PluginEvent::parse(""), None);
    }
}
