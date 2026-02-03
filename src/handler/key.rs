//! Keyboard event handling

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

use super::keymap::KeyBindingRegistry;
use crate::core::{AppState, FocusTarget, ViewMode};

/// Actions that can result from key handling
#[derive(Debug, Clone)]
pub enum KeyAction {
    /// No action needed
    None,
    /// Quit the application
    Quit,
    /// Quit and change directory (shell integration)
    QuitAndCd,
    /// Move focus up
    MoveUp,
    /// Move focus down
    MoveDown,
    /// Move to top
    MoveToTop,
    /// Move to bottom
    MoveToBottom,
    /// Expand current entry
    Expand,
    /// Collapse current entry
    Collapse,
    /// Toggle expand/collapse
    ToggleExpand,
    /// Collapse all entries
    CollapseAll,
    /// Expand all entries
    ExpandAll,
    /// Toggle selection mark
    ToggleMark,
    /// Clear all marks
    ClearMarks,
    /// Copy selected to clipboard
    Copy,
    /// Cut selected to clipboard
    Cut,
    /// Paste from clipboard
    Paste,
    /// Start delete confirmation
    ConfirmDelete,
    /// Execute confirmed delete
    ExecuteDelete,
    /// Start rename input
    StartRename,
    /// Start new file input
    StartNewFile,
    /// Start new directory input
    StartNewDir,
    /// Start search input
    StartSearch,
    /// Search for next match
    SearchNext,
    /// Refresh tree
    Refresh,
    /// Toggle hidden files
    ToggleHidden,
    /// Copy path to system clipboard
    CopyPath,
    /// Copy filename to system clipboard
    CopyFilename,
    /// Open preview
    OpenPreview,
    /// Toggle quick preview panel
    ToggleQuickPreview,
    /// Confirm current input
    ConfirmInput { value: String },
    /// Cancel current input/mode
    Cancel,
    /// Scroll preview up
    PreviewScrollUp,
    /// Scroll preview down
    PreviewScrollDown,
    /// Preview page up
    PreviewPageUp,
    /// Preview page down
    PreviewPageDown,
    /// Preview scroll to top
    PreviewToTop,
    /// Preview scroll to bottom
    PreviewToBottom,
    /// Select and quit (pick mode)
    PickSelect,
    /// Show help message
    ShowHelp,
    /// Toggle focus between tree and preview (side preview mode)
    ToggleFocus,
    /// Focus on tree panel (left)
    FocusTree,
    /// Focus on preview panel (right)
    FocusPreview,
    /// Open fuzzy finder
    OpenFuzzyFinder,
    /// Move up in fuzzy finder results
    FuzzyUp,
    /// Move down in fuzzy finder results
    FuzzyDown,
    /// Confirm fuzzy finder selection
    FuzzyConfirm { path: std::path::PathBuf },
    /// Enter bookmark set mode
    StartBookmarkSet,
    /// Enter bookmark jump mode
    StartBookmarkJump,
    /// Set bookmark at slot (1-9)
    SetBookmark { slot: u8 },
    /// Jump to bookmark at slot (1-9)
    JumpToBookmark { slot: u8 },
    /// Start file filter input
    StartFilter,
    /// Apply filter pattern
    ApplyFilter { pattern: String },
    /// Clear filter
    ClearFilter,
    /// Cycle sort mode (Name -> Size -> Date -> Name)
    CycleSort,
    /// Search for previous match
    SearchPrev,
    /// Go to previous PDF page
    PdfPrevPage,
    /// Go to next PDF page
    PdfNextPage,
    /// Stage file(s) for git commit
    GitStage,
    /// Unstage file(s) from git commit
    GitUnstage,
    /// Start bulk rename mode
    StartBulkRename,
    /// Switch to next field in bulk rename
    BulkRenameNextField,
    /// Execute bulk rename
    ExecuteBulkRename {
        from_pattern: String,
        to_pattern: String,
    },
    /// Open a new tab
    NewTab,
    /// Close the current tab
    CloseTab,
    /// Switch to the next tab
    NextTab,
    /// Switch to the previous tab
    PrevTab,
    /// Run a custom command
    RunCommand { name: String },
    /// Open subshell in current directory
    OpenSubshell,
    /// Start visual selection mode
    StartVisualSelect,
    /// Select all items
    SelectAll,
    /// Invert selection
    InvertSelection,
}

