//! Display and preview action handlers
//!
//! Handles TogglePreview, OpenPreview, Refresh, ToggleHidden, ShowHelp, etc.

use std::path::PathBuf;

use crate::core::{AppState, ViewMode};
use crate::handler::key::KeyAction;
use crate::integrate::{exit_code, PickResult};
use crate::render::TextPreview;
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
        _ => {}
    }
    Ok(())
}

/// Handle preview scroll actions
pub fn handle_preview_scroll(
    action: KeyAction,
    state: &mut AppState,
    text_preview: &mut Option<TextPreview>,
) {
    match action {
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
        _ => {}
    }
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
