//! Edge Case Tests (Phase 13.4)

use tempfile::TempDir;

use super::*;

/// Edge case: Empty directory - navigation should handle gracefully
#[test]
fn test_edge_empty_directory_navigation() {
    let temp = TempDir::new().unwrap();
    // Don't create any files - empty directory

    let mut state = create_test_state(temp.path());
    let mut navigator = create_test_navigator(temp.path());
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    let focused = Some(empty_dir.clone());
    navigator.ensure_cache();
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
    navigator.ensure_cache();
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let new_entries = create_test_entries(&mut navigator);
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
        let entries = create_test_entries(&mut navigator);
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
    let final_entries = create_test_entries(&mut navigator);
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
    let collapsed_entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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

    let after_expand = create_test_entries(&mut navigator);

    // dir6 should NOT be visible (dir5 at depth 5 was not expanded due to depth limit)
    let has_dir6 = after_expand.iter().any(|e| e.name == "dir6");
    assert!(
        !has_dir6,
        "dir6 should not be visible - depth limit prevents expansion"
    );
}
