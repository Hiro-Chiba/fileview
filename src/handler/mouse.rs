//! Mouse and keyboard input handling for file operations

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Timing constants for input detection
const RAPID_INPUT_THRESHOLD_MS: u64 = 50;
const INPUT_TIMEOUT_MS: u64 = 100;

/// Actions triggered by mouse events
#[derive(Debug, Clone)]
pub enum MouseAction {
    None,
    Click { row: u16, col: u16 },
    DoubleClick { row: u16, col: u16 },
    ScrollUp { amount: usize, col: u16 },
    ScrollDown { amount: usize, col: u16 },
    FileDrop { paths: Vec<PathBuf> },
}

/// Detects double-clicks by tracking click timing
pub struct ClickDetector {
    last_click: Option<(Instant, u16)>,
    threshold: Duration,
}

impl Default for ClickDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ClickDetector {
    pub fn new() -> Self {
        Self {
            last_click: None,
            threshold: Duration::from_millis(500),
        }
    }

    /// Returns true if this click forms a double-click
    pub fn click(&mut self, row: u16) -> bool {
        let now = Instant::now();
        let is_double = self
            .last_click
            .map(|(t, r)| r == row && now.duration_since(t) < self.threshold)
            .unwrap_or(false);

        self.last_click = if is_double { None } else { Some((now, row)) };
        is_double
    }
}

/// Buffers rapid keyboard input to detect file paths from drag-and-drop
///
/// Some terminals (e.g., Ghostty) send dropped file paths as rapid key events
/// rather than paste events. This buffer collects rapid input and extracts
/// valid file paths after a brief timeout.
pub struct PathBuffer {
    data: String,
    last_input: Option<Instant>,
}

impl Default for PathBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl PathBuffer {
    pub fn new() -> Self {
        Self {
            data: String::new(),
            last_input: None,
        }
    }

    /// Add a character to the buffer, resetting if input is too slow
    pub fn push(&mut self, c: char) {
        let now = Instant::now();
        let is_rapid = self
            .last_input
            .map(|t| now.duration_since(t) < Duration::from_millis(RAPID_INPUT_THRESHOLD_MS))
            .unwrap_or(true);

        if !is_rapid {
            self.data.clear();
        }

        self.data.push(c);
        self.last_input = Some(now);
    }

    /// Check if buffer has content ready to process (input has paused)
    pub fn is_ready(&self) -> bool {
        !self.data.is_empty()
            && self
                .last_input
                .map(|t| t.elapsed() >= Duration::from_millis(INPUT_TIMEOUT_MS))
                .unwrap_or(false)
    }

    /// Extract valid file paths from buffer, consuming the content
    pub fn take_paths(&mut self) -> Vec<PathBuf> {
        let content = self.take_raw();
        parse_paths(&content)
    }

    /// Take raw buffer content for fallback processing
    pub fn take_raw(&mut self) -> String {
        self.last_input = None;
        std::mem::take(&mut self.data)
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.data.clear();
        self.last_input = None;
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// View current buffer content
    pub fn content(&self) -> &str {
        &self.data
    }
}

/// Normalize a shell-style path by handling quotes and escape sequences
fn normalize_shell_path(input: &str) -> String {
    let s = input.trim();

    // Strip surrounding quotes
    let s = match (s.chars().next(), s.chars().last()) {
        (Some(q @ ('"' | '\'')), Some(end)) if q == end && s.len() > 1 => &s[1..s.len() - 1],
        _ => s,
    };

    // Process escape sequences and URL encoding
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if " '\"\\()[]&;!$`".contains(next) {
                    result.push(chars.next().unwrap());
                    continue;
                }
            }
        }
        result.push(c);
    }

    // URL decode common sequences
    result
        .replace("%20", " ")
        .replace("%23", "#")
        .replace("%25", "%")
        .replace("%5B", "[")
        .replace("%5D", "]")
}

