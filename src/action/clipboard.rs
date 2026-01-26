//! Clipboard management for copy/cut/paste operations

use std::path::PathBuf;

/// Clipboard content type
#[derive(Debug, Clone)]
pub enum ClipboardContent {
    Copy(Vec<PathBuf>),
    Cut(Vec<PathBuf>),
}

/// Clipboard state
#[derive(Debug, Default)]
pub struct Clipboard {
    content: Option<ClipboardContent>,
}

impl Clipboard {
    /// Create new empty clipboard
    pub fn new() -> Self {
        Self { content: None }
    }

    /// Copy paths to clipboard
    pub fn copy(&mut self, paths: Vec<PathBuf>) {
        self.content = Some(ClipboardContent::Copy(paths));
    }

    /// Cut paths to clipboard
    pub fn cut(&mut self, paths: Vec<PathBuf>) {
        self.content = Some(ClipboardContent::Cut(paths));
    }

    /// Get clipboard content
    pub fn content(&self) -> Option<&ClipboardContent> {
        self.content.as_ref()
    }

    /// Take clipboard content (clears it)
    pub fn take(&mut self) -> Option<ClipboardContent> {
        self.content.take()
    }

    /// Check if clipboard has cut content
    pub fn is_cut(&self) -> bool {
        matches!(self.content, Some(ClipboardContent::Cut(_)))
    }

    /// Get paths in clipboard
    pub fn paths(&self) -> &[PathBuf] {
        match &self.content {
            Some(ClipboardContent::Copy(paths)) | Some(ClipboardContent::Cut(paths)) => paths,
            None => &[],
        }
    }

    /// Check if clipboard is empty
    pub fn is_empty(&self) -> bool {
        self.content.is_none()
    }

    /// Clear clipboard
    pub fn clear(&mut self) {
        self.content = None;
    }
}
