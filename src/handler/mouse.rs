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
        }
    }

    /// Add a character to the buffer
    /// Returns true if this looks like a file drop (rapid character input)
    pub fn push_char(&mut self, c: char) -> bool {
        let now = Instant::now();

        // Check if this is rapid input (potential drop)
        let is_rapid = self
            .last_char_time
            .map(|t| now.duration_since(t) < self.char_threshold)
            .unwrap_or(false);

        self.last_char_time = Some(now);

        if is_rapid || self.buffer.is_empty() {
            self.buffer.push(c);
            true
        } else {
            // Too slow, reset buffer
            self.buffer.clear();
            self.buffer.push(c);
            false
        }
    }

    /// Check if buffer contains valid paths and extract them
    pub fn extract_paths(&mut self) -> Vec<PathBuf> {
        if self.buffer.is_empty() {
            return Vec::new();
        }

        let content = std::mem::take(&mut self.buffer);
        let paths: Vec<PathBuf> = content
            .lines()
            .flat_map(|line| {
                // Handle various path formats
                let mut path = line.trim().trim_matches('"').trim_matches('\'');

                // Handle file:// URLs (Unix: file:///path, Windows: file:///C:/path)
                if let Some(stripped) = path.strip_prefix("file://") {
                    path = stripped;
                    // Windows file URLs have extra slash: file:///C:/...
                    // After stripping "file://", we have "/C:/..." - need to remove leading /
                    if path.len() > 2 && path.starts_with('/') && path.chars().nth(2) == Some(':') {
                        path = &path[1..];
                    }
                }

                // URL decode common characters
                let decoded = path
                    .replace("%20", " ")
                    .replace("%23", "#")
                    .replace("%25", "%")
                    .replace("%5B", "[")
                    .replace("%5D", "]");

                if decoded.is_empty() {
                    None
                } else {
                    let path_buf = PathBuf::from(&decoded);
                    if path_buf.exists() {
                        Some(path_buf)
                    } else {
                        None
                    }
                }
            })
            .collect();

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
}
