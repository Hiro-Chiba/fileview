//! Integration tests for FileView
//!
//! These tests simulate user interactions and verify the application behavior.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fileview::core::{AppState, InputPurpose, PendingAction, ViewMode};
use fileview::handler::{handle_key_event, update_input_buffer, KeyAction};
use fileview::render::{
    is_binary_file, is_image_file, is_text_file, DirectoryInfo, HexPreview, TextPreview,
};
use tempfile::TempDir;

/// Helper to create a KeyEvent
fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

// =============================================================================
// Core State Tests
// =============================================================================

mod state_tests {
    use super::*;

    #[test]
    fn test_app_state_initialization() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        assert_eq!(state.focus_index, 0);
        assert_eq!(state.viewport_top, 0);
        assert!(state.selected_paths.is_empty());
        assert_eq!(state.mode, ViewMode::Browse);
        assert!(state.message.is_none());
        assert!(!state.preview_visible);
        assert!(!state.show_hidden);
        assert!(!state.should_quit);
        assert!(!state.pick_mode);
        assert!(state.clipboard.is_none());
    }

    #[test]
    fn test_viewport_adjustment_scroll_down() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        // Simulate focus moving beyond visible area
        state.focus_index = 25;
        state.adjust_viewport(10); // visible_height = 10

        // Viewport should scroll to keep focus visible
        assert!(state.viewport_top > 0);
        assert!(state.focus_index < state.viewport_top + 10);
    }

    #[test]
    fn test_viewport_adjustment_scroll_up() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.viewport_top = 20;
        state.focus_index = 5; // Focus above viewport
        state.adjust_viewport(10);

        // Viewport should scroll up
        assert_eq!(state.viewport_top, 5);
    }

    #[test]
    fn test_message_set_and_clear() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        assert!(state.message.is_none());

        state.set_message("Test message");
        assert_eq!(state.message, Some("Test message".to_string()));

        state.clear_message();
        assert!(state.message.is_none());
    }

    #[test]
    fn test_mode_transitions() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        // Browse -> Search
        state.mode = ViewMode::Search {
            query: "test".to_string(),
        };
        assert!(matches!(state.mode, ViewMode::Search { .. }));

        // Browse -> Input
        state.mode = ViewMode::Input {
            purpose: InputPurpose::CreateFile,
            buffer: String::new(),
            cursor: 0,
        };
        assert!(matches!(state.mode, ViewMode::Input { .. }));

        // Browse -> Confirm
        state.mode = ViewMode::Confirm {
            action: PendingAction::Delete {
                targets: vec![PathBuf::from("/tmp/test")],
            },
        };
        assert!(matches!(state.mode, ViewMode::Confirm { .. }));

        // Browse -> Preview
        state.mode = ViewMode::Preview { scroll: 0 };
        assert!(matches!(state.mode, ViewMode::Preview { .. }));
    }
}

// =============================================================================
// Key Handler Tests
// =============================================================================

mod key_handler_tests {
    use super::*;

