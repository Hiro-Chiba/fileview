//! Keymap configuration and registry
//!
//! Loads key bindings from `~/.config/fileview/keymap.toml`

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::key::KeyAction;
use crate::app::ConfigFile;

/// Keymap configuration file structure
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct KeymapFile {
    /// Key bindings for browse mode
    pub browse: HashMap<String, String>,
    /// Key bindings for preview mode
    pub preview: HashMap<String, String>,
    /// Key bindings for search mode
    pub search: HashMap<String, String>,
    /// Key bindings for confirm mode
    pub confirm: HashMap<String, String>,
    /// Key bindings for fuzzy finder mode
    pub fuzzy: HashMap<String, String>,
    /// Key bindings for help mode
    pub help: HashMap<String, String>,
    /// Key bindings for filter mode
    pub filter: HashMap<String, String>,
}

impl KeymapFile {
    /// Get the keymap file path (~/.config/fileview/keymap.toml)
    pub fn keymap_path() -> Option<PathBuf> {
        ConfigFile::config_dir().map(|p| p.join("keymap.toml"))
    }

    /// Load keymap from file
    pub fn load() -> Self {
        Self::keymap_path()
            .and_then(|path| {
                if path.exists() {
                    fs::read_to_string(&path).ok()
                } else {
                    None
                }
            })
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or_default()
    }
}

/// Key binding registry for dynamic key dispatch
pub struct KeyBindingRegistry {
    /// Browse mode bindings: key_str -> action_name
    browse: HashMap<String, String>,
    /// Preview mode bindings
    preview: HashMap<String, String>,
    /// Search mode bindings
    search: HashMap<String, String>,
    /// Confirm mode bindings
    confirm: HashMap<String, String>,
    /// Fuzzy finder mode bindings
    fuzzy: HashMap<String, String>,
    /// Help mode bindings
    help: HashMap<String, String>,
    /// Filter mode bindings
    filter: HashMap<String, String>,
}

impl Default for KeyBindingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyBindingRegistry {
    /// Create a new registry with default bindings
    pub fn new() -> Self {
        let mut registry = Self {
            browse: HashMap::new(),
            preview: HashMap::new(),
            search: HashMap::new(),
            confirm: HashMap::new(),
            fuzzy: HashMap::new(),
            help: HashMap::new(),
            filter: HashMap::new(),
        };
        registry.load_defaults();
        registry
    }

    /// Load registry from keymap file, merging with defaults
    pub fn from_file() -> Self {
        let mut registry = Self::new();
        let keymap = KeymapFile::load();

        // Merge user bindings (override defaults)
        for (key, action) in keymap.browse {
            registry.browse.insert(key, action);
        }
        for (key, action) in keymap.preview {
            registry.preview.insert(key, action);
        }
        for (key, action) in keymap.search {
            registry.search.insert(key, action);
        }
        for (key, action) in keymap.confirm {
            registry.confirm.insert(key, action);
        }
        for (key, action) in keymap.fuzzy {
            registry.fuzzy.insert(key, action);
        }
        for (key, action) in keymap.help {
            registry.help.insert(key, action);
        }
        for (key, action) in keymap.filter {
            registry.filter.insert(key, action);
        }

        registry
    }

