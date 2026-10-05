//! Background previews with bounded latest-request and latest-result slots.
//!
//! Moves heavy preview generation (text highlighting, git diff, directory scan,
//! archive listing, video metadata) off the UI thread to prevent frame drops.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

use crate::app::video::{find_ffprobe, get_metadata, VideoMetadata};
use crate::git::{self, FileStatus};
use crate::render::preview::window::{self, FileStamp, TextAnchor};
use crate::render::{
    is_archive_file, is_tar_gz_file, ArchivePreview, DiffPreview, DirectoryInfo, TextPreview,
};

/// What kind of preview to generate
#[derive(Debug, Clone)]
pub enum PreviewKind {
    Text,
    TextWindow {
        anchor: TextAnchor,
        match_offset: Option<u64>,
        backward: bool,
    },
    TextBottom,
    Diff,
    Directory,
    Archive,
    VideoMeta,
}

/// Request sent to the worker thread
pub struct PreviewRequest {
    pub path: PathBuf,
    pub kind: PreviewKind,
    pub serial: u64,
    /// For Diff: git repo root
    pub git_repo_root: Option<PathBuf>,
    /// For Diff: file status
    pub git_file_status: Option<FileStatus>,
}

/// Payload returned by the worker
pub enum PreviewPayload {
    Text(TextPreview),
    Diff(DiffPreview),
    Directory(DirectoryInfo),
    Archive(ArchivePreview),
    VideoMeta(VideoMetadata),
}

/// Response from the worker thread
pub struct PreviewResponse {
    pub path: PathBuf,
    pub serial: u64,
    /// A background syntax update for the same visible text window.
    pub refinement: bool,
    pub payload: Result<PreviewPayload, String>,
}

#[derive(Default)]
struct Mailbox {
    request: Option<PreviewRequest>,
    response: Option<PreviewResponse>,
}

#[derive(Default)]
struct Shared {
    mailbox: Mutex<Mailbox>,
    wake: Condvar,
    generation: AtomicU64,
    stopped: AtomicBool,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

/// Background preview worker. Text work checks cancellation between bounded
/// reads and highlighted lines. Other existing preview backends may finish an
/// outstanding filesystem or subprocess operation before observing replacement.
pub struct PreviewWorker {
    shared: Arc<Shared>,
    _worker: JoinHandle<()>,
}

impl PreviewWorker {
    pub fn new() -> Self {
        let shared = Arc::new(Shared::default());
        let worker_shared = shared.clone();
        let worker = thread::spawn(move || Self::worker_loop(worker_shared));
        Self {
            shared,
            _worker: worker,
        }
    }

    fn worker_loop(shared: Arc<Shared>) {
        loop {
            let req = {
                let mut mailbox = lock(&shared.mailbox);
                while mailbox.request.is_none() && !shared.stopped.load(Ordering::Acquire) {
                    mailbox = shared
                        .wake
                        .wait(mailbox)
                        .unwrap_or_else(|error| error.into_inner());
                }
                if shared.stopped.load(Ordering::Acquire) {
                    return;
                }
                let Some(request) = mailbox.request.take() else {
                    continue;
                };
                request
            };
            let cancelled = || {
                shared.stopped.load(Ordering::Acquire)
                    || shared.generation.load(Ordering::Acquire) != req.serial
            };
            if cancelled() {
                continue;
            }
            let payload = match &req.kind {
                PreviewKind::Text | PreviewKind::TextWindow { .. } | PreviewKind::TextBottom => {
                    Self::generate_text(&req.path, &req.kind, &cancelled)
                }
                PreviewKind::Diff => Self::generate_diff(
                    &req.path,
                    req.git_repo_root.as_deref(),
                    req.git_file_status,
                ),
                PreviewKind::Directory => Self::generate_directory(&req.path),
                PreviewKind::Archive => Self::generate_archive(&req.path),
                PreviewKind::VideoMeta => Self::generate_video_meta(&req.path),
            };
            let mut refine = match &payload {
                Ok(PreviewPayload::Text(text)) if text.needs_context_highlighting() => {
                    Some(text.clone())
                }
                _ => None,
            };
            {
                let mut mailbox = lock(&shared.mailbox);
                if !cancelled() {
                    mailbox.response = Some(PreviewResponse {
                        path: req.path.clone(),
                        serial: req.serial,
                        refinement: false,
                        payload,
                    });
                }
            }
            // Publish readable content before replaying syntax context. The same
            // generation guard prevents an obsolete refinement replacing a new page.
            if let Some(text) = &mut refine {
                if !cancelled() {
                    let result = text.highlight_with_context(&req.path, &cancelled);
                    let mut mailbox = lock(&shared.mailbox);
                    if !cancelled()
                        && (text.styled_lines.is_some()
                            || text.syntax_note.is_some()
                            || result.is_err())
                    {
                        mailbox.response = Some(PreviewResponse {
                            path: req.path,
                            serial: req.serial,
                            refinement: true,
                            payload: result
                                .map(|()| PreviewPayload::Text(text.clone()))
                                .map_err(|error| format!("Failed: syntax preview - {error}")),
                        });
                    }
                }
            }
        }
    }

