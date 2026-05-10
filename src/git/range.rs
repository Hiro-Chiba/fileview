//! Diff range computation for the diff-aware tree.
//!
//! Wraps `git diff --numstat -M <revspec>` and `git status --porcelain` to
//! produce a per-path map of added and deleted line counts. The resulting
//! map drives the diff scope filter in `tree::Scope::DiffRange` and the
//! `+N/-M` annotation rendered next to each file in the tree.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Per-file added and deleted line counts.
///
/// Lookups use the absolute path so the same map can be used regardless of
/// where the tree was rooted relative to the git work tree.
#[derive(Debug, Clone, Default)]
pub struct DiffRange {
    pub revspec: Option<String>,
    by_path: HashMap<PathBuf, (i32, i32)>,
}

impl DiffRange {
    pub fn new(revspec: Option<String>, by_path: HashMap<PathBuf, (i32, i32)>) -> Self {
        Self { revspec, by_path }
    }

    /// Look up the (added, deleted) line counts for `path`.
    pub fn get(&self, path: &Path) -> Option<(i32, i32)> {
        self.by_path.get(path).copied()
    }

    /// Number of files included in the range.
    pub fn file_count(&self) -> usize {
        self.by_path.len()
    }

    /// Whether this path or any descendant is in the diff range. The lookup is
    /// O(N) over the diff entries, which is fine for the typical PR-sized
    /// range (tens to hundreds of files).
    pub fn touches_any_under(&self, dir: &Path) -> bool {
        self.by_path.keys().any(|p| p.starts_with(dir))
    }

    /// Aggregate added and deleted counts under `dir`.
    pub fn totals_under(&self, dir: &Path) -> (i32, i32) {
        let mut add = 0i32;
        let mut del = 0i32;
        for (p, (a, d)) in &self.by_path {
            if p.starts_with(dir) {
                add += a;
                del += d;
            }
        }
        (add, del)
    }

    /// All paths in the range. Mainly useful for tests and debugging.
    pub fn paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.by_path.keys()
    }
}

/// Compute a diff range against `revspec` (e.g. `origin/main..HEAD`).
///
/// Falls back to `git status --porcelain` when `revspec` is `None`, which
/// gives the user the working-tree changes.
pub fn compute(root: &Path, revspec: Option<&str>) -> anyhow::Result<DiffRange> {
    if let Some(rev) = revspec {
        compute_from_numstat(root, rev)
    } else {
        compute_from_status(root)
    }
}

fn compute_from_numstat(root: &Path, revspec: &str) -> anyhow::Result<DiffRange> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["diff", "--numstat", "-M", revspec])
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run git diff: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git diff --numstat {} failed: {}", revspec, stderr.trim());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let by_path = parse_numstat(root, &stdout);
    Ok(DiffRange::new(Some(revspec.to_string()), by_path))
}

fn compute_from_status(root: &Path) -> anyhow::Result<DiffRange> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["status", "--porcelain"])
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run git status: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git status --porcelain failed: {}", stderr.trim());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut by_path = HashMap::new();

    for line in stdout.lines() {
        // Each line is `XY <path>` where XY is two status characters.
        if line.len() < 4 {
            continue;
        }
        let status = &line[..2];
        let rest = line[3..].trim();
        // Skip submodule pseudo-entries: they end with a slash and have no
        // line counts to report.
        if rest.ends_with('/') {
            continue;
        }
        // Detect rename "old -> new" for renamed entries.
        let rel = if let Some(idx) = rest.find(" -> ") {
            &rest[idx + 4..]
        } else {
            rest
        };
        let abs = root.join(rel);

        // Without a numstat we don't know the line counts. Use 0 / 0 so the
        // file still shows up in the filter and the renderer can surface a
        // marker even when counts aren't available.
        let _ = status;
        by_path.entry(abs).or_insert((0, 0));
    }

    let mut numstat_by_path = HashMap::new();
    if let Ok(output) = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["diff", "--numstat", "-M"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            numstat_by_path.extend(parse_numstat(root, &stdout));
        }
    }
    if let Ok(output) = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["diff", "--cached", "--numstat", "-M"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for (path, (add, del)) in parse_numstat(root, &stdout) {
                let entry = numstat_by_path.entry(path).or_insert((0, 0));
                entry.0 += add;
                entry.1 += del;
            }
        }
    }
    for (path, counts) in numstat_by_path {
        by_path.insert(path, counts);
    }

    Ok(DiffRange::new(None, by_path))
}

