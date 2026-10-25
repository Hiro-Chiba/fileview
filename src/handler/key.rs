//! Keyboard event handling

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

use super::keymap::KeyBindingRegistry;
use crate::core::{AppState, FocusTarget, ViewMode};

#[derive(Debug, Clone)]
pub enum KeyAction {
    None,
    Quit,
    /// Quit and change directory (shell integration)
    QuitAndCd,
    MoveUp,
    MoveDown,
    MoveToTop,
    MoveToBottom,
    Expand,
    Collapse,
    ToggleExpand,
    CollapseAll,
    ExpandAll,
    ToggleMark,
    ClearMarks,
    Copy,
    Cut,
    Paste,
    /// Start delete confirmation
    ConfirmDelete,
    ExecuteDelete,
    StartRename,
    StartNewFile,
    StartNewDir,
    StartSearch,
    SearchNext,
    Refresh,
    ToggleHidden,
    /// Copy path to system clipboard
    CopyPath,
    /// Copy filename to system clipboard
    CopyFilename,
    /// Copy file content to system clipboard
    CopyContent,
    /// Copy file content in Claude format to system clipboard
    CopyForClaude,
    /// Copy AI context pack to clipboard
    CopyContextPack,
    /// Copy review-oriented AI context pack to clipboard
    CopyContextPackReview,
    OpenPreview,
    /// Toggle quick preview panel
    ToggleQuickPreview,
    ConfirmInput {
        value: String,
    },
    Cancel,
    PreviewScrollUp,
    PreviewScrollDown,
    PreviewPageUp,
    PreviewPageDown,
    PreviewToTop,
    PreviewToBottom,
    /// Select and quit (pick mode)
    PickSelect,
    /// Select and quit (select mode)
    SelectConfirm,
    ShowHelp,
    /// Toggle focus between tree and preview (side preview mode)
    ToggleFocus,
    /// Focus on tree panel (left)
    FocusTree,
    /// Focus on preview panel (right)
    FocusPreview,
    OpenFuzzyFinder,
    FuzzyUp,
    FuzzyDown,
    FuzzyConfirm {
        path: std::path::PathBuf,
    },
    StartBookmarkSet,
    StartBookmarkJump,
    /// Set bookmark at slot (1-9)
    SetBookmark {
        slot: u8,
    },
    /// Jump to bookmark at slot (1-9)
    JumpToBookmark {
        slot: u8,
    },
    StartFilter,
    ApplyFilter {
        pattern: String,
    },
    ClearFilter,
    /// Cycle sort mode (Name -> Size -> Date -> Name)
    CycleSort,
    SearchPrev,
    PdfPrevPage,
    PdfNextPage,
    GitStage,
    GitUnstage,
    StartBulkRename,
    BulkRenameNextField,
    ExecuteBulkRename {
        from_pattern: String,
        to_pattern: String,
    },
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    /// Run a custom command
    RunCommand {
        name: String,
    },
    OpenSubshell,
    StartVisualSelect,
    SelectAll,
    InvertSelection,
    SelectGitChanged,
    SelectTestPair,
    /// Toggle peek mode (mini preview in status bar)
    TogglePeekMode,
    /// Toggle AI focus mode (ultra-compact UI)
    ToggleAiFocus,
    /// Select files by extension (Ctrl+1..9)
    SelectByExtension {
        index: u8,
    },
    SelectRecentCommit,
    SelectGitStaged,
    SelectRelated,
    SelectErrorContext,
    /// Copy content in compact format (for small AI contexts)
    CopyCompact,
    OpenAiHistory,
    AiHistoryUp,
    AiHistoryDown,
    AiHistorySelect,
    /// Toggle AI follow mode (auto-focus on AI's most recent path)
    ToggleAiFollow,
    OpenAiActivityLog,
    AiActivityLogUp,
    AiActivityLogDown,
    /// Jump to the path under the AI activity log cursor
    AiActivityLogSelect,
    /// Cycle the context budget bar's target model
    CycleBudgetModel,
    /// Open the repository-wide TODO/FIXME aggregator popup
    OpenTodos,
    TodosUp,
    TodosDown,
    /// Jump to the file under the TODO popup cursor
    TodosSelect,
}

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
        ViewMode::AiHistory { .. } => handle_ai_history_mode(key),
        ViewMode::AiActivityLog => handle_ai_activity_log_mode(key),
        ViewMode::BookmarkSet => handle_bookmark_set_mode(key),
        ViewMode::BookmarkJump => handle_bookmark_jump_mode(key),
        ViewMode::Filter { query } => handle_filter_mode(key, query),
        ViewMode::BulkRename {
            from_pattern,
            to_pattern,
            ..
        } => handle_bulk_rename_mode(key, from_pattern, to_pattern),
        ViewMode::Todos { .. } => handle_todos_mode(key),
    }
}