    /// Load default key bindings
    fn load_defaults(&mut self) {
        // Browse mode defaults
        let browse = &mut self.browse;
        browse.insert("q".to_string(), "quit".to_string());
        browse.insert("Q".to_string(), "quit_and_cd".to_string());
        browse.insert("esc".to_string(), "cancel_or_clear".to_string());
        browse.insert("up".to_string(), "move_up".to_string());
        browse.insert("k".to_string(), "move_up".to_string());
        browse.insert("down".to_string(), "move_down".to_string());
        browse.insert("j".to_string(), "move_down".to_string());
        browse.insert("g".to_string(), "move_to_top".to_string());
        browse.insert("G".to_string(), "move_to_bottom".to_string());
        browse.insert("right".to_string(), "expand_or_focus".to_string());
        browse.insert("l".to_string(), "expand".to_string());
        browse.insert("left".to_string(), "collapse_or_focus".to_string());
        browse.insert("h".to_string(), "collapse".to_string());
        browse.insert("backspace".to_string(), "collapse".to_string());
        browse.insert("tab".to_string(), "toggle_focus_or_expand".to_string());
        browse.insert("H".to_string(), "collapse_all".to_string());
        browse.insert("L".to_string(), "expand_all".to_string());
        browse.insert("space".to_string(), "toggle_mark".to_string());
        browse.insert("enter".to_string(), "pick_or_toggle".to_string());
        browse.insert("y".to_string(), "copy".to_string());
        browse.insert("d".to_string(), "cut".to_string());
        browse.insert("D".to_string(), "confirm_delete".to_string());
        browse.insert("delete".to_string(), "confirm_delete".to_string());
        browse.insert("ctrl+p".to_string(), "open_fuzzy_finder".to_string());
        browse.insert("p".to_string(), "paste".to_string());
        browse.insert("r".to_string(), "start_rename".to_string());
        browse.insert("a".to_string(), "start_new_file".to_string());
        browse.insert("A".to_string(), "start_new_dir".to_string());
        browse.insert("/".to_string(), "start_search".to_string());
        browse.insert("n".to_string(), "search_next".to_string());
        browse.insert("N".to_string(), "search_prev".to_string());
        browse.insert("S".to_string(), "cycle_sort".to_string());
        browse.insert("R".to_string(), "refresh_or_bulk_rename".to_string());
        browse.insert("f5".to_string(), "refresh".to_string());
        browse.insert(".".to_string(), "toggle_hidden".to_string());
        browse.insert("c".to_string(), "copy_path".to_string());
        browse.insert("C".to_string(), "copy_filename".to_string());
        browse.insert("o".to_string(), "open_preview".to_string());
        browse.insert("P".to_string(), "toggle_quick_preview".to_string());
        browse.insert("?".to_string(), "show_help".to_string());
        browse.insert("[".to_string(), "pdf_prev_page".to_string());
        browse.insert("]".to_string(), "pdf_next_page".to_string());
        browse.insert("m".to_string(), "start_bookmark_set".to_string());
        browse.insert("'".to_string(), "start_bookmark_jump".to_string());
        browse.insert("F".to_string(), "toggle_filter".to_string());
        browse.insert("s".to_string(), "git_stage".to_string());
        browse.insert("u".to_string(), "git_unstage".to_string());
        browse.insert("ctrl+t".to_string(), "new_tab".to_string());
        browse.insert("ctrl+w".to_string(), "close_tab".to_string());
        browse.insert("alt+t".to_string(), "next_tab".to_string());
        browse.insert("alt+T".to_string(), "prev_tab".to_string());
        browse.insert("pageup".to_string(), "preview_page_up".to_string());
        browse.insert("pagedown".to_string(), "preview_page_down".to_string());
        browse.insert("b".to_string(), "preview_page_up_if_preview".to_string());
        browse.insert("f".to_string(), "preview_page_down_if_preview".to_string());

        // Preview mode defaults
        let preview = &mut self.preview;
        preview.insert("esc".to_string(), "cancel".to_string());
        preview.insert("q".to_string(), "cancel".to_string());
        preview.insert("o".to_string(), "cancel".to_string());
        preview.insert("enter".to_string(), "cancel".to_string());
        preview.insert("up".to_string(), "scroll_up".to_string());
        preview.insert("k".to_string(), "scroll_up".to_string());
        preview.insert("down".to_string(), "scroll_down".to_string());
        preview.insert("j".to_string(), "scroll_down".to_string());
        preview.insert("pageup".to_string(), "page_up".to_string());
        preview.insert("b".to_string(), "page_up".to_string());
        preview.insert("pagedown".to_string(), "page_down".to_string());
        preview.insert("f".to_string(), "page_down".to_string());
        preview.insert("space".to_string(), "page_down".to_string());
        preview.insert("g".to_string(), "to_top".to_string());
        preview.insert("G".to_string(), "to_bottom".to_string());
        preview.insert("[".to_string(), "pdf_prev_page".to_string());
        preview.insert("]".to_string(), "pdf_next_page".to_string());

        // Search mode defaults
        let search = &mut self.search;
        search.insert("enter".to_string(), "confirm".to_string());
        search.insert("/".to_string(), "cancel".to_string());
        search.insert("esc".to_string(), "cancel".to_string());

        // Confirm mode defaults
        let confirm = &mut self.confirm;
        confirm.insert("y".to_string(), "execute".to_string());
        confirm.insert("Y".to_string(), "execute".to_string());
        confirm.insert("enter".to_string(), "execute".to_string());
        confirm.insert("n".to_string(), "cancel".to_string());
        confirm.insert("N".to_string(), "cancel".to_string());
        confirm.insert("esc".to_string(), "cancel".to_string());

        // Fuzzy finder defaults
        let fuzzy = &mut self.fuzzy;
        fuzzy.insert("esc".to_string(), "cancel".to_string());
        fuzzy.insert("ctrl+p".to_string(), "cancel".to_string());
        fuzzy.insert("up".to_string(), "up".to_string());
        fuzzy.insert("ctrl+k".to_string(), "up".to_string());
        fuzzy.insert("down".to_string(), "down".to_string());
        fuzzy.insert("ctrl+j".to_string(), "down".to_string());
        fuzzy.insert("enter".to_string(), "confirm".to_string());

        // Help mode defaults
        let help = &mut self.help;
        help.insert("esc".to_string(), "cancel".to_string());
        help.insert("enter".to_string(), "cancel".to_string());
        help.insert("q".to_string(), "cancel".to_string());
        help.insert("?".to_string(), "cancel".to_string());

        // Filter mode defaults
        let filter = &mut self.filter;
        filter.insert("enter".to_string(), "apply".to_string());
        filter.insert("F".to_string(), "cancel".to_string());
        filter.insert("esc".to_string(), "cancel".to_string());
    }

