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
    Click { row: u16 },
    DoubleClick { row: u16 },
    ScrollUp(usize),
    ScrollDown(usize),
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
            if click_detector.click(row) {
                MouseAction::DoubleClick { row }
            } else {
                MouseAction::Click { row }
            }
        }
        MouseEventKind::ScrollUp => MouseAction::ScrollUp(3),
        MouseEventKind::ScrollDown => MouseAction::ScrollDown(3),
        _ => MouseAction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
