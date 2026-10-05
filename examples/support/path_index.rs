//! Experimental path/type-only candidate. This is not the production index.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};

use anyhow::{bail, Result};
use fileview::workspace::{WorkspaceIndex, WorkspaceMatch};
use ignore::WalkBuilder;
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Matcher, Utf32Str,
};
use serde_json::{json, Value};

struct Entry {
    display: String,
    is_dir: bool,
    hidden: bool,
    extension: String,
}

enum Entries {
    Tree(BTreeMap<PathBuf, Entry>),
    Flat(Vec<(PathBuf, Entry)>),
}

impl Entries {
    fn len(&self) -> usize {
        match self {
            Self::Tree(entries) => entries.len(),
            Self::Flat(entries) => entries.len(),
        }
    }

    fn get(&self, path: &Path) -> Result<&Entry> {
        match self {
            Self::Tree(entries) => entries
                .get(path)
                .ok_or_else(|| anyhow::anyhow!("missing benchmark winner")),
            Self::Flat(entries) => entries
                .binary_search_by(|(key, _)| key.as_path().cmp(path))
                .map(|position| &entries[position].1)
                .map_err(|_| anyhow::anyhow!("missing benchmark winner")),
        }
    }
}

struct PathIndex {
    root: PathBuf,
    entries: Entries,
}

impl PathIndex {
    fn build(root: &Path, flat: bool) -> Result<Self> {
        let root = root.canonicalize()?;
        let mut builder = WalkBuilder::new(&root);
        builder
            .hidden(false)
            .git_global(false)
            .parents(false)
            .follow_links(false)
            .require_git(false)
            .filter_entry(|entry| entry.file_name() != ".git");
        let mut entries = if flat {
            Entries::Flat(Vec::new())
        } else {
            Entries::Tree(BTreeMap::new())
        };
        for result in builder.build() {
            let item = result?;
            if item.depth() == 0 {
                continue;
            }
            let relative = item.path().strip_prefix(&root)?;
            let entry = Entry {
                display: relative.to_string_lossy().into_owned(),
                is_dir: item.file_type().is_some_and(|kind| kind.is_dir()),
                hidden: relative
                    .components()
                    .any(|part| part.as_os_str().to_string_lossy().starts_with('.')),
                extension: relative
                    .extension()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            };
            match &mut entries {
                Entries::Tree(entries) => {
                    entries.insert(relative.to_path_buf(), entry);
                }
                Entries::Flat(entries) => entries.push((relative.to_path_buf(), entry)),
            }
            if entries.len() > 1_000_000 {
                bail!("candidate exceeds entry cap");
            }
        }
        if let Entries::Flat(entries) = &mut entries {
            entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
        }
        Ok(Self { root, entries })
    }

    fn search(&self, input: &str, limit: usize) -> Result<Vec<WorkspaceMatch>> {
        match &self.entries {
            Entries::Tree(entries) => self.search_entries(input, limit, entries.iter()),
            Entries::Flat(entries) => self.search_entries(
                input,
                limit,
                entries.iter().map(|(path, entry)| (path, entry)),
            ),
        }
    }

