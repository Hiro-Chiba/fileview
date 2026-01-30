//! File system watcher for real-time updates

use notify_debouncer_mini::{new_debouncer, Debouncer};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
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
    debouncer: Debouncer<notify::RecommendedWatcher>,
    rx: Receiver<Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>>,
    watched_paths: HashSet<PathBuf>,
}

impl FileWatcher {
    /// Create a new file watcher (initially watches only root)
    pub fn new(root: &Path) -> anyhow::Result<Self> {
        let (tx, rx) = channel();

        let mut debouncer = new_debouncer(Duration::from_millis(500), move |res| {
            let _ = tx.send(res);
        })?;

        // Watch root directory only (non-recursive)
        debouncer
            .watcher()
            .watch(root, notify::RecursiveMode::NonRecursive)?;

        let mut watched_paths = HashSet::new();
        watched_paths.insert(root.to_path_buf());

        Ok(Self {
            debouncer,
            rx,
            watched_paths,
        })
    }

    /// Sync watched directories with expanded paths
    ///
    /// Adds watches for newly expanded directories and removes watches for collapsed ones.
    pub fn sync_with_expanded(&mut self, expanded_paths: &[PathBuf]) {
        let new_set: HashSet<PathBuf> = expanded_paths
            .iter()
            .filter(|p| !Self::is_excluded(p))
            .cloned()
            .collect();

        // Remove watches for collapsed directories
        for path in self.watched_paths.difference(&new_set) {
            let _ = self.debouncer.watcher().unwatch(path);
        }

        // Add watches for newly expanded directories
        for path in new_set.difference(&self.watched_paths) {
            let _ = self
                .debouncer
                .watcher()
                .watch(path, notify::RecursiveMode::NonRecursive);
        }

        self.watched_paths = new_set;
    }

    /// Check if a path should be excluded from watching
    fn is_excluded(path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|name| EXCLUDED_DIRS.contains(&name))
            .unwrap_or(false)
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
