//! Focus Management Tests (Phase 14)

use tempfile::TempDir;

use super::*;

/// Focus: Toggle focus switches between Tree and Preview
#[test]
fn test_focus_toggle_switches_target() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("test.txt"), "content").unwrap();

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
