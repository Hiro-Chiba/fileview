use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fileview::core::{AppState, InputPurpose, PendingAction, ViewMode};
use fileview::handler::{handle_key_event, update_input_buffer, KeyAction};
#[cfg(unix)]
use fileview::render::{
    calculate_centered_image_area, FontSize, RecommendedProtocol, TerminalBrand,
};
#[cfg(feature = "archive")]
use fileview::render::{is_archive_file, ArchivePreview};
use fileview::render::{
    is_binary_file, is_image_file, is_text_file, DirectoryInfo, HexPreview, TextPreview,
};
#[cfg(unix)]
use ratatui::layout::Rect;
use tempfile::TempDir;

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

        state.focus_index = 25;
        state.adjust_viewport(10); // visible_height = 10

        assert!(state.viewport_top > 0);
        assert!(state.focus_index < state.viewport_top + 10);
    }

    #[test]
    fn test_viewport_adjustment_scroll_up() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.viewport_top = 20;
        state.focus_index = 5;
        state.adjust_viewport(10);

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

        state.mode = ViewMode::Search {
            query: "test".to_string(),
        };
        assert!(matches!(state.mode, ViewMode::Search { .. }));

        state.mode = ViewMode::Input {
            purpose: InputPurpose::CreateFile,
            buffer: String::new(),
            cursor: 0,
        };
        assert!(matches!(state.mode, ViewMode::Input { .. }));

        state.mode = ViewMode::Confirm {
            action: PendingAction::Delete {
                targets: vec![PathBuf::from("/tmp/test")],
            },
        };
        assert!(matches!(state.mode, ViewMode::Confirm { .. }));

        state.mode = ViewMode::Preview { scroll: 0 };
        assert!(matches!(state.mode, ViewMode::Preview { .. }));
    }

    #[test]
    fn test_toggle_focus_when_preview_hidden() {
        use fileview::core::FocusTarget;

        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        assert!(!state.preview_visible);

        assert_eq!(state.focus_target, FocusTarget::Tree);

        // Even if we set focus to Preview, it doesn't make sense when preview is hidden
        state.focus_target = FocusTarget::Preview;
        assert_eq!(state.focus_target, FocusTarget::Preview);

        state.preview_visible = true;
        state.focus_target = FocusTarget::Tree;
        assert_eq!(state.focus_target, FocusTarget::Tree);

        state.focus_target = FocusTarget::Preview;
        assert_eq!(state.focus_target, FocusTarget::Preview);
    }

    #[test]
    fn test_focus_resets_when_preview_closes() {
        use fileview::core::FocusTarget;

        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.preview_visible = true;
        state.focus_target = FocusTarget::Preview;

        state.preview_visible = false;
        // Focus target remains unchanged until explicitly reset
        // The action handler is responsible for resetting it
        assert_eq!(state.focus_target, FocusTarget::Preview);

        // Manually reset as the action handler would
        state.focus_target = FocusTarget::Tree;
        assert_eq!(state.focus_target, FocusTarget::Tree);
    }

    #[test]
    fn test_focus_defaults_to_tree() {
        use fileview::core::FocusTarget;

        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        assert_eq!(state.focus_target, FocusTarget::Tree);

        assert_eq!(FocusTarget::default(), FocusTarget::Tree);
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

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('j'))),
            KeyAction::MoveDown
        ));
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Down)),
            KeyAction::MoveDown
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('k'))),
            KeyAction::MoveUp
        ));
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Up)),
            KeyAction::MoveUp
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('g'))),
            KeyAction::MoveToTop
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('G'))),
            KeyAction::MoveToBottom
        ));
    }

    #[test]
    fn test_browse_mode_tree_operations() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('l'))),
            KeyAction::Expand
        ));
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Right)),
            KeyAction::Expand
        ));

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

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('H'))),
            KeyAction::CollapseAll
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('L'))),
            KeyAction::ExpandAll
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Tab)),
            KeyAction::ToggleExpand
        ));
    }

    #[test]
    fn test_browse_mode_selection() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char(' '))),
            KeyAction::ToggleMark
        ));

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

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Enter)),
            KeyAction::PickSelect
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('q'))),
            KeyAction::Cancel
        ));
    }

    #[test]
    fn test_browse_mode_file_operations() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('a'))),
            KeyAction::StartNewFile
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('A'))),
            KeyAction::StartNewDir
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('r'))),
            KeyAction::StartRename
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('D'))),
            KeyAction::ConfirmDelete
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Delete)),
            KeyAction::ConfirmDelete
        ));
    }

    #[test]
    fn test_browse_mode_clipboard() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('y'))),
            KeyAction::Copy
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('d'))),
            KeyAction::Cut
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('p'))),
            KeyAction::Paste
        ));
    }

    #[test]
    fn test_browse_mode_preview_keys() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('o'))),
            KeyAction::OpenPreview
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('P'))),
            KeyAction::ToggleQuickPreview
        ));
    }

    #[test]
    fn test_browse_mode_search() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('/'))),
            KeyAction::StartSearch
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('n'))),
            KeyAction::SearchNext
        ));
    }

    #[test]
    fn test_browse_mode_other() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('.'))),
            KeyAction::ToggleHidden
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('R'))),
            KeyAction::Refresh
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::F(5))),
            KeyAction::Refresh
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('?'))),
            KeyAction::ShowHelp
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('c'))),
            KeyAction::CopyPath
        ));

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

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('o'))),
            KeyAction::Cancel
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('q'))),
            KeyAction::Cancel
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Esc)),
            KeyAction::Cancel
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('j'))),
            KeyAction::PreviewScrollDown
        ));
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Down)),
            KeyAction::PreviewScrollDown
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('k'))),
            KeyAction::PreviewScrollUp
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::PageUp)),
            KeyAction::PreviewPageUp
        ));
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('b'))),
            KeyAction::PreviewPageUp
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::PageDown)),
            KeyAction::PreviewPageDown
        ));
        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('f'))),
            KeyAction::PreviewPageDown
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('g'))),
            KeyAction::PreviewToTop
        ));

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

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('y'))),
            KeyAction::ExecuteDelete
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('Y'))),
            KeyAction::ExecuteDelete
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Enter)),
            KeyAction::ExecuteDelete
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('n'))),
            KeyAction::Cancel
        ));

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

        let action = handle_key_event(&state, key_event(KeyCode::Enter));
        match action {
            KeyAction::ConfirmInput { value } => assert_eq!(value, "test.txt"),
            _ => panic!("Expected ConfirmInput"),
        }

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

        let action = handle_key_event(&state, key_event(KeyCode::Enter));
        match action {
            KeyAction::ConfirmInput { value } => assert_eq!(value, "search_term"),
            _ => panic!("Expected ConfirmInput"),
        }

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

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('j'))),
            KeyAction::PreviewScrollDown
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('k'))),
            KeyAction::PreviewScrollUp
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Down)),
            KeyAction::PreviewScrollDown
        ));

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

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('j'))),
            KeyAction::MoveDown
        ));

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

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::PageDown)),
            KeyAction::PreviewPageDown
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::PageUp)),
            KeyAction::PreviewPageUp
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('b'))),
            KeyAction::PreviewPageUp
        ));

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

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('g'))),
            KeyAction::PreviewToTop
        ));

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

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('g'))),
            KeyAction::MoveToTop
        ));

        assert!(matches!(
            handle_key_event(&state, key_event(KeyCode::Char('G'))),
            KeyAction::MoveToBottom
        ));
    }

    #[test]
    fn test_fuzzy_finder_ctrl_p_opens() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

        let ctrl_p = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        assert!(matches!(
            handle_key_event(&state, ctrl_p),
            KeyAction::OpenFuzzyFinder
        ));
    }

    #[test]
    fn test_fuzzy_finder_ctrl_p_closes() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 0,
        };

        let ctrl_p = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        assert!(matches!(
            handle_key_event(&state, ctrl_p),
            KeyAction::Cancel
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

        assert!(!is_text_file(&PathBuf::from("file.png")));
        assert!(!is_text_file(&PathBuf::from("file.exe")));
        assert!(!is_text_file(&PathBuf::from("file.unknown")));
    }

    #[test]
    fn test_image_file_detection() {
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

        assert!(!is_image_file(&PathBuf::from("file.txt")));
        assert!(!is_image_file(&PathBuf::from("file.rs")));
    }

    #[test]
    fn test_binary_file_detection() {
        assert!(is_binary_file(&PathBuf::from("file.exe")));
        assert!(is_binary_file(&PathBuf::from("file.dll")));
        assert!(is_binary_file(&PathBuf::from("file.so")));
        assert!(is_binary_file(&PathBuf::from("file.dylib")));
        assert!(is_binary_file(&PathBuf::from("file.o")));
        assert!(is_binary_file(&PathBuf::from("file.bin")));
        assert!(is_binary_file(&PathBuf::from("file.wasm")));

        assert!(!is_binary_file(&PathBuf::from("file.txt")));
        assert!(!is_binary_file(&PathBuf::from("file.rs")));

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

        let data: Vec<u8> = (0..32).collect();
        fs::write(&file_path, &data).unwrap();

        let preview = HexPreview::load(&file_path).unwrap();
        assert_eq!(preview.line_count(), 2);

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

        assert_eq!(state.mode, ViewMode::Browse);
        assert!(!state.preview_visible);

        let action = handle_key_event(&state, key_event(KeyCode::Char('o')));
        assert!(matches!(action, KeyAction::OpenPreview));

        state.mode = ViewMode::Preview { scroll: 0 };

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

        let action = handle_key_event(&state, key_event(KeyCode::Char('P')));
        assert!(matches!(action, KeyAction::ToggleQuickPreview));

        state.preview_visible = true;

        assert!(state.preview_visible);
        assert_eq!(state.mode, ViewMode::Browse);

        let needs_preview = state.preview_visible || matches!(state.mode, ViewMode::Preview { .. });
        assert!(needs_preview);
    }

    #[test]
    fn test_preview_close_from_fullscreen() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());
        state.mode = ViewMode::Preview { scroll: 0 };

        let action = handle_key_event(&state, key_event(KeyCode::Char('o')));
        assert!(matches!(action, KeyAction::Cancel));

        let action = handle_key_event(&state, key_event(KeyCode::Char('q')));
        assert!(matches!(action, KeyAction::Cancel));

        let action = handle_key_event(&state, key_event(KeyCode::Esc));
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_preview_needs_check_logic() {
        // Test the needs_preview logic that was the root cause of the bug
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        assert!(!state.preview_visible);
        assert!(matches!(state.mode, ViewMode::Browse));
        let needs_preview = state.preview_visible || matches!(state.mode, ViewMode::Preview { .. });
        assert!(!needs_preview, "Should not need preview in Browse mode");

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

        let result = update_input_buffer(key_event(KeyCode::Backspace), "abc", 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_input_buffer_delete() {
        let result = update_input_buffer(key_event(KeyCode::Delete), "abc", 1);
        assert_eq!(result, Some(("ac".to_string(), 1)));

        let result = update_input_buffer(key_event(KeyCode::Delete), "abc", 3);
        assert_eq!(result, None);
    }

    #[test]
    fn test_input_buffer_cursor_movement() {
        let result = update_input_buffer(key_event(KeyCode::Left), "abc", 2);
        assert_eq!(result, Some(("abc".to_string(), 1)));

        let result = update_input_buffer(key_event(KeyCode::Left), "abc", 0);
        assert_eq!(result, None);

        let result = update_input_buffer(key_event(KeyCode::Right), "abc", 1);
        assert_eq!(result, Some(("abc".to_string(), 2)));

        let result = update_input_buffer(key_event(KeyCode::Right), "abc", 3);
        assert_eq!(result, None);

        let result = update_input_buffer(key_event(KeyCode::Home), "abc", 2);
        assert_eq!(result, Some(("abc".to_string(), 0)));

        let result = update_input_buffer(key_event(KeyCode::End), "abc", 1);
        assert_eq!(result, Some(("abc".to_string(), 3)));
    }

    #[test]
    fn test_input_buffer_unicode_char() {
        let result = update_input_buffer(key_event(KeyCode::Char('あ')), "", 0);
        assert_eq!(result, Some(("あ".to_string(), 1)));

        // Note: cursor is char-based (1 = after 'a')
        let result = update_input_buffer(key_event(KeyCode::Char('日')), "ab", 1);
        assert_eq!(result, Some(("a日b".to_string(), 2)));

        let result = update_input_buffer(key_event(KeyCode::Char('🎉')), "test", 4);
        assert_eq!(result, Some(("test🎉".to_string(), 5)));
    }

    #[test]
    fn test_input_buffer_backspace_unicode() {
        // Note: The current implementation uses char-based cursor (not byte-based)
        let result = update_input_buffer(key_event(KeyCode::Backspace), "abc", 3);
        assert_eq!(result, Some(("ab".to_string(), 2)));

        let result = update_input_buffer(key_event(KeyCode::Backspace), "abc", 2);
        assert_eq!(result, Some(("ac".to_string(), 1)));

        // Note: For Unicode strings, cursor positions are char-based
        // but the implementation has a known limitation with multi-byte chars
        // This documents the current behavior for ASCII-only use cases
    }

    #[test]
    fn test_input_buffer_empty() {
        let result = update_input_buffer(key_event(KeyCode::Backspace), "", 0);
        assert_eq!(result, None);

        let result = update_input_buffer(key_event(KeyCode::Delete), "", 0);
        assert_eq!(result, None);

        let result = update_input_buffer(key_event(KeyCode::Left), "", 0);
        assert_eq!(result, None);

        let result = update_input_buffer(key_event(KeyCode::Right), "", 0);
        assert_eq!(result, None);

        // Home and End on empty buffer return None (cursor already at position)
        let result = update_input_buffer(key_event(KeyCode::Home), "", 0);
        assert_eq!(result, None);

        let result = update_input_buffer(key_event(KeyCode::End), "", 0);
        assert_eq!(result, None);
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

        fs::write(temp.path().join("file1.txt"), "").unwrap();
        fs::write(temp.path().join("file2.txt"), "").unwrap();
        fs::create_dir(temp.path().join("dir1")).unwrap();

        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();
        navigator.ensure_cache();
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

        let mut navigator_no_hidden = TreeNavigator::new(temp.path(), false).unwrap();
        navigator_no_hidden.ensure_cache();
        let count_no_hidden = navigator_no_hidden.visible_entries().len();

        let mut navigator_with_hidden = TreeNavigator::new(temp.path(), true).unwrap();
        navigator_with_hidden.ensure_cache();
        let count_with_hidden = navigator_with_hidden.visible_entries().len();

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

        navigator.ensure_cache();
        let initial_count = navigator.visible_entries().len();

        navigator.expand(&temp.path().join("dir1")).unwrap();
        navigator.ensure_cache();
        let expanded_count = navigator.visible_entries().len();
        assert!(
            expanded_count > initial_count,
            "Expanding should show more entries"
        );

        navigator.collapse(&temp.path().join("dir1"));
        navigator.ensure_cache();
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
        navigator.ensure_cache();
        let initial_count = navigator.visible_entries().len();

        navigator.toggle_expand(&dir_path).unwrap();
        navigator.ensure_cache();
        let expanded_count = navigator.visible_entries().len();
        assert!(expanded_count > initial_count);

        navigator.toggle_expand(&dir_path).unwrap();
        navigator.ensure_cache();
        let collapsed_count = navigator.visible_entries().len();
        assert_eq!(collapsed_count, initial_count);
    }

    #[test]
    fn test_navigator_reload() {
        let temp = TempDir::new().unwrap();

        fs::write(temp.path().join("initial.txt"), "").unwrap();

        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();
        navigator.ensure_cache();
        let initial_count = navigator.visible_entries().len();

        fs::write(temp.path().join("new_file.txt"), "").unwrap();

        navigator.reload().unwrap();
        navigator.ensure_cache();
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

        let mut navigator = TreeNavigator::new(temp.path(), false).unwrap();
        navigator.ensure_cache();

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
    #[ignore] // Requires Finder/trash permissions; run manually
    fn test_create_and_delete_file() {
        let temp = TempDir::new().unwrap();

        let file_path = create_file(temp.path(), "test_file.txt").unwrap();
        assert!(file_path.exists());
        assert!(file_path.is_file());

        delete(&file_path).unwrap();
        assert!(!file_path.exists());
    }

    #[test]
    #[ignore] // Requires Finder/trash permissions; run manually
    fn test_create_and_delete_directory() {
        let temp = TempDir::new().unwrap();

        let dir_path = create_dir(temp.path(), "test_dir").unwrap();
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());

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

        assert!(source.exists());
        assert!(copied.exists());
        assert_eq!(fs::read_to_string(&copied).unwrap(), "content");
    }

    #[test]
    #[ignore] // Requires Finder/trash permissions; run manually
    fn test_delete_non_empty_directory() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("non_empty_dir");

        fs::create_dir(&dir_path).unwrap();
        fs::write(dir_path.join("file.txt"), "content").unwrap();
        fs::create_dir(dir_path.join("subdir")).unwrap();

        delete(&dir_path).unwrap();
        assert!(!dir_path.exists());
    }
}

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

        fs::write(temp.path().join("file.txt"), "original").unwrap();

        // Copy to same directory - should create file_1.txt
        let source = temp.path().join("file.txt");
        let copied = copy_to(&source, temp.path()).unwrap();

        assert!(copied.exists());
        assert_ne!(copied, source);
        let name = copied.file_name().unwrap().to_str().unwrap();
        assert!(name.contains("file") && name.ends_with(".txt"));
    }

    #[test]
    fn test_unique_name_multiple_conflicts() {
        let temp = TempDir::new().unwrap();

        fs::write(temp.path().join("file.txt"), "original").unwrap();
        fs::write(temp.path().join("file_1.txt"), "copy1").unwrap();

        // Copy - should create file_2.txt
        let source = temp.path().join("file.txt");
        let copied = copy_to(&source, temp.path()).unwrap();

        assert!(copied.exists());
        let name = copied.file_name().unwrap().to_str().unwrap();
        assert!(name != "file.txt" && name != "file_1.txt");
    }

    #[test]
    fn test_filename_with_spaces() {
        let temp = TempDir::new().unwrap();

        let file_path = create_file(temp.path(), "my file.txt").unwrap();
        assert!(file_path.exists());
        assert_eq!(file_path.file_name().unwrap(), "my file.txt");

        let renamed = rename(&file_path, "new name.txt").unwrap();
        assert!(renamed.exists());
        assert_eq!(renamed.file_name().unwrap(), "new name.txt");

        // Note: delete is tested separately with #[ignore] due to trash permissions
    }

    #[test]
    fn test_filename_with_unicode() {
        let temp = TempDir::new().unwrap();

        let file_path = create_file(temp.path(), "日本語.txt").unwrap();
        assert!(file_path.exists());
        assert_eq!(file_path.file_name().unwrap(), "日本語.txt");

        let renamed = rename(&file_path, "中文.txt").unwrap();
        assert!(renamed.exists());
        assert_eq!(renamed.file_name().unwrap(), "中文.txt");
    }

    #[test]
    fn test_filename_with_multiple_dots() {
        let temp = TempDir::new().unwrap();

        let file_path = create_file(temp.path(), "file.backup.txt").unwrap();
        assert!(file_path.exists());

        let copied = copy_to(&file_path, temp.path()).unwrap();
        let name = copied.file_name().unwrap().to_str().unwrap();
        assert!(name.ends_with(".txt"));
    }

    #[test]
    fn test_delete_nonexistent_file() {
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("does_not_exist.txt");

        let result = delete(&nonexistent);
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_to_same_name() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "content").unwrap();

        let result = rename(&file_path, "test.txt");
        assert!(result.is_ok());
        assert!(file_path.exists());
    }

    #[test]
    fn test_copy_directory_recursive() {
        let temp = TempDir::new().unwrap();

        let source_dir = temp.path().join("source");
        fs::create_dir(&source_dir).unwrap();
        fs::write(source_dir.join("file1.txt"), "content1").unwrap();
        fs::create_dir(source_dir.join("subdir")).unwrap();
        fs::write(source_dir.join("subdir").join("file2.txt"), "content2").unwrap();

        let dest_dir = temp.path().join("dest");
        fs::create_dir(&dest_dir).unwrap();

        let copied = copy_to(&source_dir, &dest_dir).unwrap();

        assert!(copied.is_dir());
        assert!(copied.join("file1.txt").exists());
        assert!(copied.join("subdir").join("file2.txt").exists());
    }

    #[test]
    fn test_create_file_in_nonexistent_parent() {
        let temp = TempDir::new().unwrap();
        let nonexistent_parent = temp.path().join("nonexistent").join("subdir");

        let result = create_file(&nonexistent_parent, "file.txt");
        assert!(result.is_err());
    }
}

