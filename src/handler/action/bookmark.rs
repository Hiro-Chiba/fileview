//! Bookmark action handlers
//!
//! Handles bookmark set and jump operations

use std::path::PathBuf;

use crate::core::{AppState, ViewMode, BOOKMARK_SLOTS};
use crate::handler::key::KeyAction;
use crate::tree::TreeNavigator;

/// Handle bookmark-related actions
pub fn handle(
    action: KeyAction,
    state: &mut AppState,
    navigator: &mut TreeNavigator,
    focused_path: &Option<PathBuf>,
) -> anyhow::Result<()> {
    match action {
        KeyAction::StartBookmarkSet => {
            state.mode = ViewMode::BookmarkSet;
        }
        KeyAction::StartBookmarkJump => {
            state.mode = ViewMode::BookmarkJump;
        }
        KeyAction::SetBookmark { slot } => {
            if let Some(path) = focused_path {
                let idx = (slot - 1) as usize;
                if idx < BOOKMARK_SLOTS {
                    state.bookmarks[idx] = Some(path.clone());
                    state.set_message(format!("Bookmark {}: {}", slot, path.display()));
                }
            }
            state.mode = ViewMode::Browse;
        }
        KeyAction::JumpToBookmark { slot } => {
            let idx = (slot - 1) as usize;
            if idx < BOOKMARK_SLOTS {
                if let Some(ref path) = state.bookmarks[idx] {
                    let target = path.clone();
                    // Reveal the path in the tree
                    if let Err(e) = navigator.reveal_path(&target) {
                        state.set_message(format!("Failed: jump to bookmark - {}", e));
                    } else {
                        // Find and focus the target
                        let entries = navigator.visible_entries();
                        if let Some(idx) = entries.iter().position(|e| e.path == target) {
                            state.focus_index = idx;
                        }
                    }
                } else {
                    state.set_message(format!("Bookmark {} not set", slot));
                }
            }
            state.mode = ViewMode::Browse;
        }
        _ => {}
    }
    Ok(())
}
