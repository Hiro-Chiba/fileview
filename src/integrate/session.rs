//! Session persistence for fileview
//!
//! Saves and restores selection state to `.fileview-session.json`

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const SESSION_FILENAME: &str = ".fileview-session.json";

/// Session data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Selected file paths (relative to root)
    pub selected_paths: Vec<String>,
    /// Currently focused path (relative to root)
    pub focus_path: Option<String>,
    /// Timestamp when session was saved
    pub timestamp: u64,
    /// Root directory (for verification)
    pub root: String,
}

impl Session {
    /// Create a new session from current state
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

        // Convert absolute paths to relative paths
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

    /// Get session file path for a root directory
    fn session_path(root: &Path) -> PathBuf {
        root.join(SESSION_FILENAME)
    }

    /// Save session to file
    pub fn save(&self, root: &Path) -> io::Result<()> {
        let path = Self::session_path(root);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(path, json)
    }

    /// Load session from file
    pub fn load(root: &Path) -> io::Result<Self> {
        let path = Self::session_path(root);
        let json = fs::read_to_string(path)?;
        serde_json::from_str(&json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Convert relative paths back to absolute paths
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

        // 24 hours in seconds
        const DAY_SECS: u64 = 24 * 60 * 60;
        now.saturating_sub(self.timestamp) < DAY_SECS
    }

    /// Delete session file
    pub fn delete(root: &Path) -> io::Result<()> {
        let path = Self::session_path(root);
        if path.exists() {
            fs::remove_file(path)
        } else {
            Ok(())
        }
    }
}

/// Save current session state
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

/// Load session and return paths to restore
pub fn load_session(root: &Path) -> io::Result<(HashSet<PathBuf>, Option<PathBuf>)> {
    let session = Session::load(root)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_session_save_load() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create test files
        let file1 = root.join("file1.txt");
        let file2 = root.join("file2.txt");
        fs::write(&file1, "test1").unwrap();
        fs::write(&file2, "test2").unwrap();

        // Create session
        let mut selected = HashSet::new();
        selected.insert(file1.clone());
        selected.insert(file2.clone());

        // Save and load
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

        // Create and then delete a file
        let file1 = root.join("file1.txt");
        fs::write(&file1, "test1").unwrap();

        let mut selected = HashSet::new();
        selected.insert(file1.clone());
        save_session(root, &selected, None).unwrap();

        // Delete the file
        fs::remove_file(&file1).unwrap();

        // Load should filter out missing files
        let (loaded_selected, _) = load_session(root).unwrap();
        assert_eq!(loaded_selected.len(), 0);
    }
}