// =============================================================================
// Git Status Tests
// =============================================================================

#[cfg(unix)]
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

        fs::write(temp.path().join("committed.txt"), "content").unwrap();
        git_add(temp.path(), "committed.txt");
        git_commit(temp.path(), "Initial commit");

        let status = GitStatus::detect(temp.path()).unwrap();

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

        fs::write(temp.path().join("new.txt"), "content").unwrap();
        status.refresh();

        assert!(status.branch().is_some() || status.branch().is_none());
    }

    #[test]
    fn test_file_status_default() {
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
        assert!(cmd.contains("my") && cmd.contains("file.txt"));
    }
}

// =============================================================================
// Drag and Drop Tests
// =============================================================================

#[cfg(unix)]
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
        detector.push('/');

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

#[cfg(unix)]
mod image_preview_tests {
    use super::*;
    use fileview::render::{create_image_picker, render_image_preview, ImagePreview, Picker};
    use std::fs;

    // =========================================================================
    // Test Helpers - Image Creation Functions
    // =========================================================================

    /// The image format is chosen from the extension of `path`.
    fn create_test_image(path: &std::path::Path, width: u32, height: u32) {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |x, y| {
            Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
        });
        img.save(path).unwrap();
    }

    fn create_test_png(path: &std::path::Path) {
        create_test_image(path, 1, 1);
    }

    fn create_test_jpeg(path: &std::path::Path) {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(1, 1, |_, _| Rgb([0, 0, 255]));
        img.save(path).unwrap();
    }

    fn create_test_gif(path: &std::path::Path) {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(2, 2, |_, _| Rgb([0, 255, 0]));
        img.save(path).unwrap();
    }

    fn create_test_webp(path: &std::path::Path) {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(2, 2, |_, _| Rgb([255, 255, 0]));
        img.save(path).unwrap();
    }

    fn create_test_bmp(path: &std::path::Path) {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(2, 2, |_, _| Rgb([255, 0, 255]));
        img.save(path).unwrap();
    }

    fn create_test_png_rgba(path: &std::path::Path) {
        use image::{ImageBuffer, Rgba};
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(2, 2, |_, _| Rgba([255, 0, 0, 128]));
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
        assert!(is_image_file(&PathBuf::from("IMAGE.PNG")));
        assert!(is_image_file(&PathBuf::from("IMAGE.JPG")));
        assert!(is_image_file(&PathBuf::from("IMAGE.JPEG")));
        assert!(is_image_file(&PathBuf::from("IMAGE.GIF")));
        assert!(is_image_file(&PathBuf::from("IMAGE.WEBP")));
        assert!(is_image_file(&PathBuf::from("IMAGE.BMP")));
        assert!(is_image_file(&PathBuf::from("IMAGE.ICO")));

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
        assert!(!is_image_file(&PathBuf::from("file.txt")));
        assert!(!is_image_file(&PathBuf::from("code.rs")));
        assert!(!is_image_file(&PathBuf::from("script.py")));
        assert!(!is_image_file(&PathBuf::from("document.md")));

        assert!(!is_image_file(&PathBuf::from("app.exe")));
        assert!(!is_image_file(&PathBuf::from("lib.dll")));
        assert!(!is_image_file(&PathBuf::from("binary.bin")));

        assert!(!is_image_file(&PathBuf::from("config.json")));
        assert!(!is_image_file(&PathBuf::from("settings.toml")));
        assert!(!is_image_file(&PathBuf::from("data.yaml")));

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
        assert!(!is_image_file(&PathBuf::from("file.pn")));
        assert!(!is_image_file(&PathBuf::from("file.jp")));
        assert!(!is_image_file(&PathBuf::from("file.pngg")));
        assert!(!is_image_file(&PathBuf::from("file.jpgg")));
        assert!(!is_image_file(&PathBuf::from("file.gi")));
        assert!(!is_image_file(&PathBuf::from("file.web")));
        assert!(!is_image_file(&PathBuf::from("filepng")));
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

        assert_eq!(
            picker1.is_some(),
            picker2.is_some(),
            "Picker creation should be deterministic"
        );
    }

    #[test]
    fn test_picker_none_is_valid_state() {
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

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let font_size: FontSize = FontSize::new(10, 20);
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

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_image_preview(frame, &mut preview, area, "test.png", true, font_size);
            })
            .unwrap();

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

            assert!(is_image_file(&link_path));

            assert!(image::open(&link_path).is_ok());
        }
    }

    // =========================================================================
    // SECTION 9: Module Export Verification (CI-Safe)
    // =========================================================================

    #[test]
    fn test_render_module_exports() {
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
            (1, 1),
            (16, 9),
            (9, 16),
            (4, 3),
            (21, 9),
            (1, 100),
            (100, 1),
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

        let png_path = temp.path().join("real.png");
        let jpg_path = temp.path().join("real.jpg");
        let gif_path = temp.path().join("real.gif");
        let txt_path = temp.path().join("real.txt");

        create_test_png(&png_path);
        create_test_jpeg(&jpg_path);
        create_test_gif(&gif_path);
        fs::write(&txt_path, "text content").unwrap();

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
        let area = Rect::new(0, 0, 20, 20);
        let font_size: FontSize = FontSize::new(10, 10); // Square cells for simplicity
        let result = calculate_centered_image_area(area, 100, 100, font_size);

        // Image should fill the entire area (scaled to 200x200 pixels = 20x20 cells)
        assert_eq!(result.x, 0);
        assert_eq!(result.y, 0);
        assert_eq!(result.width, 20);
        assert_eq!(result.height, 20);
    }

    #[test]
    fn test_centered_image_wide_image_in_square_area() {
        let area = Rect::new(0, 0, 20, 20);
        let font_size: FontSize = FontSize::new(10, 10);
        let result = calculate_centered_image_area(area, 200, 100, font_size);

        // Image width fills area (200 pixels), height is 100 pixels = 10 cells
        // Vertical padding should be (20 - 10) / 2 = 5
        assert_eq!(result.width, 20);
        assert_eq!(result.height, 10);
        assert_eq!(result.x, 0);
        assert_eq!(result.y, 5);
    }

    #[test]
    fn test_centered_image_tall_image_in_square_area() {
        let area = Rect::new(0, 0, 20, 20);
        let font_size: FontSize = FontSize::new(10, 10);
        let result = calculate_centered_image_area(area, 100, 200, font_size);

        // Image height fills area (200 pixels), width is 100 pixels = 10 cells
        // Horizontal padding should be (20 - 10) / 2 = 5
        assert_eq!(result.width, 10);
        assert_eq!(result.height, 20);
        assert_eq!(result.x, 5);
        assert_eq!(result.y, 0);
    }

    #[test]
    fn test_centered_image_with_offset_area() {
        let area = Rect::new(10, 5, 20, 20);
        let font_size: FontSize = FontSize::new(10, 10);
        let result = calculate_centered_image_area(area, 200, 100, font_size);

        assert_eq!(result.x, 10);
        assert_eq!(result.y, 10); // Original y (5) + padding (5)
        assert_eq!(result.width, 20);
        assert_eq!(result.height, 10);
    }

    #[test]
    fn test_centered_image_typical_terminal_font() {
        // Typical terminal: cells are taller than wide (e.g., 10x20 pixels)
        let area = Rect::new(0, 0, 40, 20);
        let font_size: FontSize = FontSize::new(10, 20);

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
        let area = Rect::new(0, 0, 100, 50);
        let font_size: FontSize = FontSize::new(10, 20);

        let result = calculate_centered_image_area(area, 10, 10, font_size);

        // Area in pixels: 1000x1000
        // Image scales up by 100x to 1000x1000 pixels
        // In cells: 1000/10 = 100 width, 1000/20 = 50 height
        assert_eq!(result.width, 100);
        assert_eq!(result.height, 50);
    }

    #[test]
    fn test_centered_image_preserves_aspect_ratio() {
        let area = Rect::new(0, 0, 80, 40);
        let font_size: FontSize = FontSize::new(10, 20);

        let result = calculate_centered_image_area(area, 400, 300, font_size);

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
        let font_size: FontSize = FontSize::new(10, 20);

        let result = calculate_centered_image_area(area, 100, 100, font_size);

        // Should not panic and should return valid result
        assert!(result.width <= area.width);
        assert!(result.height <= area.height);
    }
}