/// Parse input text into valid, existing file paths
fn parse_paths(content: &str) -> Vec<PathBuf> {
    let content = content.trim();
    if content.is_empty() {
        return Vec::new();
    }

    // Handle newline-separated paths (multiple files)
    if content.contains('\n') {
        return content.lines().filter_map(to_path).collect();
    }

    // Handle single path or space-separated paths with quote awareness
    let mut paths = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut quote_char = ' ';
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' | '\'' if !in_quote => {
                in_quote = true;
                quote_char = c;
            }
            c if in_quote && c == quote_char => {
                in_quote = false;
            }
            '\\' if !in_quote => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            ' ' if !in_quote => {
                if let Some(path) = to_path(&current) {
                    paths.push(path);
                }
                current.clear();
            }
            _ => current.push(c),
        }
    }

    if let Some(path) = to_path(&current) {
        paths.push(path);
    }

    paths
}

/// Convert a string to a valid, existing absolute path
fn to_path(s: &str) -> Option<PathBuf> {
    let normalized = normalize_shell_path(s);
    if normalized.is_empty() {
        return None;
    }

    // Handle file:// URLs
    let path_str = normalized
        .strip_prefix("file://")
        .map(|s| {
            // Windows: file:///C:/... -> C:/...
            if s.len() > 2 && s.starts_with('/') && s.chars().nth(2) == Some(':') {
                &s[1..]
            } else {
                s
            }
        })
        .unwrap_or(&normalized);

    let path = PathBuf::from(path_str);
    (path.is_absolute() && path.exists()).then_some(path)
}