    /// Look up action for a key event in browse mode
    pub fn lookup_browse(&self, key: &KeyEvent) -> Option<KeyAction> {
        let key_str = key_event_to_string(key);
        self.browse
            .get(&key_str)
            .and_then(|action| parse_browse_action(action))
    }

    /// Look up action for a key event in preview mode
    pub fn lookup_preview(&self, key: &KeyEvent) -> Option<KeyAction> {
        let key_str = key_event_to_string(key);
        self.preview
            .get(&key_str)
            .and_then(|action| parse_preview_action(action))
    }

    /// Look up action for a key event in search mode
    pub fn lookup_search(&self, key: &KeyEvent) -> Option<KeyAction> {
        let key_str = key_event_to_string(key);
        self.search
            .get(&key_str)
            .and_then(|action| match action.as_str() {
                "confirm" => Some(KeyAction::ConfirmInput {
                    value: String::new(),
                }),
                "cancel" => Some(KeyAction::Cancel),
                _ => None,
            })
    }

    /// Look up action for a key event in confirm mode
    pub fn lookup_confirm(&self, key: &KeyEvent) -> Option<KeyAction> {
        let key_str = key_event_to_string(key);
        self.confirm
            .get(&key_str)
            .and_then(|action| match action.as_str() {
                "execute" => Some(KeyAction::ExecuteDelete),
                "cancel" => Some(KeyAction::Cancel),
                _ => None,
            })
    }

    /// Look up action for a key event in fuzzy finder mode
    pub fn lookup_fuzzy(&self, key: &KeyEvent) -> Option<KeyAction> {
        let key_str = key_event_to_string(key);
        self.fuzzy
            .get(&key_str)
            .and_then(|action| match action.as_str() {
                "cancel" => Some(KeyAction::Cancel),
                "up" => Some(KeyAction::FuzzyUp),
                "down" => Some(KeyAction::FuzzyDown),
                "confirm" => Some(KeyAction::FuzzyConfirm {
                    path: PathBuf::new(),
                }),
                _ => None,
            })
    }

    /// Look up action for a key event in help mode
    pub fn lookup_help(&self, key: &KeyEvent) -> Option<KeyAction> {
        let key_str = key_event_to_string(key);
        self.help
            .get(&key_str)
            .and_then(|action| match action.as_str() {
                "cancel" => Some(KeyAction::Cancel),
                _ => None,
            })
    }

    /// Look up action for a key event in filter mode
    pub fn lookup_filter(&self, key: &KeyEvent) -> Option<KeyAction> {
        let key_str = key_event_to_string(key);
        self.filter
            .get(&key_str)
            .and_then(|action| match action.as_str() {
                "apply" => Some(KeyAction::ApplyFilter {
                    pattern: String::new(),
                }),
                "cancel" => Some(KeyAction::Cancel),
                _ => None,
            })
    }
}

