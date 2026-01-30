//! Search action handlers
//!
//! Handles StartSearch, SearchNext, SearchPrev, and fuzzy finder actions

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
            state.search_matches = None;
        }
        KeyAction::SearchNext => {
            search_direction(state, entries, SearchDirection::Forward);
        }
        KeyAction::SearchPrev => {
            search_direction(state, entries, SearchDirection::Backward);
        }
        _ => {}
    }
}

/// Search direction
enum SearchDirection {
    Forward,
    Backward,
}

/// Search in specified direction and update match info
fn search_direction(state: &mut AppState, entries: &[EntrySnapshot], direction: SearchDirection) {
    if let ViewMode::Search { query } = &state.mode {
        if query.is_empty() {
            state.search_matches = None;
            return;
        }

        let query_lower = query.to_lowercase();

        // Collect all matching indices
        let matches: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.name.to_lowercase().contains(&query_lower))
            .map(|(i, _)| i)
            .collect();

        if matches.is_empty() {
            state.search_matches = None;
            state.set_message("No matches");
            return;
        }

        // Calculate next match index based on direction
        let next_match_idx = match direction {
            SearchDirection::Forward => {
                // Find next match after current focus
                matches
                    .iter()
                    .position(|&i| i > state.focus_index)
                    .unwrap_or(0) // Wrap to first
            }
            SearchDirection::Backward => {
                // Find previous match before current focus
                matches
                    .iter()
                    .rev()
                    .position(|&i| i < state.focus_index)
                    .map(|p| matches.len() - 1 - p)
                    .unwrap_or(matches.len() - 1) // Wrap to last
            }
        };

        state.focus_index = matches[next_match_idx];
        state.search_matches = Some((next_match_idx + 1, matches.len()));
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
