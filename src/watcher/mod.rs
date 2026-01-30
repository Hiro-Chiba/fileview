//! File system watcher for real-time updates

use notify::Watcher;
use notify_debouncer_mini::{new_debouncer, Debouncer};
use std::fs;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// Directories to exclude from watching (common large/generated directories)
const EXCLUDED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    ".cache",
    "dist",
    "build",
    ".next",
    ".nuxt",
    "vendor",
];

/// File watcher with debouncing for real-time file system monitoring
pub struct FileWatcher {
    _debouncer: Debouncer<notify::RecommendedWatcher>,
    rx: Receiver<Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>>,
}

impl FileWatcher {
    /// Create a new file watcher for the given root directory
    ///
    /// Excludes common large directories like .git, target, node_modules, etc.
    pub fn new(root: &Path) -> anyhow::Result<Self> {
        let (tx, rx) = channel();

        let mut debouncer = new_debouncer(Duration::from_millis(500), move |res| {
            let _ = tx.send(res);
        })?;

        // Watch directories individually, excluding large/generated ones
        Self::watch_directory_tree(debouncer.watcher(), root)?;

        Ok(Self {
            _debouncer: debouncer,
            rx,
        })
    }

    /// Recursively watch directories, excluding common large directories
    fn watch_directory_tree(watcher: &mut dyn Watcher, dir: &Path) -> anyhow::Result<()> {
        // Watch this directory (non-recursive)
        watcher.watch(dir, notify::RecursiveMode::NonRecursive)?;

        // Recursively process subdirectories
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Check if this directory should be excluded
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if EXCLUDED_DIRS.contains(&name) {
                            continue; // Skip excluded directories
                        }
                    }
                    // Recursively watch subdirectory
                    // Ignore errors for individual subdirectories (permission issues, etc.)
                    let _ = Self::watch_directory_tree(watcher, &path);
                }
            }
        }

        Ok(())
    }

    /// Check for pending file change events (non-blocking)
    ///
    /// Drains all pending events from the channel and returns true if any were found.
    /// This prevents event buildup that could cause repeated expensive reloads.
    pub fn poll(&self) -> bool {
        let mut has_events = false;
        // Drain all pending events to avoid buildup
        while let Ok(Ok(_)) = self.rx.try_recv() {
            has_events = true;
        }
        has_events
    }
}