/// Convert a KeyEvent to a string representation
fn key_event_to_string(key: &KeyEvent) -> String {
    let mut parts = Vec::new();

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("ctrl");
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        parts.push("alt");
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        // Only add shift for non-character keys or when combined with ctrl/alt
        if !matches!(key.code, KeyCode::Char(_)) || !parts.is_empty() {
            parts.push("shift");
        }
    }

    let key_name = match key.code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::F(n) => format!("f{}", n),
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Left => "left".to_string(),
        KeyCode::Right => "right".to_string(),
        KeyCode::Up => "up".to_string(),
        KeyCode::Down => "down".to_string(),
        KeyCode::Home => "home".to_string(),
        KeyCode::End => "end".to_string(),
        KeyCode::PageUp => "pageup".to_string(),
        KeyCode::PageDown => "pagedown".to_string(),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::Delete => "delete".to_string(),
        KeyCode::Insert => "insert".to_string(),
        KeyCode::Esc => "esc".to_string(),
        _ => return String::new(),
    };

    if parts.is_empty() {
        key_name
    } else {
        parts.push(&key_name);
        parts.join("+")
    }
}

/// Parse browse mode action string to KeyAction
fn parse_browse_action(action: &str) -> Option<KeyAction> {
    match action {
        "quit" => Some(KeyAction::Quit),
        "quit_and_cd" => Some(KeyAction::QuitAndCd),
        "cancel" | "cancel_or_clear" => Some(KeyAction::Cancel),
        "move_up" => Some(KeyAction::MoveUp),
        "move_down" => Some(KeyAction::MoveDown),
        "move_to_top" => Some(KeyAction::MoveToTop),
        "move_to_bottom" => Some(KeyAction::MoveToBottom),
        "expand" | "expand_or_focus" => Some(KeyAction::Expand),
        "collapse" | "collapse_or_focus" => Some(KeyAction::Collapse),
        "toggle_expand" | "toggle_focus_or_expand" => Some(KeyAction::ToggleExpand),
        "collapse_all" => Some(KeyAction::CollapseAll),
        "expand_all" => Some(KeyAction::ExpandAll),
        "toggle_mark" => Some(KeyAction::ToggleMark),
        "clear_marks" => Some(KeyAction::ClearMarks),
        "copy" => Some(KeyAction::Copy),
        "cut" => Some(KeyAction::Cut),
        "paste" => Some(KeyAction::Paste),
        "confirm_delete" => Some(KeyAction::ConfirmDelete),
        "start_rename" => Some(KeyAction::StartRename),
        "start_new_file" => Some(KeyAction::StartNewFile),
        "start_new_dir" => Some(KeyAction::StartNewDir),
        "start_search" => Some(KeyAction::StartSearch),
        "search_next" => Some(KeyAction::SearchNext),
        "search_prev" => Some(KeyAction::SearchPrev),
        "refresh" | "refresh_or_bulk_rename" => Some(KeyAction::Refresh),
        "toggle_hidden" => Some(KeyAction::ToggleHidden),
        "copy_path" => Some(KeyAction::CopyPath),
        "copy_filename" => Some(KeyAction::CopyFilename),
        "open_preview" => Some(KeyAction::OpenPreview),
        "toggle_quick_preview" => Some(KeyAction::ToggleQuickPreview),
        "show_help" => Some(KeyAction::ShowHelp),
        "toggle_focus" => Some(KeyAction::ToggleFocus),
        "focus_tree" => Some(KeyAction::FocusTree),
        "focus_preview" => Some(KeyAction::FocusPreview),
        "open_fuzzy_finder" => Some(KeyAction::OpenFuzzyFinder),
        "start_bookmark_set" => Some(KeyAction::StartBookmarkSet),
        "start_bookmark_jump" => Some(KeyAction::StartBookmarkJump),
        "start_filter" | "toggle_filter" => Some(KeyAction::StartFilter),
        "clear_filter" => Some(KeyAction::ClearFilter),
        "cycle_sort" => Some(KeyAction::CycleSort),
        "pdf_prev_page" => Some(KeyAction::PdfPrevPage),
        "pdf_next_page" => Some(KeyAction::PdfNextPage),
        "git_stage" => Some(KeyAction::GitStage),
        "git_unstage" => Some(KeyAction::GitUnstage),
        "start_bulk_rename" => Some(KeyAction::StartBulkRename),
        "new_tab" => Some(KeyAction::NewTab),
        "close_tab" => Some(KeyAction::CloseTab),
        "next_tab" => Some(KeyAction::NextTab),
        "prev_tab" => Some(KeyAction::PrevTab),
        "pick_select" | "pick_or_toggle" => Some(KeyAction::PickSelect),
        "preview_scroll_up" => Some(KeyAction::PreviewScrollUp),
        "preview_scroll_down" => Some(KeyAction::PreviewScrollDown),
        "preview_page_up" | "preview_page_up_if_preview" => Some(KeyAction::PreviewPageUp),
        "preview_page_down" | "preview_page_down_if_preview" => Some(KeyAction::PreviewPageDown),
        "preview_to_top" => Some(KeyAction::PreviewToTop),
        "preview_to_bottom" => Some(KeyAction::PreviewToBottom),
        _ => {
            // Check for command:name pattern
            action
                .strip_prefix("command:")
                .map(|name| KeyAction::RunCommand {
                    name: name.to_string(),
                })
        }
    }
}

