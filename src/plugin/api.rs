//! Plugin API context
//!
//! This module provides the context that plugins can access to interact
//! with FileView's current state.

use std::path::PathBuf;

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
}

impl PluginContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            current_file: None,
            current_dir: PathBuf::new(),
            selected_files: Vec::new(),
            notifications: Vec::new(),
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
}
