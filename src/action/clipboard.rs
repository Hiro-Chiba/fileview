use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum ClipboardContent {
    Copy(Vec<PathBuf>),
    Cut(Vec<PathBuf>),
}

#[derive(Debug, Default)]
pub struct Clipboard {
    content: Option<ClipboardContent>,
}

impl Clipboard {
    pub fn new() -> Self {
        Self { content: None }
    }

    pub fn copy(&mut self, paths: Vec<PathBuf>) {
        self.content = Some(ClipboardContent::Copy(paths));
    }

    pub fn cut(&mut self, paths: Vec<PathBuf>) {
        self.content = Some(ClipboardContent::Cut(paths));
    }

    pub fn content(&self) -> Option<&ClipboardContent> {
        self.content.as_ref()
    }

    /// Leaves the clipboard empty.
    pub fn take(&mut self) -> Option<ClipboardContent> {
        self.content.take()
    }

    pub fn is_cut(&self) -> bool {
        matches!(self.content, Some(ClipboardContent::Cut(_)))
    }

    pub fn paths(&self) -> &[PathBuf] {
        match &self.content {
            Some(ClipboardContent::Copy(paths)) | Some(ClipboardContent::Cut(paths)) => paths,
            None => &[],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_none()
    }

    pub fn clear(&mut self) {
        self.content = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let paths = vec![PathBuf::from("/path/to/file")];

        clipboard.copy(paths);
        let taken = clipboard.take();

        assert!(clipboard.is_empty());
        assert!(taken.is_some());
        assert!(matches!(taken, Some(ClipboardContent::Copy(_))));
    }

    #[test]
    fn test_clipboard_clear() {
        let mut clipboard = Clipboard::new();
        clipboard.copy(vec![PathBuf::from("/path")]);

        clipboard.clear();

        assert!(clipboard.is_empty());
        assert!(clipboard.content().is_none());
    }

    #[test]
    fn test_clipboard_paths() {
        let mut clipboard = Clipboard::new();
        let paths = vec![
            PathBuf::from("/a"),
            PathBuf::from("/b"),
            PathBuf::from("/c"),
        ];

        clipboard.copy(paths.clone());

        assert_eq!(clipboard.paths(), &paths);
    }
}
