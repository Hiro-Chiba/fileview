//! Smoke tests for individual actions

use tempfile::TempDir;

use super::*;

#[test]
fn test_action_result_equality() {
    assert_eq!(ActionResult::Continue, ActionResult::Continue);
    assert_eq!(ActionResult::Quit(0), ActionResult::Quit(0));
    assert_ne!(ActionResult::Continue, ActionResult::Quit(0));
    assert_ne!(ActionResult::Quit(0), ActionResult::Quit(1));
}

#[test]
fn test_get_target_directory_with_dir() {
    let temp = TempDir::new().unwrap();
    let dir_path = temp.path().join("subdir");
    std::fs::create_dir(&dir_path).unwrap();

    let result = get_target_directory(Some(&dir_path), temp.path());
    assert_eq!(result, dir_path);
}

#[test]
fn test_get_target_directory_with_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("file.txt");
    std::fs::write(&file_path, "content").unwrap();

    let result = get_target_directory(Some(&file_path), temp.path());
    assert_eq!(result, temp.path().to_path_buf());
}

#[test]
fn test_get_target_directory_none() {
    let temp = TempDir::new().unwrap();
    let result = get_target_directory(None, temp.path());
    assert_eq!(result, temp.path().to_path_buf());
}

#[test]
fn test_get_filename_str() {
    use std::path::PathBuf;
    let path = PathBuf::from("/path/to/file.txt");
    assert_eq!(get_filename_str(Some(&path)), "file.txt");
    assert_eq!(get_filename_str(None), "");
}

#[test]
fn test_move_up_action() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("file1.txt"), "").unwrap();
    std::fs::write(temp.path().join("file2.txt"), "").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    state.focus_index = 2;
    let result = call_handle_action!(
        KeyAction::MoveUp,
        &mut state,
        &mut navigator,
        &None,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();

    assert_eq!(result, ActionResult::Continue);
    assert_eq!(state.focus_index, 1);
}

#[test]
fn test_move_down_action() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("file1.txt"), "").unwrap();
    std::fs::write(temp.path().join("file2.txt"), "").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    state.focus_index = 0;
    let result = call_handle_action!(
        KeyAction::MoveDown,
        &mut state,
        &mut navigator,
        &None,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();

    assert_eq!(result, ActionResult::Continue);
    assert_eq!(state.focus_index, 1);
}

#[test]
fn test_quit_action() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    let result = call_handle_action!(
        KeyAction::Quit,
        &mut state,
        &mut navigator,
        &None,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();

    assert_eq!(result, ActionResult::Continue);
    assert!(state.should_quit);
}

#[test]
fn test_toggle_mark_action() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("file.txt");
    std::fs::write(&file_path, "").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;
    let focused = Some(file_path.clone());

    // Mark
    call_handle_action!(
        KeyAction::ToggleMark,
        &mut state,
        &mut navigator,
        &focused,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();
    assert!(state.selected_paths.contains(&file_path));

    // Unmark
    call_handle_action!(
        KeyAction::ToggleMark,
        &mut state,
        &mut navigator,
        &focused,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();
    assert!(!state.selected_paths.contains(&file_path));
}

#[test]
fn test_toggle_hidden_action() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    assert!(!state.show_hidden);

    call_handle_action!(
        KeyAction::ToggleHidden,
        &mut state,
        &mut navigator,
        &None,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();

    assert!(state.show_hidden);
}

#[test]
fn test_open_preview_action() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Open preview
    call_handle_action!(
        KeyAction::OpenPreview,
        &mut state,
        &mut navigator,
        &None,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();
    assert!(matches!(state.mode, ViewMode::Preview { .. }));

    // Close preview
    call_handle_action!(
        KeyAction::OpenPreview,
        &mut state,
        &mut navigator,
        &None,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();
    assert!(matches!(state.mode, ViewMode::Browse));
}

#[test]
fn test_toggle_quick_preview_action() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    assert!(!state.preview_visible);

    call_handle_action!(
        KeyAction::ToggleQuickPreview,
        &mut state,
        &mut navigator,
        &None,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();

    assert!(state.preview_visible);
}
