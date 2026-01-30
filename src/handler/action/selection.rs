//! Selection and clipboard action handlers
//!
//! Handles ToggleMark, ClearMarks, Copy, Cut

use std::path::PathBuf;

use crate::action::Clipboard;
use crate::core::AppState;
use crate::handler::key::KeyAction;

/// Handle selection and clipboard actions
pub fn handle(action: KeyAction, state: &mut AppState, focused_path: &Option<PathBuf>) {
    match action {
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
        _ => {}
    }
}