pub fn handle_key_event_with_registry(
    state: &AppState,
    key: KeyEvent,
    registry: &KeyBindingRegistry,
) -> KeyAction {
    match &state.mode {
        ViewMode::Browse => {
            if let Some(action) = registry.lookup_browse(&key) {
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
        ViewMode::AiHistory { .. } => handle_ai_history_mode(key),
        ViewMode::AiActivityLog => handle_ai_activity_log_mode(key),
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
        ViewMode::Todos { .. } => handle_todos_mode(key),
    }
}

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

fn handle_browse_mode(state: &AppState, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::ToggleAiFocus
        }
        // AI activity: toggle follow-mode (Alt+A) and open activity log (Alt+L).
        // These guarded arms must come before the plain 'l'/'L' and 'a'/'A' handlers.
        KeyCode::Char('a') | KeyCode::Char('A') if key.modifiers == KeyModifiers::ALT => {
            KeyAction::ToggleAiFollow
        }
        KeyCode::Char('l') | KeyCode::Char('L') if key.modifiers == KeyModifiers::ALT => {
            KeyAction::OpenAiActivityLog
        }
        KeyCode::Char('m') | KeyCode::Char('M') if key.modifiers == KeyModifiers::ALT => {
            KeyAction::CycleBudgetModel
        }
        KeyCode::Char('\\') => KeyAction::OpenTodos,
        KeyCode::Char('q') => {
            if state.pick_mode {
                KeyAction::Cancel
            } else {
                KeyAction::Quit
            }
        }
        KeyCode::Char('Q') => KeyAction::QuitAndCd,
        KeyCode::Esc => {
            if state.focus_target == FocusTarget::Preview {
                KeyAction::ToggleFocus
            } else if !state.selected_paths.is_empty() {
                KeyAction::ClearMarks
            } else {
                KeyAction::Cancel
            }
        }

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
        KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::SelectGitChanged
        }
        KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::ALT) => {
            KeyAction::SelectGitStaged
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
            if state.preview_visible {
                KeyAction::ToggleFocus
            } else {
                KeyAction::ToggleExpand
            }
        }
        KeyCode::Char('H') => KeyAction::CollapseAll,
        KeyCode::Char('L') => KeyAction::ExpandAll,

        KeyCode::Char(' ') => KeyAction::ToggleMark,
        KeyCode::Enter
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            KeyAction::CopyContextPackReview
        }
        KeyCode::Enter => {
            if state.select_mode {
                KeyAction::SelectConfirm
            } else if state.pick_mode {
                KeyAction::PickSelect
            } else {
                KeyAction::ToggleExpand
            }
        }

        // Ctrl+Shift+Y, Ctrl+Y and Alt+Y must come before plain 'y'.
        KeyCode::Char('Y')
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            KeyAction::CopyContextPack
        }
        KeyCode::Char('y')
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            KeyAction::CopyContextPack
        }
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::CopyForClaude
        }
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::ALT) => KeyAction::CopyCompact,
        KeyCode::Char('y') => KeyAction::Copy,
        KeyCode::Char('d') => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                KeyAction::ConfirmDelete
            } else {
                KeyAction::Cut
            }
        }
        KeyCode::Char('D') | KeyCode::Delete => KeyAction::ConfirmDelete,
        KeyCode::Char('P')
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            KeyAction::OpenAiHistory
        }
        KeyCode::Char('p')
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            KeyAction::OpenAiHistory
        }
        // Ctrl+P and Alt+P must be checked before plain 'p'.
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::OpenFuzzyFinder
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::ALT) => {
            KeyAction::TogglePeekMode
        }
        KeyCode::Char('p') => KeyAction::Paste,

        // Alt+R and Ctrl+R must come before plain 'r'.
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::ALT) => {
            KeyAction::SelectRecentCommit
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::SelectRelated
        }
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::SelectErrorContext
        }

        KeyCode::Char('r') => KeyAction::StartRename,
        KeyCode::Char('a') => KeyAction::StartNewFile,
        KeyCode::Char('A') => KeyAction::StartNewDir,

        KeyCode::Char('/') => KeyAction::StartSearch,
        KeyCode::Char('n') => KeyAction::SearchNext,
        KeyCode::Char('N') => KeyAction::SearchPrev,

        KeyCode::Char('S') => KeyAction::CycleSort,

        KeyCode::Char('R') => {
            if !state.selected_paths.is_empty() {
                KeyAction::StartBulkRename
            } else {
                KeyAction::Refresh
            }
        }
        KeyCode::F(5) => KeyAction::Refresh,
        KeyCode::Char('.') => KeyAction::ToggleHidden,

        KeyCode::Char('c') => KeyAction::CopyPath,
        KeyCode::Char('C') => KeyAction::CopyFilename,
        KeyCode::Char('Y') => KeyAction::CopyContent,

        KeyCode::Char('o') => KeyAction::OpenPreview,
        KeyCode::Char('P') => KeyAction::ToggleQuickPreview,

        KeyCode::Char('?') => KeyAction::ShowHelp,

        KeyCode::Char('V') => KeyAction::StartVisualSelect,
        KeyCode::Char('*') => KeyAction::SelectAll,
        KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::ALT) => {
            KeyAction::InvertSelection
        }

        KeyCode::Char('[') => KeyAction::PdfPrevPage,
        KeyCode::Char(']') => KeyAction::PdfNextPage,

        KeyCode::Char('m') => KeyAction::StartBookmarkSet,
        KeyCode::Char('\'') => KeyAction::StartBookmarkJump,

        KeyCode::Char('F') => {
            if state.filter_pattern.is_some() {
                KeyAction::ClearFilter
            } else {
                KeyAction::StartFilter
            }
        }

        // Alt+S must come before plain 's'.
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::ALT) => KeyAction::OpenSubshell,

        KeyCode::Char('s') => KeyAction::GitStage,
        KeyCode::Char('u') => KeyAction::GitUnstage,

        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::NewTab,
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::CloseTab,
        // Note: gt/gT (vim-style) requires two keystrokes, simplified to Tab for demo
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::ALT) => KeyAction::NextTab,
        KeyCode::Char('T') if key.modifiers.contains(KeyModifiers::ALT) => KeyAction::PrevTab,

        // Smart selection (Ctrl+Shift+T for test pair)
        KeyCode::Char('T') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::SelectTestPair
        }

        KeyCode::Char(c @ '1'..='9') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::SelectByExtension {
                index: c as u8 - b'0',
            }
        }

        _ => KeyAction::None,
    }
}

