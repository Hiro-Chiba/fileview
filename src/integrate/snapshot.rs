//! Snapshot capture and diff for AI workflows.
//!
//! Captures the working tree as a manifest of (path, size, mtime) tuples and
//! lets a later invocation enumerate what was added, removed, or modified
//! since the snapshot was taken.
//!
//! Designed for the case where a long-running AI session wants to know
//! "what files have I (or the user) touched since I started", without
//! depending on git or committing intermediate state.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

const SNAPSHOTS_SUBDIR: &str = ".fileview/snapshots";

/// One file entry inside a snapshot manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    pub size: u64,
    /// Unix mtime in seconds since the epoch.
    pub mtime_secs: i64,
    /// Sub-second precision in nanoseconds, kept separate so older formats
    /// stay parseable if we ever drop it.
    pub mtime_nanos: u32,
}

/// Stored snapshot manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub name: String,
    pub created_at_secs: i64,
    /// `path -> entry` keyed by repo-relative path with forward slashes.
    pub files: BTreeMap<String, FileEntry>,
}

/// One diff entry produced by [`diff_snapshot`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffOp {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEntry {
    pub op: DiffOp,
    pub path: String,
}

impl DiffEntry {
    /// Plain-text rendering: `+ path`, `- path`, `M path`.
    pub fn render(&self) -> String {
        let prefix = match self.op {
            DiffOp::Added => '+',
            DiffOp::Removed => '-',
            DiffOp::Modified => 'M',
        };
        format!("{} {}", prefix, self.path)
    }
}

/// Walk `root` and build a fresh [`Snapshot`], skipping dotfiles and the
/// fileview metadata directory itself.
pub fn capture_snapshot(name: &str, root: &Path) -> io::Result<Snapshot> {
    let mut files = BTreeMap::new();
    collect(root, root, &mut files)?;
    Ok(Snapshot {
        name: name.to_string(),
        created_at_secs: now_secs(),
        files,
    })
}

/// Persist a snapshot to `<root>/.fileview/snapshots/<name>.json`.
pub fn save_snapshot(root: &Path, snapshot: &Snapshot) -> io::Result<PathBuf> {
    let dir = root.join(SNAPSHOTS_SUBDIR);
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", snapshot.name));
    let json = serde_json::to_string_pretty(snapshot).map_err(io::Error::other)?;
    fs::write(&path, json)?;
    Ok(path)
}

/// Load a previously saved snapshot from `<root>/.fileview/snapshots/<name>.json`.
pub fn load_snapshot(root: &Path, name: &str) -> io::Result<Snapshot> {
    let path = root.join(SNAPSHOTS_SUBDIR).join(format!("{}.json", name));
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("snapshot '{}' not found at {}", name, path.display()),
        ));
    }
    let text = crate::util::read_to_string_capped(&path, crate::util::MAX_STATE_FILE_BYTES)?;
    serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Compute the diff between a stored snapshot and the current state of `root`.
pub fn diff_snapshot(root: &Path, snapshot: &Snapshot) -> io::Result<Vec<DiffEntry>> {
    let current = capture_snapshot(&snapshot.name, root)?;
    let mut out = Vec::new();

    for (path, entry) in &snapshot.files {
        match current.files.get(path) {
            None => out.push(DiffEntry {
                op: DiffOp::Removed,
                path: path.clone(),
            }),
            Some(curr) if curr != entry => out.push(DiffEntry {
                op: DiffOp::Modified,
                path: path.clone(),
            }),
            _ => {}
        }
    }

    for path in current.files.keys() {
        if !snapshot.files.contains_key(path) {
            out.push(DiffEntry {
                op: DiffOp::Added,
                path: path.clone(),
            });
        }
    }

    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn collect(root: &Path, dir: &Path, out: &mut BTreeMap<String, FileEntry>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        if file_type.is_symlink() {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if metadata.is_dir() {
            collect(root, &path, out)?;
        } else if metadata.is_file() {
            let rel = path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| path.to_string_lossy().into_owned());
            let (mtime_secs, mtime_nanos) = decompose_mtime(&metadata);
            out.insert(
                rel,
                FileEntry {
                    size: metadata.len(),
                    mtime_secs,
                    mtime_nanos,
                },
            );
        }
    }
    Ok(())
}

fn decompose_mtime(metadata: &fs::Metadata) -> (i64, u32) {
    metadata
        .modified()
        .ok()
        .and_then(|m| m.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| (d.as_secs() as i64, d.subsec_nanos()))
        .unwrap_or((0, 0))
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn touch(dir: &Path, rel: &str, contents: &str) {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, contents).unwrap();
    }

    #[test]
    fn capture_then_diff_with_no_changes_is_empty() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "src/lib.rs", "fn lib() {}\n");
        let snap = capture_snapshot("base", tmp.path()).unwrap();
        let diff = diff_snapshot(tmp.path(), &snap).unwrap();
        assert!(diff.is_empty(), "expected empty diff, got {:?}", diff);
    }

    #[test]
    fn diff_detects_added_removed_modified() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "keep.txt", "stable\n");
        touch(tmp.path(), "drop.txt", "doomed\n");
        touch(tmp.path(), "edit.txt", "before\n");
        let snap = capture_snapshot("base", tmp.path()).unwrap();

        fs::remove_file(tmp.path().join("drop.txt")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        touch(tmp.path(), "edit.txt", "after with more bytes\n");
        touch(tmp.path(), "new.txt", "freshly minted\n");

        let diff = diff_snapshot(tmp.path(), &snap).unwrap();
        let rendered: Vec<String> = diff.iter().map(DiffEntry::render).collect();
        assert!(
            rendered.contains(&"+ new.txt".to_string()),
            "{:?}",
            rendered
        );
        assert!(
            rendered.contains(&"- drop.txt".to_string()),
            "{:?}",
            rendered
        );
        assert!(
            rendered.contains(&"M edit.txt".to_string()),
            "{:?}",
            rendered
        );
        assert!(
            !rendered.iter().any(|r| r.contains("keep.txt")),
            "{:?}",
            rendered
        );
    }

    #[test]
    fn capture_skips_dotfiles_and_dot_dirs() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "src/lib.rs", "fn lib() {}\n");
        touch(tmp.path(), ".hidden", "x\n");
        touch(tmp.path(), ".cache/index", "x\n");
        let snap = capture_snapshot("base", tmp.path()).unwrap();
        assert!(snap.files.contains_key("src/lib.rs"));
        assert!(!snap.files.iter().any(|(p, _)| p.contains(".hidden")));
        assert!(!snap.files.iter().any(|(p, _)| p.contains(".cache")));
    }

    #[cfg(unix)]
    #[test]
    fn capture_does_not_follow_symlinks() {
        use std::os::unix::fs::symlink;

        let root = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        touch(outside.path(), "outside.txt", "outside\n");
        symlink(outside.path(), root.path().join("external")).unwrap();
        symlink(root.path(), root.path().join("loop")).unwrap();

        let snapshot = capture_snapshot("base", root.path()).unwrap();
        assert!(snapshot.files.is_empty());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "src/lib.rs", "fn lib() {}\n");
        let snap = capture_snapshot("foo", tmp.path()).unwrap();
        let path = save_snapshot(tmp.path(), &snap).unwrap();
        assert!(path.exists());
        let loaded = load_snapshot(tmp.path(), "foo").unwrap();
        assert_eq!(loaded.name, snap.name);
        assert_eq!(loaded.files, snap.files);
    }

    #[test]
    fn load_missing_snapshot_returns_not_found() {
        let tmp = TempDir::new().unwrap();
        let err = load_snapshot(tmp.path(), "nope").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
