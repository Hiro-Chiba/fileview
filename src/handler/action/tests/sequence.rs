//! Sequence Tests (Phase 13.3)

use tempfile::TempDir;

use super::*;

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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let mut entries = create_test_entries(&mut navigator);
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
    entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    navigator.ensure_cache();
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
    navigator.ensure_cache();
    let expanded_count = navigator.visible_count();
    assert!(
        expanded_count > initial_count,
        "Should see nested files after expand"
    );

    // Update entries after expand
    let entries = create_test_entries(&mut navigator);

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

    navigator.ensure_cache();
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
    let entries = create_test_entries(&mut navigator);
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
