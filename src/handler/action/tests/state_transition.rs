//! State Transition Tests (Phase 13.2)

use tempfile::TempDir;

use super::*;

/// Test: ToggleExpand with file + side preview visible -> closes side preview
/// This was the bug in v0.6.1 where Enter opened fullscreen instead of closing
#[test]
fn test_toggle_expand_file_with_side_preview_closes_panel() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.txt");
    std::fs::write(&file_path, "content").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Side preview is visible
    state.preview_visible = true;
    let focused = Some(file_path);

    call_handle_action!(
        KeyAction::ToggleExpand,
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

    // Side preview should be closed, NOT fullscreen opened
    assert!(!state.preview_visible, "Side preview should be closed");
    assert!(
        matches!(state.mode, ViewMode::Browse),
        "Should stay in Browse mode, not open fullscreen"
    );
}

/// Test: ToggleExpand with file + side preview NOT visible -> opens fullscreen
#[test]
fn test_toggle_expand_file_without_preview_opens_fullscreen() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.txt");
    std::fs::write(&file_path, "content").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Side preview is NOT visible
    state.preview_visible = false;
    let focused = Some(file_path);

    call_handle_action!(
        KeyAction::ToggleExpand,
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

    // Fullscreen preview should be opened
    assert!(
        matches!(state.mode, ViewMode::Preview { scroll: 0 }),
        "Should open fullscreen preview for file"
    );
}

