//! Mouse event handling and drag-and-drop detection

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Actions that can result from mouse handling
#[derive(Debug, Clone)]
pub enum MouseAction {
    /// No action needed
    None,
    /// Click on a tree entry (row index relative to tree area)
    Click { row: u16 },
    /// Double click on a tree entry
    DoubleClick { row: u16 },
    /// Scroll up by n lines
    ScrollUp(usize),
    /// Scroll down by n lines
    ScrollDown(usize),
    /// File dropped (drag and drop)
    FileDrop { paths: Vec<PathBuf> },
}

/// Double-click detector
pub struct ClickDetector {
    last_click: Option<(Instant, u16)>,
    double_click_threshold: Duration,
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
            double_click_threshold: Duration::from_millis(500),
        }
    }

    /// Register a click and return true if it's a double-click
    pub fn click(&mut self, row: u16) -> bool {
        let now = Instant::now();

        if let Some((last_time, last_row)) = self.last_click {
            if last_row == row && now.duration_since(last_time) < self.double_click_threshold {
                self.last_click = None;
                return true;
            }
        }

        self.last_click = Some((now, row));
        false
    }
}

/// Drag-and-drop detector for terminal file drops
/// Detects rapidly incoming characters that form file paths
pub struct DropDetector {
    buffer: String,
    last_char_time: Option<Instant>,
    char_threshold: Duration,
    timeout: Duration,
}

