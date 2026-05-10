//! Repository fingerprint shown at startup.
//!
//! A one-line summary like `Rust workspace · 47 crates · 1.2M LOC ·
//! last touched 14m ago · 7 dirty files` so the user gets a quick read
//! of the repo they just opened.
//!
//! Detection is intentionally cheap and bounded:
//!
//! - Language is inferred from a small list of marker files at the root.
//! - Line-of-code count is a recursive walk that stops as soon as a budget
//!   timeout is hit, in which case the count is reported as approximate.
//! - "Last touched" is the newest mtime under the root, excluding common
//!   build directories.
//! - Dirty count comes from `git status --porcelain` if the path is inside
//!   a git work tree.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};

/// Languages we can detect from a single marker file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Node,
    Python,
    Go,
    Ruby,
    Java,
    Php,
}

impl Language {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Node => "Node",
            Self::Python => "Python",
            Self::Go => "Go",
            Self::Ruby => "Ruby",
            Self::Java => "Java",
            Self::Php => "PHP",
        }
    }

    /// Source file extensions counted toward LOC for this language.
    fn loc_extensions(&self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["rs"],
            Self::Node => &["ts", "tsx", "js", "jsx", "mjs", "cjs"],
            Self::Python => &["py"],
            Self::Go => &["go"],
            Self::Ruby => &["rb"],
            Self::Java => &["java", "kt"],
            Self::Php => &["php"],
        }
    }
}

/// One repository fingerprint.
#[derive(Debug, Clone)]
pub struct Fingerprint {
    pub primary_language: Option<Language>,
    pub is_workspace: bool,
    pub workspace_member_count: Option<usize>,
    pub line_count: Option<usize>,
    pub line_count_partial: bool,
    pub last_touched_secs_ago: Option<u64>,
    pub dirty_file_count: Option<usize>,
}

impl Fingerprint {
    /// Render as the one-liner displayed in the status bar.
    pub fn describe(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        match (self.primary_language, self.is_workspace) {
            (Some(lang), true) => parts.push(format!("{} workspace", lang.name())),
            (Some(lang), false) => parts.push(lang.name().to_string()),
            (None, _) => {}
        }

        if let Some(n) = self.workspace_member_count {
            if n > 0 {
                parts.push(format!("{} crates", n));
            }
        }

        if let Some(loc) = self.line_count {
            let prefix = if self.line_count_partial { "~" } else { "" };
            parts.push(format!("{}{} LOC", prefix, humanize_count(loc)));
        }

        if let Some(secs) = self.last_touched_secs_ago {
            parts.push(format!("last touched {}", humanize_duration(secs)));
        }

        if let Some(dirty) = self.dirty_file_count {
            if dirty > 0 {
                parts.push(format!("{} dirty", dirty));
            }
        }

        if parts.is_empty() {
            "(empty repo fingerprint)".to_string()
        } else {
            parts.join(" · ")
        }
    }
}

/// Detect a fingerprint for `root`. The whole call is bounded by `time_budget`;
/// the LOC walk stops as soon as the budget is exceeded.
pub fn detect(root: &Path, time_budget: Duration) -> Fingerprint {
    let started = Instant::now();
    let primary_language = detect_language(root);
    let (is_workspace, workspace_member_count) = detect_workspace(root, primary_language);
    let last_touched_secs_ago = newest_mtime_seconds(root);
    let dirty_file_count = git_dirty_count(root);

    let remaining = time_budget.saturating_sub(started.elapsed());
    let (line_count, line_count_partial) = match primary_language {
        Some(lang) if remaining > Duration::ZERO => {
            count_lines_bounded(root, lang.loc_extensions(), remaining)
        }
        _ => (None, false),
    };

    Fingerprint {
        primary_language,
        is_workspace,
        workspace_member_count,
        line_count,
        line_count_partial,
        last_touched_secs_ago,
        dirty_file_count,
    }
}

fn detect_language(root: &Path) -> Option<Language> {
    let candidates: &[(&str, Language)] = &[
        ("Cargo.toml", Language::Rust),
        ("package.json", Language::Node),
        ("pyproject.toml", Language::Python),
        ("setup.py", Language::Python),
        ("requirements.txt", Language::Python),
        ("go.mod", Language::Go),
        ("Gemfile", Language::Ruby),
        ("pom.xml", Language::Java),
        ("build.gradle", Language::Java),
        ("build.gradle.kts", Language::Java),
        ("composer.json", Language::Php),
    ];
    for (filename, lang) in candidates {
        if root.join(filename).is_file() {
            return Some(*lang);
        }
    }
    None
}

fn detect_workspace(root: &Path, language: Option<Language>) -> (bool, Option<usize>) {
    match language {
        Some(Language::Rust) => detect_cargo_workspace(root),
        Some(Language::Node) => detect_node_workspace(root),
        _ => (false, None),
    }
}

fn detect_cargo_workspace(root: &Path) -> (bool, Option<usize>) {
    let cargo_toml = root.join("Cargo.toml");
    let Ok(text) = std::fs::read_to_string(&cargo_toml) else {
        return (false, None);
    };
    if !text.contains("[workspace]") {
        return (false, None);
    }
    let count = text
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with('"') && t.ends_with("\",") || t.starts_with('"') && t.ends_with('"')
        })
        .count();
    (true, if count > 0 { Some(count) } else { None })
}

fn detect_node_workspace(root: &Path) -> (bool, Option<usize>) {
    let pkg = root.join("package.json");
    let Ok(text) = std::fs::read_to_string(&pkg) else {
        return (false, None);
    };
    let has_workspaces = text.contains("\"workspaces\"");
    (has_workspaces, None)
}

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".nuxt",
    ".turbo",
    ".svelte-kit",
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".git",
    ".idea",
    ".vscode",
    "vendor",
];

