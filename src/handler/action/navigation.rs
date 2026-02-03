//! Navigation action handlers
//!
//! Handles MoveUp, MoveDown, MoveToTop, MoveToBottom

use crate::core::{AppState, ViewMode};
use crate::handler::key::KeyAction;

use super::selection::select_range;
use super::EntrySnapshot;

/// Handle navigation actions
pub fn handle(action: KeyAction, state: &mut AppState, entries: &[EntrySnapshot]) {
    // Get anchor if in visual select mode (before navigation)
    let visual_anchor = if let ViewMode::VisualSelect { anchor } = state.mode {
        Some(anchor)
    } else {
        None
    };

    match action {
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
        _ => {}
    }

    // Update selection range in visual select mode
    if let Some(anchor) = visual_anchor {
        select_range(state, entries, anchor, state.focus_index);
    }
}
