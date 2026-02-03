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
    /// Visual selection mode (range selection)
    VisualSelect {
        /// Starting index of selection
        anchor: usize,
    },
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
    /// Fuzzy finder mode
    FuzzyFinder {
        /// Search query
        query: String,
        /// Index of selected item in results
        selected: usize,
    },
    /// Help popup display
    Help,
    /// Waiting for bookmark slot input (set bookmark)
    BookmarkSet,
    /// Waiting for bookmark slot input (jump to bookmark)
    BookmarkJump,
    /// File filter input mode
    Filter { query: String },
    /// Bulk rename mode
    BulkRename {
        /// Pattern to match (e.g., "*.txt", "old_")
        from_pattern: String,
        /// Pattern to replace with (e.g., "*.md", "new_")
        to_pattern: String,
        /// Currently selected field (0 = from, 1 = to)
        selected_field: usize,
        /// Cursor position in current field
        cursor: usize,
    },
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
