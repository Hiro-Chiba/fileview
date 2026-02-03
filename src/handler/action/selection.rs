//! Selection and clipboard action handlers
//!
//! Handles ToggleMark, ClearMarks, Copy, Cut, SelectAll, InvertSelection

use std::path::PathBuf;

use crate::action::Clipboard;
use crate::core::AppState;
use crate::handler::key::KeyAction;

use super::EntrySnapshot;

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

/// Handle selection with entries context
pub fn handle_with_entries(action: KeyAction, state: &mut AppState, entries: &[EntrySnapshot]) {
    match action {
        KeyAction::SelectAll => {
            // Toggle: if all are selected, deselect all; otherwise select all
            let all_paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
            let all_selected = all_paths.iter().all(|p| state.selected_paths.contains(p));

            if all_selected {
                state.selected_paths.clear();
                state.set_message("Cleared all selections");
            } else {
                for path in all_paths {
                    state.selected_paths.insert(path);
                }
                state.set_message(format!("Selected {} item(s)", entries.len()));
            }
        }
        KeyAction::InvertSelection => {
            let all_paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
            let mut new_selection = std::collections::HashSet::new();

            for path in all_paths {
                if !state.selected_paths.contains(&path) {
                    new_selection.insert(path);
                }
            }

            state.selected_paths = new_selection;
            state.set_message(format!(
                "Inverted selection: {} item(s)",
                state.selected_paths.len()
            ));
        }
        _ => {}
    }
}

/// Select range of entries (for visual select mode)
pub fn select_range(
    state: &mut AppState,
    entries: &[EntrySnapshot],
    anchor: usize,
    current: usize,
) {
    let start = anchor.min(current);
    let end = anchor.max(current);

    // Clear previous selection and select the range
    state.selected_paths.clear();
    for i in start..=end {
        if let Some(entry) = entries.get(i) {
            state.selected_paths.insert(entry.path.clone());
        }
    }
}
