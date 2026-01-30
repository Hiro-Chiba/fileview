//! Tree operation action handlers
//!
//! Handles Expand, Collapse, ToggleExpand, CollapseAll, ExpandAll

use std::path::PathBuf;

use crate::core::{AppState, ViewMode};
use crate::handler::key::KeyAction;
use crate::tree::TreeNavigator;

use super::EntrySnapshot;

/// Handle tree operations
pub fn handle(
    action: KeyAction,
    state: &mut AppState,
    navigator: &mut TreeNavigator,
    focused_path: &Option<PathBuf>,
    entries: &[EntrySnapshot],
) -> anyhow::Result<()> {
    match action {
        KeyAction::Expand => {
            if let Some(path) = focused_path {
                navigator.expand(path)?;
            }
        }
        KeyAction::Collapse => {
            if let Some(path) = focused_path {
                navigator.collapse(path);
            }
        }
        KeyAction::ToggleExpand => {
            if state.preview_visible {
                // Close side preview panel
                state.preview_visible = false;
            } else if let Some(ref path) = focused_path {
                if path.is_dir() {
                    navigator.toggle_expand(path)?;
                } else {
                    // File: open fullscreen preview
                    state.mode = ViewMode::Preview { scroll: 0 };
                }
            }
        }
        KeyAction::CollapseAll => {
            // Collapse all except root
            let entries_to_collapse: Vec<_> = entries
                .iter()
                .filter(|e| e.is_dir && e.depth > 0)
                .map(|e| e.path.clone())
                .collect();
            for path in entries_to_collapse {
                navigator.collapse(&path);
            }
        }
        KeyAction::ExpandAll => {
            // Expand all directories (limited depth to avoid huge trees)
            let entries_to_expand: Vec<_> = entries
                .iter()
                .filter(|e| e.is_dir && e.depth < 5)
                .map(|e| e.path.clone())
                .collect();
            for path in entries_to_expand {
                navigator.expand(&path)?;
            }
        }
        _ => {}
    }
    Ok(())
}