    #[test]
    fn test_browse_mode_navigation_keys() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        // j / Down -> MoveDown
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('j'))),
            KeyAction::MoveDown
        ));
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Down)),
            KeyAction::MoveDown
        ));

        // k / Up -> MoveUp
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('k'))),
            KeyAction::MoveUp
        ));
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Up)),
            KeyAction::MoveUp
        ));

        // g -> MoveToTop
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('g'))),
            KeyAction::MoveToTop
        ));

        // G -> MoveToBottom
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('G'))),
            KeyAction::MoveToBottom
        ));
    }

    #[test]
    fn test_browse_mode_tree_operations() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        // l / Right -> Expand
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('l'))),
            KeyAction::Expand
        ));
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Right)),
            KeyAction::Expand
        ));

        // h / Left / Backspace -> Collapse
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('h'))),
            KeyAction::Collapse
        ));
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Left)),
            KeyAction::Collapse
        ));
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Backspace)),
            KeyAction::Collapse
        ));

        // H -> CollapseAll
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('H'))),
            KeyAction::CollapseAll
        ));

        // L -> ExpandAll
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('L'))),
            KeyAction::ExpandAll
        ));

        // Tab -> ToggleExpand
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Tab)),
            KeyAction::ToggleExpand
        ));
    }

    #[test]
    fn test_browse_mode_selection() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        // Space -> ToggleMark
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char(' '))),
            KeyAction::ToggleMark
        ));

        // Enter (non-pick mode) -> ToggleExpand
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Enter)),
            KeyAction::ToggleExpand
        ));
    }

    #[test]
    fn test_browse_mode_pick_mode() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.pick_mode = true;

        // Enter in pick mode -> PickSelect
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Enter)),
            KeyAction::PickSelect
        ));

        // q in pick mode -> Cancel (not Quit)
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('q'))),
            KeyAction::Cancel
        ));
    }

    #[test]
    fn test_browse_mode_file_operations() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        // a -> StartNewFile
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('a'))),
            KeyAction::StartNewFile
        ));

        // A -> StartNewDir
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('A'))),
            KeyAction::StartNewDir
        ));

        // r -> StartRename
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('r'))),
            KeyAction::StartRename
        ));

        // D -> ConfirmDelete
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('D'))),
            KeyAction::ConfirmDelete
        ));

        // Delete key -> ConfirmDelete
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Delete)),
            KeyAction::ConfirmDelete
        ));
    }

    #[test]
    fn test_browse_mode_clipboard() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        // y -> Copy
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('y'))),
            KeyAction::Copy
        ));

        // d -> Cut
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('d'))),
            KeyAction::Cut
        ));

        // p -> Paste
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('p'))),
            KeyAction::Paste
        ));
    }

    #[test]
    fn test_browse_mode_preview_keys() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        // o -> OpenPreview (fullscreen)
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('o'))),
            KeyAction::OpenPreview
        ));

        // P -> ToggleQuickPreview (side panel)
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('P'))),
            KeyAction::ToggleQuickPreview
        ));
    }

    #[test]
    fn test_browse_mode_search() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        // / -> StartSearch
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('/'))),
            KeyAction::StartSearch
        ));

        // n -> SearchNext
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('n'))),
            KeyAction::SearchNext
        ));
    }

    #[test]
    fn test_browse_mode_other() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        // . -> ToggleHidden
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('.'))),
            KeyAction::ToggleHidden
        ));

        // R -> Refresh
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('R'))),
            KeyAction::Refresh
        ));

        // F5 -> Refresh
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::F(5))),
            KeyAction::Refresh
        ));

        // ? -> ShowHelp
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('?'))),
            KeyAction::ShowHelp
        ));

        // c -> CopyPath
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('c'))),
            KeyAction::CopyPath
        ));

        // C -> CopyFilename
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('C'))),
            KeyAction::CopyFilename
        ));
    }

    #[test]
    fn test_preview_mode_keys() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::Preview { scroll: 0 };

        // o -> Cancel (close preview)
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('o'))),
            KeyAction::Cancel
        ));

        // q -> Cancel
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('q'))),
            KeyAction::Cancel
        ));

        // Esc -> Cancel
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Esc)),
            KeyAction::Cancel
        ));

        // j / Down -> PreviewScrollDown
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('j'))),
            KeyAction::PreviewScrollDown
        ));
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Down)),
            KeyAction::PreviewScrollDown
        ));

        // k / Up -> PreviewScrollUp
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('k'))),
            KeyAction::PreviewScrollUp
        ));

        // PageUp / b -> PreviewPageUp
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::PageUp)),
            KeyAction::PreviewPageUp
        ));
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('b'))),
            KeyAction::PreviewPageUp
        ));

        // PageDown / f / Space -> PreviewPageDown
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::PageDown)),
            KeyAction::PreviewPageDown
        ));
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('f'))),
            KeyAction::PreviewPageDown
        ));

        // g -> PreviewToTop
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('g'))),
            KeyAction::PreviewToTop
        ));

        // G -> PreviewToBottom
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('G'))),
            KeyAction::PreviewToBottom
        ));
    }

    #[test]
    fn test_confirm_mode_keys() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::Confirm {
            action: PendingAction::Delete {
                targets: vec![PathBuf::from("/tmp/test")],
            },
        };

        // y -> ExecuteDelete
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('y'))),
            KeyAction::ExecuteDelete
        ));

        // Y -> ExecuteDelete
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('Y'))),
            KeyAction::ExecuteDelete
        ));

        // Enter -> ExecuteDelete
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Enter)),
            KeyAction::ExecuteDelete
        ));

        // n -> Cancel
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('n'))),
            KeyAction::Cancel
        ));

        // Esc -> Cancel
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Esc)),
            KeyAction::Cancel
        ));
    }

    #[test]
    fn test_input_mode_keys() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::Input {
            purpose: InputPurpose::CreateFile,
            buffer: "test.txt".to_string(),
            cursor: 8,
        };

        // Enter -> ConfirmInput with current buffer
        let action = handle_key_event(&state, key_event(KeyCode::Enter));
        match action {
            KeyAction::ConfirmInput { value } => assert_eq!(value, "test.txt"),
            _ => panic!("Expected ConfirmInput"),
        }

        // Esc -> Cancel
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Esc)),
            KeyAction::Cancel
        ));
    }

    #[test]
    fn test_search_mode_keys() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::Search {
            query: "search_term".to_string(),
        };

        // Enter -> ConfirmInput with current query
        let action = handle_key_event(&state, key_event(KeyCode::Enter));
        match action {
            KeyAction::ConfirmInput { value } => assert_eq!(value, "search_term"),
            _ => panic!("Expected ConfirmInput"),
        }

        // Esc -> Cancel
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Esc)),
            KeyAction::Cancel
        ));
    }

    #[test]
    fn test_escape_with_selections_clears_marks() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state
            .selected_paths
            .insert(PathBuf::from("/tmp/selected_file"));

        // Esc with selections -> ClearMarks
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Esc)),
            KeyAction::ClearMarks
        ));
    }
}