fn should_skip_dir_name(name: &str) -> bool {
    SKIP_DIRS.contains(&name)
}

fn count_lines_bounded(
    root: &Path,
    extensions: &[&str],
    budget: Duration,
) -> (Option<usize>, bool) {
    let started = Instant::now();
    let mut total: usize = 0;
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    let mut partial = false;

    while let Some(dir) = stack.pop() {
        if started.elapsed() >= budget {
            partial = true;
            break;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            if started.elapsed() >= budget {
                partial = true;
                break;
            }
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if ft.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !should_skip_dir_name(name) {
                        stack.push(path);
                    }
                }
                continue;
            }
            if !ft.is_file() {
                continue;
            }
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if !extensions.contains(&ext) {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path) {
                total += text.lines().count();
            }
        }
    }

    (Some(total), partial)
}

fn newest_mtime_seconds(root: &Path) -> Option<u64> {
    let mut newest: Option<SystemTime> = None;
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    let started = Instant::now();
    let budget = Duration::from_millis(300);

    while let Some(dir) = stack.pop() {
        if started.elapsed() >= budget {
            break;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if ft.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !should_skip_dir_name(name) {
                        stack.push(path);
                    }
                }
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                if let Ok(m) = meta.modified() {
                    newest = Some(match newest {
                        Some(prev) if prev > m => prev,
                        _ => m,
                    });
                }
            }
        }
    }

    let now = SystemTime::now();
    newest.and_then(|t| now.duration_since(t).ok().map(|d| d.as_secs()))
}

fn git_dirty_count(root: &Path) -> Option<usize> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let count = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    Some(count)
}

fn humanize_count(n: usize) -> String {
    if n < 1_000 {
        return n.to_string();
    }
    if n < 10_000 {
        format!("{:.1}k", n as f32 / 1_000.0)
    } else if n < 1_000_000 {
        format!("{}k", n / 1_000)
    } else if n < 10_000_000 {
        format!("{:.1}M", n as f32 / 1_000_000.0)
    } else {
        format!("{}M", n / 1_000_000)
    }
}

fn humanize_duration(secs: u64) -> String {
    if secs < 60 {
        return format!("{}s ago", secs);
    }
    let minutes = secs / 60;
    if minutes < 60 {
        return format!("{}m ago", minutes);
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{}h ago", hours);
    }
    let days = hours / 24;
    if days < 30 {
        return format!("{}d ago", days);
    }
    let months = days / 30;
    if months < 12 {
        return format!("{}mo ago", months);
    }
    let years = days / 365;
    format!("{}y ago", years)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn humanize_count_brackets() {
        assert_eq!(humanize_count(0), "0");
        assert_eq!(humanize_count(999), "999");
        assert_eq!(humanize_count(1234), "1.2k");
        assert_eq!(humanize_count(42_000), "42k");
        assert_eq!(humanize_count(1_200_000), "1.2M");
        assert_eq!(humanize_count(15_000_000), "15M");
    }

    #[test]
    fn humanize_duration_brackets() {
        assert_eq!(humanize_duration(5), "5s ago");
        assert_eq!(humanize_duration(125), "2m ago");
        assert_eq!(humanize_duration(7_200), "2h ago");
        assert_eq!(humanize_duration(86_400 * 3), "3d ago");
        assert_eq!(humanize_duration(86_400 * 60), "2mo ago");
        assert_eq!(humanize_duration(86_400 * 800), "2y ago");
    }

    #[test]
    fn detects_rust_project() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "fn a() {}\nfn b() {}\n").unwrap();

        let fp = detect(dir.path(), Duration::from_secs(2));
        assert_eq!(fp.primary_language, Some(Language::Rust));
        assert!(!fp.is_workspace);
        let line = fp.describe();
        assert!(line.starts_with("Rust"), "describe was: {}", line);
        assert!(line.contains("LOC"));
    }

    #[test]
    fn detects_rust_workspace() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/a\", \"crates/b\"]\n",
        )
        .unwrap();
        let fp = detect(dir.path(), Duration::from_secs(2));
        assert_eq!(fp.primary_language, Some(Language::Rust));
        assert!(fp.is_workspace);
        assert!(fp.describe().contains("Rust workspace"));
    }

    #[test]
    fn detects_node_project_without_workspace() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            "{\"name\":\"x\",\"version\":\"0.0.1\"}",
        )
        .unwrap();
        let fp = detect(dir.path(), Duration::from_secs(2));
        assert_eq!(fp.primary_language, Some(Language::Node));
        assert!(!fp.is_workspace);
    }

    #[test]
    fn unknown_root_returns_no_language() {
        let dir = tempdir().unwrap();
        let fp = detect(dir.path(), Duration::from_secs(1));
        assert_eq!(fp.primary_language, None);
        assert!(fp.line_count.is_none());
    }

    #[test]
    fn skip_dirs_are_excluded_from_loc() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "fn a() {}\n").unwrap();
        fs::create_dir_all(dir.path().join("target/debug")).unwrap();
        fs::write(
            dir.path().join("target/debug/should_not_count.rs"),
            (0..5000).map(|_| "x\n").collect::<String>(),
        )
        .unwrap();

        let fp = detect(dir.path(), Duration::from_secs(2));
        let loc = fp.line_count.unwrap();
        assert!(loc < 100, "target/ should be skipped, got {}", loc);
    }
}