/// Handle key event and return the resulting action
pub fn handle_key_event(state: &AppState, key: KeyEvent) -> KeyAction {
    match &state.mode {
        ViewMode::Browse => handle_browse_mode(state, key),
        ViewMode::VisualSelect { .. } => handle_visual_select_mode(state, key),
        ViewMode::Search { query } => handle_search_mode(key, query),
        ViewMode::Input { buffer, .. } => handle_input_mode(key, buffer),
        ViewMode::Confirm { .. } => handle_confirm_mode(key),
        ViewMode::Preview { .. } => handle_preview_mode(key),
        ViewMode::FuzzyFinder { .. } => handle_fuzzy_finder_mode(key),
        ViewMode::Help => handle_help_mode(key),
        ViewMode::BookmarkSet => handle_bookmark_set_mode(key),
        ViewMode::BookmarkJump => handle_bookmark_jump_mode(key),
        ViewMode::Filter { query } => handle_filter_mode(key, query),
        ViewMode::BulkRename {
            from_pattern,
            to_pattern,
            ..
        } => handle_bulk_rename_mode(key, from_pattern, to_pattern),
    }
}

/// Handle key event with custom registry
pub fn handle_key_event_with_registry(
    state: &AppState,
    key: KeyEvent,
    registry: &KeyBindingRegistry,
) -> KeyAction {
    match &state.mode {
        ViewMode::Browse => {
            // Try registry first, fall back to built-in
            if let Some(action) = registry.lookup_browse(&key) {
                // Handle special cases that need state context
                apply_browse_context(state, action)
            } else {
                handle_browse_mode(state, key)
            }
        }
        ViewMode::VisualSelect { .. } => {
            // Visual select mode uses browse bindings but with different actions
            handle_visual_select_mode(state, key)
        }
        ViewMode::Search { query } => {
            if let Some(mut action) = registry.lookup_search(&key) {
                if let KeyAction::ConfirmInput { ref mut value } = action {
                    *value = query.clone();
                }
                action
            } else {
                handle_search_mode(key, query)
            }
        }
        ViewMode::Input { buffer, .. } => handle_input_mode(key, buffer),
        ViewMode::Confirm { .. } => registry
            .lookup_confirm(&key)
            .unwrap_or_else(|| handle_confirm_mode(key)),
        ViewMode::Preview { .. } => registry
            .lookup_preview(&key)
            .unwrap_or_else(|| handle_preview_mode(key)),
        ViewMode::FuzzyFinder { .. } => registry
            .lookup_fuzzy(&key)
            .unwrap_or_else(|| handle_fuzzy_finder_mode(key)),
        ViewMode::Help => registry
            .lookup_help(&key)
            .unwrap_or_else(|| handle_help_mode(key)),
        ViewMode::BookmarkSet => handle_bookmark_set_mode(key),
        ViewMode::BookmarkJump => handle_bookmark_jump_mode(key),
        ViewMode::Filter { query } => {
            if let Some(mut action) = registry.lookup_filter(&key) {
                if let KeyAction::ApplyFilter { ref mut pattern } = action {
                    if query.is_empty() {
                        return KeyAction::ClearFilter;
                    }
                    *pattern = query.clone();
                }
                action
            } else {
                handle_filter_mode(key, query)
            }
        }
        ViewMode::BulkRename {
            from_pattern,
            to_pattern,
            ..
        } => handle_bulk_rename_mode(key, from_pattern, to_pattern),
    }
}

