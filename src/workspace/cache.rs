//! Versioned metadata cache. Loading does not establish filesystem freshness.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use super::index::{validate_relative, Entry, WorkspaceIndex, MAX_ENTRIES};

const MAX_CACHE_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cache {
    version: u32,
    root: PathBuf,
    entries: BTreeMap<PathBuf, Entry>,
}

impl WorkspaceIndex {
    /// Load unverified cached metadata. Reconcile before serving current results.
    pub fn load_cache(root: &Path, cache_path: &Path) -> Result<Self> {
        let mut index = Self::new(root)?;
        let file = fs::File::open(cache_path)?;
        if file.metadata()?.len() > MAX_CACHE_BYTES {
            bail!("workspace cache exceeds size limit");
        }
        let mut data = Vec::new();
        file.take(MAX_CACHE_BYTES + 1).read_to_end(&mut data)?;
        if data.len() as u64 > MAX_CACHE_BYTES {
            bail!("workspace cache exceeds size limit");
        }
        let cache: Cache = serde_json::from_slice(&data).context("decode workspace cache")?;
        if cache.version != 2 || cache.root != index.root {
            bail!("workspace cache version or root mismatch");
        }
        if cache.entries.len() > MAX_ENTRIES {
            bail!("workspace cache exceeds entry limit");
        }
        for (path, entry) in &cache.entries {
            validate_relative(path)?;
            let expected_hidden = path
                .components()
                .any(|part| part.as_os_str().to_string_lossy().starts_with('.'));
            let expected_extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            if path.to_str() != Some(entry.display.as_str())
                || entry.hidden != expected_hidden
                || entry.extension != expected_extension
            {
                bail!("workspace cache contains inconsistent metadata");
            }
        }
        index.entries = cache.entries;
        Ok(index)
    }

    /// Atomically replace a cache file. Non-UTF8 paths are never persisted lossily.
    pub fn save_cache(&self, cache_path: &Path) -> Result<()> {
        if self.root.to_str().is_none() || self.entries.keys().any(|path| path.to_str().is_none()) {
            bail!("workspace cache cannot persist non-UTF8 paths");
        }
        // Serialize by reference to avoid cloning the complete index.
        #[derive(Serialize)]
        struct CacheRef<'a> {
            version: u32,
            root: &'a Path,
            entries: &'a BTreeMap<PathBuf, Entry>,
        }
        let parent = cache_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let mut temporary = tempfile::Builder::new()
            .prefix(".fileview-index-")
            .tempfile_in(parent)?;
        {
            let mut writer = LimitedWriter {
                inner: BufWriter::new(temporary.as_file_mut()),
                remaining: MAX_CACHE_BYTES,
            };
            serde_json::to_writer(
                &mut writer,
                &CacheRef {
                    version: 2,
                    root: &self.root,
                    entries: &self.entries,
                },
            )?;
            writer.flush()?;
        }
        temporary.as_file().sync_all()?;
        temporary.persist(cache_path)?;
        Ok(())
    }
}

struct LimitedWriter<W> {
    inner: W,
    remaining: u64,
}

impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64 > self.remaining {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "workspace cache exceeds size limit",
            ));
        }
        let written = self.inner.write(bytes)?;
        self.remaining -= written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn cache_roundtrip_and_atomic_replacement() {
        let temp = tempdir().unwrap();
        let storage = tempdir().unwrap();
        let cache = storage.path().join("index.json");
        fs::write(temp.path().join("a.rs"), "abc").unwrap();
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        index.save_cache(&cache).unwrap();
        index.save_cache(&cache).unwrap();
        let loaded = WorkspaceIndex::load_cache(temp.path(), &cache).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.revision(), 0);
        assert_eq!(
            loaded.search("ext:rs", true, 10, &|| false).unwrap()[0].size,
            3
        );
        assert_eq!(fs::read_dir(storage.path()).unwrap().count(), 1);
    }

    #[test]
    fn rejects_future_schema_root_mismatch_and_traversal() {
        let temp = tempdir().unwrap();
        let storage = tempdir().unwrap();
        let cache = storage.path().join("index.json");
        fs::write(temp.path().join("a"), "").unwrap();
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        index.save_cache(&cache).unwrap();
        let original: serde_json::Value =
            serde_json::from_slice(&fs::read(&cache).unwrap()).unwrap();
        let mut future = original.clone();
        future["version"] = 999.into();
        fs::write(&cache, serde_json::to_vec(&future).unwrap()).unwrap();
        assert!(WorkspaceIndex::load_cache(temp.path(), &cache).is_err());
        let mut mismatch = original.clone();
        mismatch["root"] = "/elsewhere".into();
        fs::write(&cache, serde_json::to_vec(&mismatch).unwrap()).unwrap();
        assert!(WorkspaceIndex::load_cache(temp.path(), &cache).is_err());
        for path in ["../outside", "/outside", ".git/config"] {
            let mut malicious = original.clone();
            let entry = malicious["entries"]["a"].clone();
            malicious["entries"] = serde_json::json!({path: entry});
            fs::write(&cache, serde_json::to_vec(&malicious).unwrap()).unwrap();
            assert!(WorkspaceIndex::load_cache(temp.path(), &cache).is_err());
        }
    }
}
