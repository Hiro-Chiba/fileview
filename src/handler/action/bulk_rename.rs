//! Bulk rename action handlers
//!
//! Handles bulk rename operations for multiple files.

use std::path::PathBuf;

use crate::action::file as file_ops;
use crate::core::{AppState, ViewMode};
use crate::handler::key::KeyAction;
use crate::tree::TreeNavigator;

use super::reload_tree;

/// Handle bulk rename actions
pub fn handle(
    action: KeyAction,
    state: &mut AppState,
    navigator: &mut TreeNavigator,
) -> anyhow::Result<()> {
    match action {
        KeyAction::StartBulkRename => {
            if state.selected_paths.is_empty() {
                state.set_message("Select files first (Space to toggle selection)");
                return Ok(());
            }

            // Enter bulk rename mode
            state.mode = ViewMode::BulkRename {
                from_pattern: String::new(),
                to_pattern: String::new(),
                selected_field: 0,
                cursor: 0,
            };
        }

        KeyAction::BulkRenameNextField => {
            if let ViewMode::BulkRename {
                selected_field,
                from_pattern,
                to_pattern,
                ..
            } = &state.mode
            {
                let new_field = (*selected_field + 1) % 2;
                let new_cursor = if new_field == 0 {
                    from_pattern.len()
                } else {
                    to_pattern.len()
                };

                state.mode = ViewMode::BulkRename {
                    from_pattern: from_pattern.clone(),
                    to_pattern: to_pattern.clone(),
                    selected_field: new_field,
                    cursor: new_cursor,
                };
            }
        }

        KeyAction::ExecuteBulkRename {
            from_pattern,
            to_pattern,
        } => {
            execute_bulk_rename(state, navigator, &from_pattern, &to_pattern)?;
        }

        _ => {}
    }

    Ok(())
}

/// Execute the bulk rename operation
fn execute_bulk_rename(
    state: &mut AppState,
    navigator: &mut TreeNavigator,
    from_pattern: &str,
    to_pattern: &str,
) -> anyhow::Result<()> {
    if from_pattern.is_empty() {
        state.set_message("Please enter a pattern to match");
        return Ok(());
    }

    let targets: Vec<PathBuf> = state.selected_paths.iter().cloned().collect();
    let mut success_count = 0;
    let mut fail_count = 0;

    for target in &targets {
        if let Some(filename) = target.file_name().and_then(|n| n.to_str()) {
            if let Some(new_name) = apply_pattern(filename, from_pattern, to_pattern) {
                if new_name != filename {
                    match file_ops::rename(target, &new_name) {
                        Ok(_) => success_count += 1,
                        Err(_) => fail_count += 1,
                    }
                }
            }
        }
    }

    // Clear selection and return to browse mode
    state.selected_paths.clear();
    state.mode = ViewMode::Browse;

    // Refresh tree
    reload_tree(navigator, state)?;

    // Show result message
    let message = if fail_count == 0 {
        if success_count == 0 {
            "No files matched the pattern".to_string()
        } else if success_count == 1 {
            "Renamed 1 file".to_string()
        } else {
            format!("Renamed {} files", success_count)
        }
    } else {
        format!("Renamed {} files, {} failed", success_count, fail_count)
    };
    state.set_message(message);

    Ok(())
}

/// Apply pattern replacement to a filename
///
/// Supports:
/// - Simple string replacement: "old" -> "new"
/// - Wildcard patterns: "*.txt" -> "*.md"
/// - Prefix/suffix: "prefix_" -> "newprefix_", "_suffix" -> "_newsuffix"
fn apply_pattern(filename: &str, from_pattern: &str, to_pattern: &str) -> Option<String> {
    // Handle wildcard pattern (e.g., "*.txt" -> "*.md")
    if from_pattern.starts_with('*') && to_pattern.starts_with('*') {
        let from_suffix = &from_pattern[1..];
        let to_suffix = &to_pattern[1..];

        if let Some(base) = filename.strip_suffix(from_suffix) {
            return Some(format!("{}{}", base, to_suffix));
        }
        return None;
    }

    // Handle prefix pattern (e.g., "old_*" -> "new_*")
    if from_pattern.ends_with('*') && to_pattern.ends_with('*') {
        let from_prefix = &from_pattern[..from_pattern.len() - 1];
        let to_prefix = &to_pattern[..to_pattern.len() - 1];

        if let Some(rest) = filename.strip_prefix(from_prefix) {
            return Some(format!("{}{}", to_prefix, rest));
        }
        return None;
    }

    // Simple string replacement
    if filename.contains(from_pattern) {
        return Some(filename.replace(from_pattern, to_pattern));
    }

    None
}

