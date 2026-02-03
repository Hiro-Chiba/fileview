//! Display and preview action handlers
//!
//! Handles TogglePreview, OpenPreview, Refresh, ToggleHidden, ShowHelp, etc.

use std::fs;
use std::path::PathBuf;

use crate::core::{AppState, ViewMode};
use crate::handler::key::KeyAction;
use crate::integrate::{exit_code, PickResult};
use crate::render::{
    ArchivePreview, CustomPreview, DiffPreview, HexPreview, PdfPreview, Picker, TextPreview,
};
use crate::tree::TreeNavigator;

use super::{get_filename_str, reload_tree, ActionContext, ActionResult};

/// Handle app control actions (Quit, QuitAndCd, Cancel)
pub fn handle_app_control(
    action: KeyAction,
    state: &mut AppState,
    focused_path: &Option<PathBuf>,
) -> anyhow::Result<ActionResult> {
    match action {
        KeyAction::Quit => {
            state.should_quit = true;
            Ok(ActionResult::Continue)
        }
        KeyAction::QuitAndCd => {
            // Store the current directory for shell integration
            if let Some(path) = focused_path {
                if path.is_dir() {
                    state.choosedir_path = Some(path.clone());
                } else if let Some(parent) = path.parent() {
                    state.choosedir_path = Some(parent.to_path_buf());
                }
            }
            state.should_quit = true;
            Ok(ActionResult::Continue)
        }
        KeyAction::Cancel => match &state.mode {
            ViewMode::Browse => {
                if state.pick_mode {
                    // Cancel in pick mode = exit with cancelled code
                    Ok(ActionResult::Quit(exit_code::CANCELLED))
                } else {
                    state.should_quit = true;
                    Ok(ActionResult::Continue)
                }
            }
            _ => {
                state.mode = ViewMode::Browse;
                state.clear_message();
                Ok(ActionResult::Continue)
            }
        },
        _ => Ok(ActionResult::Continue),
    }
}

