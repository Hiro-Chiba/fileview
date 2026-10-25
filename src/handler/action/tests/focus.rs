//! Focus Management Tests (Phase 14)

use tempfile::TempDir;

use super::*;

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

    state.preview_visible = true;
    assert_eq!(state.focus_target, FocusTarget::Tree);

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

    state.preview_visible = false;
    assert_eq!(state.focus_target, FocusTarget::Tree);

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

    state.preview_visible = true;
    state.focus_target = FocusTarget::Preview;

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

    state.preview_visible = true;
    state.focus_target = FocusTarget::Preview;

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

    state.preview_visible = true;
    state.focus_target = FocusTarget::Tree;
    state.focus_index = 0;

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

    state.preview_visible = true;
    state.focus_index = 0;

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
    assert_eq!(state.focus_index, 0);

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

    // Only a lower bound is checked because the value is clamped during render.
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