/// Test: ToggleExpand with directory -> toggles expand
#[test]
fn test_toggle_expand_directory_toggles_expand() {
    let temp = TempDir::new().unwrap();
    let dir_path = temp.path().join("subdir");
    std::fs::create_dir(&dir_path).unwrap();
    std::fs::write(dir_path.join("file.txt"), "content").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    let focused = Some(dir_path.clone());
    navigator.ensure_cache();
    let initial_count = navigator.visible_count();

    // Expand
    call_handle_action!(
        KeyAction::ToggleExpand,
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

    navigator.ensure_cache();
    assert!(
        navigator.visible_count() > initial_count,
        "Directory should be expanded"
    );

    // Collapse
    call_handle_action!(
        KeyAction::ToggleExpand,
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

    navigator.ensure_cache();
    assert_eq!(
        navigator.visible_count(),
        initial_count,
        "Directory should be collapsed"
    );
}

/// Test: Cancel in Preview mode -> returns to Browse mode
#[test]
fn test_cancel_in_preview_mode_returns_to_browse() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Start in Preview mode
    state.mode = ViewMode::Preview { scroll: 5 };

    call_handle_action!(
        KeyAction::Cancel,
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

    assert!(
        matches!(state.mode, ViewMode::Browse),
        "Should return to Browse mode"
    );
}

/// Test: Cancel in Browse mode with pick_mode -> returns Quit(CANCELLED)
#[test]
fn test_cancel_in_browse_pick_mode_returns_cancelled() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    state.pick_mode = true;

    let result = call_handle_action!(
        KeyAction::Cancel,
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

    assert_eq!(
        result,
        ActionResult::Quit(exit_code::CANCELLED),
        "Should return Quit with CANCELLED code"
    );
}

/// Test: Cancel in Browse mode without pick_mode -> sets should_quit
#[test]
fn test_cancel_in_browse_normal_mode_sets_quit() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    state.pick_mode = false;

    call_handle_action!(
        KeyAction::Cancel,
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

    assert!(state.should_quit, "Should set should_quit flag");
}

/// Test: Cancel in Input mode -> returns to Browse mode and clears message
#[test]
fn test_cancel_in_input_mode_returns_to_browse() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    state.mode = ViewMode::Input {
        purpose: crate::core::InputPurpose::CreateFile,
        buffer: "test.txt".to_string(),
        cursor: 8,
    };
    state.set_message("Creating file...");

    call_handle_action!(
        KeyAction::Cancel,
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

    assert!(
        matches!(state.mode, ViewMode::Browse),
        "Should return to Browse mode"
    );
    assert!(state.message.is_none(), "Message should be cleared");
}

/// Test: Preview scroll maintains state within text_preview
#[test]
fn test_preview_scroll_updates_text_preview() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();

    // Create a text preview with some lines
    let mut text_preview = Some(TextPreview::new("line1\nline2\nline3\nline4\nline5"));
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;
    text_preview.as_mut().unwrap().scroll = 0;

    // Scroll down
    call_handle_action!(
        KeyAction::PreviewScrollDown,
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

    assert_eq!(
        text_preview.as_ref().unwrap().scroll,
        1,
        "Scroll should increase by 1"
    );

    // Scroll up
    call_handle_action!(
        KeyAction::PreviewScrollUp,
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

    assert_eq!(
        text_preview.as_ref().unwrap().scroll,
        0,
        "Scroll should decrease by 1"
    );
}

/// Test: Preview scroll at zero doesn't go negative (saturating)
#[test]
fn test_preview_scroll_saturates_at_zero() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();

    let mut text_preview = Some(TextPreview::new("line1\nline2"));
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;
    text_preview.as_mut().unwrap().scroll = 0;

    // Try to scroll up when already at 0
    call_handle_action!(
        KeyAction::PreviewScrollUp,
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

    assert_eq!(
        text_preview.as_ref().unwrap().scroll,
        0,
        "Scroll should stay at 0 (saturating)"
    );
}

/// Test: Preview page up/down moves by 20 lines
#[test]
fn test_preview_page_scroll() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();

    let mut text_preview = Some(TextPreview::new("a\n".repeat(100).as_str()));
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;
    text_preview.as_mut().unwrap().scroll = 0;

    // Page down
    call_handle_action!(
        KeyAction::PreviewPageDown,
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

    assert_eq!(
        text_preview.as_ref().unwrap().scroll,
        20,
        "Page down should move by 20"
    );

    // Page up
    call_handle_action!(
        KeyAction::PreviewPageUp,
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

    assert_eq!(
        text_preview.as_ref().unwrap().scroll,
        0,
        "Page up should move back by 20"
    );
}

/// Test: Preview scroll in fullscreen mode updates ViewMode scroll
#[test]
fn test_preview_scroll_updates_viewmode_scroll() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Start in Preview mode with scroll at 0
    state.mode = ViewMode::Preview { scroll: 0 };

    call_handle_action!(
        KeyAction::PreviewScrollDown,
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

    if let ViewMode::Preview { scroll } = state.mode {
        assert_eq!(scroll, 1, "ViewMode scroll should increase");
    } else {
        panic!("Should still be in Preview mode");
    }
}

/// Test: MoveToTop sets focus_index to 0
#[test]
fn test_move_to_top() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("a.txt"), "").unwrap();
    std::fs::write(temp.path().join("b.txt"), "").unwrap();
    std::fs::write(temp.path().join("c.txt"), "").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    state.focus_index = 3;

    call_handle_action!(
        KeyAction::MoveToTop,
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

    assert_eq!(state.focus_index, 0);
}

/// Test: MoveToBottom sets focus_index to last entry
#[test]
fn test_move_to_bottom() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("a.txt"), "").unwrap();
    std::fs::write(temp.path().join("b.txt"), "").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    state.focus_index = 0;
    let last_index = entries.len().saturating_sub(1);

    call_handle_action!(
        KeyAction::MoveToBottom,
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

    assert_eq!(state.focus_index, last_index);
}

/// Test: ClearMarks clears all selected paths
#[test]
fn test_clear_marks() {
    let temp = TempDir::new().unwrap();
    let file1 = temp.path().join("a.txt");
    let file2 = temp.path().join("b.txt");
    std::fs::write(&file1, "").unwrap();
    std::fs::write(&file2, "").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    state.selected_paths.insert(file1);
    state.selected_paths.insert(file2);
    assert_eq!(state.selected_paths.len(), 2);

    call_handle_action!(
        KeyAction::ClearMarks,
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

    assert!(state.selected_paths.is_empty());
}

/// Test: StartSearch enters Search mode
#[test]
fn test_start_search() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    call_handle_action!(
        KeyAction::StartSearch,
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

    assert!(matches!(state.mode, ViewMode::Search { query } if query.is_empty()));
}

/// Test: StartNewFile enters Input mode for CreateFile
#[test]
fn test_start_new_file() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    call_handle_action!(
        KeyAction::StartNewFile,
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

    assert!(matches!(
        state.mode,
        ViewMode::Input {
            purpose: crate::core::InputPurpose::CreateFile,
            ..
        }
    ));
}

/// Test: StartNewDir enters Input mode for CreateDir
#[test]
fn test_start_new_dir() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    call_handle_action!(
        KeyAction::StartNewDir,
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

    assert!(matches!(
        state.mode,
        ViewMode::Input {
            purpose: crate::core::InputPurpose::CreateDir,
            ..
        }
    ));
}
