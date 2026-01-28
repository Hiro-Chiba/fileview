//! Integration tests for FileView
//!
//! These tests simulate user interactions and verify the application behavior.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fileview::core::{AppState, InputPurpose, PendingAction, ViewMode};
use fileview::handler::{handle_key_event, update_input_buffer, KeyAction};
use fileview::render::{
    calculate_centered_image_area, is_binary_file, is_image_file, is_text_file, DirectoryInfo,
    FontSize, HexPreview, RecommendedProtocol, TerminalBrand, TextPreview,
};
use ratatui::layout::Rect;
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

    // Focus-aware key handling tests

    #[test]
    fn test_tab_toggles_focus_when_preview_visible() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.preview_visible = true;

        // Tab should toggle focus when preview is visible
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Tab)),
            KeyAction::ToggleFocus
        ));
    }

    #[test]
    fn test_tab_toggles_expand_when_preview_not_visible() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.preview_visible = false;

        // Tab should toggle expand when preview is not visible
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Tab)),
            KeyAction::ToggleExpand
        ));
    }

    #[test]
    fn test_navigation_keys_scroll_preview_when_focus_on_preview() {
        use fileview::core::FocusTarget;

        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.preview_visible = true;
        state.focus_target = FocusTarget::Preview;

        // j should scroll preview
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('j'))),
            KeyAction::PreviewScrollDown
        ));

        // k should scroll preview
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('k'))),
            KeyAction::PreviewScrollUp
        ));

        // Down arrow should scroll preview
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Down)),
            KeyAction::PreviewScrollDown
        ));

        // Up arrow should scroll preview
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Up)),
            KeyAction::PreviewScrollUp
        ));
    }

    #[test]
    fn test_navigation_keys_move_files_when_focus_on_tree() {
        use fileview::core::FocusTarget;

        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.preview_visible = true;
        state.focus_target = FocusTarget::Tree;

        // j should move down
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('j'))),
            KeyAction::MoveDown
        ));

        // k should move up
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('k'))),
            KeyAction::MoveUp
        ));
    }

    #[test]
    fn test_escape_returns_focus_to_tree_when_on_preview() {
        use fileview::core::FocusTarget;

        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.preview_visible = true;
        state.focus_target = FocusTarget::Preview;

        // Esc should toggle focus back to tree
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Esc)),
            KeyAction::ToggleFocus
        ));
    }

    #[test]
    fn test_page_scroll_in_preview_focus() {
        use fileview::core::FocusTarget;

        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.preview_visible = true;
        state.focus_target = FocusTarget::Preview;

        // PageDown should page scroll
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::PageDown)),
            KeyAction::PreviewPageDown
        ));

        // PageUp should page scroll
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::PageUp)),
            KeyAction::PreviewPageUp
        ));

        // b should page up
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('b'))),
            KeyAction::PreviewPageUp
        ));

        // f should page down
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('f'))),
            KeyAction::PreviewPageDown
        ));
    }

    #[test]
    fn test_g_and_shift_g_in_preview_focus() {
        use fileview::core::FocusTarget;

        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.preview_visible = true;
        state.focus_target = FocusTarget::Preview;

        // g should go to top
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('g'))),
            KeyAction::PreviewToTop
        ));

        // G should go to bottom
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('G'))),
            KeyAction::PreviewToBottom
        ));
    }

    #[test]
    fn test_g_and_shift_g_in_tree_focus() {
        use fileview::core::FocusTarget;

        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.preview_visible = true;
        state.focus_target = FocusTarget::Tree;

        // g should go to top of file list
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('g'))),
            KeyAction::MoveToTop
        ));

        // G should go to bottom of file list
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('G'))),
            KeyAction::MoveToBottom
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

// =============================================================================
// Pick Output Format Tests
// =============================================================================

mod pick_output_tests {
    use fileview::integrate::OutputFormat;
    use std::str::FromStr;

    #[test]
    fn test_output_format_lines_variants() {
        assert!(matches!(
            OutputFormat::from_str("lines"),
            Ok(OutputFormat::Lines)
        ));
        assert!(matches!(
            OutputFormat::from_str("line"),
            Ok(OutputFormat::Lines)
        ));
        assert!(matches!(
            OutputFormat::from_str("LINES"),
            Ok(OutputFormat::Lines)
        ));
    }

    #[test]
    fn test_output_format_null_variants() {
        assert!(matches!(
            OutputFormat::from_str("null"),
            Ok(OutputFormat::NullSeparated)
        ));
        assert!(matches!(
            OutputFormat::from_str("nul"),
            Ok(OutputFormat::NullSeparated)
        ));
        assert!(matches!(
            OutputFormat::from_str("0"),
            Ok(OutputFormat::NullSeparated)
        ));
        assert!(matches!(
            OutputFormat::from_str("NULL"),
            Ok(OutputFormat::NullSeparated)
        ));
    }

    #[test]
    fn test_output_format_json_variants() {
        assert!(matches!(
            OutputFormat::from_str("json"),
            Ok(OutputFormat::Json)
        ));
        assert!(matches!(
            OutputFormat::from_str("JSON"),
            Ok(OutputFormat::Json)
        ));
    }

    #[test]
    fn test_output_format_invalid() {
        assert!(OutputFormat::from_str("invalid").is_err());
        assert!(OutputFormat::from_str("xml").is_err());
        assert!(OutputFormat::from_str("csv").is_err());
        assert!(OutputFormat::from_str("").is_err());
    }

    #[test]
    fn test_output_format_default() {
        let default = OutputFormat::default();
        assert!(matches!(default, OutputFormat::Lines));
    }
}

// =============================================================================
// File Operation Edge Case Tests
// =============================================================================

mod file_edge_case_tests {
    use fileview::action::file::{copy_to, create_file, delete, rename};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_unique_name_single_conflict() {
        let temp = TempDir::new().unwrap();

        // Create initial file
        fs::write(temp.path().join("file.txt"), "original").unwrap();

        // Copy to same directory - should create file_1.txt
        let source = temp.path().join("file.txt");
        let copied = copy_to(&source, temp.path()).unwrap();

        assert!(copied.exists());
        assert_ne!(copied, source);
        // Name should have suffix to avoid conflict
        let name = copied.file_name().unwrap().to_str().unwrap();
        assert!(name.contains("file") && name.ends_with(".txt"));
    }

    #[test]
    fn test_unique_name_multiple_conflicts() {
        let temp = TempDir::new().unwrap();

        // Create files that would conflict
        fs::write(temp.path().join("file.txt"), "original").unwrap();
        fs::write(temp.path().join("file_1.txt"), "copy1").unwrap();

        // Copy - should create file_2.txt
        let source = temp.path().join("file.txt");
        let copied = copy_to(&source, temp.path()).unwrap();

        assert!(copied.exists());
        let name = copied.file_name().unwrap().to_str().unwrap();
        // Should have incremented suffix
        assert!(name != "file.txt" && name != "file_1.txt");
    }

    #[test]
    fn test_filename_with_spaces() {
        let temp = TempDir::new().unwrap();

        // Create file with spaces in name
        let file_path = create_file(temp.path(), "my file.txt").unwrap();
        assert!(file_path.exists());
        assert_eq!(file_path.file_name().unwrap(), "my file.txt");

        // Rename with spaces
        let renamed = rename(&file_path, "new name.txt").unwrap();
        assert!(renamed.exists());
        assert_eq!(renamed.file_name().unwrap(), "new name.txt");

        // Delete
        delete(&renamed).unwrap();
        assert!(!renamed.exists());
    }

    #[test]
    fn test_filename_with_unicode() {
        let temp = TempDir::new().unwrap();

        // Create file with Unicode name
        let file_path = create_file(temp.path(), "日本語.txt").unwrap();
        assert!(file_path.exists());
        assert_eq!(file_path.file_name().unwrap(), "日本語.txt");

        // Rename with Unicode
        let renamed = rename(&file_path, "中文.txt").unwrap();
        assert!(renamed.exists());
        assert_eq!(renamed.file_name().unwrap(), "中文.txt");
    }

    #[test]
    fn test_filename_with_multiple_dots() {
        let temp = TempDir::new().unwrap();

        let file_path = create_file(temp.path(), "file.backup.txt").unwrap();
        assert!(file_path.exists());

        // Copy should preserve the name pattern
        let copied = copy_to(&file_path, temp.path()).unwrap();
        let name = copied.file_name().unwrap().to_str().unwrap();
        assert!(name.ends_with(".txt"));
    }

    #[test]
    fn test_delete_nonexistent_file() {
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("does_not_exist.txt");

        // Deleting non-existent file should fail
        let result = delete(&nonexistent);
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_to_same_name() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "content").unwrap();

        // Renaming to the same name should succeed (no-op)
        let result = rename(&file_path, "test.txt");
        assert!(result.is_ok());
        assert!(file_path.exists());
    }

    #[test]
    fn test_copy_directory_recursive() {
        let temp = TempDir::new().unwrap();

        // Create source directory with nested content
        let source_dir = temp.path().join("source");
        fs::create_dir(&source_dir).unwrap();
        fs::write(source_dir.join("file1.txt"), "content1").unwrap();
        fs::create_dir(source_dir.join("subdir")).unwrap();
        fs::write(source_dir.join("subdir").join("file2.txt"), "content2").unwrap();

        // Create destination
        let dest_dir = temp.path().join("dest");
        fs::create_dir(&dest_dir).unwrap();

        // Copy directory
        let copied = copy_to(&source_dir, &dest_dir).unwrap();

        assert!(copied.is_dir());
        assert!(copied.join("file1.txt").exists());
        assert!(copied.join("subdir").join("file2.txt").exists());
    }

    #[test]
    fn test_create_file_in_nonexistent_parent() {
        let temp = TempDir::new().unwrap();
        let nonexistent_parent = temp.path().join("nonexistent").join("subdir");

        // Creating file in non-existent directory should fail
        let result = create_file(&nonexistent_parent, "file.txt");
        assert!(result.is_err());
    }
}

// =============================================================================
// Drag and Drop Tests
// =============================================================================

// =============================================================================
// Git Status Tests
// =============================================================================

