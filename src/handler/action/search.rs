use std::path::PathBuf;

use crate::core::{AppState, ViewMode};
use crate::handler::key::KeyAction;

use super::EntrySnapshot;

pub fn handle(action: KeyAction, state: &mut AppState, entries: &[EntrySnapshot]) {
    match action {
        KeyAction::StartSearch => {
            state.mode = ViewMode::Search {
                query: String::new(),
            };
            state.search_matches = None;
        }
        KeyAction::ConfirmInput { value } => {
            // Leave search mode so that `n` and `N` act as navigation keys
            // instead of being typed into the query.
            state.mode = ViewMode::Browse;
            state.last_search = (!value.is_empty()).then_some(value);
            search_direction(state, entries, SearchDirection::Forward);
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

enum SearchDirection {
    Forward,
    Backward,
}

fn search_direction(state: &mut AppState, entries: &[EntrySnapshot], direction: SearchDirection) {
    let query = match &state.mode {
        ViewMode::Search { query } => Some(query.clone()),
        _ => state.last_search.clone(),
    };
    if let Some(query) = query {
        if query.is_empty() {
            state.search_matches = None;
            return;
        }

        let query_lower = query.to_lowercase();

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

        let next_match_idx = match direction {
            SearchDirection::Forward => {
                matches
                    .iter()
                    .position(|&i| i > state.focus_index)
                    .unwrap_or(0) // Wrap to first
            }
            SearchDirection::Backward => {
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

pub fn handle_fuzzy_confirm(path: PathBuf, state: &mut AppState) {
    if !path.as_os_str().is_empty() {
        state.fuzzy_jump_target = Some(path);
    }
    state.mode = ViewMode::Browse;
}
