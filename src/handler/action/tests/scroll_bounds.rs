//! Scroll Bounds Tests (v1.9.2)

use tempfile::TempDir;

use super::*;

/// Test: PreviewScrollDown is capped at max line count
#[test]
fn test_preview_scroll_down_capped_at_max() {
    let temp = TempDir::new().unwrap();
    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