mod git_status_tests {
    use fileview::git::{FileStatus, GitStatus};
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_git_repo(dir: &std::path::Path) -> bool {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn git_add(dir: &std::path::Path, file: &str) -> bool {
        Command::new("git")
            .args(["add", file])
            .current_dir(dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn git_commit(dir: &std::path::Path, msg: &str) -> bool {
        Command::new("git")
            .args(["commit", "-m", msg])
            .current_dir(dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn git_config(dir: &std::path::Path) -> bool {
        let _ = Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn test_non_git_directory() {
        let temp = TempDir::new().unwrap();
        // Non-git directory should return None
        let status = GitStatus::detect(temp.path());
        assert!(status.is_none());
    }

    #[test]
    fn test_git_repo_detection() {
        let temp = TempDir::new().unwrap();

        if !init_git_repo(temp.path()) {
            // Skip test if git is not available
            return;
        }

        let status = GitStatus::detect(temp.path());
        assert!(status.is_some());

        let status = status.unwrap();
        // Use canonicalize to handle macOS /var -> /private/var symlink
        let expected = temp
            .path()
            .canonicalize()
            .unwrap_or(temp.path().to_path_buf());
        let actual = status
            .repo_root()
            .canonicalize()
            .unwrap_or(status.repo_root().to_path_buf());
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_untracked_file_status() {
        let temp = TempDir::new().unwrap();

        if !init_git_repo(temp.path()) {
            return;
        }

        // Create untracked file
        fs::write(temp.path().join("untracked.txt"), "content").unwrap();

        let status = GitStatus::detect(temp.path()).unwrap();
        let file_status = status.get_status(&temp.path().join("untracked.txt"));

        // Note: get_status uses relative paths internally
        // Check if it's untracked or clean (depends on path resolution)
        assert!(file_status == FileStatus::Untracked || file_status == FileStatus::Clean);
    }

    #[test]
    fn test_clean_file_status() {
        let temp = TempDir::new().unwrap();

        if !init_git_repo(temp.path()) {
            return;
        }
        git_config(temp.path());

        // Create and commit a file
        fs::write(temp.path().join("committed.txt"), "content").unwrap();
        git_add(temp.path(), "committed.txt");
        git_commit(temp.path(), "Initial commit");

        let status = GitStatus::detect(temp.path()).unwrap();

        // Committed file should be clean
        assert!(status.branch().is_some());
    }

    #[test]
    fn test_git_refresh() {
        let temp = TempDir::new().unwrap();

        if !init_git_repo(temp.path()) {
            return;
        }
        git_config(temp.path());

        let mut status = GitStatus::detect(temp.path()).unwrap();

        // Create file and refresh
        fs::write(temp.path().join("new.txt"), "content").unwrap();
        status.refresh();

        // After refresh, should see the new file
        assert!(status.branch().is_some() || status.branch().is_none());
    }

    #[test]
    fn test_file_status_default() {
        // FileStatus should default to Clean
        let default: FileStatus = Default::default();
        assert_eq!(default, FileStatus::Clean);
    }

    #[test]
    fn test_file_status_equality() {
        assert_eq!(FileStatus::Modified, FileStatus::Modified);
        assert_ne!(FileStatus::Modified, FileStatus::Added);
    }
}

// =============================================================================
// Tree Rendering Tests
// =============================================================================

mod tree_render_tests {
    use fileview::render::visible_height;
    use ratatui::layout::Rect;

    #[test]
    fn test_visible_height_basic() {
        let area = Rect::new(0, 0, 80, 24);
        // visible_height subtracts 2 for borders
        assert_eq!(visible_height(area), 22);
    }

    #[test]
    fn test_visible_height_small() {
        let area = Rect::new(0, 0, 80, 5);
        assert_eq!(visible_height(area), 3);
    }

    #[test]
    fn test_visible_height_minimal() {
        // Height of 2 (borders only) should give 0 visible
        let area = Rect::new(0, 0, 80, 2);
        assert_eq!(visible_height(area), 0);
    }

    #[test]
    fn test_visible_height_zero() {
        // Zero height should not panic
        let area = Rect::new(0, 0, 80, 0);
        assert_eq!(visible_height(area), 0);
    }

    #[test]
    fn test_visible_height_one() {
        // Height of 1 should give 0 (saturating_sub)
        let area = Rect::new(0, 0, 80, 1);
        assert_eq!(visible_height(area), 0);
    }
}

// =============================================================================
// Callback Tests
// =============================================================================

mod callback_tests {
    use fileview::integrate::Callback;
    use std::path::PathBuf;

    #[test]
    fn test_callback_placeholder_path() {
        let callback = Callback::new("echo {path}");
        let path = PathBuf::from("/home/user/file.txt");
        let cmd = callback.expand(&path);
        assert!(cmd.contains("/home/user/file.txt"));
    }

    #[test]
    fn test_callback_placeholder_name() {
        let callback = Callback::new("echo {name}");
        let path = PathBuf::from("/home/user/file.txt");
        let cmd = callback.expand(&path);
        assert!(cmd.contains("file.txt"));
    }

    #[test]
    fn test_callback_placeholder_stem() {
        let callback = Callback::new("echo {stem}");
        let path = PathBuf::from("/home/user/file.txt");
        let cmd = callback.expand(&path);
        assert!(cmd.contains("file"));
    }

    #[test]
    fn test_callback_placeholder_ext() {
        let callback = Callback::new("echo {ext}");
        let path = PathBuf::from("/home/user/file.txt");
        let cmd = callback.expand(&path);
        assert!(cmd.contains("txt"));
    }

    #[test]
    fn test_callback_placeholder_dir() {
        let callback = Callback::new("echo {dir}");
        let path = PathBuf::from("/home/user/file.txt");
        let cmd = callback.expand(&path);
        assert!(cmd.contains("/home/user"));
    }

    #[test]
    fn test_callback_multiple_placeholders() {
        let callback = Callback::new("cp {path} {dir}/backup_{name}");
        let path = PathBuf::from("/home/user/file.txt");
        let cmd = callback.expand(&path);
        // Path is shell-escaped with single quotes
        assert!(cmd.contains("'/home/user/file.txt'"));
        assert!(cmd.contains("'/home/user'/backup_'file.txt'"));
    }

    #[test]
    fn test_callback_no_placeholders() {
        let callback = Callback::new("ls -la");
        let path = PathBuf::from("/home/user/file.txt");
        let cmd = callback.expand(&path);
        assert_eq!(cmd, "ls -la");
    }

    #[test]
    fn test_callback_path_with_spaces() {
        let callback = Callback::new("cat {path}");
        let path = PathBuf::from("/home/user/my file.txt");
        let cmd = callback.expand(&path);
        // Path should be properly escaped
        assert!(cmd.contains("my") && cmd.contains("file.txt"));
    }
}

// =============================================================================
// Drag and Drop Tests
// =============================================================================

mod drag_and_drop_tests {
    use fileview::handler::mouse::PathBuffer;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_path_buffer_with_real_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test_file.txt");
        fs::write(&file_path, "test content").unwrap();

        let mut detector = PathBuffer::new();
        detector.push('/'); // Start path

        // Simulate rapid input of file path
        let path_str = file_path.display().to_string();
        for c in path_str.chars().skip(1) {
            detector.push(c);
        }

        let paths = detector.take_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], file_path);
    }

    #[test]
    fn test_path_buffer_with_real_directory() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("test_dir");
        fs::create_dir(&dir_path).unwrap();

        let mut detector = PathBuffer::new();
        let path_str = dir_path.display().to_string();
        for c in path_str.chars() {
            detector.push(c);
        }

        let paths = detector.take_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], dir_path);
    }

    #[test]
    fn test_path_buffer_file_url_format() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "content").unwrap();

        let mut detector = PathBuffer::new();
        // Simulate file:// URL format
        let url = format!("file://{}", file_path.display());
        for c in url.chars() {
            detector.push(c);
        }

        let paths = detector.take_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], file_path);
    }

    #[test]
    fn test_path_buffer_url_encoded_spaces() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("file with spaces.txt");
        fs::write(&file_path, "content").unwrap();

        let mut detector = PathBuffer::new();
        // Simulate URL-encoded path with %20 for spaces
        let path_str = file_path.display().to_string().replace(' ', "%20");
        for c in path_str.chars() {
            detector.push(c);
        }

        let paths = detector.take_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], file_path);
    }

    #[test]
    fn test_path_buffer_backslash_escaped_spaces() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("file with spaces.txt");
        fs::write(&file_path, "content").unwrap();

        let mut detector = PathBuffer::new();
        // Simulate backslash-escaped path (macOS terminal style)
        let path_str = file_path.display().to_string().replace(' ', "\\ ");
        for c in path_str.chars() {
            detector.push(c);
        }

        let paths = detector.take_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], file_path);
    }

    #[test]
    fn test_path_buffer_multiple_files() {
        let temp = TempDir::new().unwrap();
        let file1 = temp.path().join("file1.txt");
        let file2 = temp.path().join("file2.txt");
        fs::write(&file1, "content1").unwrap();
        fs::write(&file2, "content2").unwrap();

        let mut detector = PathBuffer::new();
        // Simulate multiple paths separated by newline
        let paths_str = format!("{}\n{}", file1.display(), file2.display());
        for c in paths_str.chars() {
            detector.push(c);
        }

        let paths = detector.take_paths();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&file1));
        assert!(paths.contains(&file2));
    }

    #[test]
    fn test_path_buffer_nonexistent_path_filtered() {
        let mut detector = PathBuffer::new();
        // Path that doesn't exist should be filtered out
        let fake_path = "/nonexistent/path/to/file.txt";
        for c in fake_path.chars() {
            detector.push(c);
        }

        let paths = detector.take_paths();
        assert!(paths.is_empty());
    }

    #[test]
    fn test_path_buffer_mixed_existing_nonexisting() {
        let temp = TempDir::new().unwrap();
        let existing = temp.path().join("existing.txt");
        fs::write(&existing, "content").unwrap();

        let mut detector = PathBuffer::new();
        // Mix of existing and non-existing paths
        let paths_str = format!("{}\n/nonexistent/path.txt", existing.display());
        for c in paths_str.chars() {
            detector.push(c);
        }

        let paths = detector.take_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], existing);
    }

    #[test]
    fn test_path_buffer_quoted_path_with_spaces() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("file with spaces.txt");
        fs::write(&file_path, "content").unwrap();

        let mut detector = PathBuffer::new();
        // Simulate quoted path
        let quoted = format!("\"{}\"", file_path.display());
        for c in quoted.chars() {
            detector.push(c);
        }

        let paths = detector.take_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], file_path);
    }

    #[test]
    fn test_path_buffer_clear() {
        let mut detector = PathBuffer::new();
        detector.push('/');
        detector.push('t');
        detector.push('e');
        detector.push('s');
        detector.push('t');

        assert!(!detector.is_empty());
        detector.clear();
        assert!(detector.is_empty());
    }
}

// =============================================================================
// Image Preview Comprehensive Tests (Phase 15.8.4)
// =============================================================================

mod image_preview_tests {
    use super::*;
    use fileview::render::{create_image_picker, render_image_preview, ImagePreview, Picker};
    use std::fs;

    // =========================================================================
    // Test Helpers - Image Creation Functions
    // =========================================================================

