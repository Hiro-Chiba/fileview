//! Action execution handler
//!
//! This module handles the execution of KeyActions, translating them into
//! actual state changes and side effects.

use std::path::{Path, PathBuf};

use crate::action::{file as file_ops, Clipboard, ClipboardContent};
use crate::core::{AppState, InputPurpose, PendingAction, ViewMode};
use crate::handler::key::{create_delete_targets, KeyAction};
use crate::integrate::{exit_code, Callback, OutputFormat, PickResult};
use crate::render::TextPreview;
use crate::tree::TreeNavigator;

/// Result of action execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionResult {
    /// Continue the event loop
    Continue,
    /// Quit with the given exit code
    Quit(i32),
}

/// Snapshot of entry data for use in action handling
#[derive(Debug, Clone)]
pub struct EntrySnapshot {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
}

/// Context for action execution (extracted from Config)
#[derive(Debug, Clone, Default)]
pub struct ActionContext {
    /// Callback to execute on file selection
    pub callback: Option<Callback>,
    /// Output format for pick mode
    pub output_format: OutputFormat,
}

/// Get the target directory for file operations.
/// If the focused path is a directory, use it directly.
/// Otherwise, use its parent directory or fall back to root.
pub fn get_target_directory(focused: Option<&PathBuf>, root: &Path) -> PathBuf {
    focused
        .and_then(|p| {
            if p.is_dir() {
                Some(p.clone())
            } else {
                p.parent().map(|pp| pp.to_path_buf())
            }
        })
        .unwrap_or_else(|| root.to_path_buf())
}