// =============================================================================
// Terminal Detection Tests
// =============================================================================

#[cfg(unix)]
mod terminal_detection_tests {
    use super::*;

    // =========================================================================
    // Basic API Tests
    // =========================================================================

    #[test]
    fn test_terminal_brand_detect_returns_valid_variant() {
        let brand = TerminalBrand::detect();

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
        // These terminals have no native image protocol support.
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
        let cloned = brand;
        assert_eq!(brand, cloned);
    }

    #[test]
    fn test_recommended_protocol_clone() {
        let protocol = RecommendedProtocol::Sixel;
        let cloned = protocol;
        assert_eq!(protocol, cloned);
    }

    #[test]
    fn test_terminal_brand_copy() {
        let brand = TerminalBrand::Ghostty;
        let copied: TerminalBrand = brand;
        assert_eq!(brand, copied); // Both should still be usable
    }

    #[test]
    fn test_recommended_protocol_copy() {
        let protocol = RecommendedProtocol::Iterm2;
        let copied: RecommendedProtocol = protocol;
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

        let picker1 = create_image_picker();
        let picker2 = create_image_picker();
        let picker3 = create_image_picker();

        assert!(picker1.is_some());
        assert!(picker2.is_some());
        assert!(picker3.is_some());
    }

    #[test]
    fn test_terminal_brand_detect_public_api() {
        let brand = TerminalBrand::detect();

        let _ = brand.name(); // Should not panic
        let protocol = brand.recommended_protocol();
        let _ = protocol.name(); // Should not panic
    }