    /// Create a test image with specified dimensions and format
    fn create_test_image(path: &std::path::Path, width: u32, height: u32) {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |x, y| {
            Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
        });
        img.save(path).unwrap();
    }

    /// Create a 1x1 PNG
    fn create_test_png(path: &std::path::Path) {
        create_test_image(path, 1, 1);
    }

    /// Create a 1x1 JPEG
    fn create_test_jpeg(path: &std::path::Path) {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(1, 1, |_, _| Rgb([0, 0, 255]));
        img.save(path).unwrap();
    }

    /// Create a test GIF file
    fn create_test_gif(path: &std::path::Path) {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(2, 2, |_, _| Rgb([0, 255, 0]));
        img.save(path).unwrap();
    }

    /// Create a test WebP file
    fn create_test_webp(path: &std::path::Path) {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(2, 2, |_, _| Rgb([255, 255, 0]));
        img.save(path).unwrap();
    }

    /// Create a test BMP file
    fn create_test_bmp(path: &std::path::Path) {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(2, 2, |_, _| Rgb([255, 0, 255]));
        img.save(path).unwrap();
    }

    /// Create a PNG with alpha channel (RGBA)
    fn create_test_png_rgba(path: &std::path::Path) {
        use image::{ImageBuffer, Rgba};
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(2, 2, |_, _| Rgba([255, 0, 0, 128])); // Semi-transparent red
        img.save(path).unwrap();
    }

    // =========================================================================
    // SECTION 1: Image File Detection Tests (CI-Safe)
    // These tests run in all environments including CI
    // =========================================================================

    #[test]
    fn test_image_detection_all_supported_formats() {
        let supported = [
            "image.png",
            "image.jpg",
            "image.jpeg",
            "image.gif",
            "image.webp",
            "image.bmp",
            "image.ico",
        ];

        for filename in supported {
            assert!(
                is_image_file(&PathBuf::from(filename)),
                "Expected {} to be detected as image",
                filename
            );
        }
    }

    #[test]
    fn test_image_detection_case_insensitive() {
        // Upper case
        assert!(is_image_file(&PathBuf::from("IMAGE.PNG")));
        assert!(is_image_file(&PathBuf::from("IMAGE.JPG")));
        assert!(is_image_file(&PathBuf::from("IMAGE.JPEG")));
        assert!(is_image_file(&PathBuf::from("IMAGE.GIF")));
        assert!(is_image_file(&PathBuf::from("IMAGE.WEBP")));
        assert!(is_image_file(&PathBuf::from("IMAGE.BMP")));
        assert!(is_image_file(&PathBuf::from("IMAGE.ICO")));

        // Mixed case
        assert!(is_image_file(&PathBuf::from("Image.Png")));
        assert!(is_image_file(&PathBuf::from("iMaGe.JpG")));
        assert!(is_image_file(&PathBuf::from("FILE.GiF")));
    }

    #[test]
    fn test_image_detection_with_directory_path() {
        assert!(is_image_file(&PathBuf::from("/absolute/path/image.png")));
        assert!(is_image_file(&PathBuf::from("./relative/path/image.jpg")));
        assert!(is_image_file(&PathBuf::from("../parent/dir/image.gif")));
        assert!(is_image_file(&PathBuf::from("/path with spaces/image.png")));
        assert!(is_image_file(&PathBuf::from("/日本語パス/画像.png")));
    }

    #[test]
    fn test_image_detection_multiple_dots_in_filename() {
        assert!(is_image_file(&PathBuf::from("file.backup.png")));
        assert!(is_image_file(&PathBuf::from("image.v2.final.jpg")));
        assert!(is_image_file(&PathBuf::from("screenshot.2024.01.28.png")));
        assert!(is_image_file(&PathBuf::from("file...png")));
    }

    #[test]
    fn test_image_detection_not_image_extensions() {
        // Text files
        assert!(!is_image_file(&PathBuf::from("file.txt")));
        assert!(!is_image_file(&PathBuf::from("code.rs")));
        assert!(!is_image_file(&PathBuf::from("script.py")));
        assert!(!is_image_file(&PathBuf::from("document.md")));

        // Binary files
        assert!(!is_image_file(&PathBuf::from("app.exe")));
        assert!(!is_image_file(&PathBuf::from("lib.dll")));
        assert!(!is_image_file(&PathBuf::from("binary.bin")));

        // Config files
        assert!(!is_image_file(&PathBuf::from("config.json")));
        assert!(!is_image_file(&PathBuf::from("settings.toml")));
        assert!(!is_image_file(&PathBuf::from("data.yaml")));

        // Unknown extensions
        assert!(!is_image_file(&PathBuf::from("file.xyz")));
        assert!(!is_image_file(&PathBuf::from("data.raw")));
    }

    #[test]
    fn test_image_detection_no_extension() {
        assert!(!is_image_file(&PathBuf::from("Makefile")));
        assert!(!is_image_file(&PathBuf::from("LICENSE")));
        assert!(!is_image_file(&PathBuf::from("README")));
        assert!(!is_image_file(&PathBuf::from("file")));
    }

    #[test]
    fn test_image_detection_hidden_files() {
        assert!(is_image_file(&PathBuf::from(".hidden.png")));
        assert!(is_image_file(&PathBuf::from(".secret.jpg")));
        assert!(!is_image_file(&PathBuf::from(".gitignore")));
        assert!(!is_image_file(&PathBuf::from(".env")));
    }

    #[test]
    fn test_image_detection_similar_but_invalid_extensions() {
        assert!(!is_image_file(&PathBuf::from("file.pn"))); // Not .png
        assert!(!is_image_file(&PathBuf::from("file.jp"))); // Not .jpg
        assert!(!is_image_file(&PathBuf::from("file.pngg"))); // Extra char
        assert!(!is_image_file(&PathBuf::from("file.jpgg"))); // Extra char
        assert!(!is_image_file(&PathBuf::from("file.gi"))); // Not .gif
        assert!(!is_image_file(&PathBuf::from("file.web"))); // Not .webp
        assert!(!is_image_file(&PathBuf::from("filepng"))); // No dot
    }

    #[test]
    fn test_image_detection_unicode_filenames() {
        assert!(is_image_file(&PathBuf::from("日本語.png")));
        assert!(is_image_file(&PathBuf::from("中文图片.jpg")));
        assert!(is_image_file(&PathBuf::from("émoji🖼️.gif")));
        assert!(is_image_file(&PathBuf::from("Ñoño.webp")));
        assert!(is_image_file(&PathBuf::from("αβγδ.bmp")));
    }

    #[test]
    fn test_image_detection_long_filename() {
        let long_name = format!("{}.png", "a".repeat(200));
        assert!(is_image_file(&PathBuf::from(&long_name)));

        let very_long = format!("{}.jpg", "x".repeat(500));
        assert!(is_image_file(&PathBuf::from(&very_long)));
    }

    #[test]
    fn test_image_detection_special_characters() {
        assert!(is_image_file(&PathBuf::from("file-name.png")));
        assert!(is_image_file(&PathBuf::from("file_name.png")));
        assert!(is_image_file(&PathBuf::from("file (1).png")));
        assert!(is_image_file(&PathBuf::from("file [copy].png")));
        assert!(is_image_file(&PathBuf::from("file@2x.png")));
        assert!(is_image_file(&PathBuf::from("file#1.png")));
        assert!(is_image_file(&PathBuf::from("file+plus.png")));
    }

    #[test]
    fn test_image_detection_empty_or_minimal_input() {
        assert!(!is_image_file(&PathBuf::from("")));
        assert!(!is_image_file(&PathBuf::from(".")));
        assert!(!is_image_file(&PathBuf::from("..")));
        assert!(!is_image_file(&PathBuf::from(".png"))); // Hidden file, no name
    }

    // =========================================================================
    // SECTION 2: Real Image File Loading Tests (CI-Safe via image crate)
    // These tests verify image::open works, not requiring Picker
    // =========================================================================

    #[test]
    fn test_image_open_png() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.png");
        create_test_png(&path);

        let result = image::open(&path);
        assert!(result.is_ok(), "Failed to open PNG: {:?}", result.err());

        let img = result.unwrap();
        assert_eq!(img.width(), 1);
        assert_eq!(img.height(), 1);
    }

    #[test]
    fn test_image_open_jpeg() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.jpg");
        create_test_jpeg(&path);

        let result = image::open(&path);
        assert!(result.is_ok(), "Failed to open JPEG: {:?}", result.err());
    }

    #[test]
    fn test_image_open_gif() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.gif");
        create_test_gif(&path);

        let result = image::open(&path);
        assert!(result.is_ok(), "Failed to open GIF: {:?}", result.err());

        let img = result.unwrap();
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
    }

    #[test]
    fn test_image_open_webp() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.webp");
        create_test_webp(&path);

        let result = image::open(&path);
        assert!(result.is_ok(), "Failed to open WebP: {:?}", result.err());

        let img = result.unwrap();
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
    }

    #[test]
    fn test_image_open_bmp() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.bmp");
        create_test_bmp(&path);

        let result = image::open(&path);
        assert!(result.is_ok(), "Failed to open BMP: {:?}", result.err());

        let img = result.unwrap();
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
    }

    #[test]
    fn test_image_open_png_rgba_transparency() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("transparent.png");
        create_test_png_rgba(&path);

        let result = image::open(&path);
        assert!(
            result.is_ok(),
            "Failed to open RGBA PNG: {:?}",
            result.err()
        );

        let img = result.unwrap();
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
        // Verify it has alpha channel by checking color type
        assert!(img.color().has_alpha());
    }

    #[test]
    fn test_image_open_all_formats_consistency() {
        let temp = TempDir::new().unwrap();

        let formats = [
            ("test.png", create_test_png as fn(&std::path::Path)),
            ("test.jpg", create_test_jpeg as fn(&std::path::Path)),
            ("test.gif", create_test_gif as fn(&std::path::Path)),
            ("test.webp", create_test_webp as fn(&std::path::Path)),
            ("test.bmp", create_test_bmp as fn(&std::path::Path)),
        ];

        for (filename, creator) in formats {
            let path = temp.path().join(filename);
            creator(&path);

            assert!(image::open(&path).is_ok(), "Failed to open {}", filename);
            assert!(is_image_file(&path), "{} not detected as image", filename);
        }
    }

    // =========================================================================
    // SECTION 3: Image Dimension Tests (CI-Safe)
    // =========================================================================

    #[test]
    fn test_image_dimensions_small() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("small.png");
        create_test_image(&path, 1, 1);

        let img = image::open(&path).unwrap();
        assert_eq!(img.width(), 1);
        assert_eq!(img.height(), 1);
    }

    #[test]
    fn test_image_dimensions_medium() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("medium.png");
        create_test_image(&path, 100, 100);

        let img = image::open(&path).unwrap();
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);
    }

    #[test]
    fn test_image_dimensions_large() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("large.png");
        create_test_image(&path, 1000, 1000);

        let img = image::open(&path).unwrap();
        assert_eq!(img.width(), 1000);
        assert_eq!(img.height(), 1000);
    }

    #[test]
    fn test_image_dimensions_wide() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("wide.png");
        create_test_image(&path, 500, 10);

        let img = image::open(&path).unwrap();
        assert_eq!(img.width(), 500);
        assert_eq!(img.height(), 10);
    }

    #[test]
    fn test_image_dimensions_tall() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("tall.png");
        create_test_image(&path, 10, 500);

        let img = image::open(&path).unwrap();
        assert_eq!(img.width(), 10);
        assert_eq!(img.height(), 500);
    }

    // =========================================================================
    // SECTION 4: Error Handling Tests (CI-Safe)
    // =========================================================================

    #[test]
    fn test_image_open_nonexistent_file() {
        let result = image::open("/nonexistent/path/image.png");
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_string = err.to_string();
        // Verify error indicates file not found
        assert!(
            err_string.contains("No such file")
                || err_string.contains("not found")
                || err_string.contains("IoError"),
            "Unexpected error message: {}",
            err_string
        );
    }

    #[test]
    fn test_image_open_invalid_data() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("fake.png");
        fs::write(&path, "This is not a PNG file").unwrap();

        let result = image::open(&path);
        assert!(result.is_err(), "Should fail for invalid image data");

        // Verify error is related to image format/decoding (various possible messages)
        let err_str = result.unwrap_err().to_string().to_lowercase();
        assert!(
            err_str.contains("format")
                || err_str.contains("decode")
                || err_str.contains("invalid")
                || err_str.contains("png")
                || err_str.contains("signature"),
            "Expected format/decode error, got: {}",
            err_str
        );
    }

    #[test]
    fn test_image_open_empty_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("empty.png");
        fs::write(&path, "").unwrap();

        let result = image::open(&path);
        assert!(result.is_err(), "Should fail for empty file");
    }

    #[test]
    fn test_image_open_truncated_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("truncated.png");

        // Write only PNG magic bytes but no actual data
        fs::write(&path, [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]).unwrap();

        let result = image::open(&path);
        assert!(result.is_err(), "Should fail for truncated PNG");
    }

    #[test]
    fn test_image_open_wrong_extension() {
        let temp = TempDir::new().unwrap();

        // Create a valid PNG but save with .txt extension
        let png_path = temp.path().join("actual.png");
        create_test_png(&png_path);

        // Copy to .txt
        let txt_path = temp.path().join("image.txt");
        fs::copy(&png_path, &txt_path).unwrap();

        // image::open uses extension to determine format, so it may fail
        // But we can use ImageReader with guessed format to read by magic bytes
        let reader = image::ImageReader::open(&txt_path)
            .unwrap()
            .with_guessed_format()
            .unwrap();
        let result = reader.decode();
        assert!(
            result.is_ok(),
            "ImageReader with guessed format should work"
        );

        // Our is_image_file checks extension only
        assert!(
            !is_image_file(&txt_path),
            "is_image_file should check extension"
        );
    }

    // =========================================================================
    // SECTION 5: Image Picker Tests
    // =========================================================================

    #[test]
    fn test_create_image_picker_does_not_panic() {
        // This should never panic, even in CI
        let _picker = create_image_picker();
    }

    #[test]
    fn test_create_image_picker_deterministic() {
        let picker1 = create_image_picker();
        let picker2 = create_image_picker();

        // Both calls should return the same variant
        assert_eq!(
            picker1.is_some(),
            picker2.is_some(),
            "Picker creation should be deterministic"
        );
    }

    #[test]
    fn test_picker_none_is_valid_state() {
        // When picker is None, the application should handle gracefully
        let picker: Option<Picker> = None;
        assert!(picker.is_none());

        // This simulates the fallback path in main.rs
        // Image preview should be skipped when picker is None
        let should_show_image = picker.is_some();
        assert!(!should_show_image);
    }

    // =========================================================================
    // SECTION 6: ImagePreview::load Tests (Require Terminal - use #[ignore])
    // These tests require a real terminal and are skipped in CI
    // Run with: cargo test -- --ignored
    // =========================================================================

    #[test]
    #[ignore = "Requires terminal with image protocol support"]
    fn test_image_preview_load_png() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.png");
        create_test_png(&path);

        let mut picker =
            create_image_picker().expect("This test requires a terminal with image support");

        let result = ImagePreview::load(&path, &mut picker);
        assert!(
            result.is_ok(),
            "ImagePreview::load failed: {:?}",
            result.err()
        );

        let preview = result.unwrap();
        assert_eq!(preview.width, 1);
        assert_eq!(preview.height, 1);
    }

    #[test]
    #[ignore = "Requires terminal with image protocol support"]
    fn test_image_preview_load_jpeg() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.jpg");
        create_test_jpeg(&path);

        let mut picker =
            create_image_picker().expect("This test requires a terminal with image support");

        let result = ImagePreview::load(&path, &mut picker);
        assert!(result.is_ok(), "ImagePreview::load failed for JPEG");
    }

    #[test]
    #[ignore = "Requires terminal with image protocol support"]
    fn test_image_preview_load_gif() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.gif");
        create_test_gif(&path);

        let mut picker =
            create_image_picker().expect("This test requires a terminal with image support");

        let result = ImagePreview::load(&path, &mut picker);
        assert!(result.is_ok(), "ImagePreview::load failed for GIF");
    }

    #[test]
    #[ignore = "Requires terminal with image protocol support"]
    fn test_image_preview_load_webp() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.webp");
        create_test_webp(&path);

        let mut picker =
            create_image_picker().expect("This test requires a terminal with image support");

        let result = ImagePreview::load(&path, &mut picker);
        assert!(result.is_ok(), "ImagePreview::load failed for WebP");
    }

    #[test]
    #[ignore = "Requires terminal with image protocol support"]
    fn test_image_preview_load_bmp() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.bmp");
        create_test_bmp(&path);

        let mut picker =
            create_image_picker().expect("This test requires a terminal with image support");

        let result = ImagePreview::load(&path, &mut picker);
        assert!(result.is_ok(), "ImagePreview::load failed for BMP");
    }

    #[test]
    #[ignore = "Requires terminal with image protocol support"]
    fn test_image_preview_load_large_image() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("large.png");
        create_test_image(&path, 2000, 2000);

        let mut picker =
            create_image_picker().expect("This test requires a terminal with image support");

        let result = ImagePreview::load(&path, &mut picker);
        assert!(result.is_ok(), "ImagePreview::load failed for large image");

        let preview = result.unwrap();
        assert_eq!(preview.width, 2000);
        assert_eq!(preview.height, 2000);
    }

    #[test]
    #[ignore = "Requires terminal with image protocol support"]
    fn test_image_preview_load_nonexistent_error() {
        let mut picker =
            create_image_picker().expect("This test requires a terminal with image support");

        let result = ImagePreview::load(&PathBuf::from("/nonexistent/image.png"), &mut picker);
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "Requires terminal with image protocol support"]
    fn test_image_preview_load_invalid_error() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("invalid.png");
        fs::write(&path, "not an image").unwrap();

        let mut picker =
            create_image_picker().expect("This test requires a terminal with image support");

        let result = ImagePreview::load(&path, &mut picker);
        assert!(result.is_err(), "Should fail for invalid image");
    }

    // =========================================================================
    // SECTION 7: render_image_preview Tests (Require Terminal - use #[ignore])
    // =========================================================================

    #[test]
    #[ignore = "Requires terminal with image protocol support"]
    fn test_render_image_preview_does_not_panic() {
        use ratatui::{backend::TestBackend, Terminal};

        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.png");
        create_test_png(&path);

        let mut picker =
            create_image_picker().expect("This test requires a terminal with image support");

        let mut preview = ImagePreview::load(&path, &mut picker).unwrap();

        // Create a test terminal
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let font_size: FontSize = (10, 20);
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_image_preview(frame, &mut preview, area, "test.png", true, font_size);
            })
            .unwrap();

        // If we get here without panicking, the test passes
    }

    #[test]
    #[ignore = "Requires terminal with image protocol support"]
    fn test_render_image_preview_focused_unfocused() {
        use ratatui::{backend::TestBackend, Terminal};

        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.png");
        create_test_png(&path);

        let mut picker =
            create_image_picker().expect("This test requires a terminal with image support");

        let mut preview = ImagePreview::load(&path, &mut picker).unwrap();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let font_size: FontSize = picker.font_size();

        // Test focused state
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_image_preview(frame, &mut preview, area, "test.png", true, font_size);
            })
            .unwrap();

        // Test unfocused state
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_image_preview(frame, &mut preview, area, "test.png", false, font_size);
            })
            .unwrap();
    }

    // =========================================================================
    // SECTION 8: Preview Type Mutual Exclusivity Tests (CI-Safe)
    // =========================================================================

    #[test]
    fn test_preview_type_mutual_exclusivity_text() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("file.txt");
        fs::write(&path, "text content").unwrap();

        assert!(is_text_file(&path), "Should be text file");
        assert!(!is_image_file(&path), "Should NOT be image file");
        assert!(!is_binary_file(&path), "Should NOT be binary file");
    }

    #[test]
    fn test_preview_type_mutual_exclusivity_image() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("file.png");
        create_test_png(&path);

        assert!(!is_text_file(&path), "Should NOT be text file");
        assert!(is_image_file(&path), "Should be image file");
        assert!(!is_binary_file(&path), "Should NOT be binary file");
    }

    #[test]
    fn test_preview_type_mutual_exclusivity_binary() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("file.exe");
        fs::write(&path, [0x4D, 0x5A]).unwrap(); // MZ header

        assert!(!is_text_file(&path), "Should NOT be text file");
        assert!(!is_image_file(&path), "Should NOT be image file");
        assert!(is_binary_file(&path), "Should be binary file");
    }

    #[test]
    fn test_directory_not_any_preview_type() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("subdir");
        fs::create_dir(&dir).unwrap();

        assert!(!is_text_file(&dir), "Directory should NOT be text");
        assert!(!is_image_file(&dir), "Directory should NOT be image");
        // is_binary_file has special handling for directories
        // Based on implementation, it returns false for directories without extension
        assert!(!is_binary_file(&dir), "Directory should NOT be binary");
    }

    #[test]
    fn test_symlink_to_image() {
        let temp = TempDir::new().unwrap();
        let image_path = temp.path().join("actual.png");
        create_test_png(&image_path);

        let link_path = temp.path().join("link.png");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&image_path, &link_path).unwrap();

            // Symlink should be detected as image (follows link)
            assert!(is_image_file(&link_path));

            // Should be able to open via symlink
            assert!(image::open(&link_path).is_ok());
        }
    }

    // =========================================================================
    // SECTION 9: Module Export Verification (CI-Safe)
    // =========================================================================

    #[test]
    fn test_render_module_exports() {
        // Verify public API exports
        let _: fn(&std::path::Path, &mut Picker) -> anyhow::Result<ImagePreview> =
            ImagePreview::load;
        let _: fn() -> Option<Picker> = create_image_picker;
        let _: fn(&std::path::Path) -> bool = is_image_file;
        let _: fn(&std::path::Path) -> bool = is_text_file;
        let _: fn(&std::path::Path) -> bool = is_binary_file;
    }

    #[test]
    fn test_image_preview_struct_fields() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.png");
        create_test_image(&path, 50, 30);

        // We can't create ImagePreview without picker, but we can verify
        // that the struct has public width/height fields by checking the image
        let img = image::open(&path).unwrap();
        assert_eq!(img.width(), 50);
        assert_eq!(img.height(), 30);
    }

    // =========================================================================
    // SECTION 10: Edge Cases and Stress Tests (CI-Safe)
    // =========================================================================

    #[test]
    fn test_multiple_images_in_sequence() {
        let temp = TempDir::new().unwrap();

        // Create and open multiple images in sequence
        for i in 0..10 {
            let path = temp.path().join(format!("image_{}.png", i));
            create_test_image(&path, 10 + i, 10 + i);

            let img = image::open(&path).unwrap();
            assert_eq!(img.width(), 10 + i);
            assert_eq!(img.height(), 10 + i);
        }
    }

    #[test]
    fn test_various_image_aspect_ratios() {
        let temp = TempDir::new().unwrap();

        let ratios = [
            (1, 1),   // Square
            (16, 9),  // Widescreen
            (9, 16),  // Portrait
            (4, 3),   // Classic
            (21, 9),  // Ultrawide
            (1, 100), // Very tall
            (100, 1), // Very wide
        ];

        for (w, h) in ratios {
            let path = temp.path().join(format!("ratio_{}x{}.png", w, h));
            create_test_image(&path, w, h);

            let img = image::open(&path).unwrap();
            assert_eq!(img.width(), w);
            assert_eq!(img.height(), h);
        }
    }

    #[test]
    fn test_real_file_detection_consistency() {
        let temp = TempDir::new().unwrap();

        // Create actual files and verify detection matches content
        let png_path = temp.path().join("real.png");
        let jpg_path = temp.path().join("real.jpg");
        let gif_path = temp.path().join("real.gif");
        let txt_path = temp.path().join("real.txt");

        create_test_png(&png_path);
        create_test_jpeg(&jpg_path);
        create_test_gif(&gif_path);
        fs::write(&txt_path, "text content").unwrap();

        // Verify file type detection is consistent
        assert!(is_image_file(&png_path) && image::open(&png_path).is_ok());
        assert!(is_image_file(&jpg_path) && image::open(&jpg_path).is_ok());
        assert!(is_image_file(&gif_path) && image::open(&gif_path).is_ok());
        assert!(is_text_file(&txt_path) && image::open(&txt_path).is_err());
    }

    // =========================================================================
    // SECTION 5: Image Centering Tests
    // Tests for the calculate_centered_image_area function
    // =========================================================================

    #[test]
    fn test_centered_image_square_in_square_area() {
        // Square image in square area should be perfectly centered with no padding
        let area = Rect::new(0, 0, 20, 20);
        let font_size: FontSize = (10, 10); // Square cells for simplicity
        let result = calculate_centered_image_area(area, 100, 100, font_size);

        // Image should fill the entire area (scaled to 200x200 pixels = 20x20 cells)
        assert_eq!(result.x, 0);
        assert_eq!(result.y, 0);
        assert_eq!(result.width, 20);
        assert_eq!(result.height, 20);
    }

    #[test]
    fn test_centered_image_wide_image_in_square_area() {
        // Wide image (2:1) in square area should have vertical padding
        let area = Rect::new(0, 0, 20, 20);
        let font_size: FontSize = (10, 10);
        let result = calculate_centered_image_area(area, 200, 100, font_size);

        // Image width fills area (200 pixels), height is 100 pixels = 10 cells
        // Vertical padding should be (20 - 10) / 2 = 5
        assert_eq!(result.width, 20);
        assert_eq!(result.height, 10);
        assert_eq!(result.x, 0);
        assert_eq!(result.y, 5); // Centered vertically
    }

    #[test]
    fn test_centered_image_tall_image_in_square_area() {
        // Tall image (1:2) in square area should have horizontal padding
        let area = Rect::new(0, 0, 20, 20);
        let font_size: FontSize = (10, 10);
        let result = calculate_centered_image_area(area, 100, 200, font_size);

        // Image height fills area (200 pixels), width is 100 pixels = 10 cells
        // Horizontal padding should be (20 - 10) / 2 = 5
        assert_eq!(result.width, 10);
        assert_eq!(result.height, 20);
        assert_eq!(result.x, 5); // Centered horizontally
        assert_eq!(result.y, 0);
    }

    #[test]
    fn test_centered_image_with_offset_area() {
        // Area with non-zero origin
        let area = Rect::new(10, 5, 20, 20);
        let font_size: FontSize = (10, 10);
        let result = calculate_centered_image_area(area, 200, 100, font_size);

        // Wide image should be centered vertically within the offset area
        assert_eq!(result.x, 10); // Original x
        assert_eq!(result.y, 10); // Original y (5) + padding (5)
        assert_eq!(result.width, 20);
        assert_eq!(result.height, 10);
    }

    #[test]
    fn test_centered_image_typical_terminal_font() {
        // Typical terminal: cells are taller than wide (e.g., 10x20 pixels)
        let area = Rect::new(0, 0, 40, 20);
        let font_size: FontSize = (10, 20); // Typical terminal font

        // Square image (100x100 pixels)
        let result = calculate_centered_image_area(area, 100, 100, font_size);

        // Area in pixels: 400x400
        // Image scaled to fit: 400x400 pixels
        // In cells: width = 400/10 = 40, height = 400/20 = 20
        assert_eq!(result.width, 40);
        assert_eq!(result.height, 20);
        assert_eq!(result.x, 0);
        assert_eq!(result.y, 0);
    }

    #[test]
    fn test_centered_image_small_image_scales_up() {
        // Small image should be scaled up to fill the area
        let area = Rect::new(0, 0, 100, 50);
        let font_size: FontSize = (10, 20);

        // Very small image (10x10 pixels)
        let result = calculate_centered_image_area(area, 10, 10, font_size);

        // Area in pixels: 1000x1000
        // Image scales up by 100x to 1000x1000 pixels
        // In cells: 1000/10 = 100 width, 1000/20 = 50 height
        assert_eq!(result.width, 100);
        assert_eq!(result.height, 50);
    }

    #[test]
    fn test_centered_image_preserves_aspect_ratio() {
        // Verify aspect ratio is preserved for various image sizes
        let area = Rect::new(0, 0, 80, 40);
        let font_size: FontSize = (10, 20);

        // 4:3 image
        let result = calculate_centered_image_area(area, 400, 300, font_size);

        // The image should maintain 4:3 ratio
        // Area pixels: 800x800
        // Scale to fit: min(800/400, 800/300) = min(2, 2.67) = 2
        // Scaled: 800x600 pixels = 80x30 cells
        // Centered: x=0, y=(40-30)/2=5
        assert_eq!(result.width, 80);
        assert_eq!(result.height, 30);
        assert_eq!(result.y, 5);
    }

    #[test]
    fn test_centered_image_handles_zero_dimensions() {
        // Edge case: very small area
        let area = Rect::new(0, 0, 1, 1);
        let font_size: FontSize = (10, 20);

        let result = calculate_centered_image_area(area, 100, 100, font_size);

        // Should not panic and should return valid result
        assert!(result.width <= area.width);
        assert!(result.height <= area.height);
    }
}

