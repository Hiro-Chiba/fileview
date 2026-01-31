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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_state(root: &Path) -> AppState {
        AppState::new(root.to_path_buf())
    }

    fn create_test_navigator(root: &Path) -> TreeNavigator {
        TreeNavigator::new(root, false).unwrap()
    }

    #[test]
    fn test_start_bookmark_set_changes_mode() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let focused: Option<PathBuf> = None;

        assert_eq!(state.mode, ViewMode::Browse);

        handle(
            KeyAction::StartBookmarkSet,
            &mut state,
            &mut navigator,
            &focused,
        )
        .unwrap();

        assert_eq!(state.mode, ViewMode::BookmarkSet);
    }

    #[test]
    fn test_start_bookmark_jump_changes_mode() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let focused: Option<PathBuf> = None;

        assert_eq!(state.mode, ViewMode::Browse);

        handle(
            KeyAction::StartBookmarkJump,
            &mut state,
            &mut navigator,
            &focused,
        )
        .unwrap();

        assert_eq!(state.mode, ViewMode::BookmarkJump);
    }

    #[test]
    fn test_set_bookmark_stores_path() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "content").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let focused = Some(file_path.clone());

        // Set bookmark at slot 1
        handle(
            KeyAction::SetBookmark { slot: 1 },
            &mut state,
            &mut navigator,
            &focused,
        )
        .unwrap();

        // Verify bookmark is stored
        assert_eq!(state.bookmarks[0], Some(file_path));
        // Verify mode returns to Browse
        assert_eq!(state.mode, ViewMode::Browse);
    }

    #[test]
    fn test_set_bookmark_invalid_slot_ignored() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "content").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let focused = Some(file_path);

        // Try to set bookmark at invalid slot (10 > BOOKMARK_SLOTS)
        handle(
            KeyAction::SetBookmark { slot: 10 },
            &mut state,
            &mut navigator,
            &focused,
        )
        .unwrap();

        // Verify all bookmarks are still None
        for bookmark in &state.bookmarks {
            assert!(bookmark.is_none());
        }
        // Mode should still return to Browse
        assert_eq!(state.mode, ViewMode::Browse);
    }

    #[test]
    fn test_jump_to_unset_bookmark_shows_message() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let focused: Option<PathBuf> = None;

        // Jump to unset bookmark
        handle(
            KeyAction::JumpToBookmark { slot: 1 },
            &mut state,
            &mut navigator,
            &focused,
        )
        .unwrap();

        // Verify error message is set
        assert!(state.message.is_some());
        assert!(state.message.as_ref().unwrap().contains("not set"));
        // Mode should return to Browse
        assert_eq!(state.mode, ViewMode::Browse);
    }

    #[test]
    fn test_jump_to_set_bookmark_reveals_path() {
        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let file_path = subdir.join("target.txt");
        std::fs::write(&file_path, "content").unwrap();

        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let focused: Option<PathBuf> = None;

        // Pre-set bookmark at slot 3
        state.bookmarks[2] = Some(file_path.clone());

        // Jump to bookmark
        handle(
            KeyAction::JumpToBookmark { slot: 3 },
            &mut state,
            &mut navigator,
            &focused,
        )
        .unwrap();

        // Mode should return to Browse
        assert_eq!(state.mode, ViewMode::Browse);
        // Focus should have moved (reveal_path expands the tree)
        // Check that subdir is now expanded in the navigator
        let entries = navigator.visible_entries();
        let target_visible = entries.iter().any(|e| e.path == file_path);
        assert!(target_visible, "Target file should be visible after jump");
    }

    #[test]
    fn test_set_bookmark_without_focus_does_nothing() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let focused: Option<PathBuf> = None;

        handle(
            KeyAction::SetBookmark { slot: 1 },
            &mut state,
            &mut navigator,
            &focused,
        )
        .unwrap();

        // Bookmark should not be set
        assert!(state.bookmarks[0].is_none());
        // Mode should still return to Browse
        assert_eq!(state.mode, ViewMode::Browse);
    }

    #[test]
    fn test_unrelated_action_is_ignored() {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_state(temp.path());
        let mut navigator = create_test_navigator(temp.path());
        let focused: Option<PathBuf> = None;

        // Set initial mode
        state.mode = ViewMode::BookmarkSet;

        // Pass an unrelated action
        handle(KeyAction::MoveUp, &mut state, &mut navigator, &focused).unwrap();

        // Mode should remain unchanged
        assert_eq!(state.mode, ViewMode::BookmarkSet);
    }
}