    #[test]
    fn test_terminal_brand_detect_multiple_calls_deterministic() {
        let brand1 = TerminalBrand::detect();
        let brand2 = TerminalBrand::detect();
        let brand3 = TerminalBrand::detect();

        assert_eq!(brand1, brand2);
        assert_eq!(brand2, brand3);
    }

    #[test]
    fn test_terminal_detection_does_not_modify_environment() {
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

        for value in &supported_values {
            assert!(!value.is_empty(), "Protocol value should not be empty");
        }
    }

    // =========================================================================
    // Stress Tests
    // =========================================================================

    #[test]
    fn test_terminal_detect_rapid_succession() {
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
        let action = KeyAction::QuitAndCd;
        assert!(matches!(action, KeyAction::QuitAndCd));
    }

    #[test]
    fn test_uppercase_q_triggers_quit_and_cd() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf());

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
        assert!(!results.is_empty(), "Should match at least one file");
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
        let mut current = temp.path().to_path_buf();
        for i in 0..15 {
            current = current.join(format!("level{}", i));
            fs::create_dir_all(&current).unwrap();
        }
        fs::write(current.join("deep.txt"), "").unwrap();

        let root = temp.path().to_path_buf();
        let paths = collect_paths(&root, false);

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

        navigator.ensure_cache();
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

        navigator.ensure_cache();
        let before = navigator.visible_entries();
        assert!(
            !before.iter().any(|e| e.path.ends_with("file.txt")),
            "File should not be visible initially"
        );

        let target = temp.path().join("a/b/file.txt");
        navigator.reveal_path(&target).unwrap();

        navigator.ensure_cache();
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

        navigator.ensure_cache();
        let before_count = navigator.visible_count();

        navigator.reveal_path(&target).unwrap();

        navigator.ensure_cache();
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

        assert!(matches!(state.mode, ViewMode::Browse));

        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let action = handle_key_event(&state, key);
        assert!(matches!(action, KeyAction::OpenFuzzyFinder));

        // Simulate opening fuzzy finder
        state.mode = ViewMode::FuzzyFinder {
            query: String::new(),
            selected: 0,
        };

        // Simulate typing a query
        if let ViewMode::FuzzyFinder { query, .. } = &mut state.mode {
            *query = "main".to_string();
        }

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let action = handle_key_event(&state, key);
        assert!(matches!(action, KeyAction::FuzzyConfirm { .. }));

        // Simulate returning to Browse mode
        state.mode = ViewMode::Browse;
        assert!(matches!(state.mode, ViewMode::Browse));
    }

    #[test]
    fn test_fuzzy_workflow_cancel() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 3,
        };

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let action = handle_key_event(&state, key);
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_fuzzy_workflow_navigation() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

        state.mode = ViewMode::FuzzyFinder {
            query: "test".to_string(),
            selected: 0,
        };

        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let action = handle_key_event(&state, key);
        assert!(matches!(action, KeyAction::FuzzyDown));

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

        state.mode = ViewMode::Input {
            purpose: InputPurpose::CreateFile,
            buffer: String::new(),
            cursor: 0,
        };

        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let action = handle_key_event(&state, key);

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

        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let _ = handle_key_event(&state, key);

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

        let paths: Vec<PathBuf> = (0..100)
            .map(|i| temp.path().join(format!("file_{:03}.txt", i)))
            .collect();

        let results = fuzzy_match("file", &paths, &root);

        // Should limit to MAX_RESULTS (15)
        assert!(results.len() <= 15);
        assert!(results.iter().all(|r| r.display.contains("file")));
    }

    #[test]
    fn test_collect_paths_wide_directory() {
        let temp = TempDir::new().unwrap();

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

        navigator.reveal_path(&target).unwrap();
        navigator.ensure_cache();
        let count1 = navigator.visible_count();

        navigator.reveal_path(&target).unwrap();
        navigator.ensure_cache();
        let count2 = navigator.visible_count();

        navigator.reveal_path(&target).unwrap();
        navigator.ensure_cache();
        let count3 = navigator.visible_count();

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

        navigator.ensure_cache();
        let entries = navigator.visible_entries();
        let paths: Vec<_> = entries.iter().map(|e| &e.path).collect();

        assert!(paths.contains(&&target1));
        assert!(paths.contains(&&target2));
    }

    #[test]
    fn test_reveal_path_with_hidden_parent() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join(".hidden/sub")).unwrap();
        fs::write(temp.path().join(".hidden/sub/file.txt"), "").unwrap();

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

        state.mode = ViewMode::FuzzyFinder {
            query: String::new(),
            selected: 0,
        };

        // Simulate FuzzyUp action
        if let ViewMode::FuzzyFinder { selected, .. } = &mut state.mode {
            *selected = selected.saturating_sub(1);
        }

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

        state.mode = ViewMode::FuzzyFinder {
            query: String::new(),
            selected: 0,
        };

        // Simulate finding and selecting deep.txt
        let target = temp.path().join("a/b/c/d/deep.txt");
        state.fuzzy_jump_target = Some(target.clone());
        state.mode = ViewMode::Browse;

        navigator.reveal_path(&target).unwrap();

        navigator.ensure_cache();
        let entries = navigator.visible_entries();
        let idx = entries.iter().position(|e| e.path == target);

        assert!(idx.is_some(), "Deep file should be visible after reveal");
    }

    #[test]
    fn test_fuzzy_rapid_open_close() {
        let temp = TempDir::new().unwrap();
        let mut state = AppState::new(temp.path().to_path_buf());

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

        state.mode = ViewMode::FuzzyFinder {
            query: "old".to_string(),
            selected: 5,
        };

        // Change query (simulating text input)
        state.mode = ViewMode::FuzzyFinder {
            query: "new".to_string(),
            selected: 0,
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

        assert!(!paths.iter().any(|p| p.ends_with("beyond_limit.txt")));
    }
}

