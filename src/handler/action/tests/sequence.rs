//! Sequence Tests (Phase 13.3)

use tempfile::TempDir;

use super::*;

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

    state.focus_index = 0;
    assert!(matches!(state.mode, ViewMode::Browse));

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

    assert!(!state.preview_visible);
    assert!(matches!(state.mode, ViewMode::Browse));

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

/// Simulates: / -> (type query) -> Enter -> n
#[test]
fn test_sequence_search_workflow() {
    let temp = TempDir::new().unwrap();
    // The root entry is searched too, and a random temp dir name can contain
    // the query, so the root gets a fixed name.
    let root = temp.path().join("fruits");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("apple.txt"), "").unwrap();
    std::fs::write(root.join("banana.txt"), "").unwrap();
    std::fs::write(root.join("apricot.txt"), "").unwrap();

    let mut state = create_test_state(&root);
    let mut navigator = create_test_navigator(&root);
    let entries = create_test_entries(&mut navigator);
    let context = ActionContext::default();
    let mut text_preview: Option<TextPreview> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut archive_preview: Option<ArchivePreview> = None;

    state.focus_index = 0;

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

    assert!(matches!(state.mode, ViewMode::Browse));
    assert_eq!(state.last_search.as_deref(), Some("ap"));
    let first = state.focus_index;
    assert!(entries[first].name.contains("ap"));

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

    let second = state.focus_index;
    assert_ne!(second, first, "SearchNext should move to the other match");
    assert!(entries[second].name.contains("ap"));
    assert_eq!(state.search_matches.map(|(_, total)| total), Some(2));
}

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

    let file1_idx = entries
        .iter()
        .position(|e| e.name == "file1.txt")
        .unwrap_or(1);
    state.focus_index = file1_idx;
    let file1_path = entries[file1_idx].path.clone();

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

    let dest_idx = entries.iter().position(|e| e.name == "dest").unwrap_or(0);
    state.focus_index = dest_idx;

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

    assert!(
        dest_dir.join("file1.txt").exists(),
        "File should be copied to destination"
    );
    assert!(state.message.as_ref().unwrap().contains("Pasted"));

    entries = create_test_entries(&mut navigator);
    assert!(!entries.is_empty());
}

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

    let subdir_idx = entries.iter().position(|e| e.name == "subdir").unwrap_or(0);
    let subdir_path = Some(subdir.clone());

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

    let entries = create_test_entries(&mut navigator);

    state.focus_index = subdir_idx + 1;

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

    let entries = create_test_entries(&mut navigator);
    let new_file_focused = Some(new_file.clone());

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

    state.selected_paths.insert(file1.clone());
    state.selected_paths.insert(file2.clone());

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

    assert!(file1.exists());
    assert!(file2.exists());

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