    fn search_entries<'a>(
        &self,
        input: &str,
        limit: usize,
        entries: impl Iterator<Item = (&'a PathBuf, &'a Entry)>,
    ) -> Result<Vec<WorkspaceMatch>> {
        let mut text = Vec::new();
        let mut extension = None;
        let mut directory = None;
        // These benchmark expressions are fixed and validated by the production parser.
        fileview::workspace::query::WorkspaceQuery::parse(input)?;
        for word in input.split_whitespace() {
            if let Some(value) = word.strip_prefix("ext:") {
                extension = Some(value);
            } else if let Some(value) = word.strip_prefix("type:") {
                directory = Some(value == "dir");
            } else if word.starts_with("git:") {
                bail!("Git filters are not part of this candidate benchmark");
            } else {
                text.push(word);
            }
        }
        let text = text.join(" ");
        let pattern = Pattern::parse(&text, CaseMatching::Smart, Normalization::Smart);
        let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
        let mut buffer = Vec::new();
        let mut winners = BinaryHeap::new();
        for (path, entry) in entries {
            if entry.hidden
                || directory.is_some_and(|is_dir| is_dir != entry.is_dir)
                || extension
                    .is_some_and(|ext| entry.is_dir || !entry.extension.eq_ignore_ascii_case(ext))
            {
                continue;
            }
            let score = if text.is_empty() {
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
            }
        }
        let mut matches = Vec::with_capacity(winners.len());
        for (Reverse(score), path) in winners.into_sorted_vec() {
            let entry = self.entries.get(path)?;
            let absolute = self.root.join(path);
            let metadata = absolute.symlink_metadata()?;
            let mut indices = Vec::new();
            if !text.is_empty() {
                pattern.indices(
                    Utf32Str::new(&entry.display, &mut buffer),
                    &mut matcher,
                    &mut indices,
                );
            }
            indices.sort_unstable();
            indices.dedup();
            matches.push(WorkspaceMatch {
                path: absolute,
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
        Ok(matches)
    }
}

fn distribution(mut samples: Vec<f64>) -> Value {
    samples.sort_by(f64::total_cmp);
    let percentile = |p: f64| samples[(samples.len() as f64 * p).ceil() as usize - 1];
    json!({"samples": samples.len(), "p50_ms": percentile(0.5), "p95_ms": percentile(0.95), "max_ms": samples.last()})
}

/// Compare path-only construction and winner metadata reads using the same fixture.
pub fn benchmark(root: &Path, iterations: usize, queries: &[&str]) -> Result<Value> {
    benchmark_kind(root, iterations, queries, false)
}

/// Compare a sorted contiguous path/type index with the BTreeMap candidate.
pub fn benchmark_vec(root: &Path, iterations: usize, queries: &[&str]) -> Result<Value> {
    benchmark_kind(root, iterations, queries, true)
}

fn benchmark_kind(root: &Path, iterations: usize, queries: &[&str], flat: bool) -> Result<Value> {
    let rss_before_build = super::current_rss_kib();
    let mut rss_after_first_build = None;
    let mut builds = Vec::new();
    let mut combined = Vec::new();
    for iteration in 0..iterations {
        let start = Instant::now();
        let index = PathIndex::build(root, flat)?;
        builds.push(start.elapsed().as_secs_f64() * 1000.0);
        black_box(index.search(queries[iteration % queries.len()], 15)?);
        combined.push(start.elapsed().as_secs_f64() * 1000.0);
        if iteration == 0 {
            rss_after_first_build = super::current_rss_kib();
        }
        super::check_memory_bound()?;
    }
    let index = PathIndex::build(root, flat)?;
    super::check_memory_bound()?;
    let rss_before_reference = super::current_rss_kib();
    let mut reference = WorkspaceIndex::new(root)?;
    reference.rebuild(&|| false)?;
    anyhow::ensure!(
        index.entries.len() == reference.len(),
        "candidate path count differs"
    );
    let mut query_results = serde_json::Map::new();
    let mut all = Vec::new();
    for query in queries
        .iter()
        .copied()
        .chain(["file ext:rs type:file", "type:dir", "ext:png"])
    {
        let actual = index.search(query, 15)?;
        let expected = reference.search(query, false, 15, &|| false)?;
        anyhow::ensure!(
            serde_json::to_value(&actual)? == serde_json::to_value(&expected)?,
            "candidate differs for {query}"
        );
        let mut samples = Vec::new();
        for _ in 0..iterations {
            let start = Instant::now();
            black_box(index.search(query, 15)?);
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            samples.push(ms);
            if queries.contains(&query) {
                all.push(ms);
            }
        }
        query_results.insert(query.to_owned(), distribution(samples));
    }
    Ok(json!({
        "experimental_candidate": "path/type index with metadata read only for top 15 winners",
        "storage": if flat { "sorted Vec" } else { "BTreeMap" },
        "initialize_without_metadata": distribution(builds),
        "initialize_and_search": distribution(combined),
        "search_including_winner_metadata": distribution(all),
        "queries": query_results,
        "results_equal_production_index": true,
        "process_rss_kib": {
            "before_build": rss_before_build,
            "after_first_build": rss_after_first_build,
            "before_reference_build": rss_before_reference,
            "scope": "whole process RSS; reference index excluded at these checkpoints"
        },
        "indexed_paths": index.entries.len(),
        "limitations": "No incremental update implementation. Search metadata reads are synchronous in benchmark worker. Git predicates excluded. Warm OS filesystem cache."
    }))
}