/// Apply browse mode context to action
fn apply_browse_context(state: &AppState, action: KeyAction) -> KeyAction {
    match action {
        KeyAction::Quit => {
            if state.pick_mode {
                KeyAction::Cancel
            } else {
                KeyAction::Quit
            }
        }
        KeyAction::Cancel => {
            if state.focus_target == FocusTarget::Preview {
                KeyAction::ToggleFocus
            } else if !state.selected_paths.is_empty() {
                KeyAction::ClearMarks
            } else {
                KeyAction::Cancel
            }
        }
        KeyAction::MoveUp => {
            if state.focus_target == FocusTarget::Preview {
                KeyAction::PreviewScrollUp
            } else {
                KeyAction::MoveUp
            }
        }
        KeyAction::MoveDown => {
            if state.focus_target == FocusTarget::Preview {
                KeyAction::PreviewScrollDown
            } else {
                KeyAction::MoveDown
            }
        }
        KeyAction::MoveToTop => {
            if state.focus_target == FocusTarget::Preview {
                KeyAction::PreviewToTop
            } else {
                KeyAction::MoveToTop
            }
        }
        KeyAction::MoveToBottom => {
            if state.focus_target == FocusTarget::Preview {
                KeyAction::PreviewToBottom
            } else {
                KeyAction::MoveToBottom
            }
        }
        KeyAction::Expand => {
            if state.preview_visible {
                KeyAction::FocusPreview
            } else {
                KeyAction::Expand
            }
        }
        KeyAction::Collapse => {
            if state.preview_visible {
                KeyAction::FocusTree
            } else {
                KeyAction::Collapse
            }
        }
        KeyAction::ToggleExpand => {
            if state.preview_visible {
                KeyAction::ToggleFocus
            } else {
                KeyAction::ToggleExpand
            }
        }
        KeyAction::PickSelect => {
            if state.pick_mode {
                KeyAction::PickSelect
            } else {
                KeyAction::ToggleExpand
            }
        }
        KeyAction::Refresh => {
            if !state.selected_paths.is_empty() {
                KeyAction::StartBulkRename
            } else {
                KeyAction::Refresh
            }
        }
        KeyAction::StartFilter => {
            if state.filter_pattern.is_some() {
                KeyAction::ClearFilter
            } else {
                KeyAction::StartFilter
            }
        }
        KeyAction::PreviewPageUp | KeyAction::PreviewPageDown => {
            if state.focus_target == FocusTarget::Preview {
                action
            } else {
                KeyAction::None
            }
        }
        _ => action,
    }
}

