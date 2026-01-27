//! View mode definitions

use std::path::PathBuf;

/// Focus target for split view (side preview mode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusTarget {
    /// Focus on file tree (default)
    #[default]
    Tree,
    /// Focus on preview panel
    Preview,
}

/// Current view/input mode with embedded state
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ViewMode {
    /// Normal browsing mode
    #[default]
    Browse,
    /// Search mode with query
    Search { query: String },
    /// Text input mode
    Input {
        purpose: InputPurpose,
        buffer: String,
        cursor: usize,
    },
    /// Confirmation dialog
    Confirm { action: PendingAction },
    /// Fullscreen preview
    Preview { scroll: usize },
}

/// Purpose of text input
#[derive(Debug, Clone, PartialEq)]
pub enum InputPurpose {
    /// Creating a new file
    CreateFile,
    /// Creating a new directory
    CreateDir,
    /// Renaming an existing item
    Rename { original: PathBuf },
}

/// Action pending confirmation
#[derive(Debug, Clone, PartialEq)]
pub enum PendingAction {
    /// Delete files/directories
    Delete { targets: Vec<PathBuf> },
}