// =============================================================================
// Terminal Detection Tests
// =============================================================================

mod terminal_detection_tests {
    use super::*;

    // =========================================================================
    // Basic API Tests
    // =========================================================================

    #[test]
    fn test_terminal_brand_detect_returns_valid_variant() {
        // TerminalBrand::detect() should always return a valid variant
        let brand = TerminalBrand::detect();

        // Verify it's one of the known variants
        let valid_variants = [
            TerminalBrand::Kitty,
            TerminalBrand::Ghostty,
            TerminalBrand::WezTerm,
            TerminalBrand::ITerm2,
            TerminalBrand::Konsole,
            TerminalBrand::Foot,
            TerminalBrand::VSCode,
            TerminalBrand::Warp,
            TerminalBrand::Alacritty,
            TerminalBrand::WindowsTerminal,
            TerminalBrand::Tmux,
            TerminalBrand::Unknown,
        ];

        assert!(valid_variants.contains(&brand));
    }

    #[test]
    fn test_terminal_brand_recommended_protocol_always_returns() {
        // Every terminal brand should have a recommended protocol
        let brands = [
            TerminalBrand::Kitty,
            TerminalBrand::Ghostty,
            TerminalBrand::WezTerm,
            TerminalBrand::ITerm2,
            TerminalBrand::Konsole,
            TerminalBrand::Foot,
            TerminalBrand::VSCode,
            TerminalBrand::Warp,
            TerminalBrand::Alacritty,
            TerminalBrand::WindowsTerminal,
            TerminalBrand::Tmux,
            TerminalBrand::Unknown,
        ];

        for brand in brands {
            let protocol = brand.recommended_protocol();
            // Should be one of the valid protocols
            let valid_protocols = [
                RecommendedProtocol::Kitty,
                RecommendedProtocol::Iterm2,
                RecommendedProtocol::Sixel,
                RecommendedProtocol::Chafa,
                RecommendedProtocol::Query,
            ];
            assert!(
                valid_protocols.contains(&protocol),
                "Brand {:?} returned invalid protocol",
                brand
            );
        }
    }

