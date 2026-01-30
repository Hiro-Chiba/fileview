//! Input confirmation handler
//!
//! Handles ConfirmInput for file creation, directory creation, and rename operations

use std::path::PathBuf;

use crate::action::file as file_ops;
use crate::core::{AppState, InputPurpose, ViewMode};
use crate::tree::TreeNavigator;

use super::get_target_directory;

/// Handle input confirmation
pub fn handle_confirm(
    value: String,
    state: &mut AppState,
    navigator: &mut TreeNavigator,
    focused_path: &Option<PathBuf>,
) -> anyhow::Result<()> {
    match &state.mode {
        ViewMode::Input { purpose, .. } => {
            let parent = get_target_directory(focused_path.as_ref(), &state.root);
            match purpose {
                InputPurpose::CreateFile => {
                    file_ops::create_file(&parent, &value)?;
                    navigator.reload()?;
                    state.refresh_git_status();
                    state.set_message(format!("Created file: {}", value));
                }
                InputPurpose::CreateDir => {
                    file_ops::create_dir(&parent, &value)?;
                    navigator.reload()?;
                    state.refresh_git_status();
                    state.set_message(format!("Created directory: {}", value));
                }
                InputPurpose::Rename { original } => {
                    file_ops::rename(original, &value)?;
                    navigator.reload()?;
                    state.refresh_git_status();
                    state.set_message(format!("Renamed to: {}", value));
                }
            }
            state.mode = ViewMode::Browse;
        }
        ViewMode::Search { .. } => {
            // Keep search mode active, just update
            state.mode = ViewMode::Search { query: value };
        }
        _ => {}
    }
    Ok(())
}
