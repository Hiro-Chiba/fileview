//! Lazy background indexing and a bounded, latest-query-wins search worker.

use std::collections::{hash_map::DefaultHasher, BTreeSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex, MutexGuard, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecursiveMode, Watcher};

use super::{WorkspaceIndex, WorkspaceMatch};

const MAX_PENDING_EVENTS: usize = 1024;
const MAX_CHANGED_PATHS: usize = 4096;
const SEARCH_LIMIT: usize = 1000;

/// Freshness and availability of the workspace metadata.
#[derive(Clone, Debug, Default)]
pub struct WorkspaceStatus {
    /// True after the initial reconciliation with the filesystem succeeds.
    pub ready: bool,
    /// True during a full scan. Cached results may be provisional.
    pub refreshing: bool,
    /// Number of indexed files and directories.
    pub entries: usize,
    /// Monotonic version including changes to freshness or errors.
    pub revision: u64,
    /// Whether native recursive notifications are available.
    pub watching: bool,
    /// Most recent indexing error, if any.
    pub error: Option<String>,
}

/// Results of the latest requested search, refreshed when files change.
#[derive(Clone, Debug)]
pub struct SearchResponse {
    /// Identifier returned by `request_search`.
    pub request_id: u64,
    /// Query that produced these results.
    pub query: String,
    /// Ranked matches, limited to the requested count.
    pub matches: Vec<WorkspaceMatch>,
    /// Freshness at the time the query ran.
    pub status: WorkspaceStatus,
    /// Query error, such as an invalid filter.
    pub error: Option<String>,
}

#[derive(Clone)]
struct SearchRequest {
    id: u64,
    query: String,
    show_hidden: bool,
    limit: usize,
}

#[derive(Default)]
struct Mailbox {
    request: Option<SearchRequest>,
    response: Option<SearchResponse>,
}

struct Shared {
    index: RwLock<Option<WorkspaceIndex>>,
    status: Mutex<WorkspaceStatus>,
    mailbox: Mutex<Mailbox>,
    wake: Condvar,
    generation: AtomicU64,
    stopped: AtomicBool,
    rescan: AtomicBool,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

/// A workspace service with one index worker and one cancellable search worker.
/// Construction does not walk the workspace; the service is intended to be
/// created only when a workspace search is first requested.
pub struct WorkspaceEngine {
    root: PathBuf,
    shared: Arc<Shared>,
}

impl WorkspaceEngine {
    /// Open a workspace in memory. JSON cache loading plus mandatory validation
    /// costs more than a fresh scan in the measured workloads.
    pub fn new(root: &Path) -> anyhow::Result<Self> {
        Self::with_cache(root, None)
    }

    /// Open a workspace with an explicit cache path, or disable persistence.
    pub fn with_cache(root: &Path, cache: Option<PathBuf>) -> anyhow::Result<Self> {
        let root = root.canonicalize()?;
        anyhow::ensure!(root.is_dir(), "Workspace root must be a directory");
        // Index writes must never feed the workspace's own watcher. Resolve
        // existing ancestors so symlink aliases cannot bypass this boundary.
        let cache = cache
            .map(|path| normalize_storage_path(&path))
            .transpose()?;
        anyhow::ensure!(
            !cache.as_ref().is_some_and(|path| path.starts_with(&root)),
            "Workspace cache must be outside the workspace"
        );
        let shared = Arc::new(Shared {
            index: RwLock::new(None),
            status: Mutex::new(WorkspaceStatus {
                refreshing: true,
                ..Default::default()
            }),
            mailbox: Mutex::new(Mailbox::default()),
            wake: Condvar::new(),
            generation: AtomicU64::new(0),
            stopped: AtomicBool::new(false),
            rescan: AtomicBool::new(false),
        });
        let index_shared = shared.clone();
        let index_root = root.clone();
        thread::Builder::new()
            .name("fv-workspace-index".into())
            .spawn(move || {
                index_worker(index_root, cache, index_shared);
            })?;
        let search_shared = shared.clone();
        if let Err(error) = thread::Builder::new()
            .name("fv-workspace-search".into())
            .spawn(move || {
                search_worker(search_shared);
            })
        {
            shared.stopped.store(true, Ordering::Release);
            return Err(error.into());
        }
        Ok(Self { root, shared })
    }

    /// Canonical workspace root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Read progress without waiting for the index's filesystem work.
    pub fn status(&self) -> WorkspaceStatus {
        lock(&self.shared.status).clone()
    }