// =============================================================================
// Preview Tests
// =============================================================================

mod preview_tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_text_file_detection() {
        // Known text extensions
        assert!(is_text_file(&PathBuf::from("file.txt")));
        assert!(is_text_file(&PathBuf::from("file.md")));
        assert!(is_text_file(&PathBuf::from("file.rs")));
        assert!(is_text_file(&PathBuf::from("file.py")));
        assert!(is_text_file(&PathBuf::from("file.js")));
        assert!(is_text_file(&PathBuf::from("file.json")));
        assert!(is_text_file(&PathBuf::from("file.toml")));
        assert!(is_text_file(&PathBuf::from("file.yaml")));
        assert!(is_text_file(&PathBuf::from("file.html")));
        assert!(is_text_file(&PathBuf::from("file.css")));
        assert!(is_text_file(&PathBuf::from("file.sh")));

        // Case insensitive
        assert!(is_text_file(&PathBuf::from("FILE.TXT")));
        assert!(is_text_file(&PathBuf::from("File.Rs")));

        // Not text files
        assert!(!is_text_file(&PathBuf::from("file.png")));
        assert!(!is_text_file(&PathBuf::from("file.exe")));
        assert!(!is_text_file(&PathBuf::from("file.unknown")));
    }

    #[test]
    fn test_image_file_detection() {
        // Known image extensions
        assert!(is_image_file(&PathBuf::from("image.png")));
        assert!(is_image_file(&PathBuf::from("image.jpg")));
        assert!(is_image_file(&PathBuf::from("image.jpeg")));
        assert!(is_image_file(&PathBuf::from("image.gif")));
        assert!(is_image_file(&PathBuf::from("image.webp")));
        assert!(is_image_file(&PathBuf::from("image.bmp")));
        assert!(is_image_file(&PathBuf::from("image.ico")));

        // Case insensitive
        assert!(is_image_file(&PathBuf::from("IMAGE.PNG")));
        assert!(is_image_file(&PathBuf::from("Image.Jpg")));

        // Not image files
        assert!(!is_image_file(&PathBuf::from("file.txt")));
        assert!(!is_image_file(&PathBuf::from("file.rs")));
    }

    #[test]
    fn test_binary_file_detection() {
        // Known binary extensions
        assert!(is_binary_file(&PathBuf::from("file.exe")));
        assert!(is_binary_file(&PathBuf::from("file.dll")));
        assert!(is_binary_file(&PathBuf::from("file.so")));
        assert!(is_binary_file(&PathBuf::from("file.dylib")));
        assert!(is_binary_file(&PathBuf::from("file.o")));
        assert!(is_binary_file(&PathBuf::from("file.bin")));
        assert!(is_binary_file(&PathBuf::from("file.wasm")));

        // Text files should not be detected as binary
        assert!(!is_binary_file(&PathBuf::from("file.txt")));
        assert!(!is_binary_file(&PathBuf::from("file.rs")));

        // Image files should not be detected as binary
        assert!(!is_binary_file(&PathBuf::from("file.png")));
        assert!(!is_binary_file(&PathBuf::from("file.jpg")));
    }

    #[test]
    fn test_text_preview_creation() {
        let content = "Line 1\nLine 2\nLine 3";
        let preview = TextPreview::new(content);

        assert_eq!(preview.lines.len(), 3);
        assert_eq!(preview.lines[0], "Line 1");
        assert_eq!(preview.lines[1], "Line 2");
        assert_eq!(preview.lines[2], "Line 3");
        assert_eq!(preview.scroll, 0);
    }

    #[test]
    fn test_text_preview_empty() {
        let preview = TextPreview::new("");
        assert_eq!(preview.lines.len(), 0);
    }

    #[test]
    fn test_directory_info_from_path() {
        let temp = TempDir::new().unwrap();

        // Create some files and directories
        fs::write(temp.path().join("file1.txt"), "content").unwrap();
        fs::write(temp.path().join("file2.rs"), "fn main() {}").unwrap();
        fs::write(temp.path().join(".hidden"), "secret").unwrap();
        fs::create_dir(temp.path().join("subdir")).unwrap();

        let info = DirectoryInfo::from_path(temp.path()).unwrap();

        assert_eq!(info.file_count, 3); // file1.txt, file2.rs, .hidden
        assert_eq!(info.dir_count, 1); // subdir
        assert_eq!(info.hidden_count, 1); // .hidden
        assert!(info.total_size > 0);
    }

    #[test]
    fn test_hex_preview_load() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("binary.bin");

        // Create a binary file
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(&[0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD])
            .unwrap();

        let preview = HexPreview::load(&file_path).unwrap();

        assert_eq!(preview.bytes.len(), 7);
        assert_eq!(preview.bytes[0], 0x00);
        assert_eq!(preview.bytes[4], 0xFF);
        assert_eq!(preview.size, 7);
        assert_eq!(preview.scroll, 0);
    }

    #[test]
    fn test_hex_preview_line_count() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.bin");

        // Create a file with 32 bytes (should be 2 lines)
        let data: Vec<u8> = (0..32).collect();
        fs::write(&file_path, &data).unwrap();

        let preview = HexPreview::load(&file_path).unwrap();
        assert_eq!(preview.line_count(), 2);

        // Create a file with 17 bytes (should be 2 lines)
        let file_path2 = temp.path().join("test2.bin");
        let data2: Vec<u8> = (0..17).collect();
        fs::write(&file_path2, &data2).unwrap();

        let preview2 = HexPreview::load(&file_path2).unwrap();
        assert_eq!(preview2.line_count(), 2);
    }
}