impl Default for DropDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl DropDetector {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            last_char_time: None,
            char_threshold: Duration::from_millis(50),
            timeout: Duration::from_millis(100),
        }
    }

    /// Add a character to the buffer
    /// Call this for each character input
    pub fn push_char(&mut self, c: char) {
        let now = Instant::now();
        let elapsed = self
            .last_char_time
            .map(|t| now.duration_since(t))
            .unwrap_or(Duration::MAX);

        // If more than 50ms since last char, start new buffer
        if elapsed > self.char_threshold {
            self.buffer.clear();
        }

        self.buffer.push(c);
        self.last_char_time = Some(now);
    }

    /// Check if buffer has timed out and should be processed
    /// Returns true if there's buffered content that has timed out
    pub fn check_timeout(&self) -> bool {
        if self.buffer.is_empty() {
            return false;
        }

        self.last_char_time
            .map(|t| t.elapsed() >= self.timeout)
            .unwrap_or(false)
    }

    /// Check if buffer contains valid paths and extract them
    /// Clears the buffer after extraction
    pub fn extract_paths(&mut self) -> Vec<PathBuf> {
        if self.buffer.is_empty() {
            return Vec::new();
        }

        let content = std::mem::take(&mut self.buffer);
        self.last_char_time = None;

        Self::parse_paths(&content)
    }

    /// Take the buffer content without parsing as paths
    /// Used for fallback processing (e.g., treating as search query)
    pub fn take_buffer(&mut self) -> String {
        self.last_char_time = None;
        std::mem::take(&mut self.buffer)
    }

    /// Normalize a dropped path by removing quotes and unescaping backslashes
    fn normalize_path(text: &str) -> String {
        let text = text.trim();

        // Remove surrounding quotes if present
        let text = if (text.starts_with('\'') && text.ends_with('\''))
            || (text.starts_with('"') && text.ends_with('"'))
        {
            &text[1..text.len().saturating_sub(1)]
        } else {
            text
        };

        // Unescape backslash-escaped characters
        let mut result = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(&next) = chars.peek() {
                    // Common escaped characters in shell paths
                    if matches!(
                        next,
                        ' ' | '\''
                            | '"'
                            | '\\'
                            | '('
                            | ')'
                            | '['
                            | ']'
                            | '&'
                            | ';'
                            | '!'
                            | '$'
                            | '`'
                    ) {
                        result.push(chars.next().unwrap());
                        continue;
                    }
                }
            }
            result.push(c);
        }

        // URL decode common characters
        result
            .replace("%20", " ")
            .replace("%23", "#")
            .replace("%25", "%")
            .replace("%5B", "[")
            .replace("%5D", "]")
    }

    /// Parse text content into valid file paths
    fn parse_paths(content: &str) -> Vec<PathBuf> {
        let content = content.trim();

        // Try newline-separated first (multiple files)
        if content.contains('\n') {
            return content
                .lines()
                .filter_map(|line| {
                    let normalized = Self::normalize_path(line);
                    if normalized.is_empty() {
                        return None;
                    }

                    // Handle file:// URLs
                    let path_str = if let Some(stripped) = normalized.strip_prefix("file://") {
                        let mut s = stripped.to_string();
                        // Windows file URLs: file:///C:/...
                        if s.len() > 2 && s.starts_with('/') && s.chars().nth(2) == Some(':') {
                            s = s[1..].to_string();
                        }
                        s
                    } else {
                        normalized
                    };

                    let path = PathBuf::from(&path_str);
                    if path.is_absolute() && path.exists() {
                        Some(path)
                    } else {
                        None
                    }
                })
                .collect();
        }

        // Single path or space-separated paths with quote handling
        let mut paths = Vec::new();
        let mut chars = content.chars().peekable();
        let mut current = String::new();
        let mut in_quote = false;
        let mut quote_char: Option<char> = None;

        while let Some(c) = chars.next() {
            match c {
                '"' | '\'' => {
                    if in_quote && Some(c) == quote_char {
                        in_quote = false;
                        quote_char = None;
                    } else if !in_quote {
                        in_quote = true;
                        quote_char = Some(c);
                    } else {
                        current.push(c);
                    }
                }
                '\\' if !in_quote => {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                }
                ' ' if !in_quote => {
                    if !current.is_empty() {
                        let normalized = Self::normalize_path(&current);
                        let path = PathBuf::from(&normalized);
                        if path.is_absolute() && path.exists() {
                            paths.push(path);
                        }
                        current.clear();
                    }
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            let normalized = Self::normalize_path(&current);

            // Handle file:// URLs
            let path_str = if let Some(stripped) = normalized.strip_prefix("file://") {
                let mut s = stripped.to_string();
                if s.len() > 2 && s.starts_with('/') && s.chars().nth(2) == Some(':') {
                    s = s[1..].to_string();
                }
                s
            } else {
                normalized
            };

            let path = PathBuf::from(&path_str);
            if path.is_absolute() && path.exists() {
                paths.push(path);
            }
        }

        paths
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.last_char_time = None;
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Get current buffer content
    pub fn buffer(&self) -> &str {
        &self.buffer
    }
}

/// Handle mouse event and return the resulting action
pub fn handle_mouse_event(
    event: MouseEvent,
    click_detector: &mut ClickDetector,
    tree_area_top: u16,
) -> MouseAction {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Check if click is within tree area (accounting for border)
            if event.row > tree_area_top {
                let row = event.row - tree_area_top - 1; // -1 for border
                if click_detector.click(row) {
                    MouseAction::DoubleClick { row }
                } else {
                    MouseAction::Click { row }
                }
            } else {
                MouseAction::None
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
    fn test_click_detector_single_click() {
        let mut detector = ClickDetector::new();
        assert!(!detector.click(5));
    }

    #[test]
    fn test_click_detector_double_click() {
        let mut detector = ClickDetector::new();
        assert!(!detector.click(5));
        assert!(detector.click(5)); // Same row, quick enough
    }

    #[test]
    fn test_click_detector_different_rows() {
        let mut detector = ClickDetector::new();
        assert!(!detector.click(5));
        assert!(!detector.click(6)); // Different row
    }

    #[test]
    fn test_drop_detector_empty() {
        let detector = DropDetector::new();
        assert!(detector.is_empty());
    }

    #[test]
    fn test_drop_detector_url_decode() {
        let mut detector = DropDetector::new();
        // Simulate path with spaces (URL encoded)
        detector.buffer = "file:///path/with%20spaces/file.txt".to_string();
        // Note: extract_paths checks if path exists, so we just verify it doesn't panic
        let _ = detector.extract_paths();
        assert!(detector.is_empty()); // Buffer should be cleared
    }

    #[test]
    fn test_drop_detector_windows_file_url() {
        let mut detector = DropDetector::new();
        // Windows file URL format: file:///C:/Users/...
        detector.buffer = "file:///C:/Users/test/file.txt".to_string();
        let _ = detector.extract_paths();
        assert!(detector.is_empty()); // Buffer should be cleared
    }

    #[test]
    fn test_drop_detector_quoted_path() {
        let mut detector = DropDetector::new();
        detector.buffer = "\"/path/to/file with spaces.txt\"".to_string();
        let _ = detector.extract_paths();
        assert!(detector.is_empty());
    }

    #[test]
    fn test_drop_detector_backslash_escaped_path() {
        let mut detector = DropDetector::new();
        // macOS terminal drag-and-drop format with backslash-escaped spaces
        detector.buffer = "/path/to/file\\ with\\ spaces.txt".to_string();
        // Note: extract_paths checks if path exists, so we just verify it doesn't panic
        let _ = detector.extract_paths();
        assert!(detector.is_empty()); // Buffer should be cleared
    }

    #[test]
    fn test_drop_detector_backslash_escaped_special_chars() {
        let mut detector = DropDetector::new();
        // macOS terminal drag-and-drop format with various escaped characters
        detector.buffer = "/path/to/file\\(1\\).txt".to_string();
        let _ = detector.extract_paths();
        assert!(detector.is_empty());
    }

    #[test]
    fn test_drop_detector_with_existing_path() {
        use std::env;
        let mut detector = DropDetector::new();
        // Use current directory which should always exist
        let current_dir = env::current_dir().unwrap();
        detector.buffer = current_dir.display().to_string();
        let paths = detector.extract_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], current_dir);
    }

    #[test]
    fn test_drop_detector_push_char() {
        let mut detector = DropDetector::new();
        // Push a character
        detector.push_char('/');
        assert!(!detector.is_empty());
        assert_eq!(detector.buffer(), "/");
    }

    #[test]
    fn test_drop_detector_timeout() {
        let mut detector = DropDetector::new();
        // Initially no timeout
        assert!(!detector.check_timeout());

        // Push some chars
        detector.push_char('/');
        detector.push_char('t');
        detector.push_char('e');

        // Immediately after push, should not timeout yet
        // (timeout is 100ms, we just pushed)
        assert!(!detector.check_timeout());
    }

    #[test]
    fn test_drop_detector_multiple_paths() {
        use std::env;
        let mut detector = DropDetector::new();
        let current_dir = env::current_dir().unwrap();
        // Simulate multiple paths separated by newlines
        detector.buffer = format!("{}\n{}", current_dir.display(), current_dir.display());
        let paths = detector.extract_paths();
        assert_eq!(paths.len(), 2);
    }
}