/// Handle display actions
pub fn handle(
    action: KeyAction,
    state: &mut AppState,
    navigator: &mut TreeNavigator,
    focused_path: &Option<PathBuf>,
) -> anyhow::Result<()> {
    match action {
        KeyAction::Refresh => {
            reload_tree(navigator, state)?;
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
                    Ok(_) => state.set_message("Copied path"),
                    Err(_) => state.set_message("Failed: copy path"),
                }
            }
        }
        KeyAction::CopyFilename => {
            if let Some(path) = focused_path {
                let name = get_filename_str(Some(path));
                match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(name)) {
                    Ok(_) => state.set_message("Copied filename"),
                    Err(_) => state.set_message("Failed: copy filename"),
                }
            }
        }
        KeyAction::CopyContent => {
            let paths = get_copy_target_paths(state, focused_path);
            if paths.is_empty() {
                state.set_message("No file selected");
            } else {
                match copy_file_contents_to_clipboard(&paths) {
                    Ok(count) => state.set_message(format!("Copied {} file(s) content", count)),
                    Err(e) => state.set_message(format!("Failed: {}", e)),
                }
            }
        }
        KeyAction::CopyForClaude => {
            let paths = get_copy_target_paths(state, focused_path);
            if paths.is_empty() {
                state.set_message("No file selected");
            } else {
                match copy_file_contents_claude_format(&paths) {
                    Ok(count) => {
                        state.set_message(format!("Copied {} file(s) (Claude format)", count))
                    }
                    Err(e) => state.set_message(format!("Failed: {}", e)),
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
            // Reset focus to tree when closing preview
            if !state.preview_visible {
                state.reset_focus();
            }
        }
        KeyAction::ShowHelp => {
            state.mode = ViewMode::Help;
        }
        KeyAction::ToggleFocus => {
            state.toggle_focus();
        }
        KeyAction::FocusTree => {
            state.set_focus(crate::core::FocusTarget::Tree);
        }
        KeyAction::FocusPreview => {
            state.set_focus(crate::core::FocusTarget::Preview);
        }
        KeyAction::CycleSort => {
            let new_mode = state.sort_mode.next();
            state.sort_mode = new_mode;
            navigator.set_sort_mode(new_mode)?;
            state.set_message(format!("Sort: {}", new_mode.display_name()));
        }
        KeyAction::TogglePeekMode => {
            state.toggle_peek_mode();
            let mode_name = match state.preview_display_mode {
                crate::core::PreviewDisplayMode::Normal => "Normal",
                crate::core::PreviewDisplayMode::Peek => "Peek",
            };
            state.set_message(format!("Preview: {}", mode_name));
        }
        KeyAction::CopyCompact => {
            let paths = get_copy_target_paths(state, focused_path);
            if paths.is_empty() {
                state.set_message("No file selected");
            } else {
                match copy_file_contents_compact(&paths) {
                    Ok(count) => state.set_message(format!("Copied {} file(s) (compact)", count)),
                    Err(e) => state.set_message(format!("Failed: {}", e)),
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Handle preview scroll actions for text, hex, archive, diff, and custom previews
pub fn handle_preview_scroll(
    action: KeyAction,
    state: &mut AppState,
    text_preview: &mut Option<TextPreview>,
    hex_preview: &mut Option<HexPreview>,
    archive_preview: &mut Option<ArchivePreview>,
    diff_preview: &mut Option<DiffPreview>,
    custom_preview: &mut Option<CustomPreview>,
) {
    match action {
        KeyAction::PreviewScrollUp => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll = tp.scroll.saturating_sub(1);
            }
            if let Some(ref mut hp) = hex_preview {
                hp.scroll = hp.scroll.saturating_sub(1);
            }
            if let Some(ref mut ap) = archive_preview {
                ap.scroll = ap.scroll.saturating_sub(1);
            }
            if let Some(ref mut dp) = diff_preview {
                dp.scroll = dp.scroll.saturating_sub(1);
            }
            if let Some(ref mut cp) = custom_preview {
                cp.scroll = cp.scroll.saturating_sub(1);
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll = scroll.saturating_sub(1);
            }
        }
        KeyAction::PreviewScrollDown => {
            if let Some(ref mut tp) = text_preview {
                let max_scroll = tp.lines.len().saturating_sub(1);
                tp.scroll = (tp.scroll + 1).min(max_scroll);
            }
            if let Some(ref mut hp) = hex_preview {
                let max_scroll = hp.line_count().saturating_sub(1);
                hp.scroll = (hp.scroll + 1).min(max_scroll);
            }
            if let Some(ref mut ap) = archive_preview {
                let max_scroll = ap.line_count().saturating_sub(1);
                ap.scroll = (ap.scroll + 1).min(max_scroll);
            }
            if let Some(ref mut dp) = diff_preview {
                let max_scroll = dp.line_count().saturating_sub(1);
                dp.scroll = (dp.scroll + 1).min(max_scroll);
            }
            if let Some(ref mut cp) = custom_preview {
                let max_scroll = cp.line_count().saturating_sub(1);
                cp.scroll = (cp.scroll + 1).min(max_scroll);
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll += 1;
            }
        }
        KeyAction::PreviewPageUp => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll = tp.scroll.saturating_sub(20);
            }
            if let Some(ref mut hp) = hex_preview {
                hp.scroll = hp.scroll.saturating_sub(20);
            }
            if let Some(ref mut ap) = archive_preview {
                ap.scroll = ap.scroll.saturating_sub(20);
            }
            if let Some(ref mut dp) = diff_preview {
                dp.scroll = dp.scroll.saturating_sub(20);
            }
            if let Some(ref mut cp) = custom_preview {
                cp.scroll = cp.scroll.saturating_sub(20);
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll = scroll.saturating_sub(20);
            }
        }
        KeyAction::PreviewPageDown => {
            if let Some(ref mut tp) = text_preview {
                let max_scroll = tp.lines.len().saturating_sub(1);
                tp.scroll = (tp.scroll + 20).min(max_scroll);
            }
            if let Some(ref mut hp) = hex_preview {
                let max_scroll = hp.line_count().saturating_sub(1);
                hp.scroll = (hp.scroll + 20).min(max_scroll);
            }
            if let Some(ref mut ap) = archive_preview {
                let max_scroll = ap.line_count().saturating_sub(1);
                ap.scroll = (ap.scroll + 20).min(max_scroll);
            }
            if let Some(ref mut dp) = diff_preview {
                let max_scroll = dp.line_count().saturating_sub(1);
                dp.scroll = (dp.scroll + 20).min(max_scroll);
            }
            if let Some(ref mut cp) = custom_preview {
                let max_scroll = cp.line_count().saturating_sub(1);
                cp.scroll = (cp.scroll + 20).min(max_scroll);
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll += 20;
            }
        }
        KeyAction::PreviewToTop => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll = 0;
            }
            if let Some(ref mut hp) = hex_preview {
                hp.scroll = 0;
            }
            if let Some(ref mut ap) = archive_preview {
                ap.scroll = 0;
            }
            if let Some(ref mut dp) = diff_preview {
                dp.scroll = 0;
            }
            if let Some(ref mut cp) = custom_preview {
                cp.scroll = 0;
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll = 0;
            }
        }
        KeyAction::PreviewToBottom => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll = tp.lines.len().saturating_sub(1);
            }
            if let Some(ref mut hp) = hex_preview {
                hp.scroll = hp.line_count().saturating_sub(1);
            }
            if let Some(ref mut ap) = archive_preview {
                ap.scroll = ap.line_count().saturating_sub(1);
            }
            if let Some(ref mut dp) = diff_preview {
                dp.scroll = dp.line_count().saturating_sub(1);
            }
            if let Some(ref mut cp) = custom_preview {
                cp.scroll = cp.line_count().saturating_sub(1);
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                // Set to max for ViewMode as well
                if let Some(ref tp) = text_preview {
                    *scroll = tp.lines.len().saturating_sub(1);
                } else if let Some(ref hp) = hex_preview {
                    *scroll = hp.line_count().saturating_sub(1);
                } else if let Some(ref ap) = archive_preview {
                    *scroll = ap.line_count().saturating_sub(1);
                } else if let Some(ref dp) = diff_preview {
                    *scroll = dp.line_count().saturating_sub(1);
                } else if let Some(ref cp) = custom_preview {
                    *scroll = cp.line_count().saturating_sub(1);
                }
            }
        }
        _ => {}
    }
}

