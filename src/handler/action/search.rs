//! Search action handlers
//!
//! Handles StartSearch, SearchNext, and fuzzy finder actions

use std::path::PathBuf;

use crate::core::{AppState, ViewMode};
use crate::handler::key::KeyAction;

use super::EntrySnapshot;

/// Handle search actions
pub fn handle(action: KeyAction, state: &mut AppState, entries: &[EntrySnapshot]) {
    match action {
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
        _ => {}
    }
}

/// Handle fuzzy finder actions
pub fn handle_fuzzy(action: KeyAction, state: &mut AppState) {
    match action {
        KeyAction::OpenFuzzyFinder => {
            state.mode = ViewMode::FuzzyFinder {
                query: String::new(),
                selected: 0,
            };
        }
        KeyAction::FuzzyUp => {
            if let ViewMode::FuzzyFinder { selected, .. } = &mut state.mode {
                *selected = selected.saturating_sub(1);
            }
        }
        KeyAction::FuzzyDown => {
            if let ViewMode::FuzzyFinder { selected, .. } = &mut state.mode {
                *selected += 1;
                // Upper bound will be enforced by the render function
            }
        }
        _ => {}
    }
}

/// Handle fuzzy finder confirm action
pub fn handle_fuzzy_confirm(path: PathBuf, state: &mut AppState) {
    if !path.as_os_str().is_empty() {
        // Jump to the selected path in the tree
        state.fuzzy_jump_target = Some(path);
    }
    state.mode = ViewMode::Browse;
}
