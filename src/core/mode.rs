use std::path::PathBuf;

/// Focus target for split view (side preview mode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusTarget {
    #[default]
    Tree,
    Preview,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ViewMode {
    #[default]
    Browse,
    VisualSelect {
        /// Starting index of selection
        anchor: usize,
    },
    Search {
        query: String,
    },
    Input {
        purpose: InputPurpose,
        buffer: String,
        cursor: usize,
    },
    Confirm {
        action: PendingAction,
    },
    Preview {
        scroll: usize,
    },
    FuzzyFinder {
        query: String,
        /// Index of selected item in results
        selected: usize,
    },
    Help,
    /// AI history popup (context packs already generated this session)
    AiHistory {
        selected: usize,
    },
    /// Live AI activity log popup (events streamed from the MCP server)
    AiActivityLog,
    /// Waiting for bookmark slot input (set bookmark)
    BookmarkSet,
    /// Waiting for bookmark slot input (jump to bookmark)
    BookmarkJump,
    Filter {
        query: String,
    },
    /// Repository-wide TODO/FIXME aggregator popup
    Todos {
        selected: usize,
    },
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

#[derive(Debug, Clone, PartialEq)]
pub enum InputPurpose {
    CreateFile,
    CreateDir,
    Rename { original: PathBuf },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PendingAction {
    Delete { targets: Vec<PathBuf> },
}
