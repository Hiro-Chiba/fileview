//! Related file discovery for AI workflows.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Collect related paths for a target file.
///
/// Heuristics:
/// - target itself
/// - same directory + same/near stem
/// - common test filename patterns
pub fn collect_related_paths(target: &Path) -> Vec<PathBuf> {
    let mut out = BTreeSet::new();
    if !target.exists() || !target.is_file() {
        return Vec::new();
    }

    out.insert(target.to_path_buf());

    let stem = target
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    let ext = target.extension().and_then(|e| e.to_str()).unwrap_or("");

    if let Some(parent) = target.parent() {
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() || path == target {
                    continue;
                }
                let entry_stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                if entry_stem == stem
                    || entry_stem.starts_with(&stem)
                    || stem.starts_with(entry_stem)
                {
                    out.insert(path);
                }
            }
        }

        // Common test pairs
        out.insert(parent.join(format!("{}_test.{}", stem, ext)));
        out.insert(parent.join(format!("test_{}.{}", stem, ext)));
        out.insert(parent.join(format!("{}.test.{}", stem, ext)));
        out.insert(parent.join(format!("{}.spec.{}", stem, ext)));
        out.insert(parent.join("tests").join(format!("{}.{}", stem, ext)));
    }

    out.into_iter().filter(|p| p.exists()).collect()
}
