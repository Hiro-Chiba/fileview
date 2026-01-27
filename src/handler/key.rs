//! Keyboard event handling

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

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
    /// Open fuzzy finder
    OpenFuzzyFinder,
    /// Move up in fuzzy finder results
    FuzzyUp,
    /// Move down in fuzzy finder results
    FuzzyDown,
    /// Confirm fuzzy finder selection
    FuzzyConfirm { path: std::path::PathBuf },
}

/// Handle key event and return the resulting action
pub fn handle_key_event(state: &AppState, key: KeyEvent) -> KeyAction {
    match &state.mode {
        ViewMode::Browse => handle_browse_mode(state, key),
        ViewMode::Search { query } => handle_input_mode(key, query),
        ViewMode::Input { buffer, .. } => handle_input_mode(key, buffer),
        ViewMode::Confirm { .. } => handle_confirm_mode(key),
        ViewMode::Preview { .. } => handle_preview_mode(key),
        ViewMode::FuzzyFinder { .. } => handle_fuzzy_finder_mode(key),
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

        // Expand/Collapse
        KeyCode::Right | KeyCode::Char('l') => KeyAction::Expand,
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => KeyAction::Collapse,
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

        // Refresh and toggle
        KeyCode::Char('R') | KeyCode::F(5) => KeyAction::Refresh,
        KeyCode::Char('.') => KeyAction::ToggleHidden,

        // Copy to system clipboard
        KeyCode::Char('c') => KeyAction::CopyPath,
        KeyCode::Char('C') => KeyAction::CopyFilename,

        // Preview
        KeyCode::Char('o') => KeyAction::OpenPreview,
        KeyCode::Char('P') => KeyAction::ToggleQuickPreview,

        // Help
        KeyCode::Char('?') => KeyAction::ShowHelp,

        _ => KeyAction::None,
    }
}

/// Handle keys in input mode (search, rename, new file/dir)
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
        _ => KeyAction::None,
    }
}

/// Handle keys in fuzzy finder mode
fn handle_fuzzy_finder_mode(key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => KeyAction::Cancel,
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