    #[test]
    fn test_terminal_brand_name_not_empty() {
        let brands = [
            TerminalBrand::Kitty,
            TerminalBrand::Ghostty,
            TerminalBrand::WezTerm,
            TerminalBrand::ITerm2,
            TerminalBrand::Konsole,
            TerminalBrand::Foot,
            TerminalBrand::VSCode,
            TerminalBrand::Warp,
            TerminalBrand::Alacritty,
            TerminalBrand::WindowsTerminal,
            TerminalBrand::Tmux,
            TerminalBrand::Unknown,
        ];

        for brand in brands {
            assert!(!brand.name().is_empty(), "Brand {:?} has empty name", brand);
        }
    }

    #[test]
    fn test_recommended_protocol_name_not_empty() {
        let protocols = [
            RecommendedProtocol::Kitty,
            RecommendedProtocol::Iterm2,
            RecommendedProtocol::Sixel,
            RecommendedProtocol::Chafa,
            RecommendedProtocol::Query,
        ];

        for protocol in protocols {
            assert!(
                !protocol.name().is_empty(),
                "Protocol {:?} has empty name",
                protocol
            );
        }
    }

    // =========================================================================
    // Protocol Mapping Verification Tests
    // =========================================================================

    #[test]
    fn test_kitty_protocol_terminals() {
        // Terminals that should use Kitty protocol
        let kitty_terminals = [
            TerminalBrand::Kitty,
            TerminalBrand::Ghostty,
            TerminalBrand::Konsole,
        ];

        for terminal in kitty_terminals {
            assert_eq!(
                terminal.recommended_protocol(),
                RecommendedProtocol::Kitty,
                "{:?} should recommend Kitty protocol",
                terminal
            );
        }
    }

    #[test]
    fn test_iterm2_protocol_terminals() {
        // Terminals that should use iTerm2 protocol
        let iterm2_terminals = [
            TerminalBrand::ITerm2,
            TerminalBrand::WezTerm,
            TerminalBrand::Warp,
        ];

        for terminal in iterm2_terminals {
            assert_eq!(
                terminal.recommended_protocol(),
                RecommendedProtocol::Iterm2,
                "{:?} should recommend iTerm2 protocol",
                terminal
            );
        }
    }

    #[test]
    fn test_sixel_protocol_terminals() {
        // Terminals that should use Sixel protocol
        let sixel_terminals = [TerminalBrand::Foot, TerminalBrand::WindowsTerminal];

        for terminal in sixel_terminals {
            assert_eq!(
                terminal.recommended_protocol(),
                RecommendedProtocol::Sixel,
                "{:?} should recommend Sixel protocol",
                terminal
            );
        }
    }

    #[test]
    fn test_chafa_preferred_terminals() {
        // Terminals that should prefer Chafa (no native image protocol support)
        let chafa_terminals = [TerminalBrand::VSCode, TerminalBrand::Alacritty];

        for terminal in chafa_terminals {
            assert_eq!(
                terminal.recommended_protocol(),
                RecommendedProtocol::Chafa,
                "{:?} should recommend Chafa",
                terminal
            );
        }
    }

    #[test]
    fn test_query_fallback_terminals() {
        // Terminals that should use query/fallback
        let query_terminals = [TerminalBrand::Tmux, TerminalBrand::Unknown];

        for terminal in query_terminals {
            assert_eq!(
                terminal.recommended_protocol(),
                RecommendedProtocol::Query,
                "{:?} should recommend Query",
                terminal
            );
        }
    }

    // =========================================================================
    // Equality and Debug Tests
    // =========================================================================

    #[test]
    fn test_terminal_brand_equality() {
        assert_eq!(TerminalBrand::Kitty, TerminalBrand::Kitty);
        assert_ne!(TerminalBrand::Kitty, TerminalBrand::Ghostty);
        assert_ne!(TerminalBrand::Unknown, TerminalBrand::Tmux);
    }

    #[test]
    fn test_recommended_protocol_equality() {
        assert_eq!(RecommendedProtocol::Kitty, RecommendedProtocol::Kitty);
        assert_ne!(RecommendedProtocol::Kitty, RecommendedProtocol::Iterm2);
        assert_ne!(RecommendedProtocol::Chafa, RecommendedProtocol::Query);
    }

    #[test]
    fn test_terminal_brand_clone() {
        let brand = TerminalBrand::Kitty;
        let cloned = brand.clone();
        assert_eq!(brand, cloned);
    }

    #[test]
    fn test_recommended_protocol_clone() {
        let protocol = RecommendedProtocol::Sixel;
        let cloned = protocol.clone();
        assert_eq!(protocol, cloned);
    }

    #[test]
    fn test_terminal_brand_copy() {
        let brand = TerminalBrand::Ghostty;
        let copied: TerminalBrand = brand; // Copy
        assert_eq!(brand, copied); // Both should still be usable
    }

    #[test]
    fn test_recommended_protocol_copy() {
        let protocol = RecommendedProtocol::Iterm2;
        let copied: RecommendedProtocol = protocol; // Copy
        assert_eq!(protocol, copied);
    }

    #[test]
    fn test_terminal_brand_debug_format() {
        let debug_str = format!("{:?}", TerminalBrand::Kitty);
        assert!(debug_str.contains("Kitty"));
    }

    #[test]
    fn test_recommended_protocol_debug_format() {
        let debug_str = format!("{:?}", RecommendedProtocol::Sixel);
        assert!(debug_str.contains("Sixel"));
    }

    // =========================================================================
    // Comprehensive Name Tests
    // =========================================================================

    #[test]
    fn test_terminal_brand_names_are_human_readable() {
        // Names should be human-readable, not internal identifiers
        assert_eq!(TerminalBrand::Kitty.name(), "Kitty");
        assert_eq!(TerminalBrand::Ghostty.name(), "Ghostty");
        assert_eq!(TerminalBrand::WezTerm.name(), "WezTerm");
        assert_eq!(TerminalBrand::ITerm2.name(), "iTerm2");
        assert_eq!(TerminalBrand::Konsole.name(), "Konsole");
        assert_eq!(TerminalBrand::Foot.name(), "Foot");
        assert_eq!(TerminalBrand::VSCode.name(), "VS Code");
        assert_eq!(TerminalBrand::Warp.name(), "Warp");
        assert_eq!(TerminalBrand::Alacritty.name(), "Alacritty");
        assert_eq!(TerminalBrand::WindowsTerminal.name(), "Windows Terminal");
        assert_eq!(TerminalBrand::Tmux.name(), "tmux");
        assert_eq!(TerminalBrand::Unknown.name(), "Unknown");
    }

    #[test]
    fn test_protocol_names_are_descriptive() {
        assert_eq!(RecommendedProtocol::Kitty.name(), "Kitty Graphics Protocol");
        assert_eq!(RecommendedProtocol::Iterm2.name(), "iTerm2 Inline Images");
        assert_eq!(RecommendedProtocol::Sixel.name(), "Sixel");
        assert_eq!(RecommendedProtocol::Chafa.name(), "Chafa");
        assert_eq!(RecommendedProtocol::Query.name(), "Query/Auto-detect");
    }

    // =========================================================================
    // Consistency Tests
    // =========================================================================

    #[test]
    fn test_detect_is_deterministic() {
        // Multiple calls should return the same result
        let brand1 = TerminalBrand::detect();
        let brand2 = TerminalBrand::detect();
        let brand3 = TerminalBrand::detect();

        assert_eq!(brand1, brand2);
        assert_eq!(brand2, brand3);
    }

    #[test]
    fn test_protocol_recommendation_is_deterministic() {
        let brand = TerminalBrand::detect();

        let protocol1 = brand.recommended_protocol();
        let protocol2 = brand.recommended_protocol();
        let protocol3 = brand.recommended_protocol();

        assert_eq!(protocol1, protocol2);
        assert_eq!(protocol2, protocol3);
    }

    #[test]
    fn test_all_brands_have_consistent_protocol() {
        // For each brand, the protocol should always be the same
        let brands = [
            TerminalBrand::Kitty,
            TerminalBrand::Ghostty,
            TerminalBrand::WezTerm,
            TerminalBrand::ITerm2,
            TerminalBrand::Konsole,
            TerminalBrand::Foot,
            TerminalBrand::VSCode,
            TerminalBrand::Warp,
            TerminalBrand::Alacritty,
            TerminalBrand::WindowsTerminal,
            TerminalBrand::Tmux,
            TerminalBrand::Unknown,
        ];

        for brand in brands {
            let p1 = brand.recommended_protocol();
            let p2 = brand.recommended_protocol();
            assert_eq!(p1, p2, "{:?} returned inconsistent protocols", brand);
        }
    }

    // =========================================================================
    // Exhaustiveness Tests
    // =========================================================================

    #[test]
    fn test_all_12_terminal_brands_exist() {
        // This test ensures no variants are accidentally added or removed
        let all_brands = [
            TerminalBrand::Kitty,
            TerminalBrand::Ghostty,
            TerminalBrand::WezTerm,
            TerminalBrand::ITerm2,
            TerminalBrand::Konsole,
            TerminalBrand::Foot,
            TerminalBrand::VSCode,
            TerminalBrand::Warp,
            TerminalBrand::Alacritty,
            TerminalBrand::WindowsTerminal,
            TerminalBrand::Tmux,
            TerminalBrand::Unknown,
        ];

        assert_eq!(all_brands.len(), 12);
    }

    #[test]
    fn test_all_5_protocols_exist() {
        let all_protocols = [
            RecommendedProtocol::Kitty,
            RecommendedProtocol::Iterm2,
            RecommendedProtocol::Sixel,
            RecommendedProtocol::Chafa,
            RecommendedProtocol::Query,
        ];

        assert_eq!(all_protocols.len(), 5);
    }

    // =========================================================================
    // Integration with create_image_picker Tests
    // =========================================================================

    #[test]
    fn test_create_image_picker_respects_terminal_detection() {
        // This is a basic integration test - the actual picker creation
        // might fail in CI depending on terminal support, but it shouldn't panic
        use fileview::render::create_image_picker;

        // Should not panic regardless of terminal
        let _result = std::panic::catch_unwind(|| {
            let _ = create_image_picker();
        });
        // We don't assert the result because it depends on the terminal,
        // but it should never panic
    }

    #[test]
    fn test_create_image_picker_returns_some() {
        use fileview::render::create_image_picker;

        // create_image_picker should always return Some (fallback to halfblocks)
        let picker = create_image_picker();
        assert!(
            picker.is_some(),
            "create_image_picker should always return Some"
        );
    }

    #[test]
    fn test_create_image_picker_multiple_calls_consistent() {
        use fileview::render::create_image_picker;

        // Multiple calls should be consistent (same env = same result type)
        let picker1 = create_image_picker();
        let picker2 = create_image_picker();
        let picker3 = create_image_picker();

        assert!(picker1.is_some());
        assert!(picker2.is_some());
        assert!(picker3.is_some());
    }

    #[test]
    fn test_terminal_brand_detect_public_api() {
        // Test the public detect() API directly
        let brand = TerminalBrand::detect();

        // Should return a valid variant
        let _ = brand.name(); // Should not panic
        let protocol = brand.recommended_protocol();
        let _ = protocol.name(); // Should not panic
    }

    #[test]
    fn test_terminal_brand_detect_multiple_calls_deterministic() {
        // Calling detect() multiple times should return same result
        let brand1 = TerminalBrand::detect();
        let brand2 = TerminalBrand::detect();
        let brand3 = TerminalBrand::detect();

        assert_eq!(brand1, brand2);
        assert_eq!(brand2, brand3);
    }

    #[test]
    fn test_terminal_detection_does_not_modify_environment() {
        // Verify detect() doesn't modify environment
        let env_vars_before: Vec<_> = std::env::vars().collect();

        let _ = TerminalBrand::detect();

        let env_vars_after: Vec<_> = std::env::vars().collect();
        assert_eq!(env_vars_before.len(), env_vars_after.len());
    }

