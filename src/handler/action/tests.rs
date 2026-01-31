//! Tests for action handlers

use std::path::Path;

use tempfile::TempDir;

use crate::core::{AppState, FocusTarget, ViewMode};
use crate::handler::key::KeyAction;
use crate::integrate::exit_code;
use crate::render::{ArchiveEntry, ArchivePreview, HexPreview, PdfPreview, Picker, TextPreview};
use crate::tree::TreeNavigator;

use super::{
    get_filename_str, get_target_directory, handle_action, ActionContext, ActionResult,
    EntrySnapshot,
};

/// Helper macro to call handle_action with all required preview arguments
macro_rules! call_handle_action {
    ($action:expr, $state:expr, $navigator:expr, $path:expr, $entries:expr, $context:expr,
     $text_preview:expr, $hex_preview:expr, $archive_preview:expr) => {{
        let mut pdf_preview: Option<PdfPreview> = None;
        let mut image_picker: Option<Picker> = None;
        handle_action(
            $action,
            $state,
            $navigator,
            $path,
            $entries,
            $context,
            $text_preview,
            $hex_preview,
            $archive_preview,
            &mut pdf_preview,
            &mut image_picker,
        )
    }};
}

fn create_test_state(root: &Path) -> AppState {
    AppState::new(root.to_path_buf())
}

fn create_test_navigator(root: &Path) -> TreeNavigator {
    TreeNavigator::new(root, false).unwrap()
}

fn create_test_entries(navigator: &TreeNavigator) -> Vec<EntrySnapshot> {
    navigator
        .visible_entries()
        .iter()
        .map(|e| EntrySnapshot {
            path: e.path.clone(),
            name: e.name.clone(),
            is_dir: e.is_dir,
            depth: e.depth,
        })
        .collect()
}

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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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

// =========================================================================
// State Transition Tests (Phase 13.2)
// These tests verify the behavior fixed in v0.6.1
// =========================================================================

/// Test: ToggleExpand with file + side preview visible -> closes side preview
/// This was the bug in v0.6.1 where Enter opened fullscreen instead of closing
#[test]
fn test_toggle_expand_file_with_side_preview_closes_panel() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.txt");
    std::fs::write(&file_path, "content").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    let focused = Some(dir_path.clone());
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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
    let entries = create_test_entries(&navigator);
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

// =========================================================================
// Sequence Tests (Phase 13.3)
// These tests verify multi-step user workflows
// =========================================================================