/// Get the filename from a path as a string for display purposes.
pub fn get_filename_str(path: Option<&PathBuf>) -> String {
    path.and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Handle a KeyAction and update state accordingly
pub fn handle_action(
    action: KeyAction,
    state: &mut AppState,
    navigator: &mut TreeNavigator,
    focused_path: &Option<PathBuf>,
    entries: &[EntrySnapshot],
    context: &ActionContext,
    text_preview: &mut Option<TextPreview>,
) -> anyhow::Result<ActionResult> {
    match action {
        KeyAction::None => {}
        KeyAction::Quit => {
            state.should_quit = true;
        }
        KeyAction::Cancel => {
            match &state.mode {
                ViewMode::Browse => {
                    if state.pick_mode {
                        // Cancel in pick mode = exit with cancelled code
                        return Ok(ActionResult::Quit(exit_code::CANCELLED));
                    }
                    state.should_quit = true;
                }
                _ => {
                    state.mode = ViewMode::Browse;
                    state.clear_message();
                }
            }
        }
        KeyAction::MoveUp => {
            state.focus_index = state.focus_index.saturating_sub(1);
        }
        KeyAction::MoveDown => {
            if state.focus_index < entries.len().saturating_sub(1) {
                state.focus_index += 1;
            }
        }
        KeyAction::MoveToTop => {
            state.focus_index = 0;
        }
        KeyAction::MoveToBottom => {
            state.focus_index = entries.len().saturating_sub(1);
        }
        KeyAction::Expand => {
            if let Some(path) = focused_path {
                navigator.expand(path)?;
            }
        }
        KeyAction::Collapse => {
            if let Some(path) = focused_path {
                navigator.collapse(path);
            }
        }
        KeyAction::ToggleExpand => {
            if state.preview_visible {
                // Close side preview panel
                state.preview_visible = false;
            } else if let Some(ref path) = focused_path {
                if path.is_dir() {
                    navigator.toggle_expand(path)?;
                } else {
                    // File: open fullscreen preview
                    state.mode = ViewMode::Preview { scroll: 0 };
                }
            }
        }
        KeyAction::CollapseAll => {
            // Collapse all except root
            let entries_to_collapse: Vec<_> = entries
                .iter()
                .filter(|e| e.is_dir && e.depth > 0)
                .map(|e| e.path.clone())
                .collect();
            for path in entries_to_collapse {
                navigator.collapse(&path);
            }
        }
        KeyAction::ExpandAll => {
            // Expand all directories (limited depth to avoid huge trees)
            let entries_to_expand: Vec<_> = entries
                .iter()
                .filter(|e| e.is_dir && e.depth < 5)
                .map(|e| e.path.clone())
                .collect();
            for path in entries_to_expand {
                navigator.expand(&path)?;
            }
        }
        KeyAction::ToggleMark => {
            if let Some(path) = focused_path {
                if state.selected_paths.contains(path) {
                    state.selected_paths.remove(path);
                } else {
                    state.selected_paths.insert(path.clone());
                }
            }
        }
        KeyAction::ClearMarks => {
            state.selected_paths.clear();
        }
        KeyAction::Copy => {
            let paths: Vec<PathBuf> = if state.selected_paths.is_empty() {
                focused_path.clone().into_iter().collect()
            } else {
                state.selected_paths.iter().cloned().collect()
            };
            if !paths.is_empty() {
                let mut clipboard = Clipboard::new();
                let count = paths.len();
                clipboard.copy(paths);
                state.clipboard = Some(clipboard);
                state.set_message(format!("Copied {} item(s)", count));
            }
        }
        KeyAction::Cut => {
            let paths: Vec<PathBuf> = if state.selected_paths.is_empty() {
                focused_path.clone().into_iter().collect()
            } else {
                state.selected_paths.iter().cloned().collect()
            };
            if !paths.is_empty() {
                let mut clipboard = Clipboard::new();
                let count = paths.len();
                clipboard.cut(paths);
                state.clipboard = Some(clipboard);
                state.set_message(format!("Cut {} item(s)", count));
            }
        }
        KeyAction::Paste => {
            if let Some(ref mut clipboard) = state.clipboard {
                if let Some(content) = clipboard.take() {
                    let dest = get_target_directory(focused_path.as_ref(), &state.root);

                    match content {
                        ClipboardContent::Copy(paths) => {
                            for src in &paths {
                                file_ops::copy_to(src, &dest)?;
                            }
                            state.set_message(format!("Pasted {} item(s)", paths.len()));
                        }
                        ClipboardContent::Cut(paths) => {
                            for src in &paths {
                                if let Some(name) = src.file_name() {
                                    let new_path = dest.join(name);
                                    std::fs::rename(src, new_path)?;
                                }
                            }
                            state.set_message(format!("Moved {} item(s)", paths.len()));
                        }
                    }
                    navigator.reload()?;
                    state.refresh_git_status();
                }
            }
        }
        KeyAction::ConfirmDelete => {
            let targets = create_delete_targets(state, focused_path.as_ref());
            if !targets.is_empty() {
                state.mode = ViewMode::Confirm {
                    action: PendingAction::Delete { targets },
                };
            }
        }
        KeyAction::ExecuteDelete => {
            if let ViewMode::Confirm {
                action: PendingAction::Delete { targets },
            } = &state.mode
            {
                for path in targets {
                    file_ops::delete(path)?;
                }
                state.set_message(format!("Deleted {} item(s)", targets.len()));
                state.selected_paths.clear();
                state.mode = ViewMode::Browse;
                navigator.reload()?;
                state.refresh_git_status();
            }
        }
        KeyAction::StartRename => {
            if let Some(path) = focused_path {
                let name = get_filename_str(Some(path));
                state.mode = ViewMode::Input {
                    purpose: InputPurpose::Rename {
                        original: path.clone(),
                    },
                    buffer: name.clone(),
                    cursor: name.len(),
                };
            }
        }
        KeyAction::StartNewFile => {
            state.mode = ViewMode::Input {
                purpose: InputPurpose::CreateFile,
                buffer: String::new(),
                cursor: 0,
            };
        }
        KeyAction::StartNewDir => {
            state.mode = ViewMode::Input {
                purpose: InputPurpose::CreateDir,
                buffer: String::new(),
                cursor: 0,
            };
        }
        KeyAction::StartSearch => {
            state.mode = ViewMode::Search {
                query: String::new(),
            };
        }
        KeyAction::SearchNext => {
            if let ViewMode::Search { query } = &state.mode {
                if !query.is_empty() {
                    let query_lower = query.to_lowercase();
                    // Find next match starting from current position
                    let start = (state.focus_index + 1) % entries.len();
                    for i in 0..entries.len() {
                        let idx = (start + i) % entries.len();
                        if entries[idx].name.to_lowercase().contains(&query_lower) {
                            state.focus_index = idx;
                            break;
                        }
                    }
                }
            }
        }
        KeyAction::Refresh => {
            navigator.reload()?;
            state.refresh_git_status();
            state.set_message("Refreshed");
        }
        KeyAction::ToggleHidden => {
            state.show_hidden = !state.show_hidden;
            navigator.set_show_hidden(state.show_hidden)?;
            state.set_message(if state.show_hidden {
                "Showing hidden files"
            } else {
                "Hiding hidden files"
            });
        }
        KeyAction::CopyPath => {
            if let Some(path) = focused_path {
                match arboard::Clipboard::new()
                    .and_then(|mut cb| cb.set_text(path.display().to_string()))
                {
                    Ok(_) => state.set_message("Path copied to clipboard"),
                    Err(_) => state.set_message("Failed to copy path to clipboard"),
                }
            }
        }
        KeyAction::CopyFilename => {
            if let Some(path) = focused_path {
                let name = get_filename_str(Some(path));
                match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(name)) {
                    Ok(_) => state.set_message("Filename copied to clipboard"),
                    Err(_) => state.set_message("Failed to copy filename to clipboard"),
                }
            }
        }
        KeyAction::OpenPreview => {
            if matches!(state.mode, ViewMode::Preview { .. }) {
                state.mode = ViewMode::Browse;
            } else {
                state.mode = ViewMode::Preview { scroll: 0 };
            }
        }
        KeyAction::ToggleQuickPreview => {
            state.preview_visible = !state.preview_visible;
        }
        KeyAction::ConfirmInput { value } => {
            match &state.mode {
                ViewMode::Input { purpose, .. } => {
                    let parent = get_target_directory(focused_path.as_ref(), &state.root);
                    match purpose {
                        InputPurpose::CreateFile => {
                            file_ops::create_file(&parent, &value)?;
                            navigator.reload()?;
                            state.refresh_git_status();
                            state.set_message(format!("Created file: {}", value));
                        }
                        InputPurpose::CreateDir => {
                            file_ops::create_dir(&parent, &value)?;
                            navigator.reload()?;
                            state.refresh_git_status();
                            state.set_message(format!("Created directory: {}", value));
                        }
                        InputPurpose::Rename { original } => {
                            file_ops::rename(original, &value)?;
                            navigator.reload()?;
                            state.refresh_git_status();
                            state.set_message(format!("Renamed to: {}", value));
                        }
                    }
                    state.mode = ViewMode::Browse;
                }
                ViewMode::Search { .. } => {
                    // Keep search mode active, just update
                    state.mode = ViewMode::Search { query: value };
                }
                _ => {}
            }
        }
        KeyAction::PreviewScrollUp => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll = tp.scroll.saturating_sub(1);
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll = scroll.saturating_sub(1);
            }
        }
        KeyAction::PreviewScrollDown => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll += 1;
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll += 1;
            }
        }
        KeyAction::PreviewPageUp => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll = tp.scroll.saturating_sub(20);
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll = scroll.saturating_sub(20);
            }
        }
        KeyAction::PreviewPageDown => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll += 20;
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll += 20;
            }
        }
        KeyAction::PreviewToTop => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll = 0;
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll = 0;
            }
        }
        KeyAction::PreviewToBottom => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll = tp.lines.len().saturating_sub(20);
            }
        }
        KeyAction::PickSelect => {
            if state.pick_mode {
                let paths: Vec<PathBuf> = if state.selected_paths.is_empty() {
                    focused_path.clone().into_iter().collect()
                } else {
                    state.selected_paths.iter().cloned().collect()
                };

                if !paths.is_empty() {
                    // Execute callback if configured
                    if let Some(ref callback) = context.callback {
                        for path in &paths {
                            let _ = callback.execute(path);
                        }
                    }

                    // Output paths
                    let result = PickResult::Selected(paths);
                    return Ok(ActionResult::Quit(result.output(context.output_format)?));
                }
            }
        }
        KeyAction::ShowHelp => {
            state.set_message("j/k:move l/h:expand/collapse Space:mark y/d/p:copy/cut/paste D:delete a/A:new r:rename /:search ?:help");
        }
    }

    Ok(ActionResult::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_state(root: &Path) -> AppState {
        AppState::new(root.to_path_buf())
    }

    fn create_test_navigator(root: &Path) -> TreeNavigator {
        TreeNavigator::new(root, false).unwrap()
    }

    fn create_test_entries(navigator: &TreeNavigator) -> Vec<EntrySnapshot> {
        navigator
            .visible_entries()
            .iter()
            .map(|e| EntrySnapshot {
                path: e.path.clone(),
                name: e.name.clone(),
                is_dir: e.is_dir,
                depth: e.depth,
            })
            .collect()
    }

    #[test]
    fn test_action_result_equality() {
        assert_eq!(ActionResult::Continue, ActionResult::Continue);
        assert_eq!(ActionResult::Quit(0), ActionResult::Quit(0));
        assert_ne!(ActionResult::Continue, ActionResult::Quit(0));
        assert_ne!(ActionResult::Quit(0), ActionResult::Quit(1));
    }

    #[test]
    fn test_get_target_directory_with_dir() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("subdir");
        std::fs::create_dir(&dir_path).unwrap();

        let result = get_target_directory(Some(&dir_path), temp.path());
        assert_eq!(result, dir_path);
    }

    #[test]
    fn test_get_target_directory_with_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("file.txt");
        std::fs::write(&file_path, "content").unwrap();

        let result = get_target_directory(Some(&file_path), temp.path());
        assert_eq!(result, temp.path().to_path_buf());
    }

    #[test]
    fn test_get_target_directory_none() {
        let temp = TempDir::new().unwrap();
        let result = get_target_directory(None, temp.path());
        assert_eq!(result, temp.path().to_path_buf());
    }

    #[test]
    fn test_get_filename_str() {
        let path = PathBuf::from("/path/to/file.txt");
        assert_eq!(get_filename_str(Some(&path)), "file.txt");
        assert_eq!(get_filename_str(None), "");
    }

    #[test]
    fn test_move_up_action() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("file1.txt"), "").unwrap();
        std::fs::write(temp.path().join("file2.txt"), "").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        state.focus_index = 2;
        let result = handle_action(
            KeyAction::MoveUp,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert_eq!(result, ActionResult::Continue);
        assert_eq!(state.focus_index, 1);
    }

    #[test]
    fn test_move_down_action() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("file1.txt"), "").unwrap();
        std::fs::write(temp.path().join("file2.txt"), "").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        state.focus_index = 0;
        let result = handle_action(
            KeyAction::MoveDown,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert_eq!(result, ActionResult::Continue);
        assert_eq!(state.focus_index, 1);
    }

    #[test]
    fn test_quit_action() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        let result = handle_action(
            KeyAction::Quit,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert_eq!(result, ActionResult::Continue);
        assert!(state.should_quit);
    }

    #[test]
    fn test_toggle_mark_action() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("file.txt");
        std::fs::write(&file_path, "").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;
        let focused = Some(file_path.clone());

        // Mark
        handle_action(
            KeyAction::ToggleMark,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(state.selected_paths.contains(&file_path));

        // Unmark
        handle_action(
            KeyAction::ToggleMark,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(!state.selected_paths.contains(&file_path));
    }

    #[test]
    fn test_toggle_hidden_action() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        assert!(!state.show_hidden);

        handle_action(
            KeyAction::ToggleHidden,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert!(state.show_hidden);
    }

    #[test]
    fn test_open_preview_action() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // Open preview
        handle_action(
            KeyAction::OpenPreview,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(matches!(state.mode, ViewMode::Preview { .. }));

        // Close preview
        handle_action(
            KeyAction::OpenPreview,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(matches!(state.mode, ViewMode::Browse));
    }

    #[test]
    fn test_toggle_quick_preview_action() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        assert!(!state.preview_visible);

        handle_action(
            KeyAction::ToggleQuickPreview,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert!(state.preview_visible);
    }

    // =========================================================================
    // State Transition Tests (Phase 13.2)
    // These tests verify the behavior fixed in v0.6.1
    // =========================================================================

    /// Test: ToggleExpand with file + side preview visible → closes side preview
    /// This was the bug in v0.6.1 where Enter opened fullscreen instead of closing
    #[test]
    fn test_toggle_expand_file_with_side_preview_closes_panel() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "content").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // Side preview is visible
        state.preview_visible = true;
        let focused = Some(file_path);

        handle_action(
            KeyAction::ToggleExpand,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // Side preview should be closed, NOT fullscreen opened
        assert!(!state.preview_visible, "Side preview should be closed");
        assert!(
            matches!(state.mode, ViewMode::Browse),
            "Should stay in Browse mode, not open fullscreen"
        );
    }

    /// Test: ToggleExpand with file + side preview NOT visible → opens fullscreen
    #[test]
    fn test_toggle_expand_file_without_preview_opens_fullscreen() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "content").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // Side preview is NOT visible
        state.preview_visible = false;
        let focused = Some(file_path);

        handle_action(
            KeyAction::ToggleExpand,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // Fullscreen preview should be opened
        assert!(
            matches!(state.mode, ViewMode::Preview { scroll: 0 }),
            "Should open fullscreen preview for file"
        );
    }

    /// Test: ToggleExpand with directory → toggles expand
    #[test]
    fn test_toggle_expand_directory_toggles_expand() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("subdir");
        std::fs::create_dir(&dir_path).unwrap();
        std::fs::write(dir_path.join("file.txt"), "content").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        let focused = Some(dir_path.clone());
        let initial_count = navigator.visible_count();

        // Expand
        handle_action(
            KeyAction::ToggleExpand,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert!(
            navigator.visible_count() > initial_count,
            "Directory should be expanded"
        );

        // Collapse
        handle_action(
            KeyAction::ToggleExpand,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert_eq!(
            navigator.visible_count(),
            initial_count,
            "Directory should be collapsed"
        );
    }

    /// Test: Cancel in Preview mode → returns to Browse mode
    #[test]
    fn test_cancel_in_preview_mode_returns_to_browse() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // Start in Preview mode
        state.mode = ViewMode::Preview { scroll: 5 };

        handle_action(
            KeyAction::Cancel,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert!(
            matches!(state.mode, ViewMode::Browse),
            "Should return to Browse mode"
        );
    }

    /// Test: Cancel in Browse mode with pick_mode → returns Quit(CANCELLED)
    #[test]
    fn test_cancel_in_browse_pick_mode_returns_cancelled() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        state.pick_mode = true;

        let result = handle_action(
            KeyAction::Cancel,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert_eq!(
            result,
            ActionResult::Quit(exit_code::CANCELLED),
            "Should return Quit with CANCELLED code"
        );
    }

    /// Test: Cancel in Browse mode without pick_mode → sets should_quit
    #[test]
    fn test_cancel_in_browse_normal_mode_sets_quit() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        state.pick_mode = false;

        handle_action(
            KeyAction::Cancel,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert!(state.should_quit, "Should set should_quit flag");
    }

    /// Test: Cancel in Input mode → returns to Browse mode and clears message
    #[test]
    fn test_cancel_in_input_mode_returns_to_browse() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        state.mode = ViewMode::Input {
            purpose: crate::core::InputPurpose::CreateFile,
            buffer: "test.txt".to_string(),
            cursor: 8,
        };
        state.set_message("Creating file...");

        handle_action(
            KeyAction::Cancel,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert!(
            matches!(state.mode, ViewMode::Browse),
            "Should return to Browse mode"
        );
        assert!(state.message.is_none(), "Message should be cleared");
    }

    /// Test: Preview scroll maintains state within text_preview
    #[test]
    fn test_preview_scroll_updates_text_preview() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();

        // Create a text preview with some lines
        let mut text_preview = Some(TextPreview::new("line1\nline2\nline3\nline4\nline5"));
        text_preview.as_mut().unwrap().scroll = 0;

        // Scroll down
        handle_action(
            KeyAction::PreviewScrollDown,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert_eq!(
            text_preview.as_ref().unwrap().scroll,
            1,
            "Scroll should increase by 1"
        );

        // Scroll up
        handle_action(
            KeyAction::PreviewScrollUp,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert_eq!(
            text_preview.as_ref().unwrap().scroll,
            0,
            "Scroll should decrease by 1"
        );
    }

    /// Test: Preview scroll at zero doesn't go negative (saturating)
    #[test]
    fn test_preview_scroll_saturates_at_zero() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();

        let mut text_preview = Some(TextPreview::new("line1\nline2"));
        text_preview.as_mut().unwrap().scroll = 0;

        // Try to scroll up when already at 0
        handle_action(
            KeyAction::PreviewScrollUp,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert_eq!(
            text_preview.as_ref().unwrap().scroll,
            0,
            "Scroll should stay at 0 (saturating)"
        );
    }

    /// Test: Preview page up/down moves by 20 lines
    #[test]
    fn test_preview_page_scroll() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();

        let mut text_preview = Some(TextPreview::new("a\n".repeat(100).as_str()));
        text_preview.as_mut().unwrap().scroll = 0;

        // Page down
        handle_action(
            KeyAction::PreviewPageDown,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert_eq!(
            text_preview.as_ref().unwrap().scroll,
            20,
            "Page down should move by 20"
        );

        // Page up
        handle_action(
            KeyAction::PreviewPageUp,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert_eq!(
            text_preview.as_ref().unwrap().scroll,
            0,
            "Page up should move back by 20"
        );
    }

    /// Test: Preview scroll in fullscreen mode updates ViewMode scroll
    #[test]
    fn test_preview_scroll_updates_viewmode_scroll() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // Start in Preview mode with scroll at 0
        state.mode = ViewMode::Preview { scroll: 0 };

        handle_action(
            KeyAction::PreviewScrollDown,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        if let ViewMode::Preview { scroll } = state.mode {
            assert_eq!(scroll, 1, "ViewMode scroll should increase");
        } else {
            panic!("Should still be in Preview mode");
        }
    }

    /// Test: MoveToTop sets focus_index to 0
    #[test]
    fn test_move_to_top() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("a.txt"), "").unwrap();
        std::fs::write(temp.path().join("b.txt"), "").unwrap();
        std::fs::write(temp.path().join("c.txt"), "").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        state.focus_index = 3;

        handle_action(
            KeyAction::MoveToTop,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert_eq!(state.focus_index, 0);
    }

    /// Test: MoveToBottom sets focus_index to last entry
    #[test]
    fn test_move_to_bottom() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("a.txt"), "").unwrap();
        std::fs::write(temp.path().join("b.txt"), "").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        state.focus_index = 0;
        let last_index = entries.len().saturating_sub(1);

        handle_action(
            KeyAction::MoveToBottom,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert_eq!(state.focus_index, last_index);
    }

    /// Test: ClearMarks clears all selected paths
    #[test]
    fn test_clear_marks() {
        let temp = TempDir::new().unwrap();
        let file1 = temp.path().join("a.txt");
        let file2 = temp.path().join("b.txt");
        std::fs::write(&file1, "").unwrap();
        std::fs::write(&file2, "").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        state.selected_paths.insert(file1);
        state.selected_paths.insert(file2);
        assert_eq!(state.selected_paths.len(), 2);

        handle_action(
            KeyAction::ClearMarks,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert!(state.selected_paths.is_empty());
    }

    /// Test: StartSearch enters Search mode
    #[test]
    fn test_start_search() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        handle_action(
            KeyAction::StartSearch,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert!(matches!(state.mode, ViewMode::Search { query } if query.is_empty()));
    }

    /// Test: StartNewFile enters Input mode for CreateFile
    #[test]
    fn test_start_new_file() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        handle_action(
            KeyAction::StartNewFile,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert!(matches!(
            state.mode,
            ViewMode::Input {
                purpose: crate::core::InputPurpose::CreateFile,
                ..
            }
        ));
    }

    /// Test: StartNewDir enters Input mode for CreateDir
    #[test]
    fn test_start_new_dir() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        handle_action(
            KeyAction::StartNewDir,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert!(matches!(
            state.mode,
            ViewMode::Input {
                purpose: crate::core::InputPurpose::CreateDir,
                ..
            }
        ));
    }

    // =========================================================================
    // Sequence Tests (Phase 13.3)
    // These tests verify multi-step user workflows
    // =========================================================================

    /// Sequence: Navigation → Open Preview → Navigate → Close Preview
    /// Simulates: j → j → o → j → j → q (or Cancel)
    #[test]
    fn test_sequence_navigation_with_preview() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("a.txt"), "content a").unwrap();
        std::fs::write(temp.path().join("b.txt"), "content b").unwrap();
        std::fs::write(temp.path().join("c.txt"), "content c").unwrap();
        std::fs::write(temp.path().join("d.txt"), "content d").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // Initial state
        state.focus_index = 0;
        assert!(matches!(state.mode, ViewMode::Browse));

        // Step 1: Move down (j)
        handle_action(
            KeyAction::MoveDown,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert_eq!(state.focus_index, 1);

        // Step 2: Move down again (j)
        handle_action(
            KeyAction::MoveDown,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert_eq!(state.focus_index, 2);

        // Step 3: Open preview (o)
        handle_action(
            KeyAction::OpenPreview,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(matches!(state.mode, ViewMode::Preview { .. }));

        // Step 4: Scroll in preview (j in preview = scroll down)
        handle_action(
            KeyAction::PreviewScrollDown,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // Step 5: Close preview (Cancel)
        handle_action(
            KeyAction::Cancel,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(matches!(state.mode, ViewMode::Browse));

        // Focus should be preserved after closing preview
        assert_eq!(state.focus_index, 2);
    }

    /// Sequence: Toggle side preview → Enter closes it (v0.6.1 fix)
    /// Simulates: P → Enter
    #[test]
    fn test_sequence_side_preview_toggle_enter() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "content").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;
        let focused = Some(file_path);

        // Initial state: Browse mode, no preview
        assert!(!state.preview_visible);
        assert!(matches!(state.mode, ViewMode::Browse));

        // Step 1: Toggle quick preview (P)
        handle_action(
            KeyAction::ToggleQuickPreview,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(state.preview_visible, "Side preview should be visible");

        // Step 2: ToggleExpand (Enter) should close side preview
        handle_action(
            KeyAction::ToggleExpand,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(
            !state.preview_visible,
            "Side preview should be closed by Enter"
        );
        assert!(
            matches!(state.mode, ViewMode::Browse),
            "Should stay in Browse mode"
        );
    }

    /// Sequence: Search → Enter → SearchNext
    /// Simulates: / → (type query) → Enter → n
    #[test]
    fn test_sequence_search_workflow() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("apple.txt"), "").unwrap();
        std::fs::write(temp.path().join("banana.txt"), "").unwrap();
        std::fs::write(temp.path().join("apricot.txt"), "").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        state.focus_index = 0;

        // Step 1: Start search (/)
        handle_action(
            KeyAction::StartSearch,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(matches!(state.mode, ViewMode::Search { .. }));

        // Step 2: Simulate typing "ap" and confirm (Enter)
        // In real app, buffer is updated by update_input_buffer
        handle_action(
            KeyAction::ConfirmInput {
                value: "ap".to_string(),
            },
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // Mode should stay Search with query
        assert!(matches!(state.mode, ViewMode::Search { ref query } if query == "ap"));

        // Step 3: Search next (n) - should find next match
        // Need to update entries to have current search query in state
        let initial_focus = state.focus_index;
        handle_action(
            KeyAction::SearchNext,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // Focus should have moved to a matching entry
        // (depends on sort order, but should find apple or apricot)
        let new_focus = state.focus_index;
        let focused_name = &entries.get(new_focus).map(|e| e.name.clone());
        assert!(
            focused_name
                .as_ref()
                .map(|n| n.to_lowercase().contains("ap"))
                .unwrap_or(false)
                || new_focus != initial_focus,
            "SearchNext should find a matching entry"
        );
    }

    /// Sequence: Mark files → Copy → Navigate → Paste
    /// Simulates: Space → j → Space → y → G → p
    #[test]
    fn test_sequence_copy_paste_workflow() {
        let temp = TempDir::new().unwrap();
        let source_dir = temp.path();
        let dest_dir = temp.path().join("dest");
        std::fs::create_dir(&dest_dir).unwrap();
        std::fs::write(source_dir.join("file1.txt"), "content1").unwrap();
        std::fs::write(source_dir.join("file2.txt"), "content2").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let mut entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // Find file1.txt in entries
        let file1_idx = entries
            .iter()
            .position(|e| e.name == "file1.txt")
            .unwrap_or(1);
        state.focus_index = file1_idx;
        let file1_path = entries[file1_idx].path.clone();

        // Step 1: Mark file1 (Space)
        handle_action(
            KeyAction::ToggleMark,
            &mut state,
            &mut navigator,
            &Some(file1_path.clone()),
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(state.selected_paths.contains(&file1_path));

        // Step 2: Copy marked files (y)
        handle_action(
            KeyAction::Copy,
            &mut state,
            &mut navigator,
            &Some(file1_path),
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(state.clipboard.is_some());
        assert!(state.message.as_ref().unwrap().contains("Copied"));

        // Step 3: Navigate to dest directory
        let dest_idx = entries.iter().position(|e| e.name == "dest").unwrap_or(0);
        state.focus_index = dest_idx;

        // Step 4: Paste (p)
        let dest_path = Some(dest_dir.clone());
        handle_action(
            KeyAction::Paste,
            &mut state,
            &mut navigator,
            &dest_path,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // Verify file was copied
        assert!(
            dest_dir.join("file1.txt").exists(),
            "File should be copied to destination"
        );
        assert!(state.message.as_ref().unwrap().contains("Pasted"));

        // Refresh entries after paste
        entries = create_test_entries(&navigator);
        assert!(entries.len() > 0);
    }

    /// Sequence: Start rename → Cancel → Start rename again → Confirm
    #[test]
    fn test_sequence_rename_cancel_rename_confirm() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("original.txt");
        std::fs::write(&file_path, "content").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;
        let focused = Some(file_path.clone());

        // Step 1: Start rename (r)
        handle_action(
            KeyAction::StartRename,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(matches!(
            state.mode,
            ViewMode::Input {
                purpose: crate::core::InputPurpose::Rename { .. },
                ..
            }
        ));

        // Step 2: Cancel (Esc)
        handle_action(
            KeyAction::Cancel,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(matches!(state.mode, ViewMode::Browse));
        assert!(file_path.exists(), "File should not be renamed on cancel");

        // Step 3: Start rename again (r)
        handle_action(
            KeyAction::StartRename,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(matches!(state.mode, ViewMode::Input { .. }));

        // Step 4: Confirm with new name (Enter)
        handle_action(
            KeyAction::ConfirmInput {
                value: "renamed.txt".to_string(),
            },
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert!(matches!(state.mode, ViewMode::Browse));
        assert!(
            temp.path().join("renamed.txt").exists(),
            "File should be renamed"
        );
        assert!(!file_path.exists(), "Original file should not exist");
    }

    /// Sequence: Expand directory → Navigate into → Collapse all
    #[test]
    fn test_sequence_expand_navigate_collapse_all() {
        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("nested.txt"), "").unwrap();
        std::fs::write(temp.path().join("root.txt"), "").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        let initial_count = navigator.visible_count();

        // Find subdir
        let subdir_idx = entries.iter().position(|e| e.name == "subdir").unwrap_or(0);
        let subdir_path = Some(subdir.clone());

        // Step 1: Expand directory (l or Enter on dir)
        handle_action(
            KeyAction::Expand,
            &mut state,
            &mut navigator,
            &subdir_path,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        let expanded_count = navigator.visible_count();
        assert!(
            expanded_count > initial_count,
            "Should see nested files after expand"
        );

        // Update entries after expand
        let entries = create_test_entries(&navigator);

        // Step 2: Move focus into expanded directory
        state.focus_index = subdir_idx + 1; // Move to first child

        // Step 3: Collapse all (H)
        handle_action(
            KeyAction::CollapseAll,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        let collapsed_count = navigator.visible_count();
        assert_eq!(
            collapsed_count, initial_count,
            "All directories should be collapsed"
        );
    }

    /// Sequence: Create file → Verify exists → Delete → Confirm
    #[test]
    fn test_sequence_create_delete_workflow() {
        let temp = TempDir::new().unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;
        let focused = Some(temp.path().to_path_buf());

        // Step 1: Start new file (a)
        handle_action(
            KeyAction::StartNewFile,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(matches!(state.mode, ViewMode::Input { .. }));

        // Step 2: Confirm file creation
        handle_action(
            KeyAction::ConfirmInput {
                value: "newfile.txt".to_string(),
            },
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        let new_file = temp.path().join("newfile.txt");
        assert!(new_file.exists(), "New file should be created");

        // Refresh entries
        let entries = create_test_entries(&navigator);
        let new_file_focused = Some(new_file.clone());

        // Step 3: Delete (D) - starts confirmation
        handle_action(
            KeyAction::ConfirmDelete,
            &mut state,
            &mut navigator,
            &new_file_focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(matches!(state.mode, ViewMode::Confirm { .. }));

        // Step 4: Execute delete (y)
        handle_action(
            KeyAction::ExecuteDelete,
            &mut state,
            &mut navigator,
            &new_file_focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert!(!new_file.exists(), "File should be deleted");
        assert!(matches!(state.mode, ViewMode::Browse));
    }

    /// Sequence: Multiple marks → Cut → Paste (move operation)
    #[test]
    fn test_sequence_cut_paste_multiple() {
        let temp = TempDir::new().unwrap();
        let dest_dir = temp.path().join("dest");
        std::fs::create_dir(&dest_dir).unwrap();
        let file1 = temp.path().join("move1.txt");
        let file2 = temp.path().join("move2.txt");
        std::fs::write(&file1, "content1").unwrap();
        std::fs::write(&file2, "content2").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // Mark both files
        state.selected_paths.insert(file1.clone());
        state.selected_paths.insert(file2.clone());

        // Step 1: Cut (d)
        handle_action(
            KeyAction::Cut,
            &mut state,
            &mut navigator,
            &Some(file1.clone()),
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(state.clipboard.is_some());
        assert!(state.message.as_ref().unwrap().contains("Cut"));

        // Files should still exist (not moved yet)
        assert!(file1.exists());
        assert!(file2.exists());

        // Step 2: Navigate to dest and paste
        handle_action(
            KeyAction::Paste,
            &mut state,
            &mut navigator,
            &Some(dest_dir.clone()),
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // Files should be moved
        assert!(
            dest_dir.join("move1.txt").exists(),
            "File1 should be moved to dest"
        );
        assert!(
            dest_dir.join("move2.txt").exists(),
            "File2 should be moved to dest"
        );
        assert!(!file1.exists(), "Original file1 should not exist");
        assert!(!file2.exists(), "Original file2 should not exist");
    }

    // =========================================================================
    // Edge Case Tests (Phase 13.4)
    // These tests verify behavior in unusual or boundary conditions
    // =========================================================================

    /// Edge case: Empty directory - navigation should handle gracefully
    #[test]
    fn test_edge_empty_directory_navigation() {
        let temp = TempDir::new().unwrap();
        // Don't create any files - empty directory

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // Root is always present, so entries should have at least 1
        assert!(entries.len() >= 1);

        // Move down should not panic
        handle_action(
            KeyAction::MoveDown,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // Move up should not panic
        handle_action(
            KeyAction::MoveUp,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // MoveToBottom should work
        handle_action(
            KeyAction::MoveToBottom,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // MoveToTop should work
        handle_action(
            KeyAction::MoveToTop,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
    }

    /// Edge case: Empty directory - expand/collapse should handle gracefully
    #[test]
    fn test_edge_empty_directory_expand() {
        let temp = TempDir::new().unwrap();
        let empty_dir = temp.path().join("empty");
        std::fs::create_dir(&empty_dir).unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        let focused = Some(empty_dir.clone());
        let initial_count = navigator.visible_count();

        // Expand empty directory should not crash
        handle_action(
            KeyAction::Expand,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // Count should be same (no children to show)
        assert_eq!(navigator.visible_count(), initial_count);

        // Collapse should work
        handle_action(
            KeyAction::Collapse,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
    }

    /// Edge case: Symlink handling
    #[cfg(unix)]
    #[test]
    fn test_edge_symlink_file() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let real_file = temp.path().join("real.txt");
        let link_file = temp.path().join("link.txt");
        std::fs::write(&real_file, "content").unwrap();
        symlink(&real_file, &link_file).unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // Find the symlink in entries
        let link_entry = entries.iter().find(|e| e.name == "link.txt");
        assert!(link_entry.is_some(), "Symlink should appear in tree");

        let focused = Some(link_file.clone());

        // Operations on symlink should work
        handle_action(
            KeyAction::ToggleMark,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
        assert!(state.selected_paths.contains(&link_file));
    }

    /// Edge case: Symlink to directory
    #[cfg(unix)]
    #[test]
    fn test_edge_symlink_directory() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let real_dir = temp.path().join("real_dir");
        let link_dir = temp.path().join("link_dir");
        std::fs::create_dir(&real_dir).unwrap();
        std::fs::write(real_dir.join("file.txt"), "content").unwrap();
        symlink(&real_dir, &link_dir).unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        let focused = Some(link_dir.clone());

        // Expand symlink directory should work
        handle_action(
            KeyAction::Expand,
            &mut state,
            &mut navigator,
            &focused,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // Should be able to see contents through symlink
        let new_entries = create_test_entries(&navigator);
        let has_nested = new_entries.iter().any(|e| e.name == "file.txt");
        assert!(has_nested, "Should see files through symlink directory");
    }

    /// Edge case: Deep directory structure
    #[test]
    fn test_edge_deep_directory_structure() {
        let temp = TempDir::new().unwrap();

        // Create a deep structure: /a/b/c/d/e/f/file.txt
        let mut current = temp.path().to_path_buf();
        for dir_name in ["a", "b", "c", "d", "e", "f"] {
            current = current.join(dir_name);
            std::fs::create_dir(&current).unwrap();
        }
        std::fs::write(current.join("deep.txt"), "deep content").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let context = ActionContext::default();
        let mut text_preview = None;

        // Expand all levels
        let mut path = temp.path().to_path_buf();
        for dir_name in ["a", "b", "c", "d", "e", "f"] {
            path = path.join(dir_name);
            let entries = create_test_entries(&navigator);
            handle_action(
                KeyAction::Expand,
                &mut state,
                &mut navigator,
                &Some(path.clone()),
                &entries,
                &context,
                &mut text_preview,
            )
            .unwrap();
        }

        // Verify deep file is visible
        let final_entries = create_test_entries(&navigator);
        let has_deep_file = final_entries.iter().any(|e| e.name == "deep.txt");
        assert!(has_deep_file, "Deep file should be visible after expanding");

        // CollapseAll should work on deep structure
        handle_action(
            KeyAction::CollapseAll,
            &mut state,
            &mut navigator,
            &None,
            &final_entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // Only root should be visible now (plus first level dir)
        let collapsed_entries = create_test_entries(&navigator);
        let no_deep_file = !collapsed_entries.iter().any(|e| e.name == "deep.txt");
        assert!(no_deep_file, "Deep file should not be visible after collapse");
    }

    /// Edge case: Navigation boundary - move up at top
    #[test]
    fn test_edge_move_up_at_top() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("file.txt"), "").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        state.focus_index = 0;

        // Move up at top should stay at 0 (saturating)
        handle_action(
            KeyAction::MoveUp,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert_eq!(state.focus_index, 0, "Should stay at top");
    }

    /// Edge case: Navigation boundary - move down at bottom
    #[test]
    fn test_edge_move_down_at_bottom() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("file.txt"), "").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        let last_index = entries.len().saturating_sub(1);
        state.focus_index = last_index;

        // Move down at bottom should stay at last
        handle_action(
            KeyAction::MoveDown,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert_eq!(state.focus_index, last_index, "Should stay at bottom");
    }

    /// Edge case: File with special characters in name
    #[test]
    fn test_edge_special_characters_filename() {
        let temp = TempDir::new().unwrap();
        let special_file = temp.path().join("file with spaces & special!.txt");
        std::fs::write(&special_file, "content").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // Find the file
        let file_entry = entries
            .iter()
            .find(|e| e.name.contains("special"))
            .map(|e| e.path.clone());
        assert!(file_entry.is_some(), "Special filename should be in tree");

        // Operations should work
        handle_action(
            KeyAction::ToggleMark,
            &mut state,
            &mut navigator,
            &file_entry,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert!(
            state
                .selected_paths
                .iter()
                .any(|p| p.display().to_string().contains("special")),
            "Should be able to mark file with special chars"
        );
    }

    /// Edge case: Unicode filename
    #[test]
    fn test_edge_unicode_filename() {
        let temp = TempDir::new().unwrap();
        let unicode_file = temp.path().join("日本語ファイル.txt");
        std::fs::write(&unicode_file, "日本語の内容").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // Find the file
        let file_entry = entries
            .iter()
            .find(|e| e.name.contains("日本語"))
            .map(|e| e.path.clone());
        assert!(file_entry.is_some(), "Unicode filename should be in tree");

        // Mark should work
        handle_action(
            KeyAction::ToggleMark,
            &mut state,
            &mut navigator,
            &file_entry,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        assert!(!state.selected_paths.is_empty());
    }

    /// Edge case: Copy to system clipboard with no focused path
    #[test]
    fn test_edge_copy_path_no_focus() {
        let temp = TempDir::new().unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // CopyPath with None should not crash
        handle_action(
            KeyAction::CopyPath,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // Should not set a success message (no path to copy)
        // Message might be None or might be an error message
    }

    /// Edge case: SearchNext with empty entries
    #[test]
    fn test_edge_search_next_with_query() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("apple.txt"), "").unwrap();
        std::fs::write(temp.path().join("banana.txt"), "").unwrap();
        std::fs::write(temp.path().join("cherry.txt"), "").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // Set search mode with a query that has no matches
        state.mode = ViewMode::Search {
            query: "xyz_no_match".to_string(),
        };
        state.focus_index = 0;

        // SearchNext should not crash even with no matches
        handle_action(
            KeyAction::SearchNext,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // Focus should wrap around but eventually return to start
        // (or stay in place if no match found)
    }

    /// Edge case: Paste with empty clipboard
    #[test]
    fn test_edge_paste_empty_clipboard() {
        let temp = TempDir::new().unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // Clipboard is None
        assert!(state.clipboard.is_none());

        // Paste should not crash
        handle_action(
            KeyAction::Paste,
            &mut state,
            &mut navigator,
            &Some(temp.path().to_path_buf()),
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();
    }

    /// Edge case: ConfirmDelete with no targets
    #[test]
    fn test_edge_confirm_delete_no_targets() {
        let temp = TempDir::new().unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let entries = create_test_entries(&navigator);
        let context = ActionContext::default();
        let mut text_preview = None;

        // No marks and no focused path
        state.selected_paths.clear();

        handle_action(
            KeyAction::ConfirmDelete,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        // Should not enter confirm mode without targets
        assert!(
            !matches!(state.mode, ViewMode::Confirm { .. }),
            "Should not enter confirm mode without targets"
        );
    }

    /// Edge case: ExpandAll respects depth limit
    #[test]
    fn test_edge_expand_all_depth_limit() {
        let temp = TempDir::new().unwrap();

        // Create nested structure: dir0/dir1/dir2/dir3/dir4/dir5/dir6/deep.txt
        let mut current = temp.path().to_path_buf();
        for i in 0..7 {
            current = current.join(format!("dir{}", i));
            std::fs::create_dir(&current).unwrap();
        }
        std::fs::write(current.join("deep.txt"), "").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let context = ActionContext::default();
        let mut text_preview = None;

        // Manually expand to depth 5 so we can test the depth limit
        // Expand dir0 through dir4 to make dir5 visible
        let mut path = temp.path().to_path_buf();
        for i in 0..5 {
            path = path.join(format!("dir{}", i));
            navigator.toggle_expand(&path).unwrap();
        }

        // Now dir5 should be visible at depth 5
        let entries = create_test_entries(&navigator);
        let has_dir5 = entries.iter().any(|e| e.name == "dir5");
        assert!(has_dir5, "dir5 should be visible after manual expansion");

        // Now call ExpandAll - it should NOT expand dir5 (depth 5 is not < 5)
        handle_action(
            KeyAction::ExpandAll,
            &mut state,
            &mut navigator,
            &None,
            &entries,
            &context,
            &mut text_preview,
        )
        .unwrap();

        let after_expand = create_test_entries(&navigator);

        // dir6 should NOT be visible (dir5 at depth 5 was not expanded due to depth limit)
        let has_dir6 = after_expand.iter().any(|e| e.name == "dir6");
        assert!(
            !has_dir6,
            "dir6 should not be visible - depth limit prevents expansion"
        );
    }
}
