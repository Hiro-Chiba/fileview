//! Background preview worker using std::thread and mpsc channels
//!
//! Moves heavy preview generation (text highlighting, git diff, directory scan,
//! archive listing, video metadata) off the UI thread to prevent frame drops.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::app::video::{find_ffprobe, get_metadata, VideoMetadata};
use crate::git::{self, FileStatus};
use crate::render::{
    is_archive_file, is_tar_gz_file, ArchivePreview, DiffPreview, DirectoryInfo, TextPreview,
};

/// What kind of preview to generate
#[derive(Debug, Clone)]
pub enum PreviewKind {
    Text,
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
    pub payload: Result<PreviewPayload, String>,
}

/// Background preview worker
pub struct PreviewWorker {
    request_tx: Sender<PreviewRequest>,
    result_rx: Receiver<PreviewResponse>,
    _worker: JoinHandle<()>,
    current_serial: u64,
}

impl PreviewWorker {
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<PreviewRequest>();
        let (result_tx, result_rx) = mpsc::channel::<PreviewResponse>();

        let worker = thread::spawn(move || {
            Self::worker_loop(request_rx, result_tx);
        });

        Self {
            request_tx,
            result_rx,
            _worker: worker,
            current_serial: 0,
        }
    }

    fn worker_loop(request_rx: Receiver<PreviewRequest>, result_tx: Sender<PreviewResponse>) {
        while let Ok(req) = request_rx.recv() {
            let payload = match req.kind {
                PreviewKind::Text => Self::generate_text(&req.path),
                PreviewKind::Diff => {
                    Self::generate_diff(&req.path, req.git_repo_root.as_deref(), req.git_file_status)
                }
                PreviewKind::Directory => Self::generate_directory(&req.path),
                PreviewKind::Archive => Self::generate_archive(&req.path),
                PreviewKind::VideoMeta => Self::generate_video_meta(&req.path),
            };

            let response = PreviewResponse {
                path: req.path,
                serial: req.serial,
                payload,
            };

            if result_tx.send(response).is_err() {
                break;
            }
        }
    }

    fn generate_text(path: &Path) -> Result<PreviewPayload, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed: preview - {}", e))?;
        let preview = TextPreview::with_highlighting(&content, path);
        Ok(PreviewPayload::Text(preview))
    }

    fn generate_diff(
        path: &Path,
        repo_root: Option<&Path>,
        _status: Option<FileStatus>,
    ) -> Result<PreviewPayload, String> {
        let repo_root = repo_root.ok_or_else(|| "No git repo root".to_string())?;
        // Try staged diff first, then unstaged
        let diff = git::get_diff(repo_root, path, true)
            .or_else(|| git::get_diff(repo_root, path, false));

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
    pub fn request(&mut self, req_path: PathBuf, kind: PreviewKind, git_repo_root: Option<PathBuf>, git_file_status: Option<FileStatus>) -> u64 {
        self.current_serial += 1;
        let serial = self.current_serial;
        let _ = self.request_tx.send(PreviewRequest {
            path: req_path,
            kind,
            serial,
            git_repo_root,
            git_file_status,
        });
        serial
    }

    /// Non-blocking poll for a completed result.
    pub fn try_recv(&self) -> Option<PreviewResponse> {
        self.result_rx.try_recv().ok()
    }

    /// Current serial number (latest request).
    #[cfg(test)]
    pub fn current_serial(&self) -> u64 {
        self.current_serial
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
        self.entries.push_front(entry);
        self.entries.front().map(|(_, c)| c)
    }

    /// Insert a preview into the cache.
    pub fn insert(&mut self, path: PathBuf, preview: CachedPreview) {
        // Remove existing entry for same path
        self.entries.retain(|(p, _)| p != &path);
        self.entries.push_front((path, preview));
        while self.entries.len() > self.max_size {
            self.entries.pop_back();
        }
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
}
