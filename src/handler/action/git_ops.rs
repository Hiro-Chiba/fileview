//! Git operation action handlers
//!
//! Handles git stage and unstage actions.

use std::path::PathBuf;

use crate::core::AppState;
use crate::git;
use crate::handler::key::KeyAction;

/// Handle git operations (stage, unstage)
pub fn handle(action: KeyAction, state: &mut AppState, focused_path: Option<&PathBuf>) {
    // Git operations require a git repo
    let Some(ref git_status) = state.git_status else {
        state.set_message("Not in a git repository");
        return;
    };

    let repo_root = git_status.repo_root().to_path_buf();

    // Get target files (selected or focused)
    let targets: Vec<PathBuf> = if state.selected_paths.is_empty() {
        focused_path.map(|p| vec![p.clone()]).unwrap_or_default()
    } else {
        state.selected_paths.iter().cloned().collect()
    };

    if targets.is_empty() {
        state.set_message("No file selected");
        return;
    }

    match action {
        KeyAction::GitStage => {
            let mut success_count = 0;
            let mut fail_count = 0;

            for target in &targets {
                match git::stage(&repo_root, target) {
                    Ok(()) => success_count += 1,
                    Err(_) => fail_count += 1,
                }
            }

            // Refresh git status after staging
            state.refresh_git_status();

            // Show result message
            let message = if fail_count == 0 {
                if success_count == 1 {
                    "Staged 1 file".to_string()
                } else {
                    format!("Staged {} files", success_count)
                }
            } else {
                format!("Staged {} files, {} failed", success_count, fail_count)
            };
            state.set_message(message);
        }

        KeyAction::GitUnstage => {
            let mut success_count = 0;
            let mut fail_count = 0;

            for target in &targets {
                match git::unstage(&repo_root, target) {
                    Ok(()) => success_count += 1,
                    Err(_) => fail_count += 1,
                }
            }

            // Refresh git status after unstaging
            state.refresh_git_status();

            // Show result message
            let message = if fail_count == 0 {
                if success_count == 1 {
                    "Unstaged 1 file".to_string()
                } else {
                    format!("Unstaged {} files", success_count)
                }
            } else {
                format!("Unstaged {} files, {} failed", success_count, fail_count)
            };
            state.set_message(message);
        }

        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> AppState {
        AppState::new(PathBuf::from("/tmp"))
    }

    #[test]
    fn test_git_stage_no_repo() {
        let mut state = test_state();
        handle(KeyAction::GitStage, &mut state, None);
        assert_eq!(state.message, Some("Not in a git repository".to_string()));
    }

    #[test]
    fn test_git_unstage_no_repo() {
        let mut state = test_state();
        handle(KeyAction::GitUnstage, &mut state, None);
        assert_eq!(state.message, Some("Not in a git repository".to_string()));
    }
}