/// Handle keys in browse mode
fn handle_browse_mode(state: &AppState, key: KeyEvent) -> KeyAction {
    match key.code {
        // Quit
        KeyCode::Char('q') => {
            if state.pick_mode {
                KeyAction::Cancel
            } else {
                KeyAction::Quit
            }
        }
        // Quit and cd (shell integration)
        KeyCode::Char('Q') => KeyAction::QuitAndCd,
        KeyCode::Esc => {
            if state.focus_target == FocusTarget::Preview {
                // Esc returns focus to tree when on preview
                KeyAction::ToggleFocus
            } else if !state.selected_paths.is_empty() {
                KeyAction::ClearMarks
            } else {
                KeyAction::Cancel
            }
        }

        // Navigation (focus-aware: Tree navigates files, Preview scrolls content)
        KeyCode::Up | KeyCode::Char('k') => {
            if state.focus_target == FocusTarget::Preview {
                KeyAction::PreviewScrollUp
            } else {
                KeyAction::MoveUp
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.focus_target == FocusTarget::Preview {
                KeyAction::PreviewScrollDown
            } else {
                KeyAction::MoveDown
            }
        }
        KeyCode::Char('g') => {
            if state.focus_target == FocusTarget::Preview {
                KeyAction::PreviewToTop
            } else {
                KeyAction::MoveToTop
            }
        }
        KeyCode::Char('G') => {
            if state.focus_target == FocusTarget::Preview {
                KeyAction::PreviewToBottom
            } else {
                KeyAction::MoveToBottom
            }
        }
        // Page scroll (only when focus is on preview)
        KeyCode::PageUp => {
            if state.focus_target == FocusTarget::Preview {
                KeyAction::PreviewPageUp
            } else {
                KeyAction::None
            }
        }
        KeyCode::PageDown => {
            if state.focus_target == FocusTarget::Preview {
                KeyAction::PreviewPageDown
            } else {
                KeyAction::None
            }
        }
        KeyCode::Char('b') if state.focus_target == FocusTarget::Preview => {
            KeyAction::PreviewPageUp
        }
        KeyCode::Char('f') if state.focus_target == FocusTarget::Preview => {
            KeyAction::PreviewPageDown
        }

        // Expand/Collapse and Focus switching
        // Arrow keys switch focus when preview is visible, l/h always expand/collapse
        KeyCode::Right => {
            if state.preview_visible {
                KeyAction::FocusPreview
            } else {
                KeyAction::Expand
            }
        }
        KeyCode::Char('l') => KeyAction::Expand,
        KeyCode::Left => {
            if state.preview_visible {
                KeyAction::FocusTree
            } else {
                KeyAction::Collapse
            }
        }
        KeyCode::Char('h') | KeyCode::Backspace => KeyAction::Collapse,
        KeyCode::Tab => {
            // Tab toggles focus when side preview is visible, otherwise toggles expand
            if state.preview_visible {
                KeyAction::ToggleFocus
            } else {
                KeyAction::ToggleExpand
            }
        }
        KeyCode::Char('H') => KeyAction::CollapseAll,
        KeyCode::Char('L') => KeyAction::ExpandAll,

        // Selection
        KeyCode::Char(' ') => KeyAction::ToggleMark,
        KeyCode::Enter => {
            if state.pick_mode {
                KeyAction::PickSelect
            } else {
                KeyAction::ToggleExpand
            }
        }

        // Clipboard
        KeyCode::Char('y') => KeyAction::Copy,
        KeyCode::Char('d') => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                KeyAction::ConfirmDelete
            } else {
                KeyAction::Cut
            }
        }
        KeyCode::Char('D') | KeyCode::Delete => KeyAction::ConfirmDelete,
        // Fuzzy finder (Ctrl+P) - must be checked before plain 'p'
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::OpenFuzzyFinder
        }
        KeyCode::Char('p') => KeyAction::Paste,

        // File operations
        KeyCode::Char('r') => KeyAction::StartRename,
        KeyCode::Char('a') => KeyAction::StartNewFile,
        KeyCode::Char('A') => KeyAction::StartNewDir,

        // Search
        KeyCode::Char('/') => KeyAction::StartSearch,
        KeyCode::Char('n') => KeyAction::SearchNext,
        KeyCode::Char('N') => KeyAction::SearchPrev,

        // Sort
        KeyCode::Char('S') => KeyAction::CycleSort,

        // Refresh, bulk rename, and toggle
        KeyCode::Char('R') => {
            // R for bulk rename when files are selected, F5 for refresh
            if !state.selected_paths.is_empty() {
                KeyAction::StartBulkRename
            } else {
                KeyAction::Refresh
            }
        }
        KeyCode::F(5) => KeyAction::Refresh,
        KeyCode::Char('.') => KeyAction::ToggleHidden,

        // Copy to system clipboard
        KeyCode::Char('c') => KeyAction::CopyPath,
        KeyCode::Char('C') => KeyAction::CopyFilename,

        // Preview
        KeyCode::Char('o') => KeyAction::OpenPreview,
        KeyCode::Char('P') => KeyAction::ToggleQuickPreview,

        // Help
        KeyCode::Char('?') => KeyAction::ShowHelp,

        // Visual selection and batch operations
        KeyCode::Char('V') => KeyAction::StartVisualSelect,
        KeyCode::Char('*') => KeyAction::SelectAll,
        KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::ALT) => {
            KeyAction::InvertSelection
        }

        // PDF navigation
        KeyCode::Char('[') => KeyAction::PdfPrevPage,
        KeyCode::Char(']') => KeyAction::PdfNextPage,

        // Bookmarks
        KeyCode::Char('m') => KeyAction::StartBookmarkSet,
        KeyCode::Char('\'') => KeyAction::StartBookmarkJump,

        // Filter
        KeyCode::Char('F') => {
            if state.filter_pattern.is_some() {
                KeyAction::ClearFilter
            } else {
                KeyAction::StartFilter
            }
        }

        // Shell integration - Alt+S for subshell (before Git operations)
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::ALT) => KeyAction::OpenSubshell,

        // Git operations
        KeyCode::Char('s') => KeyAction::GitStage,
        KeyCode::Char('u') => KeyAction::GitUnstage,

        // Tab operations
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::NewTab,
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::CloseTab,
        // Note: gt/gT (vim-style) requires two keystrokes, simplified to Tab for demo
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::ALT) => KeyAction::NextTab,
        KeyCode::Char('T') if key.modifiers.contains(KeyModifiers::ALT) => KeyAction::PrevTab,

        _ => KeyAction::None,
    }
}

