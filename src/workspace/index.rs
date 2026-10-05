//! Path/type workspace index with transactional, incremental updates.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{bail, Context, Result};
use ignore::WalkBuilder;
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Matcher, Utf32Str,
};
use serde::{Deserialize, Serialize};

use super::query::WorkspaceQuery;
use crate::git::{FileStatus, GitStatus};

pub(crate) const MAX_ENTRIES: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub(crate) display: String,
    pub(crate) is_dir: bool,
    pub(crate) hidden: bool,
    pub(crate) extension: String,
}

/// One matching workspace path and its current filesystem metadata.
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceMatch {
    /// Absolute filesystem path.
    pub path: PathBuf,
    /// Relative display path.
    pub display: String,
    /// Fuzzy score, descending in returned results.
    pub score: u32,
    /// Character positions to highlight.
    pub indices: Vec<usize>,
    /// Whether the entry is a directory.
    pub is_dir: bool,
    /// Metadata size in bytes.
    pub size: u64,
    /// Modification time, when available.
    pub modified_unix_secs: Option<u64>,
}

/// Cached paths rooted at one canonical directory.
#[derive(Debug)]
pub struct WorkspaceIndex {
    pub(crate) root: PathBuf,
    pub(crate) entries: BTreeMap<PathBuf, Entry>,
    pub(crate) revision: u64,
    git: Option<GitStatus>,
}

/// A prepared update whose filesystem work has completed before publication.
#[derive(Debug)]
pub(crate) struct WorkspaceDelta {
    root: PathBuf,
    revision: u64,
    replace: bool,
    fresh: BTreeMap<PathBuf, Entry>,
    removed: Vec<PathBuf>,
}

impl WorkspaceDelta {
    /// Whether indexed paths or ignore policy may require refreshing Git predicates.
    pub(crate) fn has_indexed_paths(&self) -> bool {
        self.replace || !self.removed.is_empty() || !self.fresh.is_empty()
    }
}

impl WorkspaceIndex {
    /// Create an empty index. Call rebuild before considering it current.
    pub fn new(root: &Path) -> Result<Self> {
        let root = root.canonicalize().context("resolve workspace root")?;
        if !root.is_dir() {
            bail!("workspace root must be a directory");
        }
        Ok(Self {
            root,
            entries: BTreeMap::new(),
            revision: 0,
            git: None,
        })
    }

