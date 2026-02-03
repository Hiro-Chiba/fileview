//! Action execution handler
//!
//! This module handles the execution of KeyActions, translating them into
//! actual state changes and side effects.

mod bookmark;
mod bulk_rename;
pub mod command;
mod display;
mod file_ops;
mod filter;
mod git_ops;
mod input;
mod navigation;
mod search;
mod selection;
mod tree_ops;

pub use bulk_rename::update_bulk_rename_buffer;
pub use command::{execute_command, CommandResult};
pub use filter::matches_filter;

use std::path::{Path, PathBuf};

use crate::app::CommandsConfig;
use crate::core::{AppState, ViewMode};
use crate::handler::key::KeyAction;
use crate::integrate::{Callback, OutputFormat};
use crate::render::{
    ArchivePreview, CustomPreview, DiffPreview, HexPreview, PdfPreview, Picker, TextPreview,
};
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
    /// Custom commands configuration
    pub commands: CommandsConfig,
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

/// Reload the tree navigator and refresh git status.
/// This is a common pattern used after file operations.
pub fn reload_tree(navigator: &mut TreeNavigator, state: &mut AppState) -> anyhow::Result<()> {
    navigator.reload()?;
    state.refresh_git_status();
    Ok(())
}

/// Handle a KeyAction and update state accordingly
#[allow(clippy::too_many_arguments)]
pub fn handle_action(
    action: KeyAction,
    state: &mut AppState,
    navigator: &mut TreeNavigator,
    focused_path: &Option<PathBuf>,
    entries: &[EntrySnapshot],
    context: &ActionContext,
    text_preview: &mut Option<TextPreview>,
    hex_preview: &mut Option<HexPreview>,
    archive_preview: &mut Option<ArchivePreview>,
    pdf_preview: &mut Option<PdfPreview>,
    diff_preview: &mut Option<DiffPreview>,
    custom_preview: &mut Option<CustomPreview>,
    image_picker: &mut Option<Picker>,
) -> anyhow::Result<ActionResult> {
    // Disable CRUD operations in stdin mode
    if state.stdin_mode {
        let is_crud_action = matches!(
            action,
            KeyAction::StartNewFile
                | KeyAction::StartNewDir
                | KeyAction::StartRename
                | KeyAction::ConfirmDelete
                | KeyAction::ExecuteDelete
                | KeyAction::Paste
                | KeyAction::Refresh
        );
        if is_crud_action {
            state.set_message("File operations disabled in stdin mode");
            return Ok(ActionResult::Continue);
        }
    }

    match action {
        // No action
        KeyAction::None => Ok(ActionResult::Continue),

        // App control
        KeyAction::Quit | KeyAction::QuitAndCd | KeyAction::Cancel => {
            display::handle_app_control(action, state, focused_path)
        }

        // Navigation
        KeyAction::MoveUp
        | KeyAction::MoveDown
        | KeyAction::MoveToTop
        | KeyAction::MoveToBottom => {
            navigation::handle(action, state, entries);
            Ok(ActionResult::Continue)
        }

        // Tree operations
        KeyAction::Expand
        | KeyAction::Collapse
        | KeyAction::ToggleExpand
        | KeyAction::CollapseAll
        | KeyAction::ExpandAll => {
            tree_ops::handle(action, state, navigator, focused_path, entries)?;
            Ok(ActionResult::Continue)
        }

        // Selection and clipboard
        KeyAction::ToggleMark | KeyAction::ClearMarks | KeyAction::Copy | KeyAction::Cut => {
            selection::handle(action, state, focused_path);
            Ok(ActionResult::Continue)
        }

        // File operations
        KeyAction::Paste
        | KeyAction::ConfirmDelete
        | KeyAction::ExecuteDelete
        | KeyAction::StartRename
        | KeyAction::StartNewFile
        | KeyAction::StartNewDir => {
            file_ops::handle(action, state, navigator, focused_path, entries)?;
            Ok(ActionResult::Continue)
        }

        // Search
        KeyAction::StartSearch | KeyAction::SearchNext | KeyAction::SearchPrev => {
            search::handle(action, state, entries);
            Ok(ActionResult::Continue)
        }

        // Input confirmation
        KeyAction::ConfirmInput { value } => {
            input::handle_confirm(value, state, navigator, focused_path)?;
            Ok(ActionResult::Continue)
        }

        // Display and preview
        KeyAction::ToggleHidden
        | KeyAction::OpenPreview
        | KeyAction::ToggleQuickPreview
        | KeyAction::ShowHelp
        | KeyAction::ToggleFocus
        | KeyAction::FocusTree
        | KeyAction::FocusPreview
        | KeyAction::CopyPath
        | KeyAction::CopyFilename
        | KeyAction::Refresh
        | KeyAction::CycleSort => {
            display::handle(action, state, navigator, focused_path)?;
            Ok(ActionResult::Continue)
        }

        // Preview scroll
        KeyAction::PreviewScrollUp
        | KeyAction::PreviewScrollDown
        | KeyAction::PreviewPageUp
        | KeyAction::PreviewPageDown
        | KeyAction::PreviewToTop
        | KeyAction::PreviewToBottom => {
            display::handle_preview_scroll(
                action,
                state,
                text_preview,
                hex_preview,
                archive_preview,
                diff_preview,
                custom_preview,
            );
            Ok(ActionResult::Continue)
        }

        // Pick mode selection
        KeyAction::PickSelect => display::handle_pick_select(state, focused_path, context),

        // Fuzzy finder
        KeyAction::OpenFuzzyFinder | KeyAction::FuzzyUp | KeyAction::FuzzyDown => {
            search::handle_fuzzy(action, state);
            Ok(ActionResult::Continue)
        }

        KeyAction::FuzzyConfirm { path } => {
            search::handle_fuzzy_confirm(path, state);
            Ok(ActionResult::Continue)
        }

        // Bookmarks
        KeyAction::StartBookmarkSet
        | KeyAction::StartBookmarkJump
        | KeyAction::SetBookmark { .. }
        | KeyAction::JumpToBookmark { .. } => {
            bookmark::handle(action, state, navigator, focused_path)?;
            Ok(ActionResult::Continue)
        }

        // Filter
        KeyAction::StartFilter | KeyAction::ApplyFilter { .. } | KeyAction::ClearFilter => {
            filter::handle(action, state);
            Ok(ActionResult::Continue)
        }

        // PDF navigation
        KeyAction::PdfPrevPage | KeyAction::PdfNextPage => {
            display::handle_pdf_navigation(action, state, pdf_preview, image_picker);
            Ok(ActionResult::Continue)
        }

        // Git operations
        KeyAction::GitStage | KeyAction::GitUnstage => {
            git_ops::handle(action, state, focused_path.as_ref());
            Ok(ActionResult::Continue)
        }

        // Bulk rename operations
        KeyAction::StartBulkRename
        | KeyAction::BulkRenameNextField
        | KeyAction::ExecuteBulkRename { .. } => {
            bulk_rename::handle(action, state, navigator)?;
            Ok(ActionResult::Continue)
        }

        // Tab operations (handled in event loop)
        KeyAction::NewTab | KeyAction::CloseTab | KeyAction::NextTab | KeyAction::PrevTab => {
            Ok(ActionResult::Continue)
        }

        // Shell integration - open subshell
        KeyAction::OpenSubshell => {
            command::open_subshell(state, focused_path.as_ref());
            Ok(ActionResult::Continue)
        }

        // Visual selection mode
        KeyAction::StartVisualSelect => {
            state.mode = ViewMode::VisualSelect {
                anchor: state.focus_index,
            };
            // Select current item
            if let Some(entry) = entries.get(state.focus_index) {
                state.selected_paths.insert(entry.path.clone());
            }
            state.set_message("Visual select mode (V to exit, j/k to extend)");
            Ok(ActionResult::Continue)
        }

        // Batch selection operations
        KeyAction::SelectAll | KeyAction::InvertSelection => {
            selection::handle_with_entries(action, state, entries);
            Ok(ActionResult::Continue)
        }

        // Custom command execution
        KeyAction::RunCommand { name } => {
            let selected: Vec<PathBuf> = state.selected_paths.iter().cloned().collect();
            match command::execute_command(
                &name,
                &context.commands,
                focused_path.as_ref().map(|p| p.as_path()),
                &selected,
            ) {
                CommandResult::Success(output) => {
                    if output.is_empty() {
                        state.set_message(format!("Command '{}' executed", name));
                    } else {
                        // Show first line of output
                        let first_line = output.lines().next().unwrap_or("Done");
                        state.set_message(first_line.to_string());
                    }
                }
                CommandResult::Error(err) => {
                    state.set_message(format!("Error: {}", err));
                }
                CommandResult::NotFound => {
                    state.set_message(format!("Command '{}' not found", name));
                }
            }
            Ok(ActionResult::Continue)
        }
    }
}

#[cfg(test)]
mod tests;