    /// Replace the pending query and cancel any obsolete search.
    pub fn request_search(&self, query: &str, show_hidden: bool, limit: usize) -> u64 {
        let mut mailbox = lock(&self.shared.mailbox);
        let id = self.shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
        mailbox.request = Some(SearchRequest {
            id,
            query: query.to_string(),
            show_hidden,
            limit: limit.min(SEARCH_LIMIT),
        });
        mailbox.response = None;
        self.shared.wake.notify_all();
        id
    }

    /// Take the most recent result. There is no unbounded result queue.
    pub fn poll_search(&self) -> Option<SearchResponse> {
        lock(&self.shared.mailbox).response.take()
    }

    /// Cancel a closed search while retaining the warm workspace index.
    pub fn cancel_search(&self) {
        let mut mailbox = lock(&self.shared.mailbox);
        self.shared.generation.fetch_add(1, Ordering::AcqRel);
        mailbox.request = None;
        mailbox.response = None;
        self.shared.wake.notify_all();
    }

    /// Request a full reconciliation after an explicit refresh.
    pub fn refresh(&self) {
        self.shared.rescan.store(true, Ordering::Release);
    }

    /// Wait for a fresh index in noninteractive callers.
    pub fn wait_ready(&self, timeout: Duration) -> anyhow::Result<()> {
        let start = Instant::now();
        loop {
            let status = self.status();
            if status.ready && !status.refreshing {
                if let Some(error) = status.error {
                    anyhow::bail!("Workspace indexing failed: {error}");
                }
                return Ok(());
            }
            if let Some(error) = status.error {
                anyhow::bail!("Workspace indexing failed: {error}");
            }
            anyhow::ensure!(start.elapsed() < timeout, "Workspace indexing timed out");
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Search a reconciled index. TUI callers should use the asynchronous methods.
    pub fn search(
        &self,
        query: &str,
        show_hidden: bool,
        limit: usize,
    ) -> anyhow::Result<Vec<WorkspaceMatch>> {
        let status = self.status();
        anyhow::ensure!(status.ready, "Workspace index is still loading");
        if let Some(error) = status.error {
            anyhow::bail!("Workspace indexing failed: {error}");
        }
        let index = self
            .shared
            .index
            .read()
            .unwrap_or_else(|error| error.into_inner());
        let index = index
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Workspace index unavailable"))?;
        index.search(query, show_hidden, limit.min(SEARCH_LIMIT), &|| false)
    }
}

impl Drop for WorkspaceEngine {
    fn drop(&mut self) {
        self.shared.stopped.store(true, Ordering::Release);
        self.shared.wake.notify_all();
    }
}

fn publish_status(shared: &Shared, update: impl FnOnce(&mut WorkspaceStatus)) {
    let mut status = lock(&shared.status);
    update(&mut status);
    status.revision = status.revision.wrapping_add(1);
    shared.wake.notify_all();
}

fn search_worker(shared: Arc<Shared>) {
    let mut last = (0, u64::MAX);
    while !shared.stopped.load(Ordering::Acquire) {
        let status = lock(&shared.status).clone();
        let request = lock(&shared.mailbox).request.clone();
        if let Some(request) = request {
            if last != (request.id, status.revision) {
                // A long reconciliation must not prevent newer query requests
                // or stop requests from being observed.
                if let Ok(index) = shared.index.try_read() {
                    let cancelled = || {
                        shared.stopped.load(Ordering::Acquire)
                            || shared.generation.load(Ordering::Acquire) != request.id
                    };
                    let result = match index.as_ref() {
                        Some(index) => index.search(
                            &request.query,
                            request.show_hidden,
                            request.limit,
                            &cancelled,
                        ),
                        None => Ok(Vec::new()),
                    };
                    if !cancelled() {
                        let (matches, error) = match result {
                            Ok(matches) => (matches, None),
                            Err(error) => (Vec::new(), Some(error.to_string())),
                        };
                        let mut mailbox = lock(&shared.mailbox);
                        if shared.generation.load(Ordering::Acquire) == request.id {
                            mailbox.response = Some(SearchResponse {
                                request_id: request.id,
                                query: request.query,
                                matches,
                                status: status.clone(),
                                error,
                            });
                            last = (request.id, status.revision);
                        }
                    }
                }
            }
        }
        let guard = lock(&shared.mailbox);
        let pending = guard
            .request
            .as_ref()
            .is_some_and(|request| request.id != last.0);
        if !pending {
            let timeout = if guard.request.is_some() { 16 } else { 250 };
            let _ = shared
                .wake
                .wait_timeout(guard, Duration::from_millis(timeout));
        } else {
            // Back off if the index writer owns the lock. New requests still
            // replace this request without blocking the interactive thread.
            let _ = shared.wake.wait_timeout(guard, Duration::from_millis(1));
        }
    }
}

fn index_worker(root: PathBuf, cache: Option<PathBuf>, shared: Arc<Shared>) {
    let (sender, receiver) = mpsc::sync_channel::<notify::Result<Event>>(MAX_PENDING_EVENTS);
    let callback_shared = shared.clone();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
        enqueue_event(&sender, &callback_shared, event);
    })
    .ok();
    let watching = watcher
        .as_mut()
        .is_some_and(|watcher| watcher.watch(&root, RecursiveMode::Recursive).is_ok());
    publish_status(&shared, |status| status.watching = watching);
    if let Some(cache) = cache.as_ref() {
        if let Ok(index) = WorkspaceIndex::load_cache(&root, cache) {
            let entries = index.len();
            *shared
                .index
                .write()
                .unwrap_or_else(|error| error.into_inner()) = Some(index);
            publish_status(&shared, |status| status.entries = entries);
        }
    }
    let cancelled = || shared.stopped.load(Ordering::Acquire);
    let mut full_scan = true;
    let mut last_scan = Instant::now();
    let mut last_git = Instant::now();
    let mut last_save = Instant::now();
    let mut dirty_cache = false;
    while !cancelled() {
        let interval = if watching && lock(&shared.status).error.is_none() {
            Duration::from_secs(300)
        } else {
            Duration::from_secs(5)
        };
        full_scan |= shared.rescan.swap(false, Ordering::AcqRel) || last_scan.elapsed() >= interval;
        if full_scan {
            publish_status(&shared, |status| {
                status.refreshing = true;
                status.error = None;
            });
            let result = WorkspaceIndex::new(&root).and_then(|mut index| {
                index.rebuild(&cancelled)?;
                index.refresh_git();
                Ok(index)
            });
            match result {
                Ok(index) => {
                    let entries = index.len();
                    *shared
                        .index
                        .write()
                        .unwrap_or_else(|error| error.into_inner()) = Some(index);
                    // Cache is optional. Failure must not disable live search.
                    if let Some(cache) = cache.as_ref() {
                        if let Some(index) = shared
                            .index
                            .read()
                            .unwrap_or_else(|error| error.into_inner())
                            .as_ref()
                        {
                            let _ = index.save_cache(cache);
                        }
                    }
                    publish_status(&shared, |status| {
                        status.ready = true;
                        status.refreshing = false;
                        status.entries = entries;
                        status.error = None;
                    });
                }
                Err(error) if !cancelled() => publish_status(&shared, |status| {
                    status.refreshing = false;
                    status.error = Some(error.to_string());
                }),
                Err(_) => break,
            }
            last_scan = Instant::now();
            last_save = Instant::now();
            full_scan = false;
        }

        let mut paths = BTreeSet::new();
        let mut git_changed = false;
        let first = receiver.recv_timeout(Duration::from_millis(100));
        if matches!(first, Err(mpsc::RecvTimeoutError::Disconnected)) {
            thread::sleep(Duration::from_millis(100));
        }
        if let Ok(first) = first {
            let mut events = vec![first];
            events.extend(receiver.try_iter().take(MAX_PENDING_EVENTS));
            for event in events {
                match event {
                    Ok(event) if !event.need_rescan() => {
                        for path in event.paths {
                            if path.strip_prefix(&root).ok().is_some_and(|relative| {
                                relative.components().any(|c| c.as_os_str() == ".git")
                            }) {
                                git_changed = true;
                                if path.file_name().is_some_and(|name| name == "exclude") {
                                    full_scan = true;
                                }
                            } else if path.starts_with(&root) {
                                paths.insert(path);
                            }
                        }
                    }
                    _ => full_scan = true,
                }
            }
        }
        if paths.len() > MAX_CHANGED_PATHS {
            full_scan = true;
        }
        if full_scan {
            continue;
        }
        if !paths.is_empty() {
            // Walk changed subtrees with a shared read lock so queries continue
            // to use the previous consistent snapshot while filesystem IO runs.
            let delta = {
                let guard = shared
                    .index
                    .read()
                    .unwrap_or_else(|error| error.into_inner());
                guard.as_ref().map(|index| {
                    index.prepare_reconcile(&paths.into_iter().collect::<Vec<_>>(), &cancelled)
                })
            };
            if let Some(delta) = delta {
                let mut guard = shared
                    .index
                    .write()
                    .unwrap_or_else(|error| error.into_inner());
                let Some(index) = guard.as_mut() else {
                    continue;
                };
                let previous_revision = index.revision();
                let result = delta.and_then(|delta| {
                    git_changed |= delta.has_indexed_paths();
                    index.apply_delta(delta)
                });
                match result {
                    Ok(()) => {
                        let entries = index.len();
                        if index.revision() != previous_revision {
                            publish_status(&shared, |status| {
                                status.entries = entries;
                                status.error = None;
                            });
                            dirty_cache = true;
                        }
                    }
                    Err(error) if !cancelled() => {
                        publish_status(&shared, |status| status.error = Some(error.to_string()));
                        full_scan = true;
                    }
                    Err(_) => break,
                }
            }
        }
        if git_changed || last_git.elapsed() >= Duration::from_secs(5) {
            // Git can be slow on large or remote repositories. Do not hold the
            // writer lock while its subprocesses run.
            let git = crate::git::GitStatus::detect_for_workspace(&root);
            let mut guard = shared
                .index
                .write()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(index) = guard.as_mut() {
                if index.set_git(git) {
                    publish_status(&shared, |_| {});
                }
            }
            last_git = Instant::now();
        }
        if dirty_cache && last_save.elapsed() >= Duration::from_secs(2) {
            if let Some(cache) = cache.as_ref() {
                if let Some(index) = shared
                    .index
                    .read()
                    .unwrap_or_else(|error| error.into_inner())
                    .as_ref()
                {
                    let _ = index.save_cache(cache);
                }
            }
            dirty_cache = false;
            last_save = Instant::now();
        }
    }
}

fn normalize_storage_path(path: &Path) -> anyhow::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let mut ancestor = absolute.as_path();
    let mut suffix = Vec::new();
    while !ancestor.exists() {
        suffix.push(
            ancestor
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid cache path"))?,
        );
        ancestor = ancestor
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid cache path"))?;
    }
    let mut normalized = ancestor.canonicalize()?;
    for component in suffix.into_iter().rev() {
        normalized.push(component);
    }
    Ok(normalized)
}