/// Update bulk rename input buffer
pub fn update_bulk_rename_buffer(key: crossterm::event::KeyEvent, state: &mut AppState) -> bool {
    use crossterm::event::KeyCode;

    if let ViewMode::BulkRename {
        from_pattern,
        to_pattern,
        selected_field,
        cursor,
    } = &state.mode
    {
        let (buffer, cur) = if *selected_field == 0 {
            (from_pattern, *cursor)
        } else {
            (to_pattern, *cursor)
        };

        let result = match key.code {
            KeyCode::Char(c) => {
                let mut new_buffer = buffer.to_string();
                new_buffer.insert(cur, c);
                Some((new_buffer, cur + 1))
            }
            KeyCode::Backspace => {
                if cur > 0 {
                    let mut new_buffer = buffer.to_string();
                    new_buffer.remove(cur - 1);
                    Some((new_buffer, cur - 1))
                } else {
                    None
                }
            }
            KeyCode::Delete => {
                if cur < buffer.len() {
                    let mut new_buffer = buffer.to_string();
                    new_buffer.remove(cur);
                    Some((new_buffer, cur))
                } else {
                    None
                }
            }
            KeyCode::Left => {
                if cur > 0 {
                    Some((buffer.to_string(), cur - 1))
                } else {
                    None
                }
            }
            KeyCode::Right => {
                if cur < buffer.len() {
                    Some((buffer.to_string(), cur + 1))
                } else {
                    None
                }
            }
            KeyCode::Home => {
                if cur > 0 {
                    Some((buffer.to_string(), 0))
                } else {
                    None
                }
            }
            KeyCode::End => {
                if cur < buffer.len() {
                    Some((buffer.to_string(), buffer.len()))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some((new_buffer, new_cursor)) = result {
            state.mode = if *selected_field == 0 {
                ViewMode::BulkRename {
                    from_pattern: new_buffer,
                    to_pattern: to_pattern.clone(),
                    selected_field: *selected_field,
                    cursor: new_cursor,
                }
            } else {
                ViewMode::BulkRename {
                    from_pattern: from_pattern.clone(),
                    to_pattern: new_buffer,
                    selected_field: *selected_field,
                    cursor: new_cursor,
                }
            };
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_apply_pattern_suffix_wildcard() {
        assert_eq!(
            apply_pattern("test.txt", "*.txt", "*.md"),
            Some("test.md".to_string())
        );
        assert_eq!(apply_pattern("test.rs", "*.txt", "*.md"), None);
    }

    #[test]
    fn test_apply_pattern_prefix_wildcard() {
        assert_eq!(
            apply_pattern("old_file.txt", "old_*", "new_*"),
            Some("new_file.txt".to_string())
        );
        assert_eq!(apply_pattern("other_file.txt", "old_*", "new_*"), None);
    }

    #[test]
    fn test_apply_pattern_simple_replacement() {
        assert_eq!(
            apply_pattern("my_old_file.txt", "old", "new"),
            Some("my_new_file.txt".to_string())
        );
        assert_eq!(apply_pattern("myfile.txt", "old", "new"), None);
    }

    #[test]
    fn test_apply_pattern_multiple_occurrences() {
        // Multiple occurrences should all be replaced
        assert_eq!(
            apply_pattern("old_old_file.txt", "old", "new"),
            Some("new_new_file.txt".to_string())
        );
    }

    #[test]
    fn test_apply_pattern_extension_only() {
        // Change extension only
        assert_eq!(
            apply_pattern("document.doc", "*.doc", "*.docx"),
            Some("document.docx".to_string())
        );
    }

    #[test]
    fn test_apply_pattern_remove_prefix() {
        // Remove prefix entirely
        assert_eq!(
            apply_pattern("backup_file.txt", "backup_*", "*"),
            Some("file.txt".to_string())
        );
    }

    #[test]
    fn test_apply_pattern_add_prefix() {
        // Add prefix - "*" matches empty prefix, so "file.txt" -> "backup_file.txt"
        assert_eq!(
            apply_pattern("file.txt", "*", "backup_*"),
            Some("backup_file.txt".to_string())
        );
    }

    #[test]
    fn test_apply_pattern_case_sensitive() {
        // Should be case sensitive
        assert_eq!(apply_pattern("TEST.txt", "*.TXT", "*.md"), None);
        assert_eq!(
            apply_pattern("TEST.TXT", "*.TXT", "*.md"),
            Some("TEST.md".to_string())
        );
    }

    #[test]
    fn test_apply_pattern_empty_to_pattern() {
        // Replace with empty string
        assert_eq!(
            apply_pattern("file_backup.txt", "_backup", ""),
            Some("file.txt".to_string())
        );
    }

    #[test]
    fn test_apply_pattern_no_match() {
        assert_eq!(apply_pattern("file.txt", "*.rs", "*.md"), None);
        assert_eq!(apply_pattern("file.txt", "old_*", "new_*"), None);
        assert_eq!(apply_pattern("file.txt", "xyz", "abc"), None);
    }

    #[test]
    fn test_update_bulk_rename_buffer_char_input() {
        let mut state = AppState::new(PathBuf::from("/tmp"));
        state.mode = ViewMode::BulkRename {
            from_pattern: String::new(),
            to_pattern: String::new(),
            selected_field: 0,
            cursor: 0,
        };

        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let result = update_bulk_rename_buffer(key, &mut state);

        assert!(result);
        if let ViewMode::BulkRename {
            from_pattern,
            cursor,
            ..
        } = &state.mode
        {
            assert_eq!(from_pattern, "a");
            assert_eq!(*cursor, 1);
        } else {
            panic!("Expected BulkRename mode");
        }
    }

    #[test]
    fn test_update_bulk_rename_buffer_backspace() {
        let mut state = AppState::new(PathBuf::from("/tmp"));
        state.mode = ViewMode::BulkRename {
            from_pattern: "abc".to_string(),
            to_pattern: String::new(),
            selected_field: 0,
            cursor: 3,
        };

        let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        let result = update_bulk_rename_buffer(key, &mut state);

        assert!(result);
        if let ViewMode::BulkRename {
            from_pattern,
            cursor,
            ..
        } = &state.mode
        {
            assert_eq!(from_pattern, "ab");
            assert_eq!(*cursor, 2);
        } else {
            panic!("Expected BulkRename mode");
        }
    }

    #[test]
    fn test_update_bulk_rename_buffer_cursor_movement() {
        let mut state = AppState::new(PathBuf::from("/tmp"));
        state.mode = ViewMode::BulkRename {
            from_pattern: "test".to_string(),
            to_pattern: String::new(),
            selected_field: 0,
            cursor: 2,
        };

        // Move left
        let key = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        update_bulk_rename_buffer(key, &mut state);

        if let ViewMode::BulkRename { cursor, .. } = &state.mode {
            assert_eq!(*cursor, 1);
        }

        // Move right
        let key = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        update_bulk_rename_buffer(key, &mut state);

        if let ViewMode::BulkRename { cursor, .. } = &state.mode {
            assert_eq!(*cursor, 2);
        }
    }

    #[test]
    fn test_update_bulk_rename_buffer_home_end() {
        let mut state = AppState::new(PathBuf::from("/tmp"));
        state.mode = ViewMode::BulkRename {
            from_pattern: "test".to_string(),
            to_pattern: String::new(),
            selected_field: 0,
            cursor: 2,
        };

        // Home
        let key = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
        update_bulk_rename_buffer(key, &mut state);

        if let ViewMode::BulkRename { cursor, .. } = &state.mode {
            assert_eq!(*cursor, 0);
        }

        // End
        let key = KeyEvent::new(KeyCode::End, KeyModifiers::NONE);
        update_bulk_rename_buffer(key, &mut state);

        if let ViewMode::BulkRename { cursor, .. } = &state.mode {
            assert_eq!(*cursor, 4);
        }
    }

    #[test]
    fn test_update_bulk_rename_buffer_delete() {
        let mut state = AppState::new(PathBuf::from("/tmp"));
        state.mode = ViewMode::BulkRename {
            from_pattern: "test".to_string(),
            to_pattern: String::new(),
            selected_field: 0,
            cursor: 1,
        };

        let key = KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE);
        let result = update_bulk_rename_buffer(key, &mut state);

        assert!(result);
        if let ViewMode::BulkRename {
            from_pattern,
            cursor,
            ..
        } = &state.mode
        {
            assert_eq!(from_pattern, "tst");
            assert_eq!(*cursor, 1);
        }
    }

    #[test]
    fn test_update_bulk_rename_buffer_second_field() {
        let mut state = AppState::new(PathBuf::from("/tmp"));
        state.mode = ViewMode::BulkRename {
            from_pattern: "from".to_string(),
            to_pattern: String::new(),
            selected_field: 1, // Second field (to_pattern)
            cursor: 0,
        };

        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        update_bulk_rename_buffer(key, &mut state);

        if let ViewMode::BulkRename {
            from_pattern,
            to_pattern,
            ..
        } = &state.mode
        {
            assert_eq!(from_pattern, "from"); // Unchanged
            assert_eq!(to_pattern, "x"); // Changed
        }
    }

    #[test]
    fn test_update_bulk_rename_buffer_wrong_mode() {
        let mut state = AppState::new(PathBuf::from("/tmp"));
        state.mode = ViewMode::Browse; // Not in BulkRename mode

        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let result = update_bulk_rename_buffer(key, &mut state);

        assert!(!result);
    }
}