fn parse_numstat(root: &Path, stdout: &str) -> HashMap<PathBuf, (i32, i32)> {
    let mut by_path = HashMap::new();
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let added = parse_count(parts[0]);
        let deleted = parse_count(parts[1]);
        let raw_path = parts[2];
        // Renamed entries look like `old/path => new/path` or
        // `prefix/{old => new}/suffix`. Resolve to the new path.
        let resolved = resolve_renamed_path(raw_path);
        by_path.insert(root.join(resolved), (added, deleted));
    }
    by_path
}

fn parse_count(s: &str) -> i32 {
    if s == "-" {
        // Binary file. We surface 0 / 0 rather than skipping so users still
        // see the filename in the tree.
        0
    } else {
        s.parse::<i32>().unwrap_or(0)
    }
}

/// Resolve a numstat path that may be in either rename form:
///   `old => new`
///   `prefix/{old => new}/suffix`
/// Returns the new path. If neither form matches, returns `s` unchanged.
fn resolve_renamed_path(s: &str) -> String {
    if let Some(open) = s.find('{') {
        if let Some(close) = s[open..].find('}') {
            let close = open + close;
            let inside = &s[open + 1..close];
            if let Some(arrow) = inside.find(" => ") {
                let new_part = &inside[arrow + 4..];
                let mut out = String::new();
                out.push_str(&s[..open]);
                out.push_str(new_part);
                out.push_str(&s[close + 1..]);
                return out.replace("//", "/");
            }
        }
    }
    if let Some(arrow) = s.find(" => ") {
        return s[arrow + 4..].to_string();
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_numstat_basic_entries() {
        let stdout = "10\t2\tsrc/foo.rs\n5\t0\tREADME.md\n";
        let map = parse_numstat(Path::new("/repo"), stdout);
        assert_eq!(map.len(), 2);
        assert_eq!(map[&PathBuf::from("/repo/src/foo.rs")], (10, 2));
        assert_eq!(map[&PathBuf::from("/repo/README.md")], (5, 0));
    }

    #[test]
    fn parse_numstat_treats_dash_as_zero() {
        // Binary file: numstat returns "-\t-\t<path>".
        let stdout = "-\t-\timg/logo.png\n";
        let map = parse_numstat(Path::new("/repo"), stdout);
        assert_eq!(map[&PathBuf::from("/repo/img/logo.png")], (0, 0));
    }

    #[test]
    fn parse_numstat_handles_simple_rename_arrow() {
        let stdout = "3\t4\told/path.rs => new/path.rs\n";
        let map = parse_numstat(Path::new("/repo"), stdout);
        assert!(map.contains_key(&PathBuf::from("/repo/new/path.rs")));
        assert!(!map.contains_key(&PathBuf::from("/repo/old/path.rs")));
    }

    #[test]
    fn parse_numstat_handles_brace_rename() {
        let stdout = "1\t1\tsrc/{old => new}/main.rs\n";
        let map = parse_numstat(Path::new("/repo"), stdout);
        assert!(map.contains_key(&PathBuf::from("/repo/src/new/main.rs")));
    }

    #[test]
    fn diff_range_totals_under_aggregates() {
        let mut by_path = HashMap::new();
        by_path.insert(PathBuf::from("/repo/src/a.rs"), (10, 2));
        by_path.insert(PathBuf::from("/repo/src/b.rs"), (3, 1));
        by_path.insert(PathBuf::from("/repo/docs/x.md"), (4, 0));
        let range = DiffRange::new(Some("HEAD~1..HEAD".into()), by_path);

        assert_eq!(range.totals_under(Path::new("/repo/src")), (13, 3));
        assert_eq!(range.totals_under(Path::new("/repo/docs")), (4, 0));
        assert_eq!(range.totals_under(Path::new("/repo")), (17, 3));
        assert_eq!(range.totals_under(Path::new("/repo/missing")), (0, 0));
    }

    #[test]
    fn diff_range_touches_any_under() {
        let mut by_path = HashMap::new();
        by_path.insert(PathBuf::from("/repo/src/a.rs"), (1, 0));
        let range = DiffRange::new(None, by_path);

        assert!(range.touches_any_under(Path::new("/repo/src")));
        assert!(range.touches_any_under(Path::new("/repo")));
        assert!(!range.touches_any_under(Path::new("/repo/docs")));
    }
}
