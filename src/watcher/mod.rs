//! File system watcher for real-time updates

use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// File watcher with debouncing for real-time file system monitoring
pub struct FileWatcher {
    _debouncer: Debouncer<notify::RecommendedWatcher>,
    rx: Receiver<Result<Vec<DebouncedEvent>, notify::Error>>,
}

impl FileWatcher {
    /// Create a new file watcher for the given root directory
    ///
    /// # Arguments
    /// * `root` - Root directory to watch recursively
    ///
    /// # Returns
    /// A new FileWatcher instance or an error if watcher initialization fails
    pub fn new(root: &Path) -> anyhow::Result<Self> {
        let (tx, rx) = channel();

        let mut debouncer = new_debouncer(Duration::from_millis(500), move |res| {
            let _ = tx.send(res);
        })?;

        debouncer
            .watcher()
            .watch(root, notify::RecursiveMode::Recursive)?;

        Ok(Self {
            _debouncer: debouncer,
            rx,
        })
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
