//! Action execution handler

mod bookmark;
mod bulk_rename;
pub mod display;
mod file_ops;
mod filter;
mod git_ops;
mod input;
mod navigation;
mod search;
mod selection;
mod tree_ops;

pub use bulk_rename::update_bulk_rename_buffer;
pub use filter::matches_filter;

use std::path::{Path, PathBuf};

use crate::core::{AppState, ViewMode};
use crate::handler::key::KeyAction;
use crate::integrate::{Callback, OutputFormat};
use crate::render::{
    ArchivePreview, CustomPreview, DiffPreview, HexPreview, PdfPreview, Picker, TextPreview,
};
use crate::tree::TreeNavigator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionResult {
    Continue,
    /// Quit with the given exit code
    Quit(i32),
}

#[derive(Debug, Clone)]
pub struct EntrySnapshot {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
}

/// Extracted from Config
#[derive(Debug, Clone, Default)]
pub struct ActionContext {
    /// Callback to execute on file selection
    pub callback: Option<Callback>,
    /// Output format for pick mode
    pub output_format: OutputFormat,
}

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

pub fn get_filename_str(path: Option<&PathBuf>) -> String {
    path.and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Reload the tree navigator and refresh git status.
pub fn reload_tree(navigator: &mut TreeNavigator, state: &mut AppState) -> anyhow::Result<()> {
    navigator.reload()?;
    state.refresh_git_status();
    Ok(())
}

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
        KeyAction::None => Ok(ActionResult::Continue),

        KeyAction::Quit | KeyAction::QuitAndCd | KeyAction::Cancel => {
            display::handle_app_control(action, state, focused_path)
        }

        KeyAction::MoveUp
        | KeyAction::MoveDown
        | KeyAction::MoveToTop
        | KeyAction::MoveToBottom => {
            navigation::handle(action, state, entries);
            Ok(ActionResult::Continue)
        }

        KeyAction::Expand
        | KeyAction::Collapse
        | KeyAction::ToggleExpand
        | KeyAction::CollapseAll
        | KeyAction::ExpandAll => {
            tree_ops::handle(action, state, navigator, focused_path, entries)?;
            Ok(ActionResult::Continue)
        }

        KeyAction::ToggleMark | KeyAction::ClearMarks | KeyAction::Copy | KeyAction::Cut => {
            selection::handle(action, state, focused_path);
            Ok(ActionResult::Continue)
        }

        KeyAction::Paste
        | KeyAction::ConfirmDelete
        | KeyAction::ExecuteDelete
        | KeyAction::StartRename
        | KeyAction::StartNewFile
        | KeyAction::StartNewDir => {
            file_ops::handle(action, state, navigator, focused_path, entries)?;
            Ok(ActionResult::Continue)
        }

        KeyAction::StartSearch | KeyAction::SearchNext | KeyAction::SearchPrev => {
            search::handle(action, state, entries);
            Ok(ActionResult::Continue)
        }

        KeyAction::ConfirmInput { .. } if matches!(state.mode, ViewMode::Search { .. }) => {
            search::handle(action, state, entries);
            Ok(ActionResult::Continue)
        }
        KeyAction::ConfirmInput { value } => {
            input::handle_confirm(value, state, navigator, focused_path)?;
            Ok(ActionResult::Continue)
        }

        KeyAction::ToggleHidden
        | KeyAction::OpenPreview
        | KeyAction::ToggleQuickPreview
        | KeyAction::ShowHelp
        | KeyAction::ToggleFocus
        | KeyAction::FocusTree
        | KeyAction::FocusPreview
        | KeyAction::CopyPath
        | KeyAction::CopyFilename
        | KeyAction::CopyContent
        | KeyAction::CopyForClaude
        | KeyAction::Refresh
        | KeyAction::CycleSort => {
            display::handle(action, state, navigator, focused_path)?;
            Ok(ActionResult::Continue)
        }

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

        KeyAction::PickSelect => display::handle_pick_select(state, focused_path, context),

        KeyAction::SelectConfirm => display::handle_select_confirm(state, focused_path, context),

        KeyAction::OpenFuzzyFinder | KeyAction::FuzzyUp | KeyAction::FuzzyDown => {
            search::handle_fuzzy(action, state);
            Ok(ActionResult::Continue)
        }

        KeyAction::FuzzyConfirm { path } => {
            search::handle_fuzzy_confirm(path, state);
            Ok(ActionResult::Continue)
        }

        KeyAction::StartBookmarkSet
        | KeyAction::StartBookmarkJump
        | KeyAction::SetBookmark { .. }
        | KeyAction::JumpToBookmark { .. } => {
            bookmark::handle(action, state, navigator, focused_path)?;
            Ok(ActionResult::Continue)
        }

        KeyAction::StartFilter | KeyAction::ApplyFilter { .. } | KeyAction::ClearFilter => {
            filter::handle(action, state);
            Ok(ActionResult::Continue)
        }

        KeyAction::PdfPrevPage | KeyAction::PdfNextPage => {
            display::handle_pdf_navigation(action, state, pdf_preview, image_picker);
            Ok(ActionResult::Continue)
        }

        KeyAction::GitStage | KeyAction::GitUnstage => {
            git_ops::handle(action, state, focused_path.as_ref());
            Ok(ActionResult::Continue)
        }

        KeyAction::StartBulkRename
        | KeyAction::BulkRenameNextField
        | KeyAction::ExecuteBulkRename { .. } => {
            bulk_rename::handle(action, state, navigator)?;
            Ok(ActionResult::Continue)
        }

        // Tabs are handled in the event loop.
        KeyAction::NewTab | KeyAction::CloseTab | KeyAction::NextTab | KeyAction::PrevTab => {
            Ok(ActionResult::Continue)
        }

        KeyAction::StartVisualSelect => {
            state.mode = ViewMode::VisualSelect {
                anchor: state.focus_index,
            };
            if let Some(entry) = entries.get(state.focus_index) {
                state.selected_paths.insert(entry.path.clone());
            }
            state.set_message("Visual select mode (V to exit, j/k to extend)");
            Ok(ActionResult::Continue)
        }

        KeyAction::SelectAll | KeyAction::InvertSelection => {
            selection::handle_with_entries(action, state, entries);
            Ok(ActionResult::Continue)
        }

        KeyAction::SelectGitChanged => {
            selection::select_git_changed(state, entries);
            Ok(ActionResult::Continue)
        }

        KeyAction::SelectTestPair => {
            selection::select_test_pair(state, focused_path);
            Ok(ActionResult::Continue)
        }

        KeyAction::SelectByExtension { index } => {
            selection::select_by_extension(state, entries, index);
            Ok(ActionResult::Continue)
        }

        KeyAction::SelectRecentCommit => {
            selection::select_recent_commit(state, entries);
            Ok(ActionResult::Continue)
        }

        KeyAction::SelectGitStaged => {
            selection::select_git_staged(state, entries);
            Ok(ActionResult::Continue)
        }

        KeyAction::SelectRelated => {
            selection::select_related_files(state, entries, focused_path);
            Ok(ActionResult::Continue)
        }

        KeyAction::SelectErrorContext => {
            selection::select_error_context(state, entries);
            Ok(ActionResult::Continue)
        }

        KeyAction::TogglePeekMode
        | KeyAction::CopyCompact
        | KeyAction::CopyContextPack
        | KeyAction::CopyContextPackReview
        | KeyAction::ToggleAiFocus
        | KeyAction::OpenAiHistory
        | KeyAction::AiHistoryUp
        | KeyAction::AiHistoryDown
        | KeyAction::AiHistorySelect
        | KeyAction::ToggleAiFollow
        | KeyAction::OpenAiActivityLog
        | KeyAction::AiActivityLogUp
        | KeyAction::AiActivityLogDown
        | KeyAction::AiActivityLogSelect
        | KeyAction::CycleBudgetModel
        | KeyAction::OpenTodos
        | KeyAction::TodosUp
        | KeyAction::TodosDown
        | KeyAction::TodosSelect => {
            display::handle(action, state, navigator, focused_path)?;
            Ok(ActionResult::Continue)
        }
    }
}

#[cfg(test)]
mod tests;
