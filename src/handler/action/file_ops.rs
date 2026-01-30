//! File operation action handlers
//!
//! Handles Paste, ConfirmDelete, ExecuteDelete, StartRename, StartNewFile, StartNewDir

use std::path::PathBuf;

use crate::action::{file as file_ops, ClipboardContent};
use crate::core::{AppState, InputPurpose, PendingAction, ViewMode};
use crate::handler::key::{create_delete_targets, KeyAction};
use crate::tree::TreeNavigator;

use super::{get_filename_str, get_target_directory, reload_tree, EntrySnapshot};

/// Handle file operations
pub fn handle(
    action: KeyAction,
    state: &mut AppState,
    navigator: &mut TreeNavigator,
    focused_path: &Option<PathBuf>,
    _entries: &[EntrySnapshot],
) -> anyhow::Result<()> {
    match action {
        KeyAction::Paste => {
            if let Some(ref mut clipboard) = state.clipboard {
                if let Some(content) = clipboard.take() {
                    let dest = get_target_directory(focused_path.as_ref(), &state.root);

                    match content {
                        ClipboardContent::Copy(paths) => {
                            for src in &paths {
                                file_ops::copy_to(src, &dest)?;
                            }
                            state.set_message(format!("Pasted {} item(s)", paths.len()));
                        }
                        ClipboardContent::Cut(paths) => {
                            for src in &paths {
                                if let Some(name) = src.file_name() {
                                    let new_path = dest.join(name);
                                    std::fs::rename(src, new_path)?;
                                }
                            }
                            state.set_message(format!("Moved {} item(s)", paths.len()));
                        }
                    }
                    reload_tree(navigator, state)?;
                }
            }
        }
        KeyAction::ConfirmDelete => {
            let targets = create_delete_targets(state, focused_path.as_ref());
            if !targets.is_empty() {
                state.mode = ViewMode::Confirm {
                    action: PendingAction::Delete { targets },
                };
            }
        }
        KeyAction::ExecuteDelete => {
            if let ViewMode::Confirm {
                action: PendingAction::Delete { targets },
            } = &state.mode
            {
                for path in targets {
                    file_ops::delete(path)?;
                }
                state.set_message(format!("Deleted {} item(s)", targets.len()));
                state.selected_paths.clear();
                state.mode = ViewMode::Browse;
                reload_tree(navigator, state)?;
            }
        }
        KeyAction::StartRename => {
            if let Some(path) = focused_path {
                let name = get_filename_str(Some(path));
                state.mode = ViewMode::Input {
                    purpose: InputPurpose::Rename {
                        original: path.clone(),
                    },
                    buffer: name.clone(),
                    cursor: name.len(),
                };
            }
        }
        KeyAction::StartNewFile => {
            state.mode = ViewMode::Input {
                purpose: InputPurpose::CreateFile,
                buffer: String::new(),
                cursor: 0,
            };
        }
        KeyAction::StartNewDir => {
            state.mode = ViewMode::Input {
                purpose: InputPurpose::CreateDir,
                buffer: String::new(),
                cursor: 0,
            };
        }
        _ => {}
    }
    Ok(())
}
