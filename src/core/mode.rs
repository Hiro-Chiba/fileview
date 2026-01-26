//! View mode definitions

use std::path::PathBuf;

/// Current view/input mode with embedded state
#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    /// Normal browsing mode
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

impl Default for ViewMode {
    fn default() -> Self {
        Self::Browse
    }
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