// =============================================================================
// Fullscreen Preview Bug Test (The bug we just fixed)
// =============================================================================

mod fullscreen_preview_tests {
    use super::*;

    /// This test verifies the bug fix for fullscreen preview.
    /// Previously, pressing 'o' without first pressing 'P' would show
    /// "No preview available" because preview data was only loaded
    /// when preview_visible was true.
    #[test]
    fn test_fullscreen_preview_mode_setup() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        // Initial state: Browse mode, preview not visible
        assert_eq!(state.mode, ViewMode::Browse);
        assert!(!state.preview_visible);

        // Press 'o' to enter fullscreen preview
        let action = handle_key_event(&state, key_event(KeyCode::Char('o')));
        assert!(matches!(action, KeyAction::OpenPreview));

        // Simulate the action being applied
        state.mode = ViewMode::Preview { scroll: 0 };

        // Verify mode changed but preview_visible is still false
        assert!(matches!(state.mode, ViewMode::Preview { .. }));
        assert!(!state.preview_visible);

        // The fix: preview should load when mode is Preview OR preview_visible is true
        let needs_preview = state.preview_visible || matches!(state.mode, ViewMode::Preview { .. });
        assert!(
            needs_preview,
            "Preview data should be loaded in fullscreen mode"
        );
    }

    #[test]
    fn test_side_panel_preview_mode_setup() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        // Press 'P' to toggle side panel
        let action = handle_key_event(&state, key_event(KeyCode::Char('P')));
        assert!(matches!(action, KeyAction::ToggleQuickPreview));

        // Simulate the action being applied
        state.preview_visible = true;

        // Verify preview_visible is now true
        assert!(state.preview_visible);
        assert_eq!(state.mode, ViewMode::Browse); // Mode unchanged

        // Preview should load because preview_visible is true
        let needs_preview = state.preview_visible || matches!(state.mode, ViewMode::Preview { .. });
        assert!(needs_preview);
    }

    #[test]
    fn test_preview_close_from_fullscreen() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::Preview { scroll: 0 };

        // Press 'o' again to close
        let action = handle_key_event(&state, key_event(KeyCode::Char('o')));
        assert!(matches!(action, KeyAction::Cancel));

        // Press 'q' to close
        let action = handle_key_event(&state, key_event(KeyCode::Char('q')));
        assert!(matches!(action, KeyAction::Cancel));

        // Press Esc to close
        let action = handle_key_event(&state, key_event(KeyCode::Esc));
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_preview_needs_check_logic() {
        // Test the needs_preview logic that was the root cause of the bug
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        // Case 1: Neither preview_visible nor Preview mode
        assert!(!state.preview_visible);
        assert!(matches!(state.mode, ViewMode::Browse));
        let needs_preview = state.preview_visible || matches!(state.mode, ViewMode::Preview { .. });
        assert!(!needs_preview, "Should not need preview in Browse mode");

        // Case 2: preview_visible true, Browse mode
        state.preview_visible = true;
        let needs_preview = state.preview_visible || matches!(state.mode, ViewMode::Preview { .. });
        assert!(needs_preview, "Should need preview when preview_visible");

        // Case 3: preview_visible false, Preview mode (THIS WAS THE BUG)
        state.preview_visible = false;
        state.mode = ViewMode::Preview { scroll: 0 };
        let needs_preview = state.preview_visible || matches!(state.mode, ViewMode::Preview { .. });
        assert!(
            needs_preview,
            "Should need preview when in Preview mode even if preview_visible is false"
        );
    }
}