/// Sequence: Navigation -> Open Preview -> Navigate -> Close Preview
/// Simulates: j -> j -> o -> j -> j -> q (or Cancel)
#[test]
fn test_sequence_navigation_with_preview() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("a.txt"), "content a").unwrap();
    std::fs::write(temp.path().join("b.txt"), "content b").unwrap();
    std::fs::write(temp.path().join("c.txt"), "content c").unwrap();
    std::fs::write(temp.path().join("d.txt"), "content d").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Initial state
    state.focus_index = 0;
    assert!(matches!(state.mode, ViewMode::Browse));

    // Step 1: Move down (j)
    call_handle_action!(
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
    assert_eq!(state.focus_index, 1);

    // Step 2: Move down again (j)
    call_handle_action!(
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
    assert_eq!(state.focus_index, 2);

    // Step 3: Open preview (o)
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

    // Step 4: Scroll in preview (j in preview = scroll down)
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

    // Step 5: Close preview (Cancel)
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
    assert!(matches!(state.mode, ViewMode::Browse));

    // Focus should be preserved after closing preview
    assert_eq!(state.focus_index, 2);
}

/// Sequence: Toggle side preview -> Enter closes it (v0.6.1 fix)
/// Simulates: P -> Enter
#[test]
fn test_sequence_side_preview_toggle_enter() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.txt");
    std::fs::write(&file_path, "content").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;
    let focused = Some(file_path);

    // Initial state: Browse mode, no preview
    assert!(!state.preview_visible);
    assert!(matches!(state.mode, ViewMode::Browse));

    // Step 1: Toggle quick preview (P)
    call_handle_action!(
        KeyAction::ToggleQuickPreview,
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
    assert!(state.preview_visible, "Side preview should be visible");

    // Step 2: ToggleExpand (Enter) should close side preview
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
    assert!(
        !state.preview_visible,
        "Side preview should be closed by Enter"
    );
    assert!(
        matches!(state.mode, ViewMode::Browse),
        "Should stay in Browse mode"
    );
}

/// Sequence: Search -> Enter -> SearchNext
/// Simulates: / -> (type query) -> Enter -> n
#[test]
fn test_sequence_search_workflow() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("apple.txt"), "").unwrap();
    std::fs::write(temp.path().join("banana.txt"), "").unwrap();
    std::fs::write(temp.path().join("apricot.txt"), "").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    state.focus_index = 0;

    // Step 1: Start search (/)
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
    assert!(matches!(state.mode, ViewMode::Search { .. }));

    // Step 2: Simulate typing "ap" and confirm (Enter)
    // In real app, buffer is updated by update_input_buffer
    call_handle_action!(
        KeyAction::ConfirmInput {
            value: "ap".to_string(),
        },
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

    // Mode should stay Search with query
    assert!(matches!(state.mode, ViewMode::Search { ref query } if query == "ap"));

    // Step 3: Search next (n) - should find next match
    // Need to update entries to have current search query in state
    let initial_focus = state.focus_index;
    call_handle_action!(
        KeyAction::SearchNext,
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

    // Focus should have moved to a matching entry
    // (depends on sort order, but should find apple or apricot)
    let new_focus = state.focus_index;
    let focused_name = &entries.get(new_focus).map(|e| e.name.clone());
    assert!(
        focused_name
            .as_ref()
            .map(|n| n.to_lowercase().contains("ap"))
            .unwrap_or(false)
            || new_focus != initial_focus,
        "SearchNext should find a matching entry"
    );
}

/// Sequence: Mark files -> Copy -> Navigate -> Paste
/// Simulates: Space -> j -> Space -> y -> G -> p
#[test]
fn test_sequence_copy_paste_workflow() {
    let temp = TempDir::new().unwrap();
    let source_dir = temp.path();
    let dest_dir = temp.path().join("dest");
    std::fs::create_dir(&dest_dir).unwrap();
    std::fs::write(source_dir.join("file1.txt"), "content1").unwrap();
    std::fs::write(source_dir.join("file2.txt"), "content2").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let mut entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Find file1.txt in entries
    let file1_idx = entries
        .iter()
        .position(|e| e.name == "file1.txt")
        .unwrap_or(1);
    state.focus_index = file1_idx;
    let file1_path = entries[file1_idx].path.clone();

    // Step 1: Mark file1 (Space)
    call_handle_action!(
        KeyAction::ToggleMark,
        &mut state,
        &mut navigator,
        &Some(file1_path.clone()),
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();
    assert!(state.selected_paths.contains(&file1_path));

    // Step 2: Copy marked files (y)
    call_handle_action!(
        KeyAction::Copy,
        &mut state,
        &mut navigator,
        &Some(file1_path),
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();
    assert!(state.clipboard.is_some());
    assert!(state.message.as_ref().unwrap().contains("Copied"));

    // Step 3: Navigate to dest directory
    let dest_idx = entries.iter().position(|e| e.name == "dest").unwrap_or(0);
    state.focus_index = dest_idx;

    // Step 4: Paste (p)
    let dest_path = Some(dest_dir.clone());
    call_handle_action!(
        KeyAction::Paste,
        &mut state,
        &mut navigator,
        &dest_path,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();

    // Verify file was copied
    assert!(
        dest_dir.join("file1.txt").exists(),
        "File should be copied to destination"
    );
    assert!(state.message.as_ref().unwrap().contains("Pasted"));

    // Refresh entries after paste
    entries = create_test_entries(&navigator);
    assert!(!entries.is_empty());
}

/// Sequence: Start rename -> Cancel -> Start rename again -> Confirm
#[test]
fn test_sequence_rename_cancel_rename_confirm() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("original.txt");
    std::fs::write(&file_path, "content").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;
    let focused = Some(file_path.clone());

    // Step 1: Start rename (r)
    call_handle_action!(
        KeyAction::StartRename,
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
    assert!(matches!(
        state.mode,
        ViewMode::Input {
            purpose: crate::core::InputPurpose::Rename { .. },
            ..
        }
    ));

    // Step 2: Cancel (Esc)
    call_handle_action!(
        KeyAction::Cancel,
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
    assert!(matches!(state.mode, ViewMode::Browse));
    assert!(file_path.exists(), "File should not be renamed on cancel");

    // Step 3: Start rename again (r)
    call_handle_action!(
        KeyAction::StartRename,
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
    assert!(matches!(state.mode, ViewMode::Input { .. }));

    // Step 4: Confirm with new name (Enter)
    call_handle_action!(
        KeyAction::ConfirmInput {
            value: "renamed.txt".to_string(),
        },
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

    assert!(matches!(state.mode, ViewMode::Browse));
    assert!(
        temp.path().join("renamed.txt").exists(),
        "File should be renamed"
    );
    assert!(!file_path.exists(), "Original file should not exist");
}

/// Sequence: Expand directory -> Navigate into -> Collapse all
#[test]
fn test_sequence_expand_navigate_collapse_all() {
    let temp = TempDir::new().unwrap();
    let subdir = temp.path().join("subdir");
    std::fs::create_dir(&subdir).unwrap();
    std::fs::write(subdir.join("nested.txt"), "").unwrap();
    std::fs::write(temp.path().join("root.txt"), "").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    let initial_count = navigator.visible_count();

    // Find subdir
    let subdir_idx = entries.iter().position(|e| e.name == "subdir").unwrap_or(0);
    let subdir_path = Some(subdir.clone());

    // Step 1: Expand directory (l or Enter on dir)
    call_handle_action!(
        KeyAction::Expand,
        &mut state,
        &mut navigator,
        &subdir_path,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();
    let expanded_count = navigator.visible_count();
    assert!(
        expanded_count > initial_count,
        "Should see nested files after expand"
    );

    // Update entries after expand
    let entries = create_test_entries(&navigator);

    // Step 2: Move focus into expanded directory
    state.focus_index = subdir_idx + 1; // Move to first child

    // Step 3: Collapse all (H)
    call_handle_action!(
        KeyAction::CollapseAll,
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

    let collapsed_count = navigator.visible_count();
    assert_eq!(
        collapsed_count, initial_count,
        "All directories should be collapsed"
    );
}

/// Sequence: Create file -> Verify exists -> Delete -> Confirm
#[test]
#[ignore] // Requires Finder/trash permissions; run manually
fn test_sequence_create_delete_workflow() {
    let temp = TempDir::new().unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;
    let focused = Some(temp.path().to_path_buf());

    // Step 1: Start new file (a)
    call_handle_action!(
        KeyAction::StartNewFile,
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
    assert!(matches!(state.mode, ViewMode::Input { .. }));

    // Step 2: Confirm file creation
    call_handle_action!(
        KeyAction::ConfirmInput {
            value: "newfile.txt".to_string(),
        },
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

    let new_file = temp.path().join("newfile.txt");
    assert!(new_file.exists(), "New file should be created");

    // Refresh entries
    let entries = create_test_entries(&navigator);
    let new_file_focused = Some(new_file.clone());

    // Step 3: Delete (D) - starts confirmation
    call_handle_action!(
        KeyAction::ConfirmDelete,
        &mut state,
        &mut navigator,
        &new_file_focused,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();
    assert!(matches!(state.mode, ViewMode::Confirm { .. }));

    // Step 4: Execute delete (y)
    call_handle_action!(
        KeyAction::ExecuteDelete,
        &mut state,
        &mut navigator,
        &new_file_focused,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();

    assert!(!new_file.exists(), "File should be deleted");
    assert!(matches!(state.mode, ViewMode::Browse));
}

/// Sequence: Multiple marks -> Cut -> Paste (move operation)
#[test]
fn test_sequence_cut_paste_multiple() {
    let temp = TempDir::new().unwrap();
    let dest_dir = temp.path().join("dest");
    std::fs::create_dir(&dest_dir).unwrap();
    let file1 = temp.path().join("move1.txt");
    let file2 = temp.path().join("move2.txt");
    std::fs::write(&file1, "content1").unwrap();
    std::fs::write(&file2, "content2").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Mark both files
    state.selected_paths.insert(file1.clone());
    state.selected_paths.insert(file2.clone());

    // Step 1: Cut (d)
    call_handle_action!(
        KeyAction::Cut,
        &mut state,
        &mut navigator,
        &Some(file1.clone()),
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();
    assert!(state.clipboard.is_some());
    assert!(state.message.as_ref().unwrap().contains("Cut"));

    // Files should still exist (not moved yet)
    assert!(file1.exists());
    assert!(file2.exists());

    // Step 2: Navigate to dest and paste
    call_handle_action!(
        KeyAction::Paste,
        &mut state,
        &mut navigator,
        &Some(dest_dir.clone()),
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();

    // Files should be moved
    assert!(
        dest_dir.join("move1.txt").exists(),
        "File1 should be moved to dest"
    );
    assert!(
        dest_dir.join("move2.txt").exists(),
        "File2 should be moved to dest"
    );
    assert!(!file1.exists(), "Original file1 should not exist");
    assert!(!file2.exists(), "Original file2 should not exist");
}

// =========================================================================
// Edge Case Tests (Phase 13.4)
// These tests verify behavior in unusual or boundary conditions
// =========================================================================

/// Edge case: Empty directory - navigation should handle gracefully
#[test]
fn test_edge_empty_directory_navigation() {
    let temp = TempDir::new().unwrap();
    // Don't create any files - empty directory

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Root is always present, so entries should have at least 1
    assert!(!entries.is_empty());

    // Move down should not panic
    call_handle_action!(
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

    // Move up should not panic
    call_handle_action!(
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

    // MoveToBottom should work
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

    // MoveToTop should work
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
}

/// Edge case: Empty directory - expand/collapse should handle gracefully
#[test]
fn test_edge_empty_directory_expand() {
    let temp = TempDir::new().unwrap();
    let empty_dir = temp.path().join("empty");
    std::fs::create_dir(&empty_dir).unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    let focused = Some(empty_dir.clone());
    let initial_count = navigator.visible_count();

    // Expand empty directory should not crash
    call_handle_action!(
        KeyAction::Expand,
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

    // Count should be same (no children to show)
    assert_eq!(navigator.visible_count(), initial_count);

    // Collapse should work
    call_handle_action!(
        KeyAction::Collapse,
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
}

/// Edge case: Symlink handling
#[cfg(unix)]
#[test]
fn test_edge_symlink_file() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let real_file = temp.path().join("real.txt");
    let link_file = temp.path().join("link.txt");
    std::fs::write(&real_file, "content").unwrap();
    symlink(&real_file, &link_file).unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Find the symlink in entries
    let link_entry = entries.iter().find(|e| e.name == "link.txt");
    assert!(link_entry.is_some(), "Symlink should appear in tree");

    let focused = Some(link_file.clone());

    // Operations on symlink should work
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
    assert!(state.selected_paths.contains(&link_file));
}

/// Edge case: Symlink to directory
#[cfg(unix)]
#[test]
fn test_edge_symlink_directory() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let real_dir = temp.path().join("real_dir");
    let link_dir = temp.path().join("link_dir");
    std::fs::create_dir(&real_dir).unwrap();
    std::fs::write(real_dir.join("file.txt"), "content").unwrap();
    symlink(&real_dir, &link_dir).unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    let focused = Some(link_dir.clone());

    // Expand symlink directory should work
    call_handle_action!(
        KeyAction::Expand,
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

    // Should be able to see contents through symlink
    let new_entries = create_test_entries(&navigator);
    let has_nested = new_entries.iter().any(|e| e.name == "file.txt");
    assert!(has_nested, "Should see files through symlink directory");
}

/// Edge case: Deep directory structure
#[test]
fn test_edge_deep_directory_structure() {
    let temp = TempDir::new().unwrap();

    // Create a deep structure: /a/b/c/d/e/f/file.txt
    let mut current = temp.path().to_path_buf();
    for dir_name in ["a", "b", "c", "d", "e", "f"] {
        current = current.join(dir_name);
        std::fs::create_dir(&current).unwrap();
    }
    std::fs::write(current.join("deep.txt"), "deep content").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Expand all levels
    let mut path = temp.path().to_path_buf();
    for dir_name in ["a", "b", "c", "d", "e", "f"] {
        path = path.join(dir_name);
        let entries = create_test_entries(&navigator);
        call_handle_action!(
            KeyAction::Expand,
            &mut state,
            &mut navigator,
            &Some(path.clone()),
            &entries,
            &context,
            &mut text_preview,
            &mut hex_preview,
            &mut archive_preview
        )
        .unwrap();
    }

    // Verify deep file is visible
    let final_entries = create_test_entries(&navigator);
    let has_deep_file = final_entries.iter().any(|e| e.name == "deep.txt");
    assert!(has_deep_file, "Deep file should be visible after expanding");

    // CollapseAll should work on deep structure
    call_handle_action!(
        KeyAction::CollapseAll,
        &mut state,
        &mut navigator,
        &None,
        &final_entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();

    // Only root should be visible now (plus first level dir)
    let collapsed_entries = create_test_entries(&navigator);
    let no_deep_file = !collapsed_entries.iter().any(|e| e.name == "deep.txt");
    assert!(
        no_deep_file,
        "Deep file should not be visible after collapse"
    );
}

/// Edge case: Navigation boundary - move up at top
#[test]
fn test_edge_move_up_at_top() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("file.txt"), "").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    state.focus_index = 0;

    // Move up at top should stay at 0 (saturating)
    call_handle_action!(
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

    assert_eq!(state.focus_index, 0, "Should stay at top");
}

/// Edge case: Navigation boundary - move down at bottom
#[test]
fn test_edge_move_down_at_bottom() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("file.txt"), "").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    let last_index = entries.len().saturating_sub(1);
    state.focus_index = last_index;

    // Move down at bottom should stay at last
    call_handle_action!(
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

    assert_eq!(state.focus_index, last_index, "Should stay at bottom");
}

/// Edge case: File with special characters in name
#[test]
fn test_edge_special_characters_filename() {
    let temp = TempDir::new().unwrap();
    let special_file = temp.path().join("file with spaces & special!.txt");
    std::fs::write(&special_file, "content").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Find the file
    let file_entry = entries
        .iter()
        .find(|e| e.name.contains("special"))
        .map(|e| e.path.clone());
    assert!(file_entry.is_some(), "Special filename should be in tree");

    // Operations should work
    call_handle_action!(
        KeyAction::ToggleMark,
        &mut state,
        &mut navigator,
        &file_entry,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();

    assert!(
        state
            .selected_paths
            .iter()
            .any(|p| p.display().to_string().contains("special")),
        "Should be able to mark file with special chars"
    );
}

/// Edge case: Unicode filename
#[test]
fn test_edge_unicode_filename() {
    let temp = TempDir::new().unwrap();
    let unicode_file = temp.path().join("日本語ファイル.txt");
    std::fs::write(&unicode_file, "日本語の内容").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Find the file
    let file_entry = entries
        .iter()
        .find(|e| e.name.contains("日本語"))
        .map(|e| e.path.clone());
    assert!(file_entry.is_some(), "Unicode filename should be in tree");

    // Mark should work
    call_handle_action!(
        KeyAction::ToggleMark,
        &mut state,
        &mut navigator,
        &file_entry,
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();

    assert!(!state.selected_paths.is_empty());
}

/// Edge case: Copy to system clipboard with no focused path
#[test]
fn test_edge_copy_path_no_focus() {
    let temp = TempDir::new().unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // CopyPath with None should not crash
    call_handle_action!(
        KeyAction::CopyPath,
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

    // Should not set a success message (no path to copy)
    // Message might be None or might be an error message
}

/// Edge case: SearchNext with empty entries
#[test]
fn test_edge_search_next_with_query() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("apple.txt"), "").unwrap();
    std::fs::write(temp.path().join("banana.txt"), "").unwrap();
    std::fs::write(temp.path().join("cherry.txt"), "").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Set search mode with a query that has no matches
    state.mode = ViewMode::Search {
        query: "xyz_no_match".to_string(),
    };
    state.focus_index = 0;

    // SearchNext should not crash even with no matches
    call_handle_action!(
        KeyAction::SearchNext,
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

    // Focus should wrap around but eventually return to start
    // (or stay in place if no match found)
}

/// Edge case: Paste with empty clipboard
#[test]
fn test_edge_paste_empty_clipboard() {
    let temp = TempDir::new().unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Clipboard is None
    assert!(state.clipboard.is_none());

    // Paste should not crash
    call_handle_action!(
        KeyAction::Paste,
        &mut state,
        &mut navigator,
        &Some(temp.path().to_path_buf()),
        &entries,
        &context,
        &mut text_preview,
        &mut hex_preview,
        &mut archive_preview
    )
    .unwrap();
}

/// Edge case: ConfirmDelete with no targets
#[test]
fn test_edge_confirm_delete_no_targets() {
    let temp = TempDir::new().unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // No marks and no focused path
    state.selected_paths.clear();

    call_handle_action!(
        KeyAction::ConfirmDelete,
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

    // Should not enter confirm mode without targets
    assert!(
        !matches!(state.mode, ViewMode::Confirm { .. }),
        "Should not enter confirm mode without targets"
    );
}

/// Edge case: ExpandAll respects depth limit
#[test]
fn test_edge_expand_all_depth_limit() {
    let temp = TempDir::new().unwrap();

    // Create nested structure: dir0/dir1/dir2/dir3/dir4/dir5/dir6/deep.txt
    let mut current = temp.path().to_path_buf();
    for i in 0..7 {
        current = current.join(format!("dir{}", i));
        std::fs::create_dir(&current).unwrap();
    }
    std::fs::write(current.join("deep.txt"), "").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Manually expand to depth 5 so we can test the depth limit
    // Expand dir0 through dir4 to make dir5 visible
    let mut path = temp.path().to_path_buf();
    for i in 0..5 {
        path = path.join(format!("dir{}", i));
        navigator.toggle_expand(&path).unwrap();
    }

    // Now dir5 should be visible at depth 5
    let entries = create_test_entries(&navigator);
    let has_dir5 = entries.iter().any(|e| e.name == "dir5");
    assert!(has_dir5, "dir5 should be visible after manual expansion");

    // Now call ExpandAll - it should NOT expand dir5 (depth 5 is not < 5)
    call_handle_action!(
        KeyAction::ExpandAll,
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

    let after_expand = create_test_entries(&navigator);

    // dir6 should NOT be visible (dir5 at depth 5 was not expanded due to depth limit)
    let has_dir6 = after_expand.iter().any(|e| e.name == "dir6");
    assert!(
        !has_dir6,
        "dir6 should not be visible - depth limit prevents expansion"
    );
}

// =========================================================================
// Focus Management Tests (Phase 14)
// These tests verify focus toggle and focus-aware behavior
// =========================================================================

/// Focus: Toggle focus switches between Tree and Preview
#[test]
fn test_focus_toggle_switches_target() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("test.txt"), "content").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Enable side preview
    state.preview_visible = true;
    assert_eq!(state.focus_target, FocusTarget::Tree);

    // Toggle focus
    call_handle_action!(
        KeyAction::ToggleFocus,
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

    assert_eq!(state.focus_target, FocusTarget::Preview);

    // Toggle again
    call_handle_action!(
        KeyAction::ToggleFocus,
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

    assert_eq!(state.focus_target, FocusTarget::Tree);
}

/// Focus: Toggle has no effect when preview is not visible
#[test]
fn test_focus_toggle_no_effect_without_preview() {
    let temp = TempDir::new().unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Preview not visible
    state.preview_visible = false;
    assert_eq!(state.focus_target, FocusTarget::Tree);

    // Try to toggle focus
    call_handle_action!(
        KeyAction::ToggleFocus,
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

    // Focus should stay on Tree
    assert_eq!(state.focus_target, FocusTarget::Tree);
}

/// Focus: Closing preview resets focus to Tree
#[test]
fn test_focus_reset_when_preview_closed() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("test.txt"), "content").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Enable preview and set focus to Preview
    state.preview_visible = true;
    state.focus_target = FocusTarget::Preview;

    // Close preview
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

    assert!(!state.preview_visible);
    assert_eq!(state.focus_target, FocusTarget::Tree);
}

/// Focus: MoveDown scrolls preview when focus is on Preview
#[test]
fn test_focus_preview_navigation_scrolls() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("test.txt"), "line1\nline2\nline3").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview = Some(TextPreview::new("line1\nline2\nline3\nline4\nline5"));
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;
    text_preview.as_mut().unwrap().scroll = 0;

    // Enable preview and set focus to Preview
    state.preview_visible = true;
    state.focus_target = FocusTarget::Preview;

    // PreviewScrollDown should scroll the text preview
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

    assert_eq!(text_preview.as_ref().unwrap().scroll, 1);
}

/// Focus: Navigation works on tree when focus is on Tree
#[test]
fn test_focus_tree_navigation_moves_files() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("a.txt"), "").unwrap();
    std::fs::write(temp.path().join("b.txt"), "").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    // Enable preview but keep focus on Tree
    state.preview_visible = true;
    state.focus_target = FocusTarget::Tree;
    state.focus_index = 0;

    // MoveDown should move file selection
    call_handle_action!(
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

    assert_eq!(state.focus_index, 1);
}

/// Focus: Sequence test - Tab toggle, scroll, Tab back, navigate
#[test]
fn test_focus_sequence_toggle_scroll_navigate() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("a.txt"), "").unwrap();
    std::fs::write(temp.path().join("b.txt"), "").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview = Some(TextPreview::new("line1\nline2\nline3\nline4\nline5"));
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;
    text_preview.as_mut().unwrap().scroll = 0;

    // Enable preview
    state.preview_visible = true;
    state.focus_index = 0;

    // Step 1: Toggle focus to Preview
    call_handle_action!(
        KeyAction::ToggleFocus,
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
    assert_eq!(state.focus_target, FocusTarget::Preview);

    // Step 2: Scroll down (should affect preview, not file selection)
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
    assert_eq!(text_preview.as_ref().unwrap().scroll, 1);
    assert_eq!(state.focus_index, 0); // File selection unchanged

    // Step 3: Toggle focus back to Tree
    call_handle_action!(
        KeyAction::ToggleFocus,
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
    assert_eq!(state.focus_target, FocusTarget::Tree);

    // Step 4: Navigate down (should affect file selection, not scroll)
    let scroll_before = text_preview.as_ref().unwrap().scroll;
    call_handle_action!(
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
    assert_eq!(state.focus_index, 1);
    assert_eq!(text_preview.as_ref().unwrap().scroll, scroll_before);
}

/// Focus: Page scroll works in preview focus
#[test]
fn test_focus_preview_page_scroll() {
    let temp = TempDir::new().unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview = Some(TextPreview::new(&"line\n".repeat(100)));
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;
    text_preview.as_mut().unwrap().scroll = 0;

    state.preview_visible = true;
    state.focus_target = FocusTarget::Preview;

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

    assert_eq!(text_preview.as_ref().unwrap().scroll, 20);

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

    assert_eq!(text_preview.as_ref().unwrap().scroll, 0);
}

/// Focus: PreviewToTop and PreviewToBottom
#[test]
fn test_focus_preview_jump_to_top_bottom() {
    let temp = TempDir::new().unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();
    let mut text_preview = Some(TextPreview::new(&"line\n".repeat(100)));
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;
    text_preview.as_mut().unwrap().scroll = 50;

    state.preview_visible = true;
    state.focus_target = FocusTarget::Preview;

    // Jump to top
    call_handle_action!(
        KeyAction::PreviewToTop,
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

    assert_eq!(text_preview.as_ref().unwrap().scroll, 0);

    // Jump to bottom (large value, will be clamped during render)
    call_handle_action!(
        KeyAction::PreviewToBottom,
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

    assert!(text_preview.as_ref().unwrap().scroll > 50);
}

// =========================================================================
// Scroll Bounds Tests (v1.9.2)
// These tests verify scroll bounds checking for previews
// =========================================================================

/// Test: PreviewScrollDown is capped at max line count
#[test]
fn test_preview_scroll_down_capped_at_max() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();

    // Create a text preview with only 5 lines
    let mut text_preview = Some(TextPreview::new("line1\nline2\nline3\nline4\nline5"));
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;
    text_preview.as_mut().unwrap().scroll = 0;

    // Scroll down multiple times - should stop at max (4, since 5 lines means max scroll = 4)
    for _ in 0..10 {
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
    }

    // Scroll should be capped at lines.len() - 1 = 4
    assert_eq!(
        text_preview.as_ref().unwrap().scroll,
        4,
        "Scroll should be capped at max (line_count - 1)"
    );
}

/// Test: PreviewPageDown is capped at max line count
#[test]
fn test_preview_page_down_capped_at_max() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();

    // Create a text preview with only 10 lines
    let mut text_preview = Some(TextPreview::new("1\n2\n3\n4\n5\n6\n7\n8\n9\n10"));
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;
    text_preview.as_mut().unwrap().scroll = 0;

    // Page down once (should try to scroll by 20, but cap at 9)
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

    // Scroll should be capped at lines.len() - 1 = 9
    assert_eq!(
        text_preview.as_ref().unwrap().scroll,
        9,
        "PageDown scroll should be capped at max (line_count - 1)"
    );
}

/// Test: PreviewToBottom sets scroll to max and syncs with ViewMode
#[test]
fn test_preview_to_bottom_syncs_viewmode() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();

    // Create a text preview with 50 lines
    let mut text_preview = Some(TextPreview::new(&"line\n".repeat(50)));
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;
    text_preview.as_mut().unwrap().scroll = 0;

    // Enter Preview mode
    state.mode = ViewMode::Preview { scroll: 0 };

    // Jump to bottom
    call_handle_action!(
        KeyAction::PreviewToBottom,
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

    // Text preview scroll should be at max
    assert_eq!(
        text_preview.as_ref().unwrap().scroll,
        49,
        "TextPreview scroll should be at max (line_count - 1)"
    );

    // ViewMode scroll should also be synced
    if let ViewMode::Preview { scroll } = state.mode {
        assert_eq!(scroll, 49, "ViewMode scroll should be synced with preview");
    } else {
        panic!("Should still be in Preview mode");
    }
}

/// Test: Hex preview scroll is capped at max
#[test]
fn test_hex_preview_scroll_capped() {
    let temp = TempDir::new().unwrap();
    let hex_file = temp.path().join("test.bin");
    // Create a small binary file (32 bytes = 2 lines at 16 bytes per line)
    std::fs::write(&hex_file, vec![0u8; 32]).unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();

    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview = Some(HexPreview::load(&hex_file).unwrap());
    let mut archive_preview: Option<ArchivePreview> = None;
    hex_preview.as_mut().unwrap().scroll = 0;

    // Scroll down multiple times
    for _ in 0..10 {
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
    }

    // line_count for 32 bytes = 2 lines, max scroll = 1
    assert_eq!(
        hex_preview.as_ref().unwrap().scroll,
        1,
        "HexPreview scroll should be capped at max (line_count - 1)"
    );
}

/// Test: Archive preview scroll is capped at max
#[test]
fn test_archive_preview_scroll_capped() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&navigator);
    let context = ActionContext::default();

    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    // Create a mock archive preview with 5 entries (line_count = 5 + 2 = 7)
    let mut archive_preview = Some(ArchivePreview {
        entries: vec![
            ArchiveEntry {
                name: "file1.txt".to_string(),
                size: 100,
                is_dir: false,
                modified: None,
            },
            ArchiveEntry {
                name: "file2.txt".to_string(),
                size: 200,
                is_dir: false,
                modified: None,
            },
            ArchiveEntry {
                name: "file3.txt".to_string(),
                size: 300,
                is_dir: false,
                modified: None,
            },
            ArchiveEntry {
                name: "file4.txt".to_string(),
                size: 400,
                is_dir: false,
                modified: None,
            },
            ArchiveEntry {
                name: "file5.txt".to_string(),
                size: 500,
                is_dir: false,
                modified: None,
            },
        ],
        total_size: 1500,
        file_count: 5,
        scroll: 0,
    });

    // Scroll down multiple times
    for _ in 0..20 {
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
    }

    // line_count = 5 entries + 2 header = 7, max scroll = 6
    assert_eq!(
        archive_preview.as_ref().unwrap().scroll,
        6,
        "ArchivePreview scroll should be capped at max (line_count - 1)"
    );
}