/// Handle PDF page navigation actions
pub fn handle_pdf_navigation(
    action: KeyAction,
    state: &mut AppState,
    pdf_preview: &mut Option<PdfPreview>,
    image_picker: &mut Option<Picker>,
) {
    let Some(ref mut pdf) = pdf_preview else {
        return;
    };
    let Some(ref mut picker) = image_picker else {
        return;
    };

    match action {
        KeyAction::PdfPrevPage => {
            if pdf.current_page > 1 {
                if let Err(e) = pdf.prev_page(picker) {
                    state.set_message(format!("Failed: prev page - {}", e));
                }
            }
        }
        KeyAction::PdfNextPage => {
            if pdf.current_page < pdf.total_pages {
                if let Err(e) = pdf.next_page(picker) {
                    state.set_message(format!("Failed: next page - {}", e));
                }
            }
        }
        _ => {}
    }
}

/// Get paths to copy (selected paths or focused path)
fn get_copy_target_paths(state: &AppState, focused_path: &Option<PathBuf>) -> Vec<PathBuf> {
    if state.selected_paths.is_empty() {
        focused_path
            .as_ref()
            .filter(|p| p.is_file())
            .cloned()
            .into_iter()
            .collect()
    } else {
        state
            .selected_paths
            .iter()
            .filter(|p| p.is_file())
            .cloned()
            .collect()
    }
}

/// Copy file contents to clipboard (plain text)
fn copy_file_contents_to_clipboard(paths: &[PathBuf]) -> anyhow::Result<usize> {
    let mut contents = Vec::new();
    let mut count = 0;

    for (i, path) in paths.iter().enumerate() {
        if let Ok(content) = fs::read_to_string(path) {
            if i > 0 {
                contents.push(String::new());
            }
            contents.push(format!("--- {} ---", path.display()));
            contents.push(content);
            count += 1;
        }
    }

    if count == 0 {
        anyhow::bail!("No readable files");
    }

    let text = contents.join("\n");
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text))
        .map_err(|e| anyhow::anyhow!("Clipboard error: {}", e))?;

    Ok(count)
}

/// Copy file contents to clipboard in Claude-friendly markdown format
fn copy_file_contents_claude_format(paths: &[PathBuf]) -> anyhow::Result<usize> {
    let mut contents = Vec::new();
    let mut count = 0;

    for (i, path) in paths.iter().enumerate() {
        if let Ok(content) = fs::read_to_string(path) {
            if i > 0 {
                contents.push(String::new());
            }

            // Detect file extension for syntax highlighting
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

            contents.push(format!("### File: {}", path.display()));
            contents.push(format!("```{}", ext));
            contents.push(content.trim_end().to_string());
            contents.push("```".to_string());
            count += 1;
        }
    }

    if count == 0 {
        anyhow::bail!("No readable files");
    }

    let text = contents.join("\n");
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text))
        .map_err(|e| anyhow::anyhow!("Clipboard error: {}", e))?;

    Ok(count)
}

/// Copy file contents to clipboard in compact format (for small AI contexts)
/// Uses minimal formatting: just filename and content, no markdown headers
fn copy_file_contents_compact(paths: &[PathBuf]) -> anyhow::Result<usize> {
    let mut contents = Vec::new();
    let mut count = 0;

    for path in paths.iter() {
        if let Ok(content) = fs::read_to_string(path) {
            // Use just filename for compact format
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            contents.push(format!("// {}", filename));
            contents.push(content.trim_end().to_string());
            contents.push(String::new()); // Empty line separator
            count += 1;
        }
    }

    if count == 0 {
        anyhow::bail!("No readable files");
    }

    let text = contents.join("\n").trim_end().to_string();
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text))
        .map_err(|e| anyhow::anyhow!("Clipboard error: {}", e))?;

    Ok(count)
}

/// Handle pick mode selection
pub fn handle_pick_select(
    state: &AppState,
    focused_path: &Option<PathBuf>,
    context: &ActionContext,
) -> anyhow::Result<ActionResult> {
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
    Ok(ActionResult::Continue)
}

/// Handle select mode confirmation
///
/// In select mode:
/// - Single select: Enter outputs focused path immediately
/// - Multi select: Enter outputs all selected paths (or focused if none selected)
pub fn handle_select_confirm(
    state: &AppState,
    focused_path: &Option<PathBuf>,
    context: &ActionContext,
) -> anyhow::Result<ActionResult> {
    if state.select_mode {
        let paths: Vec<PathBuf> = if state.multi_select && !state.selected_paths.is_empty() {
            // Multi-select: output all selected paths
            state.selected_paths.iter().cloned().collect()
        } else {
            // Single select or no selections: output focused path
            focused_path.clone().into_iter().collect()
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
    Ok(ActionResult::Continue)
}