/// Handle keys in search mode
fn handle_search_mode(key: KeyEvent, current_query: &str) -> KeyAction {
    match key.code {
        KeyCode::Enter => KeyAction::ConfirmInput {
            value: current_query.to_string(),
        },
        // Same key to cancel (toggle behavior)
        KeyCode::Char('/') | KeyCode::Esc => KeyAction::Cancel,
        _ => KeyAction::None, // Buffer updates handled separately
    }
}

/// Handle keys in input mode (rename, new file/dir)
fn handle_input_mode(key: KeyEvent, current_buffer: &str) -> KeyAction {
    match key.code {
        KeyCode::Enter => KeyAction::ConfirmInput {
            value: current_buffer.to_string(),
        },
        KeyCode::Esc => KeyAction::Cancel,
        _ => KeyAction::None, // Buffer updates handled separately
    }
}

/// Handle keys in confirm mode
fn handle_confirm_mode(key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => KeyAction::ExecuteDelete,
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => KeyAction::Cancel,
        _ => KeyAction::None,
    }
}

/// Handle keys in preview mode
fn handle_preview_mode(key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('o') | KeyCode::Enter => {
            KeyAction::Cancel
        }
        KeyCode::Up | KeyCode::Char('k') => KeyAction::PreviewScrollUp,
        KeyCode::Down | KeyCode::Char('j') => KeyAction::PreviewScrollDown,
        KeyCode::PageUp | KeyCode::Char('b') => KeyAction::PreviewPageUp,
        KeyCode::PageDown | KeyCode::Char('f') | KeyCode::Char(' ') => KeyAction::PreviewPageDown,
        KeyCode::Char('g') => KeyAction::PreviewToTop,
        KeyCode::Char('G') => KeyAction::PreviewToBottom,
        // PDF navigation
        KeyCode::Char('[') => KeyAction::PdfPrevPage,
        KeyCode::Char(']') => KeyAction::PdfNextPage,
        _ => KeyAction::None,
    }
}

/// Handle keys in fuzzy finder mode
fn handle_fuzzy_finder_mode(key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => KeyAction::Cancel,
        // Ctrl+P toggles fuzzy finder off
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::Cancel,
        KeyCode::Up | KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::FuzzyUp
        }
        KeyCode::Down | KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::FuzzyDown
        }
        KeyCode::Up => KeyAction::FuzzyUp,
        KeyCode::Down => KeyAction::FuzzyDown,
        KeyCode::Enter => {
            // The actual path will be filled in by the action handler
            KeyAction::FuzzyConfirm {
                path: PathBuf::new(),
            }
        }
        _ => KeyAction::None, // Text input handled separately
    }
}

/// Handle keys in help mode
fn handle_help_mode(key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('?') => {
            KeyAction::Cancel
        }
        _ => KeyAction::None,
    }
}

/// Handle keys in visual select mode
fn handle_visual_select_mode(state: &AppState, key: KeyEvent) -> KeyAction {
    match key.code {
        // Cancel visual mode
        KeyCode::Esc | KeyCode::Char('V') => KeyAction::Cancel,

        // Navigation (same as browse mode, but extends selection)
        KeyCode::Up | KeyCode::Char('k') => KeyAction::MoveUp,
        KeyCode::Down | KeyCode::Char('j') => KeyAction::MoveDown,
        KeyCode::Char('g') => KeyAction::MoveToTop,
        KeyCode::Char('G') => KeyAction::MoveToBottom,

        // Actions on selected items
        KeyCode::Char('y') => KeyAction::Copy,
        KeyCode::Char('d') => KeyAction::Cut,
        KeyCode::Char('D') | KeyCode::Delete => KeyAction::ConfirmDelete,

        // Confirm selection and exit visual mode
        KeyCode::Enter => {
            // Just confirm (exit visual mode is handled in action handler)
            if state.pick_mode {
                KeyAction::PickSelect
            } else {
                KeyAction::Cancel
            }
        }

        _ => KeyAction::None,
    }
}