fn handle_search_mode(key: KeyEvent, current_query: &str) -> KeyAction {
    match key.code {
        KeyCode::Enter => KeyAction::ConfirmInput {
            value: current_query.to_string(),
        },
        // Same key to cancel (toggle behavior)
        KeyCode::Esc => KeyAction::Cancel,
        _ => KeyAction::None, // Buffer updates handled separately
    }
}

fn handle_input_mode(key: KeyEvent, current_buffer: &str) -> KeyAction {
    match key.code {
        KeyCode::Enter => KeyAction::ConfirmInput {
            value: current_buffer.to_string(),
        },
        KeyCode::Esc => KeyAction::Cancel,
        _ => KeyAction::None, // Buffer updates handled separately
    }
}

fn handle_confirm_mode(key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => KeyAction::ExecuteDelete,
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => KeyAction::Cancel,
        _ => KeyAction::None,
    }
}

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
        KeyCode::Char('[') => KeyAction::PdfPrevPage,
        KeyCode::Char(']') => KeyAction::PdfNextPage,
        _ => KeyAction::None,
    }
}

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

fn handle_help_mode(key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('?') => {
            KeyAction::Cancel
        }
        _ => KeyAction::None,
    }
}

fn handle_visual_select_mode(state: &AppState, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('V') => KeyAction::Cancel,

        // Navigation extends the selection in visual mode.
        KeyCode::Up | KeyCode::Char('k') => KeyAction::MoveUp,
        KeyCode::Down | KeyCode::Char('j') => KeyAction::MoveDown,
        KeyCode::Char('g') => KeyAction::MoveToTop,
        KeyCode::Char('G') => KeyAction::MoveToBottom,

        KeyCode::Char('y') => KeyAction::Copy,
        KeyCode::Char('d') => KeyAction::Cut,
        KeyCode::Char('D') | KeyCode::Delete => KeyAction::ConfirmDelete,

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

/// Returns the new buffer content, or None if no change
pub fn update_input_buffer(key: KeyEvent, buffer: &str, cursor: usize) -> Option<(String, usize)> {
    let mut chars: Vec<char> = buffer.chars().collect();
    if cursor > chars.len() {
        return None;
    }

    // Ctrl or Alt alone makes a command key, such as Ctrl+J in the fuzzy
    // finder. Both together are AltGr on Windows, which types characters.
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    match key.code {
        KeyCode::Char(c) if ctrl == alt => {
            chars.insert(cursor, c);
            Some((chars.into_iter().collect(), cursor + 1))
        }
        KeyCode::Backspace => {
            if cursor > 0 {
                chars.remove(cursor - 1);
                Some((chars.into_iter().collect(), cursor - 1))
            } else {
                None
            }
        }
        KeyCode::Delete => {
            if cursor < chars.len() {
                chars.remove(cursor);
                Some((chars.into_iter().collect(), cursor))
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
            if cursor < chars.len() {
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
            if cursor < chars.len() {
                Some((buffer.to_string(), chars.len()))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn create_delete_targets(state: &AppState, focused_path: Option<&PathBuf>) -> Vec<PathBuf> {
    if state.selected_paths.is_empty() {
        focused_path.map(|p| vec![p.clone()]).unwrap_or_default()
    } else {
        state.selected_paths.iter().cloned().collect()
    }
}

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
        KeyCode::Esc => KeyAction::Cancel,
        _ => KeyAction::None, // Text input handled separately
    }
}

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

fn handle_ai_history_mode(key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => KeyAction::Cancel,
        KeyCode::Up | KeyCode::Char('k') => KeyAction::AiHistoryUp,
        KeyCode::Down | KeyCode::Char('j') => KeyAction::AiHistoryDown,
        KeyCode::Enter => KeyAction::AiHistorySelect,
        _ => KeyAction::None,
    }
}

fn handle_ai_activity_log_mode(key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => KeyAction::Cancel,
        KeyCode::Up | KeyCode::Char('k') => KeyAction::AiActivityLogUp,
        KeyCode::Down | KeyCode::Char('j') => KeyAction::AiActivityLogDown,
        KeyCode::Enter => KeyAction::AiActivityLogSelect,
        _ => KeyAction::None,
    }
}

fn handle_todos_mode(key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('\\') => KeyAction::Cancel,
        KeyCode::Up | KeyCode::Char('k') => KeyAction::TodosUp,
        KeyCode::Down | KeyCode::Char('j') => KeyAction::TodosDown,
        KeyCode::Enter => KeyAction::TodosSelect,
        _ => KeyAction::None,
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

    #[test]
    fn command_keys_are_not_typed_into_text_input() {
        let ctrl_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL);
        let alt_p = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::ALT);
        let alt_gr_at = KeyEvent::new(
            KeyCode::Char('@'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        );
        let shift_f = KeyEvent::new(KeyCode::Char('F'), KeyModifiers::SHIFT);

        assert_eq!(update_input_buffer(ctrl_j, "ab", 2), None);
        assert_eq!(update_input_buffer(alt_p, "ab", 2), None);
        assert_eq!(
            update_input_buffer(alt_gr_at, "a", 1),
            Some(("a@".into(), 2))
        );
        assert_eq!(update_input_buffer(shift_f, "", 0), Some(("F".into(), 1)));
    }

    #[test]
    fn fuzzy_finder_ctrl_keys_move_and_close() {
        let ctrl = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
        assert!(matches!(
            handle_fuzzy_finder_mode(ctrl('j')),
            KeyAction::FuzzyDown
        ));
        assert!(matches!(
            handle_fuzzy_finder_mode(ctrl('k')),
            KeyAction::FuzzyUp
        ));
        assert!(matches!(
            handle_fuzzy_finder_mode(ctrl('p')),
            KeyAction::Cancel
        ));
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

    #[test]
    fn test_update_input_buffer_handles_multibyte_characters() {
        let (buffer, cursor) =
            update_input_buffer(key_event(KeyCode::Char('本')), "日", 1).unwrap();
        assert_eq!(buffer, "日本");
        assert_eq!(cursor, 2);

        let (buffer, cursor) =
            update_input_buffer(key_event(KeyCode::Left), &buffer, cursor).unwrap();
        assert_eq!(cursor, 1);
        let (buffer, cursor) =
            update_input_buffer(key_event(KeyCode::Delete), &buffer, cursor).unwrap();
        assert_eq!(buffer, "日");
        assert_eq!(cursor, 1);
        let (buffer, cursor) =
            update_input_buffer(key_event(KeyCode::Backspace), &buffer, cursor).unwrap();
        assert_eq!(buffer, "");
        assert_eq!(cursor, 0);
    }

    #[test]
    fn test_update_input_buffer_rejects_invalid_cursor() {
        assert!(update_input_buffer(key_event(KeyCode::Char('x')), "日", 2).is_none());
    }

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