/// Process a mouse event and return the resulting action
pub fn handle_mouse_event(
    event: MouseEvent,
    click_detector: &mut ClickDetector,
    tree_area_top: u16,
) -> MouseAction {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) if event.row > tree_area_top => {
            let row = event.row - tree_area_top - 1;
            let col = event.column;
            if click_detector.click(row) {
                MouseAction::DoubleClick { row, col }
            } else {
                MouseAction::Click { row, col }
            }
        }
        MouseEventKind::ScrollUp => MouseAction::ScrollUp {
            amount: 3,
            col: event.column,
        },
        MouseEventKind::ScrollDown => MouseAction::ScrollDown {
            amount: 3,
            col: event.column,
        },
        _ => MouseAction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // ClickDetector tests
    // ========================================

    #[test]
    fn click_detector_single() {
        let mut d = ClickDetector::new();
        assert!(!d.click(5));
    }

    #[test]
    fn click_detector_double() {
        let mut d = ClickDetector::new();
        assert!(!d.click(5));
        assert!(d.click(5));
    }

    #[test]
    fn click_detector_different_rows() {
        let mut d = ClickDetector::new();
        assert!(!d.click(5));
        assert!(!d.click(6));
    }

    #[test]
    fn click_detector_resets_after_double_click() {
        let mut d = ClickDetector::new();
        assert!(!d.click(5)); // First click
        assert!(d.click(5)); // Double click
        assert!(!d.click(5)); // Should be single click again (reset)
    }

    #[test]
    fn click_detector_default_creates_new() {
        let d1 = ClickDetector::default();
        let d2 = ClickDetector::new();
        // Both should have no last click recorded
        assert!(d1.last_click.is_none());
        assert!(d2.last_click.is_none());
    }

    #[test]
    fn click_detector_tracks_row() {
        let mut d = ClickDetector::new();
        d.click(10);
        assert!(d.last_click.is_some());
        let (_, row) = d.last_click.unwrap();
        assert_eq!(row, 10);
    }

    // ========================================
    // PathBuffer tests
    // ========================================

    #[test]
    fn path_buffer_empty() {
        let buf = PathBuffer::new();
        assert!(buf.is_empty());
        assert!(!buf.is_ready());
    }

    #[test]
    fn path_buffer_push() {
        let mut buf = PathBuffer::new();
        buf.push('/');
        assert!(!buf.is_empty());
        assert_eq!(buf.content(), "/");
    }

    #[test]
    fn path_buffer_not_ready_immediately() {
        let mut buf = PathBuffer::new();
        buf.push('/');
        buf.push('t');
        assert!(!buf.is_ready()); // Needs timeout
    }

    #[test]
    fn path_buffer_default_creates_new() {
        let b1 = PathBuffer::default();
        let b2 = PathBuffer::new();
        assert!(b1.is_empty());
        assert!(b2.is_empty());
    }

    #[test]
    fn path_buffer_clear() {
        let mut buf = PathBuffer::new();
        buf.push('/');
        buf.push('t');
        assert!(!buf.is_empty());
        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.content(), "");
    }

    #[test]
    fn path_buffer_take_raw() {
        let mut buf = PathBuffer::new();
        buf.push('/');
        buf.push('t');
        buf.push('m');
        buf.push('p');
        let content = buf.take_raw();
        assert_eq!(content, "/tmp");
        assert!(buf.is_empty());
    }

    #[test]
    fn path_buffer_take_paths_with_valid_path() {
        let mut buf = PathBuffer::new();
        let cwd = std::env::current_dir().unwrap();
        for c in cwd.display().to_string().chars() {
            buf.push(c);
        }
        let paths = buf.take_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], cwd);
        assert!(buf.is_empty());
    }

    #[test]
    fn path_buffer_take_paths_with_invalid_path() {
        let mut buf = PathBuffer::new();
        for c in "/nonexistent/path/xyz".chars() {
            buf.push(c);
        }
        let paths = buf.take_paths();
        assert!(paths.is_empty());
    }

    #[test]
    fn path_buffer_content_returns_current_buffer() {
        let mut buf = PathBuffer::new();
        buf.push('a');
        buf.push('b');
        buf.push('c');
        assert_eq!(buf.content(), "abc");
    }

    // ========================================
    // normalize_shell_path tests
    // ========================================

    #[test]
    fn normalize_quoted_path() {
        assert_eq!(normalize_shell_path("\"hello\""), "hello");
        assert_eq!(normalize_shell_path("'hello'"), "hello");
    }

    #[test]
    fn normalize_escaped_spaces() {
        assert_eq!(normalize_shell_path("hello\\ world"), "hello world");
    }

    #[test]
    fn normalize_url_encoded() {
        assert_eq!(normalize_shell_path("hello%20world"), "hello world");
    }

    #[test]
    fn normalize_empty_string() {
        assert_eq!(normalize_shell_path(""), "");
        assert_eq!(normalize_shell_path("  "), "");
    }

    #[test]
    fn normalize_mismatched_quotes_unchanged() {
        // Mismatched quotes should not be stripped
        assert_eq!(normalize_shell_path("\"hello'"), "\"hello'");
    }

    #[test]
    fn normalize_escaped_special_chars() {
        assert_eq!(normalize_shell_path("a\\&b"), "a&b");
        assert_eq!(normalize_shell_path("a\\;b"), "a;b");
        assert_eq!(normalize_shell_path("a\\$b"), "a$b");
    }

    #[test]
    fn normalize_url_encoded_hash() {
        assert_eq!(normalize_shell_path("file%23name"), "file#name");
    }

    #[test]
    fn normalize_url_encoded_brackets() {
        assert_eq!(normalize_shell_path("file%5Bname%5D"), "file[name]");
    }

    // ========================================
    // parse_paths tests
    // ========================================

    #[test]
    fn parse_existing_path() {
        let cwd = std::env::current_dir().unwrap();
        let paths = parse_paths(&cwd.display().to_string());
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], cwd);
    }

    #[test]
    fn parse_multiple_paths() {
        let cwd = std::env::current_dir().unwrap();
        let input = format!("{}\n{}", cwd.display(), cwd.display());
        let paths = parse_paths(&input);
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn parse_nonexistent_filtered() {
        let paths = parse_paths("/nonexistent/path/xyz");
        assert!(paths.is_empty());
    }

    #[test]
    fn parse_file_url() {
        let cwd = std::env::current_dir().unwrap();
        let input = format!("file://{}", cwd.display());
        let paths = parse_paths(&input);
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn parse_empty_string() {
        let paths = parse_paths("");
        assert!(paths.is_empty());
    }

    #[test]
    fn parse_whitespace_only() {
        let paths = parse_paths("   \n  \n  ");
        assert!(paths.is_empty());
    }

    #[test]
    fn parse_mixed_valid_invalid_paths() {
        let cwd = std::env::current_dir().unwrap();
        let input = format!("{}\n/nonexistent/xyz", cwd.display());
        let paths = parse_paths(&input);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], cwd);
    }

    // ========================================
    // handle_mouse_event tests
    // ========================================

    #[test]
    fn handle_mouse_event_click_in_tree_area() {
        let mut detector = ClickDetector::new();
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 5, // tree_area_top = 2, so row > 2
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let action = handle_mouse_event(event, &mut detector, 2);
        match action {
            MouseAction::Click { row, col } => {
                assert_eq!(row, 2); // 5 - 2 - 1 = 2
                assert_eq!(col, 10);
            }
            _ => panic!("Expected Click action"),
        }
    }

    #[test]
    fn handle_mouse_event_double_click() {
        let mut detector = ClickDetector::new();
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        // First click
        handle_mouse_event(event, &mut detector, 2);
        // Second click (same position)
        let action = handle_mouse_event(event, &mut detector, 2);
        match action {
            MouseAction::DoubleClick { row, col } => {
                assert_eq!(row, 2);
                assert_eq!(col, 10);
            }
            _ => panic!("Expected DoubleClick action"),
        }
    }

    #[test]
    fn handle_mouse_event_click_above_tree_area() {
        let mut detector = ClickDetector::new();
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 2, // tree_area_top = 2, row is not > 2
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let action = handle_mouse_event(event, &mut detector, 2);
        assert!(matches!(action, MouseAction::None));
    }

    #[test]
    fn handle_mouse_event_scroll_up() {
        let mut detector = ClickDetector::new();
        let event = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 5,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let action = handle_mouse_event(event, &mut detector, 2);
        match action {
            MouseAction::ScrollUp { amount, col } => {
                assert_eq!(amount, 3);
                assert_eq!(col, 5);
            }
            _ => panic!("Expected ScrollUp action"),
        }
    }

    #[test]
    fn handle_mouse_event_scroll_down() {
        let mut detector = ClickDetector::new();
        let event = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 8,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let action = handle_mouse_event(event, &mut detector, 2);
        match action {
            MouseAction::ScrollDown { amount, col } => {
                assert_eq!(amount, 3);
                assert_eq!(col, 8);
            }
            _ => panic!("Expected ScrollDown action"),
        }
    }

    #[test]
    fn handle_mouse_event_right_click_returns_none() {
        let mut detector = ClickDetector::new();
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 10,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let action = handle_mouse_event(event, &mut detector, 2);
        assert!(matches!(action, MouseAction::None));
    }

    #[test]
    fn handle_mouse_event_middle_click_returns_none() {
        let mut detector = ClickDetector::new();
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Middle),
            column: 10,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let action = handle_mouse_event(event, &mut detector, 2);
        assert!(matches!(action, MouseAction::None));
    }

    #[test]
    fn handle_mouse_event_drag_returns_none() {
        let mut detector = ClickDetector::new();
        let event = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 10,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let action = handle_mouse_event(event, &mut detector, 2);
        assert!(matches!(action, MouseAction::None));
    }

    #[test]
    fn handle_mouse_event_mouse_up_returns_none() {
        let mut detector = ClickDetector::new();
        let event = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 10,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let action = handle_mouse_event(event, &mut detector, 2);
        assert!(matches!(action, MouseAction::None));
    }

    // ========================================
    // MouseAction tests
    // ========================================

    #[test]
    fn mouse_action_debug_format() {
        let action = MouseAction::Click { row: 1, col: 2 };
        let debug = format!("{:?}", action);
        assert!(debug.contains("Click"));
    }

    #[test]
    fn mouse_action_clone() {
        let action = MouseAction::ScrollUp { amount: 3, col: 5 };
        let cloned = action.clone();
        match cloned {
            MouseAction::ScrollUp { amount, col } => {
                assert_eq!(amount, 3);
                assert_eq!(col, 5);
            }
            _ => panic!("Clone failed"),
        }
    }

    #[test]
    fn mouse_action_file_drop() {
        let paths = vec![PathBuf::from("/tmp/test")];
        let action = MouseAction::FileDrop {
            paths: paths.clone(),
        };
        match action {
            MouseAction::FileDrop { paths: p } => {
                assert_eq!(p, paths);
            }
            _ => panic!("Expected FileDrop"),
        }
    }
}