/// Update input buffer based on key event
/// Returns the new buffer content, or None if no change
pub fn update_input_buffer(key: KeyEvent, buffer: &str, cursor: usize) -> Option<(String, usize)> {
    match key.code {
        KeyCode::Char(c) => {
            let mut new_buffer = buffer.to_string();
            new_buffer.insert(cursor, c);
            Some((new_buffer, cursor + 1))
        }
        KeyCode::Backspace => {
            if cursor > 0 {
                let mut new_buffer = buffer.to_string();
                new_buffer.remove(cursor - 1);
                Some((new_buffer, cursor - 1))
            } else {
                None
            }
        }
        KeyCode::Delete => {
            if cursor < buffer.len() {
                let mut new_buffer = buffer.to_string();
                new_buffer.remove(cursor);
                Some((new_buffer, cursor))
            } else {
                None
            }
        }
        KeyCode::Left => {
            if cursor > 0 {
                Some((buffer.to_string(), cursor - 1))
            } else {
                None
            }
        }
        KeyCode::Right => {
            if cursor < buffer.len() {
                Some((buffer.to_string(), cursor + 1))
            } else {
                None
            }
        }
        KeyCode::Home => {
            if cursor > 0 {
                Some((buffer.to_string(), 0))
            } else {
                None
            }
        }
        KeyCode::End => {
            if cursor < buffer.len() {
                Some((buffer.to_string(), buffer.len()))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Create delete confirmation action from current state
pub fn create_delete_targets(state: &AppState, focused_path: Option<&PathBuf>) -> Vec<PathBuf> {
    if state.selected_paths.is_empty() {
        focused_path.map(|p| vec![p.clone()]).unwrap_or_default()
    } else {
        state.selected_paths.iter().cloned().collect()
    }
}

/// Handle keys in bookmark set mode (waiting for slot number)
fn handle_bookmark_set_mode(key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Char(c @ '1'..='9') => {
            let slot = c as u8 - b'0';
            KeyAction::SetBookmark { slot }
        }
        // Same key to cancel (toggle behavior)
        KeyCode::Char('m') | KeyCode::Esc => KeyAction::Cancel,
        _ => KeyAction::None,
    }
}

/// Handle keys in bookmark jump mode (waiting for slot number)
fn handle_bookmark_jump_mode(key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Char(c @ '1'..='9') => {
            let slot = c as u8 - b'0';
            KeyAction::JumpToBookmark { slot }
        }
        // Same key to cancel (toggle behavior)
        KeyCode::Char('\'') | KeyCode::Esc => KeyAction::Cancel,
        _ => KeyAction::None,
    }
}

/// Handle keys in filter mode
fn handle_filter_mode(key: KeyEvent, current_query: &str) -> KeyAction {
    match key.code {
        KeyCode::Enter => {
            if current_query.is_empty() {
                KeyAction::ClearFilter
            } else {
                KeyAction::ApplyFilter {
                    pattern: current_query.to_string(),
                }
            }
        }
        // Same key to cancel (toggle behavior)
        KeyCode::Char('F') | KeyCode::Esc => KeyAction::Cancel,
        _ => KeyAction::None, // Text input handled separately
    }
}

/// Handle keys in bulk rename mode
fn handle_bulk_rename_mode(key: KeyEvent, from_pattern: &str, to_pattern: &str) -> KeyAction {
    match key.code {
        KeyCode::Tab => KeyAction::BulkRenameNextField,
        KeyCode::Enter => KeyAction::ExecuteBulkRename {
            from_pattern: from_pattern.to_string(),
            to_pattern: to_pattern.to_string(),
        },
        KeyCode::Esc => KeyAction::Cancel,
        _ => KeyAction::None, // Text input handled separately
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::AppState;

    fn key_event(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn key_event_with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    // Tests for toggle behavior - same key can cancel the mode

    #[test]
    fn test_search_mode_slash_cancels() {
        let action = handle_search_mode(key_event(KeyCode::Char('/')), "query");
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_search_mode_esc_cancels() {
        let action = handle_search_mode(key_event(KeyCode::Esc), "query");
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_bookmark_set_mode_m_cancels() {
        let action = handle_bookmark_set_mode(key_event(KeyCode::Char('m')));
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_bookmark_set_mode_esc_cancels() {
        let action = handle_bookmark_set_mode(key_event(KeyCode::Esc));
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_bookmark_set_mode_digit_sets() {
        let action = handle_bookmark_set_mode(key_event(KeyCode::Char('5')));
        assert!(matches!(action, KeyAction::SetBookmark { slot: 5 }));
    }

    #[test]
    fn test_bookmark_jump_mode_quote_cancels() {
        let action = handle_bookmark_jump_mode(key_event(KeyCode::Char('\'')));
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_bookmark_jump_mode_esc_cancels() {
        let action = handle_bookmark_jump_mode(key_event(KeyCode::Esc));
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_bookmark_jump_mode_digit_jumps() {
        let action = handle_bookmark_jump_mode(key_event(KeyCode::Char('3')));
        assert!(matches!(action, KeyAction::JumpToBookmark { slot: 3 }));
    }

    #[test]
    fn test_filter_mode_f_cancels() {
        let action = handle_filter_mode(key_event(KeyCode::Char('F')), "*.rs");
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_filter_mode_esc_cancels() {
        let action = handle_filter_mode(key_event(KeyCode::Esc), "*.rs");
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_filter_mode_enter_applies() {
        let action = handle_filter_mode(key_event(KeyCode::Enter), "*.rs");
        assert!(matches!(action, KeyAction::ApplyFilter { pattern } if pattern == "*.rs"));
    }

    #[test]
    fn test_filter_mode_enter_empty_clears() {
        let action = handle_filter_mode(key_event(KeyCode::Enter), "");
        assert!(matches!(action, KeyAction::ClearFilter));
    }

    #[test]
    fn test_help_mode_question_cancels() {
        let action = handle_help_mode(key_event(KeyCode::Char('?')));
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_fuzzy_finder_ctrl_p_cancels() {
        let action = handle_fuzzy_finder_mode(key_event_with_modifiers(
            KeyCode::Char('p'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_preview_mode_o_cancels() {
        let action = handle_preview_mode(key_event(KeyCode::Char('o')));
        assert!(matches!(action, KeyAction::Cancel));
    }

    // Tests for arrow key focus switching when preview is visible

    fn test_state() -> AppState {
        AppState::new(std::path::PathBuf::from("/tmp"))
    }

    #[test]
    fn test_right_arrow_focus_preview_when_preview_visible() {
        let mut state = test_state();
        state.preview_visible = true;
        let action = handle_browse_mode(&state, key_event(KeyCode::Right));
        assert!(matches!(action, KeyAction::FocusPreview));
    }

    #[test]
    fn test_left_arrow_focus_tree_when_preview_visible() {
        let mut state = test_state();
        state.preview_visible = true;
        let action = handle_browse_mode(&state, key_event(KeyCode::Left));
        assert!(matches!(action, KeyAction::FocusTree));
    }

    #[test]
    fn test_right_arrow_expands_when_preview_not_visible() {
        let state = test_state();
        let action = handle_browse_mode(&state, key_event(KeyCode::Right));
        assert!(matches!(action, KeyAction::Expand));
    }

    #[test]
    fn test_left_arrow_collapses_when_preview_not_visible() {
        let state = test_state();
        let action = handle_browse_mode(&state, key_event(KeyCode::Left));
        assert!(matches!(action, KeyAction::Collapse));
    }

    #[test]
    fn test_l_always_expands_regardless_of_preview() {
        let mut state = test_state();
        state.preview_visible = true;
        let action = handle_browse_mode(&state, key_event(KeyCode::Char('l')));
        assert!(matches!(action, KeyAction::Expand));
    }

    #[test]
    fn test_h_always_collapses_regardless_of_preview() {
        let mut state = test_state();
        state.preview_visible = true;
        let action = handle_browse_mode(&state, key_event(KeyCode::Char('h')));
        assert!(matches!(action, KeyAction::Collapse));
    }
}