    fn generate_text(
        path: &Path,
        kind: &PreviewKind,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<PreviewPayload, String> {
        let result = (|| -> anyhow::Result<TextPreview> {
            let page = match kind {
                PreviewKind::TextWindow {
                    anchor,
                    backward: true,
                    ..
                } => window::read_previous_window(path, anchor, cancelled)?,
                PreviewKind::TextWindow {
                    anchor,
                    match_offset: Some(offset),
                    ..
                } => window::read_match_window(path, anchor, *offset, cancelled)?,
                PreviewKind::TextWindow { anchor, .. } => {
                    window::read_window(path, Some(anchor), cancelled)?
                }
                PreviewKind::TextBottom => window::read_bottom_window(path, cancelled)?,
                _ => window::read_window(path, None, cancelled)?,
            };
            TextPreview::from_window(page, path, cancelled)
        })();
        result
            .map(PreviewPayload::Text)
            .map_err(|error| format!("Failed: preview - {error}"))
    }

    fn generate_diff(
        path: &Path,
        repo_root: Option<&Path>,
        _status: Option<FileStatus>,
    ) -> Result<PreviewPayload, String> {
        let repo_root = repo_root.ok_or_else(|| "No git repo root".to_string())?;
        // Try staged diff first, then unstaged
        let diff =
            git::get_diff(repo_root, path, true).or_else(|| git::get_diff(repo_root, path, false));

        match diff {
            Some(file_diff) if !file_diff.is_empty() => {
                Ok(PreviewPayload::Diff(DiffPreview::new(file_diff)))
            }
            _ => Err("No diff available".to_string()),
        }
    }

    fn generate_directory(path: &Path) -> Result<PreviewPayload, String> {
        DirectoryInfo::from_path(path)
            .map(PreviewPayload::Directory)
            .map_err(|e| format!("Failed: directory preview - {}", e))
    }

    fn generate_archive(path: &Path) -> Result<PreviewPayload, String> {
        let result = if is_tar_gz_file(path) {
            ArchivePreview::load_tar_gz(path)
        } else if is_archive_file(path) {
            ArchivePreview::load_zip(path)
        } else {
            return Err("Not an archive file".to_string());
        };
        result
            .map(PreviewPayload::Archive)
            .map_err(|e| format!("Failed: preview - {}", e))
    }

    fn generate_video_meta(path: &Path) -> Result<PreviewPayload, String> {
        if find_ffprobe().is_none() {
            return Err("Video preview requires ffprobe (ffmpeg)".to_string());
        }
        get_metadata(path)
            .map(PreviewPayload::VideoMeta)
            .map_err(|e| format!("Failed: video preview - {}", e))
    }

    /// Send a preview request, returns the serial number assigned.
    pub fn request(
        &mut self,
        req_path: PathBuf,
        kind: PreviewKind,
        git_repo_root: Option<PathBuf>,
        git_file_status: Option<FileStatus>,
    ) -> u64 {
        let serial = self.shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let mut mailbox = lock(&self.shared.mailbox);
        mailbox.request = Some(PreviewRequest {
            path: req_path,
            kind,
            serial,
            git_repo_root,
            git_file_status,
        });
        mailbox.response = None;
        self.shared.wake.notify_one();
        serial
    }

    /// Cancel pending and active obsolete work without blocking the UI thread.
    pub fn cancel(&mut self) -> u64 {
        let serial = self.shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let mut mailbox = lock(&self.shared.mailbox);
        mailbox.request = None;
        mailbox.response = None;
        self.shared.wake.notify_one();
        serial
    }

    /// Non-blocking poll for the single latest completed result.
    pub fn try_recv(&self) -> Option<PreviewResponse> {
        lock(&self.shared.mailbox).response.take()
    }

    #[cfg(test)]
    pub fn current_serial(&self) -> u64 {
        self.shared.generation.load(Ordering::Acquire)
    }
}

impl Drop for PreviewWorker {
    fn drop(&mut self) {
        let mut mailbox = lock(&self.shared.mailbox);
        self.shared.stopped.store(true, Ordering::Release);
        self.shared.generation.fetch_add(1, Ordering::AcqRel);
        mailbox.request = None;
        mailbox.response = None;
        self.shared.wake.notify_one();
        // Dropping JoinHandle detaches instead of waiting on a non-text OS call.
    }
}

impl Default for PreviewWorker {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// LRU Preview Cache
// ---------------------------------------------------------------------------

/// Cached preview data (only types that are Send + relatively cheap to store)
pub enum CachedPreview {
    Text(TextPreview),
    Diff(DiffPreview),
    Directory(DirectoryInfo),
    Archive(ArchivePreview),
}

pub struct PreviewCache {
    entries: VecDeque<(PathBuf, CachedPreview)>,
    max_size: usize,
}

impl PreviewCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Look up a cached preview. Returns None on miss.
    pub fn get(&mut self, path: &Path) -> Option<&CachedPreview> {
        // Find the index, move to front for LRU
        let idx = self.entries.iter().position(|(p, _)| p == path)?;
        let entry = self.entries.remove(idx)?;
        if let CachedPreview::Text(preview) = &entry.1 {
            let anchor = preview.anchor.as_ref()?;
            if FileStamp::capture(path).ok().as_ref() != Some(&anchor.stamp) {
                return None;
            }
        }
        self.entries.push_front(entry);
        self.entries.front().map(|(_, c)| c)
    }