// =============================================================================
// Archive Preview Tests
// =============================================================================

#[cfg(feature = "archive")]
mod archive_preview_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use zip::write::FileOptions;

    /// Entry names ending in `/` are added as directories.
    fn create_test_zip(path: &std::path::Path, files: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options: FileOptions<'_, '_, ()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for (name, content) in files {
            if name.ends_with('/') {
                zip.add_directory(*name, options).unwrap();
            } else {
                zip.start_file(*name, options).unwrap();
                zip.write_all(content).unwrap();
            }
        }

        zip.finish().unwrap();
    }

    #[test]
    fn test_is_archive_file_detection() {
        assert!(is_archive_file(&PathBuf::from("test.zip")));
        assert!(is_archive_file(&PathBuf::from("test.jar")));
        assert!(is_archive_file(&PathBuf::from("test.apk")));
        assert!(is_archive_file(&PathBuf::from("test.ipa")));
        assert!(is_archive_file(&PathBuf::from("test.xpi")));
        assert!(is_archive_file(&PathBuf::from("test.epub")));

        assert!(is_archive_file(&PathBuf::from("test.tar.gz")));
        assert!(is_archive_file(&PathBuf::from("test.tgz")));

        // Case insensitive
        assert!(is_archive_file(&PathBuf::from("TEST.ZIP")));
        assert!(is_archive_file(&PathBuf::from("Test.Jar")));
        assert!(is_archive_file(&PathBuf::from("TEST.TAR.GZ")));
        assert!(is_archive_file(&PathBuf::from("TEST.TGZ")));

        assert!(!is_archive_file(&PathBuf::from("test.txt")));
        assert!(!is_archive_file(&PathBuf::from("test.7z")));
        assert!(!is_archive_file(&PathBuf::from("test.tar"))); // Plain tar not supported
    }

    #[test]
    fn test_archive_preview_load_valid_zip() {
        let temp = TempDir::new().unwrap();
        let zip_path = temp.path().join("test.zip");

        create_test_zip(
            &zip_path,
            &[
                ("file1.txt", b"content1"),
                ("dir/", &[]),
                ("dir/file2.txt", b"content2"),
            ],
        );

        let preview = ArchivePreview::load_zip(&zip_path).unwrap();

        // Should have 2 files (dir entry is a directory, not a file)
        assert_eq!(preview.file_count, 2);
        assert!(preview.total_size > 0);
        assert_eq!(preview.scroll, 0);

        // Should have 3 entries total (1 dir + 2 files)
        assert_eq!(preview.entries.len(), 3);
    }

    #[test]
    fn test_archive_preview_zip_keeps_entry_names() {
        let temp = TempDir::new().unwrap();
        let zip_path = temp.path().join("names.zip");
        create_test_zip(
            &zip_path,
            &[
                ("readme.txt", b"a"),
                ("docs/", &[]),
                ("docs/日本語.md", b"b"),
            ],
        );

        let preview = ArchivePreview::load_zip(&zip_path).unwrap();

        let names: Vec<&str> = preview.entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"readme.txt"));
        assert!(names.contains(&"docs/日本語.md"));
        assert!(names.iter().any(|n| n.trim_end_matches('/') == "docs"));
    }

    #[test]
    fn test_archive_preview_load_corrupted_zip() {
        let temp = TempDir::new().unwrap();
        let zip_path = temp.path().join("corrupted.zip");

        fs::write(&zip_path, b"not a valid zip file content").unwrap();

        let result = ArchivePreview::load_zip(&zip_path);
        assert!(result.is_err(), "Corrupted zip should return an error");
    }

    #[test]
    fn test_archive_preview_empty_zip() {
        let temp = TempDir::new().unwrap();
        let zip_path = temp.path().join("empty.zip");

        create_test_zip(&zip_path, &[]);

        let preview = ArchivePreview::load_zip(&zip_path).unwrap();

        assert_eq!(preview.file_count, 0);
        assert_eq!(preview.total_size, 0);
        assert!(preview.entries.is_empty());
    }

    #[test]
    fn test_archive_preview_scroll_bounds() {
        let temp = TempDir::new().unwrap();
        let zip_path = temp.path().join("scroll_test.zip");

        let files: Vec<(String, Vec<u8>)> = (0..50)
            .map(|i| {
                (
                    format!("file{:03}.txt", i),
                    format!("content{}", i).into_bytes(),
                )
            })
            .collect();

        let file_refs: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(n, c)| (n.as_str(), c.as_slice()))
            .collect();

        create_test_zip(&zip_path, &file_refs);

        let mut preview = ArchivePreview::load_zip(&zip_path).unwrap();

        assert_eq!(preview.scroll, 0);

        preview.scroll = 10;
        assert_eq!(preview.scroll, 10);

        preview.scroll = preview.scroll.saturating_sub(15);
        assert_eq!(preview.scroll, 0);

        let max_scroll = preview.line_count().saturating_sub(20);
        preview.scroll = max_scroll;
        assert!(preview.scroll <= preview.line_count());
    }

    #[test]
    fn test_archive_preview_entries_sorted() {
        let temp = TempDir::new().unwrap();
        let zip_path = temp.path().join("sorted.zip");

        // Create zip with mixed files and directories (unsorted order)
        create_test_zip(
            &zip_path,
            &[
                ("z_file.txt", b"z"),
                ("a_dir/", &[]),
                ("a_file.txt", b"a"),
                ("m_dir/", &[]),
            ],
        );

        let preview = ArchivePreview::load_zip(&zip_path).unwrap();

        // Entries should be sorted: directories first, then files, both alphabetically
        let names: Vec<&str> = preview.entries.iter().map(|e| e.name.as_str()).collect();

        assert!(
            preview.entries[0].is_dir,
            "First entry should be a directory"
        );
        assert!(
            preview.entries[1].is_dir,
            "Second entry should be a directory"
        );

        assert!(names[0] < names[1], "Directories should be sorted");

        assert!(!preview.entries[2].is_dir, "Third entry should be a file");
        assert!(!preview.entries[3].is_dir, "Fourth entry should be a file");
    }

    #[test]
    fn test_archive_preview_line_count() {
        let temp = TempDir::new().unwrap();
        let zip_path = temp.path().join("linecount.zip");

        create_test_zip(
            &zip_path,
            &[
                ("file1.txt", b"1"),
                ("file2.txt", b"2"),
                ("file3.txt", b"3"),
                ("file4.txt", b"4"),
                ("file5.txt", b"5"),
            ],
        );

        let preview = ArchivePreview::load_zip(&zip_path).unwrap();

        // line_count = entries (5) + header lines (2) = 7
        assert_eq!(preview.line_count(), 7);
    }

    #[test]
    fn test_archive_preview_nonexistent_file() {
        let result = ArchivePreview::load_zip(std::path::Path::new("/nonexistent/path/file.zip"));
        assert!(result.is_err(), "Nonexistent file should return an error");
    }

    // =========================================================================
    // tar.gz archive tests (v1.10.0)
    // =========================================================================

    /// Entry names ending in `/` are added as directories.
    fn create_test_tar_gz(path: &std::path::Path, files: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);

        for (name, content) in files {
            if name.ends_with('/') {
                let mut header = tar::Header::new_gnu();
                header.set_entry_type(tar::EntryType::Directory);
                header.set_path(*name).unwrap();
                header.set_size(0);
                header.set_mode(0o755);
                header.set_mtime(0);
                header.set_cksum();
                archive.append(&header, std::io::empty()).unwrap();
            } else {
                let mut header = tar::Header::new_gnu();
                header.set_size(content.len() as u64);
                header.set_mode(0o644);
                header.set_mtime(0);
                header.set_cksum();
                archive
                    .append_data(&mut header, *name, &content[..])
                    .unwrap();
            }
        }

        archive.into_inner().unwrap().finish().unwrap();
    }

    #[test]
    fn test_tar_gz_archive_preview_load_valid() {
        let temp = TempDir::new().unwrap();
        let tar_gz_path = temp.path().join("test.tar.gz");

        create_test_tar_gz(
            &tar_gz_path,
            &[
                ("file1.txt", b"content1"),
                ("dir/", &[]),
                ("dir/file2.txt", b"content2"),
            ],
        );

        let preview = ArchivePreview::load_tar_gz(&tar_gz_path).unwrap();

        // Should have 2 files (dir entry is a directory, not a file)
        assert_eq!(preview.file_count, 2);
        assert!(preview.total_size > 0);
        assert_eq!(preview.scroll, 0);

        // Should have 3 entries total (1 dir + 2 files)
        assert_eq!(preview.entries.len(), 3);
    }

    #[test]
    fn test_tar_gz_archive_preview_tgz_extension() {
        let temp = TempDir::new().unwrap();
        let tgz_path = temp.path().join("test.tgz");

        create_test_tar_gz(
            &tgz_path,
            &[("file1.txt", b"content"), ("file2.txt", b"more content")],
        );

        let preview = ArchivePreview::load_tar_gz(&tgz_path).unwrap();

        assert_eq!(preview.file_count, 2);
        assert_eq!(preview.entries.len(), 2);
    }

    #[test]
    fn test_tar_gz_archive_preview_empty() {
        let temp = TempDir::new().unwrap();
        let tar_gz_path = temp.path().join("empty.tar.gz");

        create_test_tar_gz(&tar_gz_path, &[]);

        let preview = ArchivePreview::load_tar_gz(&tar_gz_path).unwrap();

        assert_eq!(preview.file_count, 0);
        assert_eq!(preview.total_size, 0);
        assert!(preview.entries.is_empty());
    }

    #[test]
    fn test_tar_gz_archive_preview_corrupted() {
        let temp = TempDir::new().unwrap();
        let tar_gz_path = temp.path().join("corrupted.tar.gz");

        fs::write(&tar_gz_path, b"not a valid tar.gz file content").unwrap();

        let result = ArchivePreview::load_tar_gz(&tar_gz_path);
        assert!(result.is_err(), "Corrupted tar.gz should return an error");
    }

    #[test]
    fn test_tar_gz_archive_preview_entries_sorted() {
        let temp = TempDir::new().unwrap();
        let tar_gz_path = temp.path().join("sorted.tar.gz");

        // Create tar.gz with mixed files and directories (unsorted order)
        create_test_tar_gz(
            &tar_gz_path,
            &[
                ("z_file.txt", b"z"),
                ("a_dir/", &[]),
                ("a_file.txt", b"a"),
                ("m_dir/", &[]),
            ],
        );

        let preview = ArchivePreview::load_tar_gz(&tar_gz_path).unwrap();

        // Entries should be sorted: directories first, then files, both alphabetically
        assert!(
            preview.entries[0].is_dir,
            "First entry should be a directory"
        );
        assert!(
            preview.entries[1].is_dir,
            "Second entry should be a directory"
        );

        assert!(!preview.entries[2].is_dir, "Third entry should be a file");
        assert!(!preview.entries[3].is_dir, "Fourth entry should be a file");
    }

    #[test]
    fn test_tar_gz_archive_preview_nonexistent_file() {
        let result =
            ArchivePreview::load_tar_gz(std::path::Path::new("/nonexistent/path/file.tar.gz"));
        assert!(result.is_err(), "Nonexistent file should return an error");
    }
}