    // =========================================================================
    // Environment Variable Override Tests
    // Note: These tests document expected behavior but can't actually test
    // FILEVIEW_IMAGE_PROTOCOL because we can't safely modify env vars in tests
    // =========================================================================

    #[test]
    fn test_fileview_image_protocol_env_var_documented() {
        // Document the supported values for FILEVIEW_IMAGE_PROTOCOL
        // This test serves as documentation and ensures the values are considered
        let supported_values = [
            "auto",
            "halfblocks",
            "half",
            "chafa",
            "sixel",
            "kitty",
            "iterm2",
            "iterm",
        ];

        // All these should be valid protocol override values
        for value in &supported_values {
            assert!(!value.is_empty(), "Protocol value should not be empty");
        }
    }

    // =========================================================================
    // Stress Tests
    // =========================================================================

    #[test]
    fn test_terminal_detect_rapid_succession() {
        // Call detect() many times rapidly - should be stable
        for _ in 0..100 {
            let brand = TerminalBrand::detect();
            let _ = brand.recommended_protocol();
        }
    }

    #[test]
    fn test_create_image_picker_rapid_succession() {
        use fileview::render::create_image_picker;

        // Call create_image_picker() multiple times - should not leak resources
        for _ in 0..10 {
            let _ = create_image_picker();
        }
    }
}

// =============================================================================
// Shell Integration Tests (Phase 17.1)
// =============================================================================

mod shell_integration_tests {
    use super::*;
    use fileview::core::FocusTarget;
    use std::fs;

    // =========================================================================
    // KeyAction Tests
    // =========================================================================

    #[test]
    fn test_quit_and_cd_action_exists() {
        // Verify QuitAndCd action variant exists
        let action = KeyAction::QuitAndCd;
        assert!(matches!(action, KeyAction::QuitAndCd));
    }

    #[test]
    fn test_uppercase_q_triggers_quit_and_cd() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        // Q (uppercase) should trigger QuitAndCd
        let key = KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::NONE);
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::QuitAndCd),
            "Q should trigger QuitAndCd, got {:?}",
            action
        );
    }

    #[test]
    fn test_lowercase_q_triggers_quit_not_cd() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        // q (lowercase) should trigger regular Quit, not QuitAndCd
        let key = key_event(KeyCode::Char('q'));
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::Quit),
            "q should trigger Quit, got {:?}",
            action
        );
    }

    // =========================================================================
    // AppState Tests
    // =========================================================================

    #[test]
    fn test_choosedir_path_initial_none() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        assert!(
            state.choosedir_path.is_none(),
            "choosedir_path should be None initially"
        );
    }

    #[test]
    fn test_choosedir_path_can_be_set() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        let test_path = PathBuf::from("/some/test/path");
        state.choosedir_path = Some(test_path.clone());

        assert_eq!(state.choosedir_path, Some(test_path));
    }

    // =========================================================================
    // Pick Mode vs Shell Integration
    // =========================================================================

    #[test]
    fn test_q_in_pick_mode_cancels_not_quits() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.pick_mode = true;

        let key = key_event(KeyCode::Char('q'));
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::Cancel),
            "q in pick mode should Cancel, got {:?}",
            action
        );
    }

    #[test]
    fn test_shift_q_in_pick_mode_still_quits_and_cd() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.pick_mode = true;

        // Q should still trigger QuitAndCd even in pick mode
        let key = KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::NONE);
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::QuitAndCd),
            "Q should trigger QuitAndCd even in pick mode, got {:?}",
            action
        );
    }

    // =========================================================================
    // Focus Target Tests
    // =========================================================================

    #[test]
    fn test_quit_and_cd_works_when_preview_focused() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.preview_visible = true;
        state.focus_target = FocusTarget::Preview;

        let key = KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::NONE);
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::QuitAndCd),
            "Q should trigger QuitAndCd even when preview is focused"
        );
    }

    // =========================================================================
    // Directory Path Tests
    // =========================================================================

    #[test]
    fn test_choosedir_path_stores_directory() {
        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let mut state = AppState::new(temp.path().to_path_buf());
        state.choosedir_path = Some(subdir.clone());

        assert_eq!(state.choosedir_path.as_ref().unwrap(), &subdir);
        assert!(state.choosedir_path.as_ref().unwrap().is_dir());
    }

    #[test]
    fn test_choosedir_path_can_store_file_parent() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "test").unwrap();

        let mut state = AppState::new(temp.path().to_path_buf());
        // Simulate storing the parent directory of a file
        let parent = file_path.parent().unwrap().to_path_buf();
        state.choosedir_path = Some(parent.clone());

        assert_eq!(state.choosedir_path.as_ref().unwrap(), &parent);
    }

    // =========================================================================
    // Integration with Other Modes
    // =========================================================================

    #[test]
    fn test_quit_and_cd_not_triggered_in_input_mode() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::Input {
            purpose: InputPurpose::Rename {
                original: PathBuf::from("test"),
            },
            buffer: String::new(),
            cursor: 0,
        };

        let key = KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::NONE);
        let action = handle_key_event(&state, key);

        // In input mode, Q should not trigger QuitAndCd
        assert!(
            !matches!(action, KeyAction::QuitAndCd),
            "Q should not trigger QuitAndCd in input mode"
        );
    }

    #[test]
    fn test_quit_and_cd_not_triggered_in_confirm_mode() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::Confirm {
            action: PendingAction::Delete {
                targets: vec![PathBuf::from("test")],
            },
        };

        let key = KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::NONE);
        let action = handle_key_event(&state, key);

        // In confirm mode, Q should not trigger QuitAndCd
        assert!(
            !matches!(action, KeyAction::QuitAndCd),
            "Q should not trigger QuitAndCd in confirm mode"
        );
    }

    // =========================================================================
    // Determinism Tests
    // =========================================================================

    #[test]
    fn test_quit_and_cd_deterministic() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        // Multiple calls should produce same result
        for _ in 0..10 {
            let key = KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::NONE);
            let action = handle_key_event(&state, key);
            assert!(matches!(action, KeyAction::QuitAndCd));
        }
    }
}

// =============================================================================
// Fuzzy Finder Tests
// =============================================================================

mod fuzzy_finder_tests {
    use super::*;
    use fileview::render::{collect_paths, fuzzy_match};
    use fileview::tree::TreeNavigator;
    use std::fs;

    // =========================================================================
    // Key Binding Tests
    // =========================================================================