/// Parse preview mode action string to KeyAction
fn parse_preview_action(action: &str) -> Option<KeyAction> {
    match action {
        "cancel" => Some(KeyAction::Cancel),
        "scroll_up" => Some(KeyAction::PreviewScrollUp),
        "scroll_down" => Some(KeyAction::PreviewScrollDown),
        "page_up" => Some(KeyAction::PreviewPageUp),
        "page_down" => Some(KeyAction::PreviewPageDown),
        "to_top" => Some(KeyAction::PreviewToTop),
        "to_bottom" => Some(KeyAction::PreviewToBottom),
        "pdf_prev_page" => Some(KeyAction::PdfPrevPage),
        "pdf_next_page" => Some(KeyAction::PdfNextPage),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_event_to_string_simple() {
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
        assert_eq!(key_event_to_string(&key), "j");
    }

    #[test]
    fn test_key_event_to_string_uppercase() {
        let key = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);
        assert_eq!(key_event_to_string(&key), "G");
    }

    #[test]
    fn test_key_event_to_string_ctrl() {
        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        assert_eq!(key_event_to_string(&key), "ctrl+p");
    }

    #[test]
    fn test_key_event_to_string_alt() {
        let key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::ALT);
        assert_eq!(key_event_to_string(&key), "alt+t");
    }

    #[test]
    fn test_key_event_to_string_special() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        assert_eq!(key_event_to_string(&key), "enter");

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        assert_eq!(key_event_to_string(&key), "esc");

        let key = KeyEvent::new(KeyCode::F(5), KeyModifiers::empty());
        assert_eq!(key_event_to_string(&key), "f5");
    }

    #[test]
    fn test_default_registry() {
        let registry = KeyBindingRegistry::new();
        assert!(registry.browse.contains_key("j"));
        assert!(registry.browse.contains_key("q"));
        assert!(registry.preview.contains_key("esc"));
    }

    #[test]
    fn test_lookup_browse() {
        let registry = KeyBindingRegistry::new();
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
        let action = registry.lookup_browse(&key);
        assert!(matches!(action, Some(KeyAction::MoveDown)));
    }

    #[test]
    fn test_lookup_browse_ctrl() {
        let registry = KeyBindingRegistry::new();
        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let action = registry.lookup_browse(&key);
        assert!(matches!(action, Some(KeyAction::OpenFuzzyFinder)));
    }

    #[test]
    fn test_keymap_file_parse() {
        let toml_content = r#"
[browse]
"x" = "quit"
"ctrl+x" = "copy"
"#;
        let keymap: KeymapFile = toml::from_str(toml_content).unwrap();
        assert_eq!(keymap.browse.get("x"), Some(&"quit".to_string()));
        assert_eq!(keymap.browse.get("ctrl+x"), Some(&"copy".to_string()));
    }

    #[test]
    fn test_registry_merge() {
        // Test that user bindings override defaults
        let mut registry = KeyBindingRegistry::new();
        registry.browse.insert("j".to_string(), "quit".to_string()); // Override default

        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
        let action = registry.lookup_browse(&key);
        assert!(matches!(action, Some(KeyAction::Quit)));
    }
}