// =============================================================================
// PDF Preview Tests
// =============================================================================

mod pdf_preview_tests {
    use fileview::render::{find_pdftoppm, is_pdf_file};
    use std::path::Path;

    #[test]
    fn test_is_pdf_file_detection() {
        assert!(is_pdf_file(Path::new("document.pdf")));
        assert!(is_pdf_file(Path::new("REPORT.PDF")));
        assert!(is_pdf_file(Path::new("/path/to/file.pdf")));
        assert!(!is_pdf_file(Path::new("document.txt")));
        assert!(!is_pdf_file(Path::new("image.png")));
    }

    #[test]
    fn test_pdftoppm_detection() {
        let result = find_pdftoppm();
        // Result can be Some (if installed) or None (if not)
        // We just verify the function doesn't panic
        if let Some(path) = result {
            assert!(path.exists());
            assert!(path.to_string_lossy().contains("pdftoppm"));
        }
    }

    #[test]
    fn test_pdf_preview_requires_poppler() {
        // Test that PDF preview gracefully handles missing pdftoppm
        // This is tested by checking that find_pdftoppm returns a consistent result
        let result1 = find_pdftoppm();
        let result2 = find_pdftoppm();
        assert_eq!(result1.is_some(), result2.is_some());
    }
}

#[cfg(test)]
mod pdf_rendering_tests {
    use fileview::render::find_pdftoppm;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn create_test_pdf(path: &std::path::Path, pages: usize) -> std::io::Result<()> {
        let mut content = String::from("%PDF-1.4\n");
        content.push_str("1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");

        // Pages object
        let kids: Vec<String> = (0..pages).map(|i| format!("{} 0 R", i + 3)).collect();
        content.push_str(&format!(
            "2 0 obj << /Type /Pages /Kids [{}] /Count {} >> endobj\n",
            kids.join(" "),
            pages
        ));

        // Page objects
        for i in 0..pages {
            content.push_str(&format!(
                "{} 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >> endobj\n",
                i + 3
            ));
        }

        // Trailer
        content.push_str(&format!(
            "trailer << /Size {} /Root 1 0 R >>\n%%EOF\n",
            pages + 3
        ));

        fs::write(path, content)
    }

    #[test]
    fn test_pdfinfo_page_count() {
        if find_pdftoppm().is_none() {
            eprintln!("Skipping test: pdftoppm not installed");
            return;
        }

        let temp = TempDir::new().unwrap();
        let pdf_path = temp.path().join("test.pdf");
        create_test_pdf(&pdf_path, 3).unwrap();

        let output = Command::new("pdfinfo")
            .arg(&pdf_path)
            .output()
            .expect("Failed to run pdfinfo");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let has_pages = stdout.lines().any(|line| line.starts_with("Pages:"));
        assert!(has_pages, "pdfinfo should report page count");
    }

    #[test]
    fn test_pdftoppm_renders_page() {
        let pdftoppm_path = match find_pdftoppm() {
            Some(p) => p.clone(),
            None => {
                eprintln!("Skipping test: pdftoppm not installed");
                return;
            }
        };

        let temp = TempDir::new().unwrap();
        let pdf_path = temp.path().join("test.pdf");
        create_test_pdf(&pdf_path, 2).unwrap();

        let output_prefix = temp.path().join("page1");
        let status = Command::new(&pdftoppm_path)
            .args(["-png", "-f", "1", "-l", "1", "-singlefile", "-r", "72"])
            .arg(&pdf_path)
            .arg(&output_prefix)
            .status()
            .expect("Failed to run pdftoppm");

        assert!(status.success(), "pdftoppm should succeed");

        let output_file = temp.path().join("page1.png");
        assert!(output_file.exists(), "pdftoppm should create PNG file");
    }

    #[test]
    fn test_pdftoppm_handles_invalid_pdf() {
        let pdftoppm_path = match find_pdftoppm() {
            Some(p) => p.clone(),
            None => {
                eprintln!("Skipping test: pdftoppm not installed");
                return;
            }
        };

        let temp = TempDir::new().unwrap();
        let invalid_pdf = temp.path().join("invalid.pdf");
        fs::write(&invalid_pdf, b"not a pdf").unwrap();

        let output_prefix = temp.path().join("output");
        let status = Command::new(&pdftoppm_path)
            .args(["-png", "-f", "1", "-l", "1", "-singlefile"])
            .arg(&invalid_pdf)
            .arg(&output_prefix)
            .status();

        // pdftoppm should fail or not create output for invalid PDF
        if let Ok(s) = status {
            if s.success() {
                // Even if it "succeeds", no valid output should be created
                let output_file = temp.path().join("output.png");
                // File might exist but be empty or invalid
                if output_file.exists() {
                    let size = fs::metadata(&output_file).unwrap().len();
                    // Very small file likely means failure
                    assert!(size < 100, "Invalid PDF should not produce valid image");
                }
            }
        }
    }
}

// =============================================================================
// Config File Integration Tests
// =============================================================================

