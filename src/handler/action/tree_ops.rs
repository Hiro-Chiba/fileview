use std::path::PathBuf;

use crate::core::{AppState, ViewMode};
use crate::handler::key::KeyAction;
use crate::tree::TreeNavigator;

use super::EntrySnapshot;

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
                state.preview_visible = false;
            } else if let Some(ref path) = focused_path {
                if path.is_dir() {
                    navigator.toggle_expand(path)?;
                } else {
                    state.mode = ViewMode::Preview { scroll: 0 };
                }
            }
        }
        KeyAction::CollapseAll => {
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
            // Depth is capped to avoid expanding huge trees.
            // The depth limit keeps expand all from building a huge tree.
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