fn enqueue_event(
    sender: &mpsc::SyncSender<notify::Result<Event>>,
    shared: &Shared,
    event: notify::Result<Event>,
) {
    if matches!(&event, Ok(event) if matches!(event.kind, EventKind::Access(_))) {
        return;
    }
    if sender.try_send(event).is_err() {
        shared.rescan.store(true, Ordering::Release);
    }
}

fn directory(root: &Path, variable: &str, base: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let root = root.canonicalize()?;
    let mut hash = DefaultHasher::new();
    root.hash(&mut hash);
    let base = std::env::var_os(variable)
        .map(PathBuf::from)
        .or(base)
        .ok_or_else(|| anyhow::anyhow!("No workspace storage directory available"))?;
    Ok(base
        .join("fileview")
        .join("workspaces")
        .join(format!("{:016x}", hash.finish())))
}

/// Per-root cache directory. `FILEVIEW_WORKSPACE_CACHE_DIR` overrides its base.
pub fn cache_directory(root: &Path) -> anyhow::Result<PathBuf> {
    directory(root, "FILEVIEW_WORKSPACE_CACHE_DIR", dirs::cache_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn wait_until(mut condition: impl FnMut() -> bool) {
        let start = Instant::now();
        while !condition() {
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "workspace update timed out"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn last_query_wins_and_cancel_keeps_index() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("needle.rs"), "x").unwrap();
        let engine = WorkspaceEngine::with_cache(temp.path(), None).unwrap();
        engine.wait_ready(Duration::from_secs(10)).unwrap();
        for number in 0..500 {
            engine.request_search(&format!("missing-{number}"), false, 15);
        }
        let latest = engine.request_search("needle", false, 15);
        wait_until(|| {
            engine.poll_search().is_some_and(|response| {
                assert_eq!(response.request_id, latest);
                assert!(response.error.is_none());
                assert_eq!(response.matches.len(), 1);
                true
            })
        });
        engine.cancel_search();
        assert!(lock(&engine.shared.mailbox).request.is_none());
        assert!(engine.status().ready);
    }

    #[test]
    fn recursive_changes_update_active_query_without_reopening() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("collapsed/nested")).unwrap();
        let engine = WorkspaceEngine::with_cache(temp.path(), None).unwrap();
        engine.wait_ready(Duration::from_secs(10)).unwrap();
        assert!(engine.status().watching);
        let request = engine.request_search("ext:rs", false, 15);
        wait_until(|| engine.poll_search().is_some());
        let old = temp.path().join("collapsed/nested/old.rs");
        let new = temp.path().join("collapsed/nested/new.rs");
        fs::write(&old, "before").unwrap();
        wait_until(|| {
            engine.poll_search().is_some_and(|response| {
                response.request_id == request
                    && response
                        .matches
                        .iter()
                        .any(|entry| entry.display.ends_with("old.rs"))
            })
        });
        fs::rename(&old, &new).unwrap();
        fs::write(&new, "longer content").unwrap();
        wait_until(|| {
            engine.poll_search().is_some_and(|response| {
                response.matches.len() == 1
                    && response.matches[0].display.ends_with("new.rs")
                    && response.matches[0].size == 14
            })
        });
        fs::remove_file(&new).unwrap();
        wait_until(|| {
            engine
                .poll_search()
                .is_some_and(|response| response.matches.is_empty())
        });
    }

    #[test]
    fn corrupt_and_stale_cache_are_reconciled_before_ready() {
        let temp = tempfile::tempdir().unwrap();
        let storage = tempfile::tempdir().unwrap();
        let cache = storage.path().join("index.json");
        fs::write(temp.path().join("old.rs"), "old").unwrap();
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        index.save_cache(&cache).unwrap();
        fs::remove_file(temp.path().join("old.rs")).unwrap();
        fs::write(temp.path().join("new.rs"), "new").unwrap();
        let engine = WorkspaceEngine::with_cache(temp.path(), Some(cache.clone())).unwrap();
        engine.wait_ready(Duration::from_secs(10)).unwrap();
        let results = engine.search("ext:rs", false, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].display, "new.rs");
        drop(engine);
        fs::write(&cache, "broken").unwrap();
        let engine = WorkspaceEngine::with_cache(temp.path(), Some(cache)).unwrap();
        engine.wait_ready(Duration::from_secs(10)).unwrap();
        assert_eq!(engine.search("ext:rs", false, 10).unwrap().len(), 1);
    }

    #[test]
    fn full_event_queue_requests_reconciliation() {
        // No worker may consume the flag between enqueueing and asserting it.
        let shared = Shared {
            index: RwLock::new(None),
            status: Mutex::new(WorkspaceStatus::default()),
            mailbox: Mutex::new(Mailbox::default()),
            wake: Condvar::new(),
            generation: AtomicU64::new(0),
            stopped: AtomicBool::new(false),
            rescan: AtomicBool::new(false),
        };
        let (sender, _receiver) = mpsc::sync_channel(1);
        let event = || Ok(Event::new(EventKind::Any));
        enqueue_event(&sender, &shared, event());
        enqueue_event(&sender, &shared, event());
        assert!(shared.rescan.load(Ordering::Acquire));
    }

    #[test]
    fn explicit_refresh_advances_revision() {
        let temp = tempfile::tempdir().unwrap();
        let engine = WorkspaceEngine::with_cache(temp.path(), None).unwrap();
        engine.wait_ready(Duration::from_secs(10)).unwrap();
        let revision = engine.status().revision;
        engine.refresh();
        wait_until(|| {
            let status = engine.status();
            status.ready && !status.refreshing && status.revision > revision
        });
    }

    #[test]
    fn rejects_cache_within_workspace_to_prevent_watch_feedback() {
        let temp = tempfile::tempdir().unwrap();
        assert!(WorkspaceEngine::with_cache(
            temp.path(),
            Some(temp.path().join("cache/index.json"))
        )
        .is_err());
        #[cfg(unix)]
        {
            let aliases = tempfile::tempdir().unwrap();
            std::os::unix::fs::symlink(temp.path(), aliases.path().join("alias")).unwrap();
            assert!(WorkspaceEngine::with_cache(
                temp.path(),
                Some(aliases.path().join("alias/cache/index.json"))
            )
            .is_err());
        }
    }
}