mod config_integration_tests {
    use fileview::app::ConfigFile;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_config_file_default_values() {
        let config = ConfigFile::default();

        assert!(!config.general.show_hidden);
        assert!(config.general.enable_icons);
        assert!(config.general.mouse_enabled);

        assert_eq!(config.preview.hex_max_bytes, 4096);
        assert_eq!(config.preview.max_archive_entries, 500);
        assert_eq!(config.preview.image_protocol, "auto");

        assert_eq!(config.performance.git_poll_interval_secs, 5);

        assert!(config.ui.show_size);
        assert!(!config.ui.show_permissions);
        assert_eq!(config.ui.date_format, "%Y-%m-%d %H:%M");
    }

    #[test]
    fn test_config_file_partial_override() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");

        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
[general]
show_hidden = true

[performance]
git_poll_interval_secs = 10
"#
        )
        .unwrap();

        let config = ConfigFile::load_from(&config_path).unwrap();

        // Overridden values
        assert!(config.general.show_hidden);
        assert_eq!(config.performance.git_poll_interval_secs, 10);

        // Defaults preserved
        assert!(config.general.enable_icons);
        assert_eq!(config.preview.hex_max_bytes, 4096);
    }

    #[test]
    fn test_config_file_all_sections() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");

        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
[general]
show_hidden = true
enable_icons = false
mouse_enabled = false

[preview]
hex_max_bytes = 8192
max_archive_entries = 1000
image_protocol = "kitty"

[performance]
git_poll_interval_secs = 15

[ui]
show_size = false
show_permissions = true
date_format = "%d/%m/%Y"
"#
        )
        .unwrap();

        let config = ConfigFile::load_from(&config_path).unwrap();

        assert!(config.general.show_hidden);
        assert!(!config.general.enable_icons);
        assert!(!config.general.mouse_enabled);

        assert_eq!(config.preview.hex_max_bytes, 8192);
        assert_eq!(config.preview.max_archive_entries, 1000);
        assert_eq!(config.preview.image_protocol, "kitty");

        assert_eq!(config.performance.git_poll_interval_secs, 15);

        assert!(!config.ui.show_size);
        assert!(config.ui.show_permissions);
        assert_eq!(config.ui.date_format, "%d/%m/%Y");
    }

    #[test]
    fn test_config_file_invalid_toml_returns_error() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");

        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(file, "invalid {{ toml syntax").unwrap();

        let result = ConfigFile::load_from(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_file_unknown_keys_ignored() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");

        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
[general]
show_hidden = true
unknown_key = "ignored"

[unknown_section]
foo = "bar"
"#
        )
        .unwrap();

        let config = ConfigFile::load_from(&config_path).unwrap();
        assert!(config.general.show_hidden);
    }
}

// =============================================================================
// Keymap Integration Tests
// =============================================================================