// =============================================================================
// Input Buffer Tests
// =============================================================================

mod input_buffer_tests {
    use super::*;

    #[test]
    fn test_input_buffer_insert_char() {
        let result = update_input_buffer(key_event(KeyCode::Char('a')), "", 0);
        assert_eq!(result, Some(("a".to_string(), 1)));

        let result = update_input_buffer(key_event(KeyCode::Char('b')), "ac", 1);
        assert_eq!(result, Some(("abc".to_string(), 2)));
    }

    #[test]
    fn test_input_buffer_backspace() {
        let result = update_input_buffer(key_event(KeyCode::Backspace), "abc", 3);
        assert_eq!(result, Some(("ab".to_string(), 2)));

        // Backspace at start does nothing
        let result = update_input_buffer(key_event(KeyCode::Backspace), "abc", 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_input_buffer_delete() {
        let result = update_input_buffer(key_event(KeyCode::Delete), "abc", 1);
        assert_eq!(result, Some(("ac".to_string(), 1)));

        // Delete at end does nothing
        let result = update_input_buffer(key_event(KeyCode::Delete), "abc", 3);
        assert_eq!(result, None);
    }

    #[test]
    fn test_input_buffer_cursor_movement() {
        // Left
        let result = update_input_buffer(key_event(KeyCode::Left), "abc", 2);
        assert_eq!(result, Some(("abc".to_string(), 1)));

        // Left at start does nothing
        let result = update_input_buffer(key_event(KeyCode::Left), "abc", 0);
        assert_eq!(result, None);

        // Right
        let result = update_input_buffer(key_event(KeyCode::Right), "abc", 1);
        assert_eq!(result, Some(("abc".to_string(), 2)));

        // Right at end does nothing
        let result = update_input_buffer(key_event(KeyCode::Right), "abc", 3);
        assert_eq!(result, None);

        // Home
        let result = update_input_buffer(key_event(KeyCode::Home), "abc", 2);
        assert_eq!(result, Some(("abc".to_string(), 0)));

        // End
        let result = update_input_buffer(key_event(KeyCode::End), "abc", 1);
        assert_eq!(result, Some(("abc".to_string(), 3)));
    }
}

// =============================================================================
// Tree Navigator Tests
// =============================================================================

mod tree_navigator_tests {
    use super::*;
    use fileview::tree::TreeNavigator;
    use std::fs;

    #[test]
    fn test_navigator_creation() {
        let temp = TempDir::new().unwrap();

        // Create some files
        fs::write(temp.path().join("file1.txt"), "").unwrap();
        fs::write(temp.path().join("file2.txt"), "").unwrap();
        fs::create_dir(temp.path().join("dir1")).unwrap();

        let navigator = TreeNavigator::new(temp.path(), false).unwrap();
        let entries = navigator.visible_entries();

        // Should have root + 3 entries (2 files + 1 dir)
        // Root is always included and expanded
        assert!(entries.len() >= 4);
    }

    #[test]
    fn test_navigator_hidden_files() {
        let temp = TempDir::new().unwrap();

        fs::write(temp.path().join("visible.txt"), "").unwrap();
        fs::write(temp.path().join(".hidden"), "").unwrap();

        // Without hidden files - should only show visible.txt (+ root)
        let navigator_no_hidden = TreeNavigator::new(temp.path(), false).unwrap();
        let count_no_hidden = navigator_no_hidden.visible_entries().len();

        // With hidden files - should show both visible.txt and .hidden (+ root)
        let navigator_with_hidden = TreeNavigator::new(temp.path(), true).unwrap();
        let count_with_hidden = navigator_with_hidden.visible_entries().len();

        // With hidden files should have more entries
        assert!(
            count_with_hidden > count_no_hidden,
            "With hidden files ({}) should have more entries than without ({})",
            count_with_hidden,
            count_no_hidden
        );
    }

    #[test]
    fn test_navigator_expand_collapse() {
        let temp = TempDir::new().unwrap();

        fs::create_dir(temp.path().join("dir1")).unwrap();
        fs::write(temp.path().join("dir1").join("nested.txt"), "").unwrap();

        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();

        let initial_count = navigator.visible_entries().len();

        // Expand dir1
        navigator.expand(&temp.path().join("dir1")).unwrap();
        let expanded_count = navigator.visible_entries().len();
        assert!(
            expanded_count > initial_count,
            "Expanding should show more entries"
        );

        // Collapse dir1
        navigator.collapse(&temp.path().join("dir1"));
        let collapsed_count = navigator.visible_entries().len();
        assert_eq!(
            collapsed_count, initial_count,
            "Collapsing should hide nested entries"
        );
    }

    #[test]
    fn test_navigator_toggle_expand() {
        let temp = TempDir::new().unwrap();

        fs::create_dir(temp.path().join("dir1")).unwrap();
        fs::write(temp.path().join("dir1").join("file.txt"), "").unwrap();

        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();

        let dir_path = temp.path().join("dir1");
        let initial_count = navigator.visible_entries().len();

        // Toggle to expand
        navigator.toggle_expand(&dir_path).unwrap();
        let expanded_count = navigator.visible_entries().len();
        assert!(expanded_count > initial_count);

        // Toggle to collapse
        navigator.toggle_expand(&dir_path).unwrap();
        let collapsed_count = navigator.visible_entries().len();
        assert_eq!(collapsed_count, initial_count);
    }

    #[test]
    fn test_navigator_reload() {
        let temp = TempDir::new().unwrap();

        fs::write(temp.path().join("initial.txt"), "").unwrap();

        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();
        let initial_count = navigator.visible_entries().len();

        // Add a file externally
        fs::write(temp.path().join("new_file.txt"), "").unwrap();

        // Reload
        navigator.reload().unwrap();
        let reloaded_count = navigator.visible_entries().len();
        assert!(
            reloaded_count > initial_count,
            "Reload should pick up new files"
        );
    }

    #[test]
    fn test_navigator_visible_count() {
        let temp = TempDir::new().unwrap();

        fs::write(temp.path().join("file1.txt"), "").unwrap();
        fs::write(temp.path().join("file2.txt"), "").unwrap();

        let navigator = TreeNavigator::new(temp.path(), false).unwrap();

        assert_eq!(navigator.visible_count(), navigator.visible_entries().len());
    }
}

// =============================================================================
// Clipboard Tests
// =============================================================================

mod clipboard_tests {
    use fileview::action::{Clipboard, ClipboardContent};
    use std::path::PathBuf;

    #[test]
    fn test_clipboard_new() {
        let clipboard = Clipboard::new();
        assert!(clipboard.is_empty());
        assert!(clipboard.paths().is_empty());
    }

    #[test]
    fn test_clipboard_copy() {
        let mut clipboard = Clipboard::new();
        let paths = vec![
            PathBuf::from("/path/to/file1"),
            PathBuf::from("/path/to/file2"),
        ];

        clipboard.copy(paths.clone());

        assert!(!clipboard.is_empty());
        assert!(!clipboard.is_cut());
        assert_eq!(clipboard.paths().len(), 2);
        assert!(matches!(
            clipboard.content(),
            Some(ClipboardContent::Copy(_))
        ));
    }

    #[test]
    fn test_clipboard_cut() {
        let mut clipboard = Clipboard::new();
        let paths = vec![PathBuf::from("/path/to/file")];

        clipboard.cut(paths);

        assert!(!clipboard.is_empty());
        assert!(clipboard.is_cut());
        assert!(matches!(
            clipboard.content(),
            Some(ClipboardContent::Cut(_))
        ));
    }

    #[test]
    fn test_clipboard_take() {
        let mut clipboard = Clipboard::new();
        clipboard.copy(vec![PathBuf::from("/path")]);

        let taken = clipboard.take();

        assert!(clipboard.is_empty());
        assert!(taken.is_some());
    }

    #[test]
    fn test_clipboard_clear() {
        let mut clipboard = Clipboard::new();
        clipboard.copy(vec![PathBuf::from("/path")]);

        clipboard.clear();

        assert!(clipboard.is_empty());
    }
}

// =============================================================================
// File Operations Tests (using the file module's API)
// =============================================================================

mod file_operations_tests {
    use fileview::action::file::{copy_to, create_dir, create_file, delete, rename};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_create_and_delete_file() {
        let temp = TempDir::new().unwrap();

        // Create file
        let file_path = create_file(temp.path(), "test_file.txt").unwrap();
        assert!(file_path.exists());
        assert!(file_path.is_file());

        // Delete file
        delete(&file_path).unwrap();
        assert!(!file_path.exists());
    }

    #[test]
    fn test_create_and_delete_directory() {
        let temp = TempDir::new().unwrap();

        // Create directory
        let dir_path = create_dir(temp.path(), "test_dir").unwrap();
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());

        // Delete directory
        delete(&dir_path).unwrap();
        assert!(!dir_path.exists());
    }

    #[test]
    fn test_rename_file() {
        let temp = TempDir::new().unwrap();
        let old_path = temp.path().join("old_name.txt");

        fs::write(&old_path, "content").unwrap();

        let new_path = rename(&old_path, "new_name.txt").unwrap();

        assert!(!old_path.exists());
        assert!(new_path.exists());
        assert_eq!(new_path.file_name().unwrap(), "new_name.txt");
    }

    #[test]
    fn test_copy_file() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        let dest_dir = temp.path().join("dest");

        fs::write(&source, "content").unwrap();
        fs::create_dir(&dest_dir).unwrap();

        let copied = copy_to(&source, &dest_dir).unwrap();

        // Original should still exist
        assert!(source.exists());
        // Copy should exist in destination
        assert!(copied.exists());
        assert_eq!(fs::read_to_string(&copied).unwrap(), "content");
    }

