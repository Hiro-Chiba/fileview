//! File system watcher for real-time updates

use notify::Watcher;
use std::cell::Cell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

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
    watcher: notify::RecommendedWatcher,
    dirty: Arc<AtomicBool>,
    pending_since: Cell<Option<Instant>>,
    root: PathBuf,
    watched_paths: HashSet<PathBuf>,
}

impl FileWatcher {
    /// Create a new file watcher (initially watches only root)
    pub fn new(root: &Path) -> anyhow::Result<Self> {
        let dirty = Arc::new(AtomicBool::new(false));
        let event_dirty = dirty.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                // Linux reports open/read/close as access events. Debouncing those
                // into generic changes creates a preview-read/refresh feedback loop.
                if event.as_ref().map_or(true, |event| !event.kind.is_access()) {
                    event_dirty.store(true, Ordering::Release);
                }
            })?;
        watcher.watch(root, notify::RecursiveMode::NonRecursive)?;

        let mut watched_paths = HashSet::new();
        watched_paths.insert(root.to_path_buf());

        Ok(Self {
            watcher,
            dirty,
            pending_since: Cell::new(None),
            root: root.to_path_buf(),
            watched_paths,
        })
    }

    /// Sync watched directories with expanded paths
    ///
    /// Adds watches for newly expanded directories and removes watches for collapsed ones.
    pub fn sync_with_expanded(&mut self, expanded_paths: &[PathBuf]) {
        let mut new_set: HashSet<PathBuf> = expanded_paths
            .iter()
            .filter(|p| !Self::is_excluded(p))
            .cloned()
            .collect();
        new_set.insert(self.root.clone());

        // Remove watches for collapsed directories
        for path in self.watched_paths.difference(&new_set) {
            let _ = self.watcher.unwatch(path);
        }

        // Add watches for newly expanded directories
        for path in new_set.difference(&self.watched_paths) {
            let _ = self
                .watcher
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

    /// Coalesce mutations in a bounded flag, refreshing at most twice per second.
    /// The deadline starts at the first event, so continuous writes cannot starve
    /// updates. Access-only events never enter this queue.
    pub fn poll(&self) -> bool {
        if self.dirty.swap(false, Ordering::AcqRel) && self.pending_since.get().is_none() {
            self.pending_since.set(Some(Instant::now()));
        }
        if self
            .pending_since
            .get()
            .is_some_and(|start| start.elapsed() >= Duration::from_millis(500))
        {
            self.pending_since.set(None);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[cfg(target_os = "linux")]
    #[test]
    fn file_reads_do_not_refresh_but_writes_still_do() {
        let root = tempdir().unwrap();
        let path = root.path().join("preview.txt");
        std::fs::write(&path, "original").unwrap();
        let watcher = FileWatcher::new(root.path()).unwrap();
        for _ in 0..12 {
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
            std::thread::sleep(Duration::from_millis(100));
            assert!(!watcher.poll(), "reading a preview must not invalidate it");
        }
        std::fs::write(&path, "changed").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !watcher.poll() {
            assert!(
                Instant::now() < deadline,
                "a write must still refresh the tree"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn sync_keeps_root_even_when_its_name_is_excluded() {
        let temp = tempdir().unwrap();
        let root = temp.path().join("target");
        std::fs::create_dir(&root).unwrap();
        let mut watcher = FileWatcher::new(&root).unwrap();

        watcher.sync_with_expanded(&[]);

        assert!(watcher.watched_paths.contains(&root));
    }
}
