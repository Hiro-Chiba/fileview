//! Session persistence for fileview

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const SESSION_FILENAME: &str = ".fileview-session.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Selected file paths (relative to root)
    pub selected_paths: Vec<String>,
    /// Currently focused path (relative to root)
    pub focus_path: Option<String>,
    /// Unix time in seconds when the session was saved.
    pub timestamp: u64,
    /// Root directory (for verification)
    pub root: String,
}

impl Session {
    pub fn new(
        root: &Path,
        selected_paths: &HashSet<PathBuf>,
        focus_path: Option<&PathBuf>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let root_str = root.display().to_string();

        let selected: Vec<String> = selected_paths
            .iter()
            .filter_map(|p| {
                p.strip_prefix(root)
                    .ok()
                    .map(|rel| rel.display().to_string())
            })
            .collect();

        let focus = focus_path.and_then(|p| {
            p.strip_prefix(root)
                .ok()
                .map(|rel| rel.display().to_string())
        });

        Self {
            selected_paths: selected,
            focus_path: focus,
            timestamp,
            root: root_str,
        }
    }

    fn session_path(root: &Path, name: Option<&str>) -> PathBuf {
        match name {
            None => root.join(SESSION_FILENAME),
            Some(raw) => {
                let normalized = normalize_session_name(raw);
                if normalized.is_empty() || normalized == "default" {
                    root.join(SESSION_FILENAME)
                } else {
                    root.join(format!(".fileview-session-{}.json", normalized))
                }
            }
        }
    }

    pub fn save(&self, root: &Path) -> io::Result<()> {
        self.save_named(root, None)
    }

    pub fn save_named(&self, root: &Path, name: Option<&str>) -> io::Result<()> {
        let path = Self::session_path(root, name);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(path, json)
    }

    pub fn load(root: &Path) -> io::Result<Self> {
        Self::load_named(root, None)
    }

    pub fn load_named(root: &Path, name: Option<&str>) -> io::Result<Self> {
        let path = Self::session_path(root, name);
        let json = crate::util::read_to_string_capped(&path, crate::util::MAX_STATE_FILE_BYTES)?;
        serde_json::from_str(&json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn to_absolute_paths(&self, root: &Path) -> (HashSet<PathBuf>, Option<PathBuf>) {
        let selected: HashSet<PathBuf> = self
            .selected_paths
            .iter()
            .map(|rel| root.join(rel))
            .filter(|p| p.exists())
            .collect();

        let focus = self
            .focus_path
            .as_ref()
            .map(|rel| root.join(rel))
            .filter(|p| p.exists());

        (selected, focus)
    }

    /// Check if session is recent (within 24 hours)
    pub fn is_recent(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        const DAY_SECS: u64 = 24 * 60 * 60;
        now.saturating_sub(self.timestamp) < DAY_SECS
    }

    pub fn delete(root: &Path) -> io::Result<()> {
        Self::delete_named(root, None)
    }

    pub fn delete_named(root: &Path, name: Option<&str>) -> io::Result<()> {
        let path = Self::session_path(root, name);
        if path.exists() {
            fs::remove_file(path)
        } else {
            Ok(())
        }
    }
}

pub fn save_session(
    root: &Path,
    selected_paths: &HashSet<PathBuf>,
    focus_path: Option<&PathBuf>,
) -> io::Result<usize> {
    let session = Session::new(root, selected_paths, focus_path);
    let count = session.selected_paths.len();
    session.save(root)?;
    Ok(count)
}

pub fn save_session_named(
    root: &Path,
    selected_paths: &HashSet<PathBuf>,
    focus_path: Option<&PathBuf>,
    name: Option<&str>,
) -> io::Result<usize> {
    let session = Session::new(root, selected_paths, focus_path);
    let count = session.selected_paths.len();
    session.save_named(root, name)?;
    Ok(count)
}

pub fn load_session(root: &Path) -> io::Result<(HashSet<PathBuf>, Option<PathBuf>)> {
    load_session_named(root, None)
}

pub fn load_session_named(
    root: &Path,
    name: Option<&str>,
) -> io::Result<(HashSet<PathBuf>, Option<PathBuf>)> {
    let session = Session::load_named(root, name)?;

    // Verify this session belongs to the same root
    let session_root = PathBuf::from(&session.root);
    if session_root.canonicalize().ok() != root.canonicalize().ok() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Session root mismatch",
        ));
    }

    Ok(session.to_absolute_paths(root))
}

fn normalize_session_name(name: &str) -> String {
    name.trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_session_save_load() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let file1 = root.join("file1.txt");
        let file2 = root.join("file2.txt");
        fs::write(&file1, "test1").unwrap();
        fs::write(&file2, "test2").unwrap();

        let mut selected = HashSet::new();
        selected.insert(file1.clone());
        selected.insert(file2.clone());

        let count = save_session(root, &selected, Some(&file1)).unwrap();
        assert_eq!(count, 2);

        let (loaded_selected, loaded_focus) = load_session(root).unwrap();
        assert_eq!(loaded_selected.len(), 2);
        assert!(loaded_selected.contains(&file1));
        assert!(loaded_selected.contains(&file2));
        assert_eq!(loaded_focus, Some(file1));
    }

    #[test]
    fn test_session_missing_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let file1 = root.join("file1.txt");
        fs::write(&file1, "test1").unwrap();

        let mut selected = HashSet::new();
        selected.insert(file1.clone());
        save_session(root, &selected, None).unwrap();

        fs::remove_file(&file1).unwrap();

        // Load should filter out missing files
        let (loaded_selected, _) = load_session(root).unwrap();
        assert_eq!(loaded_selected.len(), 0);
    }

    #[test]
    fn test_named_session_save_load() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let file1 = root.join("a.txt");
        fs::write(&file1, "a").unwrap();

        let mut selected = HashSet::new();
        selected.insert(file1.clone());
        save_session_named(root, &selected, Some(&file1), Some("ai")).unwrap();

        let (loaded_selected, loaded_focus) = load_session_named(root, Some("ai")).unwrap();
        assert!(loaded_selected.contains(&file1));
        assert_eq!(loaded_focus, Some(file1));
    }
}