    /// Insert a preview into the cache.
    pub fn insert(&mut self, path: PathBuf, mut preview: CachedPreview) {
        if let CachedPreview::Text(text) = &mut preview {
            if text
                .anchor
                .as_ref()
                .is_none_or(|anchor| anchor.byte_offset != 0)
            {
                return;
            }
            text.scroll = 0;
        }
        // Remove existing entry for same path
        self.entries.retain(|(p, _)| p != &path);
        self.entries.push_front((path, preview));
        while self.entries.len() > self.max_size {
            self.entries.pop_back();
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for PreviewCache {
    fn default() -> Self {
        Self::new(32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distant_page_receives_contextual_styles_from_the_background_worker() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("source.rs");
        let prefix = format!("/*\n{}", "inside comment\n".repeat(300));
        let content = format!("{prefix}match here\n*/\nfn main() {{}}\n");
        std::fs::write(&path, &content).unwrap();
        let reference = TextPreview::with_highlighting(&content, &path);
        let anchor = TextAnchor {
            line_number: 302,
            byte_offset: prefix.len() as u64,
            stamp: FileStamp::capture(&path).unwrap(),
        };
        let mut worker = PreviewWorker::new();
        let serial = worker.request(
            path,
            PreviewKind::TextWindow {
                anchor,
                match_offset: None,
                backward: false,
            },
            None,
            None,
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            assert!(
                std::time::Instant::now() < deadline,
                "syntax refinement did not arrive"
            );
            if let Some(response) = worker.try_recv() {
                assert_eq!(response.serial, serial);
                let PreviewPayload::Text(text) = response.payload.unwrap() else {
                    panic!("expected text")
                };
                if let Some(styled) = text.styled_lines {
                    assert!(response.refinement);
                    assert_eq!(text.first_line, 302);
                    assert_eq!(
                        styled[0].segments[0].color,
                        reference.styled_lines.as_ref().unwrap()[301].segments[0].color
                    );
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    #[test]
    fn test_preview_worker_creation() {
        let worker = PreviewWorker::new();
        assert_eq!(worker.current_serial(), 0);
        assert!(worker.try_recv().is_none());
    }

    #[test]
    fn test_preview_cache_insert_and_get() {
        let mut cache = PreviewCache::new(3);
        let path = PathBuf::from("/tmp/test.txt");
        let info = DirectoryInfo {
            name: "test".to_string(),
            file_count: 1,
            dir_count: 0,
            hidden_count: 0,
            total_size: 100,
        };
        cache.insert(path.clone(), CachedPreview::Directory(info));
        assert!(cache.get(&path).is_some());
    }

    #[test]
    fn test_preview_cache_eviction() {
        let mut cache = PreviewCache::new(2);
        for i in 0..3 {
            let path = PathBuf::from(format!("/tmp/test{}.txt", i));
            let info = DirectoryInfo {
                name: format!("test{}", i),
                file_count: i,
                dir_count: 0,
                hidden_count: 0,
                total_size: 0,
            };
            cache.insert(path, CachedPreview::Directory(info));
        }
        // First entry should be evicted
        assert!(cache.get(Path::new("/tmp/test0.txt")).is_none());
        assert!(cache.get(Path::new("/tmp/test1.txt")).is_some());
        assert!(cache.get(Path::new("/tmp/test2.txt")).is_some());
    }

    #[test]
    fn test_preview_cache_lru_ordering() {
        let mut cache = PreviewCache::new(2);
        let path1 = PathBuf::from("/tmp/a.txt");
        let path2 = PathBuf::from("/tmp/b.txt");

        let mk = |name: &str| {
            CachedPreview::Directory(DirectoryInfo {
                name: name.to_string(),
                file_count: 0,
                dir_count: 0,
                hidden_count: 0,
                total_size: 0,
            })
        };

        cache.insert(path1.clone(), mk("a"));
        cache.insert(path2.clone(), mk("b"));

        // Access path1 to make it most recently used
        let _ = cache.get(&path1);

        // Insert path3 - should evict path2 (least recently used)
        let path3 = PathBuf::from("/tmp/c.txt");
        cache.insert(path3.clone(), mk("c"));

        assert!(cache.get(&path1).is_some());
        assert!(cache.get(&path2).is_none());
        assert!(cache.get(&path3).is_some());
    }

    #[test]
    fn test_worker_request_increments_serial() {
        let mut worker = PreviewWorker::new();
        let s1 = worker.request(PathBuf::from("/tmp/a"), PreviewKind::Directory, None, None);
        let s2 = worker.request(PathBuf::from("/tmp/b"), PreviewKind::Directory, None, None);
        assert_eq!(s1, 1);
        assert_eq!(s2, 2);
    }

    #[test]
    fn worker_processes_only_the_latest_queued_request() {
        let mut worker = PreviewWorker::new();
        let mut serial = 0;
        for number in 0..1000 {
            serial = worker.request(
                PathBuf::from(format!("/missing-{number}")),
                PreviewKind::Text,
                None,
                None,
            );
        }
        let start = std::time::Instant::now();
        loop {
            if let Some(response) = worker.try_recv() {
                assert_eq!(response.serial, serial);
                break;
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(5));
            thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(lock(&worker.shared.mailbox).request.is_none());
        worker.cancel();
        assert!(worker.try_recv().is_none());
    }

    #[test]
    fn bounded_text_load_and_cooperative_cancellation() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("large.txt");
        std::fs::write(&path, "line\n".repeat(100_000)).unwrap();
        assert!(PreviewWorker::generate_text(&path, &PreviewKind::Text, &|| true).is_err());
        let PreviewPayload::Text(preview) =
            PreviewWorker::generate_text(&path, &PreviewKind::Text, &|| false).unwrap()
        else {
            panic!("expected text");
        };
        assert_eq!(preview.lines.len(), window::WINDOW_LINES);
        assert!(preview.next.is_some());
        assert_eq!(preview.first_line, 1);
    }

    #[test]
    fn text_cache_rejects_changed_files_and_distant_pages() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("file.txt");
        std::fs::write(&path, "line\n".repeat(300)).unwrap();
        let page = window::read_window(&path, None, &|| false).unwrap();
        let next = page.next.clone().unwrap();
        let preview = TextPreview::from_window(page, &path, &|| false).unwrap();
        let mut cache = PreviewCache::new(2);
        cache.insert(path.clone(), CachedPreview::Text(preview));
        assert!(cache.get(&path).is_some());
        std::fs::write(&path, "changed").unwrap();
        assert!(cache.get(&path).is_none());
        std::fs::write(&path, "line\n".repeat(300)).unwrap();
        let mut next = next;
        next.stamp = FileStamp::capture(&path).unwrap();
        let page = window::read_window(&path, Some(&next), &|| false).unwrap();
        cache.insert(
            path.clone(),
            CachedPreview::Text(TextPreview::from_window(page, &path, &|| false).unwrap()),
        );
        assert!(cache.get(&path).is_none());
    }
}
