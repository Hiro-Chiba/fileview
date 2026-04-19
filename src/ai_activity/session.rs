//! Session registry: file-based rendezvous between the MCP server process and
//! interactive `fv` processes on the same machine.
//!
//! Each interactive `fv` registers a directory at
//! `<cache>/fileview/sessions/<pid>/` containing:
//! - `session.json`: metadata ({pid, root, started_at})
//! - `activity.jsonl`: append-only log of events (written by the MCP server,
//!   watched by the interactive process via `notify`).
//!
//! The MCP server discovers active sessions by scanning the sessions directory
//! and filters out those whose PID is no longer alive. Stale directories are
//! garbage-collected opportunistically on scan.

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Persistent metadata written to `session.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub pid: u32,
    pub root: PathBuf,
    pub started_at: u64,
}

/// Live session info with derived paths.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub meta: SessionMeta,
    pub dir: PathBuf,
    pub meta_file: PathBuf,
    pub activity_log: PathBuf,
}

/// Registry of interactive fileview sessions.
pub struct SessionRegistry {
    base_dir: PathBuf,
}

impl SessionRegistry {
    /// Create (or reuse) the registry directory rooted at the user's cache dir.
    pub fn new() -> Result<Self> {
        let cache = dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("cache dir unavailable on this platform"))?;
        let base_dir = cache.join("fileview").join("sessions");
        fs::create_dir_all(&base_dir)
            .with_context(|| format!("creating {}", base_dir.display()))?;
        Ok(Self { base_dir })
    }

    /// Build a registry rooted at an explicit directory (used by tests).
    pub fn at(base_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&base_dir)
            .with_context(|| format!("creating {}", base_dir.display()))?;
        Ok(Self { base_dir })
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Register the current process as an interactive session for `root`.
    pub fn register_current(&self, root: &Path) -> Result<SessionInfo> {
        let pid = process::id();
        let started_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let dir = self.base_dir.join(pid.to_string());
        // If a stale directory exists for the same PID (unlikely but possible on PID reuse),
        // wipe it first.
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;

        let meta = SessionMeta {
            pid,
            root: root.to_path_buf(),
            started_at,
        };
        let meta_file = dir.join("session.json");
        let meta_json = serde_json::to_string_pretty(&meta)?;
        fs::write(&meta_file, meta_json)
            .with_context(|| format!("writing {}", meta_file.display()))?;

        let activity_log = dir.join("activity.jsonl");
        // Touch the file so `notify` has something to watch from the start.
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&activity_log)
            .with_context(|| format!("creating {}", activity_log.display()))?;

        Ok(SessionInfo {
            meta,
            dir,
            meta_file,
            activity_log,
        })
    }

    /// Remove the session directory (idempotent, best-effort).
    pub fn unregister(&self, info: &SessionInfo) {
        let _ = fs::remove_dir_all(&info.dir);
    }

    /// List all sessions currently registered AND alive.
    ///
    /// Stale entries (dead PID / malformed metadata) are garbage-collected.
    pub fn list_alive(&self) -> Vec<SessionInfo> {
        let Ok(entries) = fs::read_dir(&self.base_dir) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let meta_file = path.join("session.json");
            let raw = match fs::read_to_string(&meta_file) {
                Ok(r) => r,
                Err(_) => {
                    let _ = fs::remove_dir_all(&path);
                    continue;
                }
            };
            let meta: SessionMeta = match serde_json::from_str(&raw) {
                Ok(m) => m,
                Err(_) => {
                    let _ = fs::remove_dir_all(&path);
                    continue;
                }
            };
            if !pid_alive(meta.pid) {
                let _ = fs::remove_dir_all(&path);
                continue;
            }
            let activity_log = path.join("activity.jsonl");
            out.push(SessionInfo {
                meta,
                dir: path,
                meta_file,
                activity_log,
            });
        }
        out
    }
}

/// Check whether a PID is still alive.
///
/// On Unix this is `kill(pid, 0)` — no signal is sent, the call just reports
/// whether the target exists. On other platforms we conservatively assume alive
/// (the session will be cleaned up on explicit unregister).
fn pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // SAFETY: `kill(pid, 0)` is the canonical POSIX existence check.
        let ret = unsafe { libc::kill(pid as libc::pid_t, 0) };
        if ret == 0 {
            return true;
        }
        // EPERM means the process exists but we can't signal it — still alive.
        std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn register_and_list_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let reg = SessionRegistry::at(tmp.path().to_path_buf()).unwrap();
        let root = tmp.path().to_path_buf();
        let info = reg.register_current(&root).unwrap();
        assert!(info.meta_file.is_file());
        assert!(info.activity_log.is_file());
        let alive = reg.list_alive();
        assert!(alive.iter().any(|s| s.meta.pid == info.meta.pid));
        reg.unregister(&info);
        assert!(!info.dir.exists());
    }

    #[test]
    fn list_alive_skips_malformed_session_json() {
        let tmp = TempDir::new().unwrap();
        let reg = SessionRegistry::at(tmp.path().to_path_buf()).unwrap();
        let garbage = tmp.path().join("99999");
        fs::create_dir_all(&garbage).unwrap();
        fs::write(garbage.join("session.json"), "not json").unwrap();
        assert!(reg.list_alive().is_empty());
        assert!(!garbage.exists(), "stale dir should be cleaned up");
    }
}