    /// Canonical root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }
    /// Indexed path count, excluding the root itself.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    /// Whether there are no indexed paths.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Monotonic revision of successful in-memory updates.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Replace metadata atomically after a complete scan succeeds.
    pub fn rebuild(&mut self, cancelled: &dyn Fn() -> bool) -> Result<()> {
        let entries = self.scan(&[], cancelled)?;
        self.entries = entries;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    /// Reconcile relative or canonical-root absolute paths and their subtrees.
    pub fn reconcile(&mut self, paths: &[PathBuf], cancelled: &dyn Fn() -> bool) -> Result<()> {
        let delta = self.prepare_reconcile(paths, cancelled)?;
        self.apply_delta(delta)
    }

    /// Prepare filesystem changes under a shared read lock without blocking queries.
    pub(crate) fn prepare_reconcile(
        &self,
        paths: &[PathBuf],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<WorkspaceDelta> {
        let mut relative = Vec::new();
        for path in paths {
            let path = if path.is_absolute() {
                path.strip_prefix(&self.root)?.to_path_buf()
            } else {
                path.clone()
            };
            if path.as_os_str().is_empty() {
                return self.prepare_rebuild(cancelled);
            }
            if path
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
            {
                bail!("invalid workspace relative path");
            }
            if path
                .file_name()
                .is_some_and(|name| name == ".gitignore" || name == ".ignore")
                || path.components().any(|part| part.as_os_str() == ".git")
            {
                return self.prepare_rebuild(cancelled);
            }
            relative.push(path);
        }
        if relative.is_empty() {
            return Ok(WorkspaceDelta {
                root: self.root.clone(),
                revision: self.revision,
                replace: false,
                fresh: BTreeMap::new(),
                removed: Vec::new(),
            });
        }
        relative.sort();
        relative.dedup();
        let mut scopes: Vec<PathBuf> = Vec::new();
        let mut directories = BTreeMap::new();
        for mut path in relative {
            // A child notification may arrive without its new, removed, or
            // replaced directory. Reconcile from the first uncertain ancestor.
            let parents: Vec<_> = path
                .ancestors()
                .skip(1)
                .filter(|parent| !parent.as_os_str().is_empty())
                .map(Path::to_path_buf)
                .collect();
            for parent in parents.into_iter().rev() {
                let valid_directory = if let Some(valid) = directories.get(&parent) {
                    *valid
                } else {
                    let valid = if self.entries.get(&parent).is_some_and(|entry| entry.is_dir) {
                        match self.root.join(&parent).symlink_metadata() {
                            Ok(metadata) => metadata.is_dir(),
                            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                            Err(error) => return Err(error.into()),
                        }
                    } else {
                        false
                    };
                    directories.insert(parent.clone(), valid);
                    valid
                };
                if !valid_directory {
                    path = parent;
                    break;
                }
            }
            if !scopes.last().is_some_and(|parent| path.starts_with(parent)) {
                scopes.push(path);
            }
        }
        let relative = scopes;
        let fresh = self.scan(&relative, cancelled)?;
        check_cancelled(cancelled)?;
        let removed: Vec<_> = relative
            .iter()
            .flat_map(|changed| {
                self.entries
                    .range(changed.clone()..)
                    .take_while(move |(path, _)| path.starts_with(changed))
                    .map(|(path, _)| path.clone())
            })
            .collect();
        if self.entries.len() - removed.len() + fresh.len() > MAX_ENTRIES {
            bail!("workspace exceeds {MAX_ENTRIES} entries");
        }
        Ok(WorkspaceDelta {
            root: self.root.clone(),
            revision: self.revision,
            replace: false,
            fresh,
            removed,
        })
    }

    fn prepare_rebuild(&self, cancelled: &dyn Fn() -> bool) -> Result<WorkspaceDelta> {
        Ok(WorkspaceDelta {
            root: self.root.clone(),
            revision: self.revision,
            replace: true,
            fresh: self.scan(&[], cancelled)?,
            removed: Vec::new(),
        })
    }

    /// Publish a prepared delta only if its source snapshot is still current.
    pub(crate) fn apply_delta(&mut self, delta: WorkspaceDelta) -> Result<()> {
        if delta.root != self.root || delta.revision != self.revision {
            bail!("workspace changed while preparing update");
        }
        if !delta.has_indexed_paths() {
            return Ok(());
        }
        if delta.replace {
            self.entries = delta.fresh;
        } else {
            for path in delta.removed {
                self.entries.remove(&path);
            }
            self.entries.extend(delta.fresh);
        }
        // Indexed touches must refresh active queries even when paths and types
        // are unchanged. Winner metadata is deliberately read at search time.
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    fn scan(
        &self,
        changed: &[PathBuf],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<BTreeMap<PathBuf, Entry>> {
        let selected: BTreeSet<_> = changed.iter().map(|path| self.root.join(path)).collect();
        let changed: BTreeSet<_> = changed.iter().cloned().collect();
        let mut builder = WalkBuilder::new(&self.root);
        builder
            .hidden(false)
            .git_global(false)
            .parents(false)
            .follow_links(false)
            .require_git(false)
            .filter_entry(move |entry| {
                entry.file_name() != ".git"
                    && (selected.is_empty()
                        || entry
                            .path()
                            .ancestors()
                            .any(|ancestor| selected.contains(ancestor))
                        || selected
                            .range(entry.path().to_path_buf()..)
                            .next()
                            .is_some_and(|path| path.starts_with(entry.path())))
            });
        let mut entries = BTreeMap::new();
        for result in builder.build() {
            check_cancelled(cancelled)?;
            let item = result.context("scan workspace metadata")?;
            if item.depth() == 0 {
                continue;
            }
            let relative = item.path().strip_prefix(&self.root)?;
            if !changed.is_empty()
                && !relative
                    .ancestors()
                    .any(|ancestor| changed.contains(ancestor))
            {
                continue;
            }
            entries.insert(
                relative.to_path_buf(),
                Entry {
                    display: relative.to_string_lossy().into_owned(),
                    is_dir: item.file_type().is_some_and(|kind| kind.is_dir()),
                    hidden: relative
                        .components()
                        .any(|part| part.as_os_str().to_string_lossy().starts_with('.')),
                    extension: relative
                        .extension()
                        .map(|value| value.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                },
            );
            if entries.len() > MAX_ENTRIES {
                bail!("workspace exceeds {MAX_ENTRIES} entries");
            }
        }
        check_cancelled(cancelled)?;
        Ok(entries)
    }

    /// Refresh Git predicates independently of the metadata index.
    pub fn refresh_git(&mut self) -> bool {
        self.set_git(GitStatus::detect_for_workspace(&self.root))
    }

    /// Publish Git metadata prepared without holding an exclusive index lock.
    pub(crate) fn set_git(&mut self, git: Option<GitStatus>) -> bool {
        if self.git == git {
            return false;
        }
        self.git = git;
        self.revision = self.revision.saturating_add(1);
        true
    }

    /// Search cached paths with bounded top-k scoring and winner-only metadata.
    /// Concurrently vanished or type-changed winners are omitted, so a racing
    /// filesystem update can produce fewer than `limit` results until reconciled.
    pub fn search(
        &self,
        query: &str,
        show_hidden: bool,
        limit: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<WorkspaceMatch>> {
        let query = WorkspaceQuery::parse(query)?;
        check_cancelled(cancelled)?;
        if limit == 0 {
            return Ok(Vec::new());
        }
        let pattern = Pattern::parse(&query.text, CaseMatching::Smart, Normalization::Smart);
        let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
        let mut buffer = Vec::new();
        // Reverse score makes the worst retained candidate the heap maximum.
        let mut winners = BinaryHeap::new();
        for (position, (path, entry)) in self.entries.iter().enumerate() {
            if position % 128 == 0 {
                check_cancelled(cancelled)?;
            }
            if !show_hidden && entry.hidden {
                continue;
            }
            let status = if query.needs_git() {
                self.git
                    .as_ref()
                    .map(|git| {
                        if git.repo_root() == self.root {
                            git.get_status(path)
                        } else {
                            git.get_status(&self.root.join(path))
                        }
                    })
                    .unwrap_or(FileStatus::Clean)
            } else {
                FileStatus::Clean
            };
            if !query.matches(entry, status) {
                continue;
            }
            let score = if query.text.is_empty() {
                Some(0)
            } else {
                pattern.score(Utf32Str::new(&entry.display, &mut buffer), &mut matcher)
            };
            if let Some(score) = score {
                let candidate = (Reverse(score), path);
                if winners.len() < limit {
                    winners.push(candidate);
                } else if winners.peek().is_some_and(|worst| candidate < *worst) {
                    winners.pop();
                    winners.push(candidate);
                }
                // Filter-only results all have score zero. Ordered paths make
                // the first accepted limit entries the final top-k candidates.
                if query.text.is_empty() && winners.len() == limit {
                    break;
                }
            }
        }
        let mut results = Vec::with_capacity(winners.len());
        for (Reverse(score), path) in winners.into_sorted_vec() {
            check_cancelled(cancelled)?;
            let entry = &self.entries[path];
            let Some(metadata) = self.winner_metadata(path)? else {
                continue;
            };
            if !query.matches_kind(metadata.is_dir()) {
                continue;
            }
            let mut indices = Vec::new();
            if !query.text.is_empty() {
                pattern.indices(
                    Utf32Str::new(&entry.display, &mut buffer),
                    &mut matcher,
                    &mut indices,
                );
            }
            indices.sort_unstable();
            indices.dedup();
            results.push(WorkspaceMatch {
                path: self.root.join(path),
                display: entry.display.clone(),
                score,
                indices: indices.into_iter().map(|index| index as usize).collect(),
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                modified_unix_secs: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|time| time.as_secs()),
            });
        }
        Ok(results)
    }

    fn winner_metadata(&self, relative: &Path) -> Result<Option<std::fs::Metadata>> {
        let mut path = self.root.clone();
        let mut parts = relative.components().peekable();
        // Do not follow a directory that has become a symlink since the scan.
        if !self.root.symlink_metadata()?.is_dir() {
            return Ok(None);
        }
        while let Some(part) = parts.next() {
            path.push(part);
            let metadata = match path.symlink_metadata() {
                Ok(metadata) => metadata,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                    ) =>
                {
                    return Ok(None)
                }
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("read workspace metadata for {}", path.display()))
                }
            };
            if parts.peek().is_none() {
                return Ok(Some(metadata));
            }
            if !metadata.is_dir() {
                return Ok(None);
            }
        }
        Ok(None)
    }
}

pub(crate) fn validate_relative(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)) || part.as_os_str() == ".git")
    {
        bail!("invalid workspace relative path");
    }
    Ok(())
}

