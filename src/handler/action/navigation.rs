//! Navigation action handlers
//!
//! Handles MoveUp, MoveDown, MoveToTop, MoveToBottom

use crate::core::AppState;
use crate::handler::key::KeyAction;

use super::EntrySnapshot;

/// Handle navigation actions
pub fn handle(action: KeyAction, state: &mut AppState, entries: &[EntrySnapshot]) {
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
}