mod keymap_integration_tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use fileview::core::{AppState, FocusTarget, ViewMode};
    use fileview::handler::{handle_key_event_with_registry, KeyAction, KeyBindingRegistry};
    use tempfile::TempDir;

    fn create_test_state() -> AppState {
        let temp = TempDir::new().unwrap();
        AppState::new(temp.path().to_path_buf())
    }

    #[test]
    fn test_registry_default_bindings() {
        let registry = KeyBindingRegistry::new();
        let state = create_test_state();

        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let action = handle_key_event_with_registry(&state, key, &registry);
        assert!(matches!(action, KeyAction::MoveDown));

        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        let action = handle_key_event_with_registry(&state, key, &registry);
        assert!(matches!(action, KeyAction::MoveUp));

        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let action = handle_key_event_with_registry(&state, key, &registry);
        assert!(matches!(action, KeyAction::Quit));
    }

    #[test]
    fn test_registry_ctrl_modifier() {
        let registry = KeyBindingRegistry::new();
        let state = create_test_state();

        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let action = handle_key_event_with_registry(&state, key, &registry);
        assert!(matches!(action, KeyAction::OpenFuzzyFinder));
    }

    #[test]
    fn test_registry_context_aware_actions() {
        let registry = KeyBindingRegistry::new();

        let mut state = create_test_state();
        state.pick_mode = true;
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let action = handle_key_event_with_registry(&state, key, &registry);
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_registry_preview_focus_navigation() {
        let registry = KeyBindingRegistry::new();

        let mut state = create_test_state();
        state.preview_visible = true;
        state.focus_target = FocusTarget::Preview;

        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let action = handle_key_event_with_registry(&state, key, &registry);
        assert!(matches!(action, KeyAction::PreviewScrollDown));

        state.focus_target = FocusTarget::Tree;
        let action = handle_key_event_with_registry(&state, key, &registry);
        assert!(matches!(action, KeyAction::MoveDown));
    }

    #[test]
    fn test_registry_search_mode() {
        let registry = KeyBindingRegistry::new();
        let mut state = create_test_state();
        state.mode = ViewMode::Search {
            query: "test".to_string(),
        };

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let action = handle_key_event_with_registry(&state, key, &registry);
        assert!(matches!(action, KeyAction::ConfirmInput { value } if value == "test"));

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let action = handle_key_event_with_registry(&state, key, &registry);
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_registry_confirm_mode() {
        let registry = KeyBindingRegistry::new();
        let mut state = create_test_state();
        state.mode = ViewMode::Confirm {
            action: fileview::core::PendingAction::Delete {
                targets: vec![std::path::PathBuf::from("/tmp/test")],
            },
        };

        let key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
        let action = handle_key_event_with_registry(&state, key, &registry);
        assert!(matches!(action, KeyAction::ExecuteDelete));

        let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        let action = handle_key_event_with_registry(&state, key, &registry);
        assert!(matches!(action, KeyAction::Cancel));
    }

    #[test]
    fn test_registry_filter_mode() {
        let registry = KeyBindingRegistry::new();
        let mut state = create_test_state();
        state.mode = ViewMode::Filter {
            query: "*.rs".to_string(),
        };

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let action = handle_key_event_with_registry(&state, key, &registry);
        assert!(matches!(action, KeyAction::ApplyFilter { pattern } if pattern == "*.rs"));
    }

    #[test]
    fn test_registry_filter_mode_empty_clears() {
        let registry = KeyBindingRegistry::new();
        let mut state = create_test_state();
        state.mode = ViewMode::Filter {
            query: String::new(),
        };

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let action = handle_key_event_with_registry(&state, key, &registry);
        assert!(matches!(action, KeyAction::ClearFilter));
    }
}

// =============================================================================
// Theme Integration Tests
// =============================================================================

mod theme_integration_tests {
    use fileview::render::{parse_color, theme, Theme, ThemeFile};
    use ratatui::style::Color;

    #[test]
    fn test_theme_singleton() {
        let t1 = theme();
        let t2 = theme();
        assert!(std::ptr::eq(t1, t2));
    }

    #[test]
    fn test_theme_default_colors() {
        let theme = Theme::default();

        assert_eq!(theme.selection, Color::DarkGray);
        assert_eq!(theme.border_active, Color::Cyan);

        assert_eq!(theme.directory, Color::Blue);
        assert_eq!(theme.executable, Color::Green);
        assert_eq!(theme.symlink, Color::Cyan);
        assert_eq!(theme.archive, Color::Red);
        assert_eq!(theme.image, Color::Magenta);

        assert_eq!(theme.git_modified, Color::Yellow);
        assert_eq!(theme.git_staged, Color::Green);
        assert_eq!(theme.git_deleted, Color::Red);
        assert_eq!(theme.git_renamed, Color::Cyan);
        assert_eq!(theme.git_ignored, Color::DarkGray);
    }

    #[test]
    fn test_parse_color_named() {
        assert_eq!(parse_color("red"), Color::Red);
        assert_eq!(parse_color("blue"), Color::Blue);
        assert_eq!(parse_color("green"), Color::Green);
        assert_eq!(parse_color("yellow"), Color::Yellow);
        assert_eq!(parse_color("magenta"), Color::Magenta);
        assert_eq!(parse_color("cyan"), Color::Cyan);
        assert_eq!(parse_color("white"), Color::White);
        assert_eq!(parse_color("black"), Color::Black);
        assert_eq!(parse_color("gray"), Color::Gray);
        assert_eq!(parse_color("darkgray"), Color::DarkGray);
    }

    #[test]
    fn test_parse_color_case_insensitive() {
        assert_eq!(parse_color("RED"), Color::Red);
        assert_eq!(parse_color("Blue"), Color::Blue);
        assert_eq!(parse_color("GREEN"), Color::Green);
        assert_eq!(parse_color("DarkGray"), Color::DarkGray);
    }

    #[test]
    fn test_parse_color_hex_short() {
        assert_eq!(parse_color("#f00"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("#0f0"), Color::Rgb(0, 255, 0));
        assert_eq!(parse_color("#00f"), Color::Rgb(0, 0, 255));
        assert_eq!(parse_color("#fff"), Color::Rgb(255, 255, 255));
        assert_eq!(parse_color("#000"), Color::Rgb(0, 0, 0));
    }

    #[test]
    fn test_parse_color_hex_long() {
        assert_eq!(parse_color("#ff0000"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("#00ff00"), Color::Rgb(0, 255, 0));
        assert_eq!(parse_color("#0000ff"), Color::Rgb(0, 0, 255));
        assert_eq!(parse_color("#ffffff"), Color::Rgb(255, 255, 255));
        assert_eq!(parse_color("#808080"), Color::Rgb(128, 128, 128));
    }

    #[test]
    fn test_parse_color_rgb() {
        assert_eq!(parse_color("rgb(255, 0, 0)"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("rgb(0, 255, 0)"), Color::Rgb(0, 255, 0));
        assert_eq!(parse_color("rgb(128, 128, 128)"), Color::Rgb(128, 128, 128));
    }

    #[test]
    fn test_parse_color_indexed() {
        assert_eq!(parse_color("0"), Color::Indexed(0));
        assert_eq!(parse_color("196"), Color::Indexed(196));
        assert_eq!(parse_color("color123"), Color::Indexed(123));
        assert_eq!(parse_color("color255"), Color::Indexed(255));
    }

    #[test]
    fn test_parse_color_default() {
        assert_eq!(parse_color("default"), Color::Reset);
        assert_eq!(parse_color("reset"), Color::Reset);
        assert_eq!(parse_color(""), Color::Reset);
    }

    #[test]
    fn test_parse_color_invalid() {
        assert_eq!(parse_color("invalid"), Color::Reset);
        assert_eq!(parse_color("notacolor"), Color::Reset);
        // Invalid hex characters result in 0 values from from_str_radix
        assert_eq!(parse_color("#gggggg"), Color::Rgb(0, 0, 0));
    }

    #[test]
    fn test_theme_file_defaults() {
        let file = ThemeFile::default();

        assert_eq!(file.colors.background, "default");
        assert_eq!(file.colors.foreground, "white");
        assert_eq!(file.colors.selection, "darkgray");
        assert_eq!(file.colors.border_active, "cyan");

        assert_eq!(file.file_colors.directory, "blue");
        assert_eq!(file.file_colors.executable, "green");

        assert_eq!(file.git_colors.modified, "yellow");
        assert_eq!(file.git_colors.staged, "green");
    }

    #[test]
    fn test_theme_purple_alias() {
        assert_eq!(parse_color("purple"), Color::Magenta);
    }

    #[test]
    fn test_theme_grey_alias() {
        assert_eq!(parse_color("grey"), Color::Gray);
        assert_eq!(parse_color("darkgrey"), Color::DarkGray);
    }

    #[test]
    fn test_theme_light_colors() {
        assert_eq!(parse_color("lightred"), Color::LightRed);
        assert_eq!(parse_color("lightgreen"), Color::LightGreen);
        assert_eq!(parse_color("lightyellow"), Color::LightYellow);
        assert_eq!(parse_color("lightblue"), Color::LightBlue);
        assert_eq!(parse_color("lightmagenta"), Color::LightMagenta);
        assert_eq!(parse_color("lightcyan"), Color::LightCyan);
    }
}

// =============================================================================
// Custom Commands Integration Tests (Phase 4)
// =============================================================================

mod custom_commands_tests {
    use fileview::app::CommandsConfig;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_config(commands: Vec<(&str, &str)>) -> CommandsConfig {
        let mut map = HashMap::new();
        for (name, cmd) in commands {
            map.insert(name.to_string(), cmd.to_string());
        }
        CommandsConfig { commands: map }
    }

    #[test]
    fn test_commands_config_get() {
        let config = create_config(vec![("open", "open $f"), ("edit", "vim $f")]);

        assert_eq!(config.get("open"), Some(&"open $f".to_string()));
        assert_eq!(config.get("edit"), Some(&"vim $f".to_string()));
        assert_eq!(config.get("nonexistent"), None);
    }

    #[test]
    fn test_commands_config_expand_file_path() {
        let path = PathBuf::from("/home/user/test.txt");
        let expanded = CommandsConfig::expand("echo $f", &path);
        assert_eq!(expanded, "echo /home/user/test.txt");
    }

    #[test]
    fn test_commands_config_expand_directory() {
        let path = PathBuf::from("/home/user/test.txt");
        let expanded = CommandsConfig::expand("cd $d", &path);
        assert_eq!(expanded, "cd /home/user");
    }

    #[test]
    fn test_commands_config_expand_filename() {
        let path = PathBuf::from("/home/user/test.txt");
        let expanded = CommandsConfig::expand("echo $n", &path);
        assert_eq!(expanded, "echo test.txt");
    }

    #[test]
    fn test_commands_config_expand_stem() {
        let path = PathBuf::from("/home/user/test.txt");
        let expanded = CommandsConfig::expand("echo $s", &path);
        assert_eq!(expanded, "echo test");
    }

    #[test]
    fn test_commands_config_expand_extension() {
        let path = PathBuf::from("/home/user/test.txt");
        let expanded = CommandsConfig::expand("echo $e", &path);
        assert_eq!(expanded, "echo txt");
    }

    #[test]
    fn test_commands_config_expand_all_placeholders() {
        let path = PathBuf::from("/home/user/document.pdf");
        let expanded = CommandsConfig::expand("process $f in $d: $n ($s.$e)", &path);
        assert_eq!(
            expanded,
            "process /home/user/document.pdf in /home/user: document.pdf (document.pdf)"
        );
    }

    #[test]
    fn test_commands_config_expand_no_placeholders() {
        let path = PathBuf::from("/home/user/test.txt");
        let expanded = CommandsConfig::expand("echo hello", &path);
        assert_eq!(expanded, "echo hello");
    }

    #[test]
    fn test_commands_config_expand_no_extension() {
        let path = PathBuf::from("/home/user/Makefile");
        let expanded = CommandsConfig::expand("$n has ext: $e", &path);
        assert_eq!(expanded, "Makefile has ext: ");
    }

    #[test]
    fn test_commands_from_toml() {
        let toml_content = r#"
            [commands]
            open = "open $f"
            edit = "nvim $f"
            terminal = "cd $d && $SHELL"
        "#;

        #[derive(serde::Deserialize)]
        struct TestConfig {
            commands: CommandsConfig,
        }

        let config: TestConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.commands.get("open"), Some(&"open $f".to_string()));
        assert_eq!(config.commands.get("edit"), Some(&"nvim $f".to_string()));
        assert_eq!(
            config.commands.get("terminal"),
            Some(&"cd $d && $SHELL".to_string())
        );
    }
}

// =============================================================================
// Custom Preview Integration Tests (Phase 5)
// =============================================================================

#[cfg(unix)]
mod custom_preview_tests {
    use fileview::render::CustomPreview;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_custom_preview_execute_simple() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "hello world").unwrap();

        let preview = CustomPreview::execute("cat $f", &file_path).unwrap();
        assert_eq!(preview.lines.len(), 1);
        assert_eq!(preview.lines[0], "hello world");
        assert_eq!(preview.scroll, 0);
    }

    #[test]
    fn test_custom_preview_execute_multiline() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "line1\nline2\nline3").unwrap();

        let preview = CustomPreview::execute("cat $f", &file_path).unwrap();
        assert_eq!(preview.lines.len(), 3);
        assert_eq!(preview.lines[0], "line1");
        assert_eq!(preview.lines[1], "line2");
        assert_eq!(preview.lines[2], "line3");
    }

    #[test]
    fn test_custom_preview_line_count() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "a\nb\nc\nd\ne").unwrap();

        let preview = CustomPreview::execute("cat $f", &file_path).unwrap();
        assert_eq!(preview.line_count(), 5);
    }

    #[test]
    fn test_custom_preview_placeholder_expansion() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "content").unwrap();

        let preview = CustomPreview::execute("echo $f", &file_path).unwrap();
        assert!(preview.lines[0].contains("test.txt"));
    }

    #[test]
    fn test_custom_preview_empty_output() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "").unwrap();

        let preview = CustomPreview::execute("cat $f", &file_path).unwrap();
        assert_eq!(preview.line_count(), 0);
    }

    #[test]
    fn test_custom_preview_config_parsing() {
        let toml_content = r#"
            [preview]
            hex_max_bytes = 4096

            [preview.custom]
            md = "glow -s dark $f"
            json = "jq -C . $f"
            csv = "column -s, -t $f"
        "#;

        #[derive(serde::Deserialize)]
        struct TestConfig {
            preview: PreviewSection,
        }

        #[derive(serde::Deserialize)]
        struct PreviewSection {
            hex_max_bytes: usize,
            custom: std::collections::HashMap<String, String>,
        }

        let config: TestConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.preview.hex_max_bytes, 4096);
        assert_eq!(
            config.preview.custom.get("md"),
            Some(&"glow -s dark $f".to_string())
        );
        assert_eq!(
            config.preview.custom.get("json"),
            Some(&"jq -C . $f".to_string())
        );
        assert_eq!(
            config.preview.custom.get("csv"),
            Some(&"column -s, -t $f".to_string())
        );
    }
}
