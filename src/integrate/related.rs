//! Related file discovery for AI workflows.

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A scored related file candidate with explainable reasons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedCandidate {
    pub path: PathBuf,
    pub score: i32,
    pub reasons: Vec<&'static str>,
}

/// Collect related paths for a target file.
///
/// This is a convenience wrapper around [`collect_related_candidates`].
pub fn collect_related_paths(target: &Path) -> Vec<PathBuf> {
    collect_related_candidates(target)
        .into_iter()
        .map(|c| c.path)
        .collect()
}

/// Collect scored related file candidates for a target file.
pub fn collect_related_candidates(target: &Path) -> Vec<RelatedCandidate> {
    if !target.exists() || !target.is_file() {
        return Vec::new();
    }

    let git_changed = changed_paths_in_repo(target);
    let mut candidates = Vec::new();

    let stem = target
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    let ext = target.extension().and_then(|e| e.to_str()).unwrap_or("");

    // Focused file always gets highest priority.
    candidates.push(RelatedCandidate {
        path: target.to_path_buf(),
        score: 100,
        reasons: vec!["focused_file"],
    });

    if let Some(parent) = target.parent() {
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() || path == target {
                    continue;
                }

                let mut score = 0;
                let mut reasons = Vec::new();
                let entry_stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();

                score += 30;
                reasons.push("same_directory");

                if entry_stem == stem {
                    score += 40;
                    reasons.push("same_stem");
                } else if entry_stem.starts_with(&stem) || stem.starts_with(&entry_stem) {
                    score += 20;
                    reasons.push("stem_prefix_match");
                }

                if is_test_like(&path) {
                    score += 20;
                    reasons.push("test_file");
                }

                if git_changed.contains(&path) {
                    score += 25;
                    reasons.push("git_changed");
                }

                if score > 0 {
                    candidates.push(RelatedCandidate {
                        path,
                        score,
                        reasons,
                    });
                }
            }
        }

        // Common test pairs.
        for candidate in test_pair_paths(parent, &stem, ext) {
            let mut score = 50;
            let mut reasons = vec!["test_pair"];
            if git_changed.contains(&candidate) {
                score += 25;
                reasons.push("git_changed");
            }
            if candidate.parent().is_some_and(|p| p.ends_with("tests")) {
                score += 10;
                reasons.push("tests_directory");
            }
            candidates.push(RelatedCandidate {
                path: candidate,
                score,
                reasons,
            });
        }
    }

    dedupe_and_sort(candidates)
}

fn dedupe_and_sort(candidates: Vec<RelatedCandidate>) -> Vec<RelatedCandidate> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for c in candidates {
        if c.path.exists() && seen.insert(c.path.clone()) {
            deduped.push(c);
        }
    }

    deduped.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()))
    });
    deduped
}

fn test_pair_paths(parent: &Path, stem: &str, ext: &str) -> Vec<PathBuf> {
    vec![
        parent.join(format!("{}_test.{}", stem, ext)),
        parent.join(format!("test_{}.{}", stem, ext)),
        parent.join(format!("{}.test.{}", stem, ext)),
        parent.join(format!("{}.spec.{}", stem, ext)),
        parent.join("tests").join(format!("{}.{}", stem, ext)),
    ]
}

fn is_test_like(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_lowercase();
    name.contains("test") || name.contains("spec")
}

fn changed_paths_in_repo(target: &Path) -> HashSet<PathBuf> {
    let mut out = HashSet::new();
    let Some(repo_root) = target.parent() else {
        return out;
    };
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root)
        .output();

    let Ok(output) = output else {
        return out;
    };
    if !output.status.success() {
        return out;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let rel = line.get(3..).unwrap_or("").trim();
        if rel.is_empty() {
            continue;
        }
        out.insert(repo_root.join(rel));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_related_candidates_nonexistent() {
        let c = collect_related_candidates(Path::new("/tmp/this/path/does/not/exist.rs"));
        assert!(c.is_empty());
    }
}