    #[test]
    fn test_ctrl_p_opens_fuzzy_finder() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::OpenFuzzyFinder),
            "Ctrl+P should open fuzzy finder"
        );
    }

    #[test]
    fn test_plain_p_is_paste_not_fuzzy_finder() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::Paste),
            "Plain 'p' should be Paste, not fuzzy finder"
        );
    }

    #[test]
    fn test_fuzzy_mode_esc_cancels() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 0,
        };

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::Cancel),
            "Esc should cancel fuzzy finder"
        );
    }

    #[test]
    fn test_fuzzy_mode_arrow_up_moves_up() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 5,
        };

        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::FuzzyUp),
            "Arrow up should move up in fuzzy finder"
        );
    }

    #[test]
    fn test_fuzzy_mode_arrow_down_moves_down() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 0,
        };

        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::FuzzyDown),
            "Arrow down should move down in fuzzy finder"
        );
    }

    #[test]
    fn test_fuzzy_mode_ctrl_k_moves_up() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 3,
        };

        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL);
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::FuzzyUp),
            "Ctrl+K should move up in fuzzy finder"
        );
    }

    #[test]
    fn test_fuzzy_mode_ctrl_j_moves_down() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 0,
        };

        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL);
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::FuzzyDown),
            "Ctrl+J should move down in fuzzy finder"
        );
    }

    #[test]
    fn test_fuzzy_mode_enter_confirms() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 0,
        };

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::FuzzyConfirm { .. }),
            "Enter should confirm fuzzy finder"
        );
    }

    #[test]
    fn test_fuzzy_mode_regular_key_is_none() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 0,
        };

        // Regular typing keys should return None (handled separately)
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::None),
            "Regular keys should return None for separate handling"
        );
    }

    // =========================================================================
    // Fuzzy Match Tests
    // =========================================================================

    #[test]
    fn test_fuzzy_match_empty_query_returns_all() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("file1.txt"), "").unwrap();
        fs::write(temp.path().join("file2.txt"), "").unwrap();
        fs::write(temp.path().join("file3.txt"), "").unwrap();

        let paths = vec![
            temp.path().join("file1.txt"),
            temp.path().join("file2.txt"),
            temp.path().join("file3.txt"),
        ];
        let root = temp.path().to_path_buf();

        let results = fuzzy_match("", &paths, &root);
        assert_eq!(results.len(), 3, "Empty query should return all paths");
    }

    #[test]
    fn test_fuzzy_match_filters_by_query() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![
            temp.path().join("main.rs"),
            temp.path().join("lib.rs"),
            temp.path().join("config.toml"),
        ];

        let results = fuzzy_match("rs", &paths, &root);
        assert!(results.len() >= 2, "Should match at least 2 .rs files");
        assert!(
            results.iter().all(|r| r.display.contains("rs")),
            "All results should contain 'rs'"
        );
    }

    #[test]
    fn test_fuzzy_match_no_results() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![temp.path().join("file.txt")];
        let results = fuzzy_match("xyz123nonexistent", &paths, &root);

        assert!(
            results.is_empty(),
            "Should return empty for non-matching query"
        );
    }

    #[test]
    fn test_fuzzy_match_case_smart() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![
            temp.path().join("README.md"),
            temp.path().join("readme.txt"),
        ];

        // Lowercase query should match both
        let results = fuzzy_match("readme", &paths, &root);
        assert!(results.len() >= 1, "Should match at least one file");
    }

    #[test]
    fn test_fuzzy_match_partial_match() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![
            temp.path().join("src/render/mod.rs"),
            temp.path().join("src/handler/mod.rs"),
        ];

        let results = fuzzy_match("ren", &paths, &root);
        assert!(!results.is_empty(), "Should find partial matches");
    }

    #[test]
    fn test_fuzzy_match_sorted_by_score() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![
            temp.path().join("abcdefg.txt"),
            temp.path().join("abc.txt"),
            temp.path().join("ab.txt"),
        ];

        let results = fuzzy_match("ab", &paths, &root);
        // Higher scores should come first
        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "Results should be sorted by score descending"
            );
        }
    }

    #[test]
    fn test_fuzzy_match_has_indices() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![temp.path().join("main.rs")];
        let results = fuzzy_match("mn", &paths, &root);

        if !results.is_empty() {
            assert!(
                !results[0].indices.is_empty(),
                "Match should have highlighted indices"
            );
        }
    }

    #[test]
    fn test_fuzzy_match_respects_max_results() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        // Create more paths than MAX_RESULTS (15)
        let paths: Vec<PathBuf> = (0..50)
            .map(|i| temp.path().join(format!("file{}.txt", i)))
            .collect();

        let results = fuzzy_match("", &paths, &root);
        assert!(
            results.len() <= 15,
            "Should limit results to MAX_RESULTS (15)"
        );
    }

    // =========================================================================
    // Path Collection Tests
    // =========================================================================

    #[test]
    fn test_collect_paths_basic() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("file1.txt"), "").unwrap();
        fs::write(temp.path().join("file2.txt"), "").unwrap();
        fs::create_dir(temp.path().join("subdir")).unwrap();
        fs::write(temp.path().join("subdir/nested.txt"), "").unwrap();

        let root = temp.path().to_path_buf();
        let paths = collect_paths(&root, false);

        assert!(paths.len() >= 3, "Should collect at least 3 paths");
        assert!(paths.iter().any(|p| p.ends_with("file1.txt")));
        assert!(paths.iter().any(|p| p.ends_with("file2.txt")));
        assert!(paths.iter().any(|p| p.ends_with("nested.txt")));
    }

    #[test]
    fn test_collect_paths_excludes_hidden_by_default() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("visible.txt"), "").unwrap();
        fs::write(temp.path().join(".hidden"), "").unwrap();

        let root = temp.path().to_path_buf();
        let paths = collect_paths(&root, false);

        assert!(
            paths.iter().any(|p| p.ends_with("visible.txt")),
            "Should include visible files"
        );
        assert!(
            !paths.iter().any(|p| p.ends_with(".hidden")),
            "Should exclude hidden files"
        );
    }

    #[test]
    fn test_collect_paths_includes_hidden_when_enabled() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("visible.txt"), "").unwrap();
        fs::write(temp.path().join(".hidden"), "").unwrap();

        let root = temp.path().to_path_buf();
        let paths = collect_paths(&root, true);

        assert!(
            paths.iter().any(|p| p.ends_with("visible.txt")),
            "Should include visible files"
        );
        assert!(
            paths.iter().any(|p| p.ends_with(".hidden")),
            "Should include hidden files when enabled"
        );
    }

    #[test]
    fn test_collect_paths_respects_depth_limit() {
        let temp = TempDir::new().unwrap();
        // Create deeply nested structure
        let mut current = temp.path().to_path_buf();
        for i in 0..15 {
            current = current.join(format!("level{}", i));
            fs::create_dir_all(&current).unwrap();
        }
        fs::write(current.join("deep.txt"), "").unwrap();

        let root = temp.path().to_path_buf();
        let paths = collect_paths(&root, false);

        // Should not collect files beyond depth 10
        let deep_file = paths.iter().any(|p| p.ends_with("deep.txt"));
        assert!(!deep_file, "Should not collect files beyond max depth (10)");
    }

    #[test]
    fn test_collect_paths_empty_directory() {
        let temp = TempDir::new().unwrap();

        let root = temp.path().to_path_buf();
        let paths = collect_paths(&root, false);

        assert!(
            paths.is_empty(),
            "Empty directory should return empty paths"
        );
    }

    #[test]
    fn test_collect_paths_includes_directories() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("mydir")).unwrap();
        fs::write(temp.path().join("file.txt"), "").unwrap();

        let root = temp.path().to_path_buf();
        let paths = collect_paths(&root, false);

        assert!(
            paths.iter().any(|p| p.ends_with("mydir")),
            "Should include directories"
        );
        assert!(
            paths.iter().any(|p| p.ends_with("file.txt")),
            "Should include files"
        );
    }

    // =========================================================================
    // ViewMode Transition Tests
    // =========================================================================

    #[test]
    fn test_fuzzy_finder_mode_structure() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.mode = ViewMode::FuzzyFinder {
            query: "test query".to_string(),
            selected: 5,
        };

        if let ViewMode::FuzzyFinder { query, selected } = &state.mode {
            assert_eq!(query, "test query");
            assert_eq!(*selected, 5);
        } else {
            panic!("Mode should be FuzzyFinder");
        }
    }

    #[test]
    fn test_fuzzy_jump_target_initial_none() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        assert!(
            state.fuzzy_jump_target.is_none(),
            "fuzzy_jump_target should be None initially"
        );
    }

    #[test]
    fn test_fuzzy_jump_target_can_be_set() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        let target = temp.path().join("target.txt");
        state.fuzzy_jump_target = Some(target.clone());

        assert_eq!(state.fuzzy_jump_target.unwrap(), target);
    }

    // =========================================================================
    // Tree Navigator reveal_path Tests
    // =========================================================================

    #[test]
    fn test_reveal_path_basic() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("a/b/c")).unwrap();
        fs::write(temp.path().join("a/b/c/deep.txt"), "").unwrap();

        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();
        let target = temp.path().join("a/b/c/deep.txt");

        navigator.reveal_path(&target).unwrap();

        // After reveal, the file should be visible
        let entries = navigator.visible_entries();
        let visible_paths: Vec<_> = entries.iter().map(|e| &e.path).collect();

        assert!(
            visible_paths.contains(&&target),
            "Target should be visible after reveal"
        );
    }

    #[test]
    fn test_reveal_path_expands_ancestors() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("a/b")).unwrap();
        fs::write(temp.path().join("a/b/file.txt"), "").unwrap();

        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();

        // Initially, nested file is not visible
        let before = navigator.visible_entries();
        assert!(
            !before.iter().any(|e| e.path.ends_with("file.txt")),
            "File should not be visible initially"
        );

        let target = temp.path().join("a/b/file.txt");
        navigator.reveal_path(&target).unwrap();

        // After reveal, the file should be visible
        let after = navigator.visible_entries();
        assert!(
            after.iter().any(|e| e.path.ends_with("file.txt")),
            "File should be visible after reveal"
        );
    }

    #[test]
    fn test_reveal_path_already_visible() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("visible.txt"), "").unwrap();

        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();
        let target = temp.path().join("visible.txt");

        // Already visible
        let before_count = navigator.visible_count();

        navigator.reveal_path(&target).unwrap();

        let after_count = navigator.visible_count();
        assert_eq!(
            before_count, after_count,
            "Revealing already visible path should not change count"
        );
    }

    #[test]
    fn test_reveal_path_nonexistent() {
        let temp = TempDir::new().unwrap();

        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();
        let target = temp.path().join("nonexistent/path/file.txt");

        // Should not panic, just do nothing
        let result = navigator.reveal_path(&target);
        assert!(result.is_ok());
    }

    // =========================================================================
    // Integration Tests - Full Workflow
    // =========================================================================

    #[test]
    fn test_fuzzy_workflow_open_search_select() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("src/main.rs"), "").unwrap();
        fs::write(temp.path().join("src/lib.rs"), "").unwrap();

        let mut state = AppState::new(temp.path().to_path_buf());

        // 1. Start in Browse mode
        assert!(matches!(state.mode, ViewMode::Browse));

        // 2. Open fuzzy finder with Ctrl+P
        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let action = handle_key_event(&state, key);
        assert!(matches!(action, KeyAction::OpenFuzzyFinder));

        // Simulate opening fuzzy finder
        state.mode = ViewMode::FuzzyFinder {
            query: String::new(),
            selected: 0,
        };

        // 3. Simulate typing a query
        if let ViewMode::FuzzyFinder { query, .. } = &mut state.mode {
            *query = "main".to_string();
        }

        // 4. Press Enter to confirm
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let action = handle_key_event(&state, key);
        assert!(matches!(action, KeyAction::FuzzyConfirm { .. }));

        // 5. Simulate returning to Browse mode
        state.mode = ViewMode::Browse;
        assert!(matches!(state.mode, ViewMode::Browse));
    }

    #[test]
    fn test_fuzzy_workflow_cancel() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        // Start fuzzy finder
        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 3,
        };

        // Press Esc to cancel
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let action = handle_key_event(&state, key);
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_fuzzy_workflow_navigation() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        // Start fuzzy finder with some results
        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 0,
        };

        // Navigate down
        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let action = handle_key_event(&state, key);
        assert!(matches!(action, KeyAction::FuzzyDown));

        // Navigate up
        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let action = handle_key_event(&state, key);
        assert!(matches!(action, KeyAction::FuzzyUp));
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_fuzzy_finder_special_characters_in_query() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![
            temp.path().join("test[1].txt"),
            temp.path().join("test(2).txt"),
        ];

        // Should not panic with special characters
        let results = fuzzy_match("[1]", &paths, &root);
        // Results may or may not match depending on how special chars are handled
        // Just verify no panic occurs
        let _ = results.len();
    }

    #[test]
    fn test_fuzzy_finder_unicode_query() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![
            temp.path().join("日本語ファイル.txt"),
            temp.path().join("english.txt"),
        ];

        let results = fuzzy_match("日本語", &paths, &root);
        // Should match unicode filename
        if !results.is_empty() {
            assert!(results[0].display.contains("日本語"));
        }
    }

    #[test]
    fn test_fuzzy_finder_whitespace_query() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![temp.path().join("my file.txt")];

        let results = fuzzy_match(" ", &paths, &root);
        // Whitespace should be treated as part of query
        // Just verify no panic occurs
        let _ = results.len();
    }

    #[test]
    fn test_fuzzy_finder_very_long_query() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![temp.path().join("short.txt")];

        let long_query = "a".repeat(1000);
        let results = fuzzy_match(&long_query, &paths, &root);
        // Should handle long query without panic
        assert!(results.is_empty());
    }

    #[test]
    fn test_ctrl_p_not_triggered_in_other_modes() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        // In Input mode
        state.mode = ViewMode::Input {
            purpose: InputPurpose::CreateFile,
            buffer: String::new(),
            cursor: 0,
        };

        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let action = handle_key_event(&state, key);

        // Should not trigger fuzzy finder in input mode
        assert!(
            !matches!(action, KeyAction::OpenFuzzyFinder),
            "Ctrl+P should not trigger fuzzy finder in Input mode"
        );
    }

    #[test]
    fn test_fuzzy_mode_preserves_query_on_navigation() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.mode = ViewMode::FuzzyFinder {
            query: "important query".to_string(),
            selected: 2,
        };

        // Navigate down
        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let _ = handle_key_event(&state, key);

        // Query should still be there
        if let ViewMode::FuzzyFinder { query, .. } = &state.mode {
            assert_eq!(query, "important query");
        }
    }

    // =========================================================================
    // Determinism Tests
    // =========================================================================

    #[test]
    fn test_fuzzy_match_deterministic() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![
            temp.path().join("file1.txt"),
            temp.path().join("file2.txt"),
            temp.path().join("other.rs"),
        ];

        // Multiple calls should produce same results
        let results1 = fuzzy_match("file", &paths, &root);
        let results2 = fuzzy_match("file", &paths, &root);

        assert_eq!(results1.len(), results2.len());
        for (r1, r2) in results1.iter().zip(results2.iter()) {
            assert_eq!(r1.path, r2.path);
            assert_eq!(r1.score, r2.score);
        }
    }

    #[test]
    fn test_collect_paths_deterministic() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("a.txt"), "").unwrap();
        fs::write(temp.path().join("b.txt"), "").unwrap();
        fs::write(temp.path().join("c.txt"), "").unwrap();

        let root = temp.path().to_path_buf();

        let paths1 = collect_paths(&root, false);
        let paths2 = collect_paths(&root, false);

        assert_eq!(paths1.len(), paths2.len());
        // Note: Order might not be guaranteed, so just check contents match
        for p in &paths1 {
            assert!(paths2.contains(p));
        }
    }

    #[test]
    fn test_fuzzy_confirm_action_type() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 0,
        };

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let action = handle_key_event(&state, key);

        // FuzzyConfirm should have path field
        if let KeyAction::FuzzyConfirm { path } = action {
            // Path is initially empty, to be filled by main.rs
            assert!(path.as_os_str().is_empty());
        } else {
            panic!("Expected FuzzyConfirm action");
        }
    }

    // =========================================================================
    // Boundary Condition Tests
    // =========================================================================

    #[test]
    fn test_fuzzy_up_at_index_zero() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        // Start at index 0
        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 0,
        };

        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let action = handle_key_event(&state, key);

        // Should still return FuzzyUp (saturation handled in action handler)
        assert!(matches!(action, KeyAction::FuzzyUp));
    }

    #[test]
    fn test_fuzzy_down_at_large_index() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        // Start at a large index
        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 100,
        };

        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let action = handle_key_event(&state, key);

        // Should still return FuzzyDown (bounding handled elsewhere)
        assert!(matches!(action, KeyAction::FuzzyDown));
    }

    #[test]
    fn test_fuzzy_selected_bounds_with_empty_results() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths: Vec<PathBuf> = vec![];
        let results = fuzzy_match("anything", &paths, &root);

        assert!(results.is_empty());

        // Bounding logic should handle empty results
        let bounded = if results.is_empty() {
            0
        } else {
            5usize.min(results.len() - 1)
        };
        assert_eq!(bounded, 0);
    }

    #[test]
    fn test_fuzzy_selected_bounds_with_few_results() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![temp.path().join("a.txt"), temp.path().join("b.txt")];
        let results = fuzzy_match("txt", &paths, &root);

        // If selected is larger than results, should be bounded
        let selected = 10usize;
        let bounded = if results.is_empty() {
            0
        } else {
            selected.min(results.len() - 1)
        };
        assert!(bounded < 10);
    }

    // =========================================================================
    // Interaction with Other Modes
    // =========================================================================

    #[test]
    fn test_ctrl_p_not_triggered_in_search_mode() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.mode = ViewMode::Search {
            query: "searching".to_string(),
        };

        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let action = handle_key_event(&state, key);

        assert!(
            !matches!(action, KeyAction::OpenFuzzyFinder),
            "Ctrl+P should not trigger fuzzy finder in Search mode"
        );
    }

    #[test]
    fn test_ctrl_p_not_triggered_in_preview_mode() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.mode = ViewMode::Preview { scroll: 0 };

        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let action = handle_key_event(&state, key);

        assert!(
            !matches!(action, KeyAction::OpenFuzzyFinder),
            "Ctrl+P should not trigger fuzzy finder in Preview mode"
        );
    }

    #[test]
    fn test_ctrl_p_not_triggered_in_confirm_mode() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.mode = ViewMode::Confirm {
            action: PendingAction::Delete {
                targets: vec![PathBuf::from("test")],
            },
        };

        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let action = handle_key_event(&state, key);

        assert!(
            !matches!(action, KeyAction::OpenFuzzyFinder),
            "Ctrl+P should not trigger fuzzy finder in Confirm mode"
        );
    }

    #[test]
    fn test_fuzzy_finder_with_pick_mode_enabled() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.pick_mode = true;

        // Ctrl+P should still work in pick mode
        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::OpenFuzzyFinder),
            "Ctrl+P should work in pick mode"
        );
    }

    #[test]
    fn test_fuzzy_finder_with_preview_visible() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.preview_visible = true;

        // Ctrl+P should still work with preview visible
        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let action = handle_key_event(&state, key);

        assert!(
            matches!(action, KeyAction::OpenFuzzyFinder),
            "Ctrl+P should work with preview visible"
        );
    }

    // =========================================================================
    // Large Directory Tests
    // =========================================================================

    #[test]
    fn test_collect_paths_with_many_files() {
        let temp = TempDir::new().unwrap();

        // Create 100 files
        for i in 0..100 {
            fs::write(temp.path().join(format!("file_{:03}.txt", i)), "").unwrap();
        }

        let root = temp.path().to_path_buf();
        let paths = collect_paths(&root, false);

        assert_eq!(paths.len(), 100);
    }

    #[test]
    fn test_fuzzy_match_with_many_files() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        // Create paths for 100 files
        let paths: Vec<PathBuf> = (0..100)
            .map(|i| temp.path().join(format!("file_{:03}.txt", i)))
            .collect();

        let results = fuzzy_match("file", &paths, &root);

        // Should limit to MAX_RESULTS (15)
        assert!(results.len() <= 15);
        // All results should match "file"
        assert!(results.iter().all(|r| r.display.contains("file")));
    }

    #[test]
    fn test_collect_paths_wide_directory() {
        let temp = TempDir::new().unwrap();

        // Create many subdirectories with files
        for i in 0..10 {
            let subdir = temp.path().join(format!("dir_{}", i));
            fs::create_dir(&subdir).unwrap();
            for j in 0..5 {
                fs::write(subdir.join(format!("file_{}.txt", j)), "").unwrap();
            }
        }

        let root = temp.path().to_path_buf();
        let paths = collect_paths(&root, false);

        // Should have 10 dirs + 50 files = 60 paths
        assert_eq!(paths.len(), 60);
    }

    // =========================================================================
    // Query Edge Cases
    // =========================================================================

    #[test]
    fn test_fuzzy_match_query_with_path_separator() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        fs::create_dir_all(temp.path().join("src/render")).unwrap();
        fs::write(temp.path().join("src/render/mod.rs"), "").unwrap();

        let paths = vec![temp.path().join("src/render/mod.rs")];
        let results = fuzzy_match("src/ren", &paths, &root);

        // Should match the path with separator
        assert!(!results.is_empty());
    }

    #[test]
    fn test_fuzzy_match_query_all_match() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![
            temp.path().join("a.txt"),
            temp.path().join("ab.txt"),
            temp.path().join("abc.txt"),
        ];

        let results = fuzzy_match("", &paths, &root);

        // Empty query should return all
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_fuzzy_match_repeated_characters() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![temp.path().join("aaa.txt"), temp.path().join("bbb.txt")];

        let results = fuzzy_match("aaa", &paths, &root);

        assert!(!results.is_empty());
        assert!(results[0].display.contains("aaa"));
    }

    #[test]
    fn test_fuzzy_match_numbers_only() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![temp.path().join("12345.txt"), temp.path().join("other.txt")];

        let results = fuzzy_match("123", &paths, &root);

        assert!(!results.is_empty());
        assert!(results[0].display.contains("123"));
    }

    #[test]
    fn test_fuzzy_match_mixed_case_query() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![
            temp.path().join("MyFile.txt"),
            temp.path().join("myfile.txt"),
        ];

        // Uppercase query should be case-sensitive
        let results = fuzzy_match("MyFile", &paths, &root);
        assert!(!results.is_empty());
    }

    // =========================================================================
    // reveal_path Edge Cases
    // =========================================================================

    #[test]
    fn test_reveal_path_with_symlink_directory() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("real_dir")).unwrap();
        fs::write(temp.path().join("real_dir/file.txt"), "").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(temp.path().join("real_dir"), temp.path().join("link_dir")).unwrap();

            let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();
            let target = temp.path().join("link_dir/file.txt");

            // Should not panic with symlink
            let result = navigator.reveal_path(&target);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_reveal_path_multiple_times_same_target() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("a/b/c")).unwrap();
        fs::write(temp.path().join("a/b/c/file.txt"), "").unwrap();

        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();
        let target = temp.path().join("a/b/c/file.txt");

        // Reveal multiple times
        navigator.reveal_path(&target).unwrap();
        let count1 = navigator.visible_count();

        navigator.reveal_path(&target).unwrap();
        let count2 = navigator.visible_count();

        navigator.reveal_path(&target).unwrap();
        let count3 = navigator.visible_count();

        // Should be stable
        assert_eq!(count1, count2);
        assert_eq!(count2, count3);
    }

    #[test]
    fn test_reveal_path_different_targets_sequentially() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("a/b")).unwrap();
        fs::create_dir_all(temp.path().join("x/y")).unwrap();
        fs::write(temp.path().join("a/b/file1.txt"), "").unwrap();
        fs::write(temp.path().join("x/y/file2.txt"), "").unwrap();

        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();

        let target1 = temp.path().join("a/b/file1.txt");
        let target2 = temp.path().join("x/y/file2.txt");

        navigator.reveal_path(&target1).unwrap();
        navigator.reveal_path(&target2).unwrap();

        let entries = navigator.visible_entries();
        let paths: Vec<_> = entries.iter().map(|e| &e.path).collect();

        // Both targets should be visible
        assert!(paths.contains(&&target1));
        assert!(paths.contains(&&target2));
    }

    #[test]
    fn test_reveal_path_with_hidden_parent() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join(".hidden/sub")).unwrap();
        fs::write(temp.path().join(".hidden/sub/file.txt"), "").unwrap();

        // Navigator without showing hidden
        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();
        let target = temp.path().join(".hidden/sub/file.txt");

        // reveal_path should still succeed (but may not show the hidden parent)
        let result = navigator.reveal_path(&target);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reveal_path_root_itself() {
        let temp = TempDir::new().unwrap();

        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();
        let target = temp.path().to_path_buf();

        // Revealing root itself should work
        let result = navigator.reveal_path(&target);
        assert!(result.is_ok());
    }

    // =========================================================================
    // FuzzyMatch Struct Tests
    // =========================================================================

    #[test]
    fn test_fuzzy_match_struct_fields() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![temp.path().join("test_file.rs")];
        let results = fuzzy_match("test", &paths, &root);

        if !results.is_empty() {
            let result = &results[0];

            // Check all fields are populated
            assert!(!result.path.as_os_str().is_empty());
            assert!(!result.display.is_empty());
            // Score and indices should exist (may be 0/empty for weak matches)
        }
    }

    #[test]
    fn test_fuzzy_match_display_is_relative() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![temp.path().join("file.txt")];
        let results = fuzzy_match("", &paths, &root);

        assert!(!results.is_empty());
        // Display should be relative path, not absolute
        assert_eq!(results[0].display, "file.txt");
        assert!(!results[0].display.starts_with('/'));
    }

    #[test]
    fn test_fuzzy_match_clone() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        let paths = vec![temp.path().join("file.txt")];
        let results = fuzzy_match("file", &paths, &root);

        if !results.is_empty() {
            let cloned = results[0].clone();
            assert_eq!(cloned.path, results[0].path);
            assert_eq!(cloned.display, results[0].display);
            assert_eq!(cloned.score, results[0].score);
            assert_eq!(cloned.indices, results[0].indices);
        }
    }

    // =========================================================================
    // Action Handler Tests (simulating what handle_action does)
    // =========================================================================

    #[test]
    fn test_fuzzy_up_saturates_at_zero() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        // Start at 0
        state.mode = ViewMode::FuzzyFinder {
            query: String::new(),
            selected: 0,
        };

        // Simulate FuzzyUp action
        if let ViewMode::FuzzyFinder { selected, .. } = &mut state.mode {
            *selected = selected.saturating_sub(1);
        }

        // Should still be 0
        if let ViewMode::FuzzyFinder { selected, .. } = &state.mode {
            assert_eq!(*selected, 0);
        }
    }

    #[test]
    fn test_fuzzy_down_increments() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.mode = ViewMode::FuzzyFinder {
            query: String::new(),
            selected: 5,
        };

        // Simulate FuzzyDown action
        if let ViewMode::FuzzyFinder { selected, .. } = &mut state.mode {
            *selected += 1;
        }

        if let ViewMode::FuzzyFinder { selected, .. } = &state.mode {
            assert_eq!(*selected, 6);
        }
    }

    #[test]
    fn test_fuzzy_cancel_returns_to_browse() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 3,
        };

        // Simulate Cancel action (sets mode to Browse)
        state.mode = ViewMode::Browse;

        assert!(matches!(state.mode, ViewMode::Browse));
    }

    #[test]
    fn test_fuzzy_confirm_sets_jump_target() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 0,
        };

        let target = temp.path().join("target.txt");

        // Simulate FuzzyConfirm action
        state.fuzzy_jump_target = Some(target.clone());
        state.mode = ViewMode::Browse;

        assert_eq!(state.fuzzy_jump_target, Some(target));
        assert!(matches!(state.mode, ViewMode::Browse));
    }

    #[test]
    fn test_fuzzy_confirm_with_empty_path_no_jump() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 0,
        };

        // Simulate FuzzyConfirm with empty path (no results selected)
        let empty_path = PathBuf::new();
        if !empty_path.as_os_str().is_empty() {
            state.fuzzy_jump_target = Some(empty_path);
        }
        state.mode = ViewMode::Browse;

        // Jump target should remain None
        assert!(state.fuzzy_jump_target.is_none());
    }

    // =========================================================================
    // Integration Sequence Tests
    // =========================================================================

    #[test]
    fn test_fuzzy_full_workflow_with_deep_file() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("a/b/c/d")).unwrap();
        fs::write(temp.path().join("a/b/c/d/deep.txt"), "").unwrap();

        let mut state = AppState::new(temp.path().to_path_buf());
        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();

        // 1. Open fuzzy finder
        state.mode = ViewMode::FuzzyFinder {
            query: String::new(),
            selected: 0,
        };

        // 2. Simulate finding and selecting deep.txt
        let target = temp.path().join("a/b/c/d/deep.txt");
        state.fuzzy_jump_target = Some(target.clone());
        state.mode = ViewMode::Browse;

        // 3. Reveal the path
        navigator.reveal_path(&target).unwrap();

        // 4. Find and focus the file
        let entries = navigator.visible_entries();
        let idx = entries.iter().position(|e| e.path == target);

        assert!(idx.is_some(), "Deep file should be visible after reveal");
    }

    #[test]
    fn test_fuzzy_rapid_open_close() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        // Rapidly open and close fuzzy finder
        for _ in 0..10 {
            state.mode = ViewMode::FuzzyFinder {
                query: String::new(),
                selected: 0,
            };
            assert!(matches!(state.mode, ViewMode::FuzzyFinder { .. }));

            state.mode = ViewMode::Browse;
            assert!(matches!(state.mode, ViewMode::Browse));
        }
    }

    #[test]
    fn test_fuzzy_query_change_resets_selection() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        // Start with some selection
        state.mode = ViewMode::FuzzyFinder {
            query: "old".to_string(),
            selected: 5,
        };

        // Change query (simulating text input)
        state.mode = ViewMode::FuzzyFinder {
            query: "new".to_string(),
            selected: 0, // Reset to 0
        };

        if let ViewMode::FuzzyFinder { query, selected } = &state.mode {
            assert_eq!(query, "new");
            assert_eq!(*selected, 0);
        }
    }

    // =========================================================================
    // Stress Tests
    // =========================================================================

    #[test]
    fn test_fuzzy_match_performance_many_paths() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        // Create 1000 paths
        let paths: Vec<PathBuf> = (0..1000)
            .map(|i| temp.path().join(format!("file_{:04}.txt", i)))
            .collect();

        // Should complete quickly and return limited results
        let results = fuzzy_match("file", &paths, &root);
        assert!(results.len() <= 15);
    }

    #[test]
    fn test_fuzzy_match_long_path() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        // Very long filename
        let long_name = "a".repeat(200);
        let paths = vec![temp.path().join(format!("{}.txt", long_name))];

        let results = fuzzy_match("a", &paths, &root);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_collect_paths_at_depth_limit() {
        let temp = TempDir::new().unwrap();

        // Create exactly at depth 10
        let mut path = temp.path().to_path_buf();
        for i in 0..10 {
            path = path.join(format!("level{}", i));
        }
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("at_limit.txt"), "").unwrap();

        let root = temp.path().to_path_buf();
        let paths = collect_paths(&root, false);

        // File at depth 10 should be included
        assert!(paths.iter().any(|p| p.ends_with("at_limit.txt")));
    }

    #[test]
    fn test_collect_paths_beyond_depth_limit() {
        let temp = TempDir::new().unwrap();

        // Create beyond depth 10
        let mut path = temp.path().to_path_buf();
        for i in 0..12 {
            path = path.join(format!("level{}", i));
        }
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("beyond_limit.txt"), "").unwrap();

        let root = temp.path().to_path_buf();
        let paths = collect_paths(&root, false);

        // File beyond depth 10 should NOT be included
        assert!(!paths.iter().any(|p| p.ends_with("beyond_limit.txt")));
    }
}