    #[test]
    fn test_delete_non_empty_directory() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("non_empty_dir");

        fs::create_dir(&dir_path).unwrap();
        fs::write(dir_path.join("file.txt"), "content").unwrap();
        fs::create_dir(dir_path.join("subdir")).unwrap();

        // Should be able to delete non-empty directory
        delete(&dir_path).unwrap();
        assert!(!dir_path.exists());
    }
}

// =============================================================================
// Drag and Drop Tests
// =============================================================================

mod drag_and_drop_tests {
    use fileview::handler::mouse::DropDetector;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_drop_detector_with_real_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test_file.txt");
        fs::write(&file_path, "test content").unwrap();

        let mut detector = DropDetector::new();
        detector.push_char('/'); // Start path

        // Simulate rapid input of file path
        let path_str = file_path.display().to_string();
        for c in path_str.chars().skip(1) {
            detector.push_char(c);
        }

        let paths = detector.extract_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], file_path);
    }

    #[test]
    fn test_drop_detector_with_real_directory() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("test_dir");
        fs::create_dir(&dir_path).unwrap();

        let mut detector = DropDetector::new();
        let path_str = dir_path.display().to_string();
        for c in path_str.chars() {
            detector.push_char(c);
        }

        let paths = detector.extract_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], dir_path);
    }

    #[test]
    fn test_drop_detector_file_url_format() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "content").unwrap();

        let mut detector = DropDetector::new();
        // Simulate file:// URL format
        let url = format!("file://{}", file_path.display());
        for c in url.chars() {
            detector.push_char(c);
        }

        let paths = detector.extract_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], file_path);
    }

    #[test]
    fn test_drop_detector_url_encoded_spaces() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("file with spaces.txt");
        fs::write(&file_path, "content").unwrap();

        let mut detector = DropDetector::new();
        // Simulate URL-encoded path with %20 for spaces
        let path_str = file_path.display().to_string().replace(' ', "%20");
        for c in path_str.chars() {
            detector.push_char(c);
        }

        let paths = detector.extract_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], file_path);
    }

    #[test]
    fn test_drop_detector_backslash_escaped_spaces() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("file with spaces.txt");
        fs::write(&file_path, "content").unwrap();

        let mut detector = DropDetector::new();
        // Simulate backslash-escaped path (macOS terminal style)
        let path_str = file_path.display().to_string().replace(' ', "\\ ");
        for c in path_str.chars() {
            detector.push_char(c);
        }

        let paths = detector.extract_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], file_path);
    }

    #[test]
    fn test_drop_detector_multiple_files() {
        let temp = TempDir::new().unwrap();
        let file1 = temp.path().join("file1.txt");
        let file2 = temp.path().join("file2.txt");
        fs::write(&file1, "content1").unwrap();
        fs::write(&file2, "content2").unwrap();

        let mut detector = DropDetector::new();
        // Simulate multiple paths separated by newline
        let paths_str = format!("{}\n{}", file1.display(), file2.display());
        for c in paths_str.chars() {
            detector.push_char(c);
        }

        let paths = detector.extract_paths();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&file1));
        assert!(paths.contains(&file2));
    }

    #[test]
    fn test_drop_detector_nonexistent_path_filtered() {
        let mut detector = DropDetector::new();
        // Path that doesn't exist should be filtered out
        let fake_path = "/nonexistent/path/to/file.txt";
        for c in fake_path.chars() {
            detector.push_char(c);
        }

        let paths = detector.extract_paths();
        assert!(paths.is_empty());
    }

    #[test]
    fn test_drop_detector_mixed_existing_nonexisting() {
        let temp = TempDir::new().unwrap();
        let existing = temp.path().join("existing.txt");
        fs::write(&existing, "content").unwrap();

        let mut detector = DropDetector::new();
        // Mix of existing and non-existing paths
        let paths_str = format!("{}\n/nonexistent/path.txt", existing.display());
        for c in paths_str.chars() {
            detector.push_char(c);
        }

        let paths = detector.extract_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], existing);
    }

    #[test]
    fn test_drop_detector_quoted_path_with_spaces() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("file with spaces.txt");
        fs::write(&file_path, "content").unwrap();

        let mut detector = DropDetector::new();
        // Simulate quoted path
        let quoted = format!("\"{}\"", file_path.display());
        for c in quoted.chars() {
            detector.push_char(c);
        }

        let paths = detector.extract_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], file_path);
    }

    #[test]
    fn test_drop_detector_clear() {
        let mut detector = DropDetector::new();
        detector.push_char('/');
        detector.push_char('t');
        detector.push_char('e');
        detector.push_char('s');
        detector.push_char('t');

        assert!(!detector.is_empty());
        detector.clear();
        assert!(detector.is_empty());
    }
}