fn check_cancelled(cancelled: &dyn Fn() -> bool) -> Result<()> {
    if cancelled() {
        bail!("workspace operation cancelled");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn names(index: &WorkspaceIndex, query: &str, hidden: bool) -> Vec<String> {
        index
            .search(query, hidden, usize::MAX, &|| false)
            .unwrap()
            .into_iter()
            .map(|entry| entry.display)
            .collect()
    }

    #[test]
    fn reconcile_insert_edit_delete_and_directory_rename_matches_fresh_scan() {
        let temp = tempdir().unwrap();
        fs::create_dir(temp.path().join("old")).unwrap();
        fs::write(temp.path().join("old/a.rs"), "a").unwrap();
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        fs::write(temp.path().join("old/a.rs"), "longer").unwrap();
        fs::write(temp.path().join("old/b.rs"), "b").unwrap();
        index
            .reconcile(&["old/a.rs".into(), "old/b.rs".into()], &|| false)
            .unwrap();
        assert_eq!(index.search("a.rs", true, 1, &|| false).unwrap()[0].size, 6);
        fs::rename(temp.path().join("old"), temp.path().join("new")).unwrap();
        index
            .reconcile(&["old".into(), "new".into()], &|| false)
            .unwrap();
        fs::remove_file(temp.path().join("new/b.rs")).unwrap();
        index.reconcile(&["new/b.rs".into()], &|| false).unwrap();
        let mut fresh = WorkspaceIndex::new(temp.path()).unwrap();
        fresh.rebuild(&|| false).unwrap();
        assert_eq!(
            serde_json::to_value(index.search("", true, usize::MAX, &|| false).unwrap()).unwrap(),
            serde_json::to_value(fresh.search("", true, usize::MAX, &|| false).unwrap()).unwrap()
        );
    }

    #[test]
    fn descendant_only_events_reconcile_new_and_removed_ancestors() {
        let temp = tempdir().unwrap();
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        fs::create_dir_all(temp.path().join("new/nested")).unwrap();
        fs::write(temp.path().join("new/nested/a.rs"), "a").unwrap();
        fs::write(temp.path().join("new/nested/b.rs"), "b").unwrap();
        index
            .reconcile(&["new/nested/a.rs".into()], &|| false)
            .unwrap();
        let mut reference = WorkspaceIndex::new(temp.path()).unwrap();
        reference.rebuild(&|| false).unwrap();
        assert_eq!(index.entries, reference.entries);
        fs::remove_dir_all(temp.path().join("new")).unwrap();
        index
            .reconcile(&["new/nested/a.rs".into()], &|| false)
            .unwrap();
        reference.rebuild(&|| false).unwrap();
        assert_eq!(index.entries, reference.entries);
    }

    #[cfg(unix)]
    #[test]
    fn descendant_event_cannot_keep_children_of_a_replaced_symlink_directory() {
        let temp = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("original/nested")).unwrap();
        fs::write(temp.path().join("original/nested/a.rs"), "a").unwrap();
        fs::write(temp.path().join("original/nested/b.rs"), "b").unwrap();
        fs::write(outside.path().join("secret.rs"), "external").unwrap();
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        fs::remove_dir_all(temp.path().join("original")).unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("original")).unwrap();
        index
            .reconcile(&["original/nested/a.rs".into()], &|| false)
            .unwrap();
        let mut reference = WorkspaceIndex::new(temp.path()).unwrap();
        reference.rebuild(&|| false).unwrap();
        assert_eq!(index.entries, reference.entries);
        assert_eq!(names(&index, "", true), ["original"]);
    }

    #[test]
    fn prepared_delta_keeps_queries_available_and_rejects_stale_publication() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("a"), "a").unwrap();
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        fs::write(temp.path().join("b"), "b").unwrap();
        let delta = index.prepare_reconcile(&["b".into()], &|| false).unwrap();
        assert_eq!(names(&index, "", true), ["a"]);
        index.apply_delta(delta).unwrap();
        assert_eq!(names(&index, "", true), ["a", "b"]);
        let stale = index.prepare_reconcile(&["a".into()], &|| false).unwrap();
        index.rebuild(&|| false).unwrap();
        let revision = index.revision();
        assert!(index.apply_delta(stale).is_err());
        assert_eq!(index.revision(), revision);
        assert_eq!(names(&index, "", true), ["a", "b"]);
    }

    #[test]
    fn ignored_write_burst_does_not_change_revision_or_require_git_refresh() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join(".gitignore"), "ignored/\n").unwrap();
        fs::create_dir(temp.path().join("ignored")).unwrap();
        fs::write(temp.path().join("visible.rs"), "unchanged").unwrap();
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        let revision = index.revision();
        let mut paths = Vec::new();
        for number in 0..1000 {
            let path = index.root().join(format!("ignored/generated_{number}.rs"));
            fs::write(&path, "generated").unwrap();
            paths.push(path);
        }
        let delta = index.prepare_reconcile(&paths, &|| false).unwrap();
        assert!(!delta.has_indexed_paths());
        index.apply_delta(delta).unwrap();
        assert_eq!(index.revision(), revision);
        let visible = index
            .prepare_reconcile(&["visible.rs".into()], &|| false)
            .unwrap();
        assert!(visible.has_indexed_paths());
        index.apply_delta(visible).unwrap();
        assert_eq!(index.revision(), revision + 1);
    }

    #[test]
    fn ignore_rules_hidden_paths_and_ignore_changes_remain_consistent() {
        // Compare path components so native Windows separators remain valid.
        let paths = |index: &WorkspaceIndex, query: &str| {
            names(index, query, false)
                .into_iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>()
        };
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("ignored/nested")).unwrap();
        fs::create_dir_all(temp.path().join("visible")).unwrap();
        fs::create_dir_all(temp.path().join(".git/objects")).unwrap();
        fs::write(temp.path().join(".gitignore"), "ignored/\n*.tmp\n").unwrap();
        fs::write(temp.path().join("visible/.gitignore"), "*.rs\n!keep.rs\n").unwrap();
        for path in [
            "ignored/nested/a.rs",
            "visible/no.rs",
            "visible/keep.rs",
            ".secret",
            "a.tmp",
        ] {
            fs::write(temp.path().join(path), "a").unwrap();
        }
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        assert_eq!(
            paths(&index, "type:file"),
            [PathBuf::from("visible/keep.rs")]
        );
        assert!(names(&index, "", true).contains(&".secret".to_owned()));
        index
            .reconcile(
                &["ignored/nested/a.rs".into(), "visible/no.rs".into()],
                &|| false,
            )
            .unwrap();
        assert_eq!(
            paths(&index, "type:file"),
            [PathBuf::from("visible/keep.rs")]
        );
        fs::write(temp.path().join("visible/.gitignore"), "").unwrap();
        index
            .reconcile(&["visible/.gitignore".into()], &|| false)
            .unwrap();
        assert_eq!(
            paths(&index, "ext:rs"),
            [
                PathBuf::from("visible/keep.rs"),
                PathBuf::from("visible/no.rs")
            ]
        );
    }

    #[test]
    fn cancelled_rebuild_and_reconcile_leave_snapshot_unchanged() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("a"), "a").unwrap();
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        let revision = index.revision();
        fs::write(temp.path().join("b"), "b").unwrap();
        assert!(index.rebuild(&|| true).is_err());
        assert!(index.reconcile(&["b".into()], &|| true).is_err());
        assert_eq!(names(&index, "", true), ["a"]);
        assert_eq!(index.revision(), revision);
        let calls = std::cell::Cell::new(0);
        assert!(index
            .rebuild(&|| {
                calls.set(calls.get() + 1);
                calls.get() > 2
            })
            .is_err());
        assert_eq!(names(&index, "", true), ["a"]);
        assert_eq!(index.revision(), revision);
        assert!(index.search("", true, 5, &|| true).is_err());
    }

    #[test]
    fn bounded_ranking_agrees_with_unbounded_and_rejects_invalid_filters() {
        let temp = tempdir().unwrap();
        for name in [
            "workspace.rs",
            "work.rs",
            "worker.rs",
            "world.rs",
            "work.txt",
            "日本.rs",
        ] {
            fs::write(temp.path().join(name), "").unwrap();
        }
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        let all = index
            .search("wrk ext:rs type:file", true, 100, &|| false)
            .unwrap();
        let top = index
            .search("wrk ext:rs type:file", true, 2, &|| false)
            .unwrap();
        assert_eq!(
            top.iter().map(|item| &item.path).collect::<Vec<_>>(),
            all.iter()
                .take(2)
                .map(|item| &item.path)
                .collect::<Vec<_>>()
        );
        assert!(top.iter().all(|item| !item.indices.is_empty()));
        assert!(index.search("type:banana", true, 5, &|| false).is_err());
        assert!(index.search("git:banana", true, 5, &|| false).is_err());
        assert!(index.search("ext:", true, 5, &|| false).is_err());
    }

    #[test]
    fn filter_only_top_k_matches_unbounded_lexicographic_prefix() {
        let temp = tempdir().unwrap();
        fs::create_dir(temp.path().join("a_directory")).unwrap();
        for name in [".hidden.rs", "a.txt", "b.rs", "c.rs", "d.rs", "e.txt"] {
            fs::write(temp.path().join(name), "").unwrap();
        }
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        for query in [
            "",
            "type:file",
            "ext:rs type:file",
            "type:dir",
            "ext:missing",
        ] {
            for hidden in [false, true] {
                let all = index.search(query, hidden, usize::MAX, &|| false).unwrap();
                let limited = index.search(query, hidden, 2, &|| false).unwrap();
                assert_eq!(
                    serde_json::to_value(&limited).unwrap(),
                    serde_json::to_value(all.into_iter().take(2).collect::<Vec<_>>()).unwrap(),
                    "{query}, hidden={hidden}"
                );
            }
        }
    }

    #[test]
    fn winner_metadata_is_current_and_indexed_touches_refresh_the_revision() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("file.rs");
        fs::write(&path, "a").unwrap();
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        let revision = index.revision();
        fs::write(&path, "changed").unwrap();
        assert_eq!(
            index.search("file", true, 15, &|| false).unwrap()[0].size,
            7
        );
        index.reconcile(&["file.rs".into()], &|| false).unwrap();
        assert_eq!(index.revision(), revision + 1);
        fs::write(&path, "another").unwrap();
        index.reconcile(&["file.rs".into()], &|| false).unwrap();
        assert_eq!(index.revision(), revision + 2);
    }

    #[test]
    fn vanished_or_changed_type_winners_are_omitted_without_failing_search() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("file.rs");
        fs::write(&path, "a").unwrap();
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        fs::remove_file(&path).unwrap();
        assert!(index
            .search("file", true, 15, &|| false)
            .unwrap()
            .is_empty());
        fs::create_dir(&path).unwrap();
        assert!(index
            .search("type:file", true, 15, &|| false)
            .unwrap()
            .is_empty());
        assert!(index
            .search("ext:rs", true, 15, &|| false)
            .unwrap()
            .is_empty());
        index.reconcile(&["file.rs".into()], &|| false).unwrap();
        assert_eq!(
            index.search("type:dir", true, 15, &|| false).unwrap().len(),
            1
        );
        fs::remove_dir(&path).unwrap();
        fs::write(&path, "a").unwrap();
        assert!(index
            .search("type:dir", true, 15, &|| false)
            .unwrap()
            .is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn winner_metadata_does_not_follow_replaced_ancestor_symlinks() {
        let temp = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::create_dir(temp.path().join("dir")).unwrap();
        fs::write(temp.path().join("dir/file.rs"), "inside").unwrap();
        fs::write(outside.path().join("file.rs"), "outside").unwrap();
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        fs::remove_dir_all(temp.path().join("dir")).unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("dir")).unwrap();
        assert!(index
            .search("file", true, 15, &|| false)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn extension_filters_preserve_unicode_and_ignore_ascii_case() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("one.Ω"), "").unwrap();
        fs::write(temp.path().join("two.RS"), "").unwrap();
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        assert_eq!(names(&index, "ext:Ω", true), ["one.Ω"]);
        assert_eq!(names(&index, "ext:rs", true), ["two.RS"]);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_directories_are_not_followed_and_non_utf8_paths_are_preserved() {
        use std::os::unix::{ffi::OsStringExt, fs::symlink};
        let temp = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::write(outside.path().join("outside.txt"), "secret").unwrap();
        symlink(outside.path(), temp.path().join("link")).unwrap();
        let raw = std::ffi::OsString::from_vec(vec![b'a', 0xff]);
        let supports_non_utf8 = fs::write(temp.path().join(&raw), "").is_ok();
        let mut index = WorkspaceIndex::new(temp.path()).unwrap();
        index.rebuild(&|| false).unwrap();
        assert_eq!(index.len(), 1 + usize::from(supports_non_utf8));
        index
            .reconcile(&["link/outside.txt".into()], &|| false)
            .unwrap();
        assert_eq!(index.len(), 1 + usize::from(supports_non_utf8));
        if supports_non_utf8 {
            assert!(index.entries.contains_key(Path::new(&raw)));
            assert!(index
                .save_cache(&outside.path().join("cache.json"))
                .is_err());
        }
        assert!(index.reconcile(&["../outside".into()], &|| false).is_err());
    }
}
