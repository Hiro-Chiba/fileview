//! Repository-wide TODO/FIXME aggregator.
//!
//! Scans the repository for `TODO`, `FIXME`, `XXX`, `HACK`, `BUG`, and
//! `NOTE` markers and returns a flat list keyed by path and line number.
//!
//! The scan is intentionally simple: a recursive `read_dir` walk bounded
//! by a time budget, with a fixed skip list for common build and cache
//! directories. No new crate dependency.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use regex::Regex;

/// One marker found in source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoItem {
    pub path: PathBuf,
    pub line: usize,
    pub tag: TodoTag,
    pub message: String,
}

/// Recognized marker tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TodoTag {
    Todo,
    Fixme,
    Xxx,
    Hack,
    Bug,
    Note,
}

impl TodoTag {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Todo => "TODO",
            Self::Fixme => "FIXME",
            Self::Xxx => "XXX",
            Self::Hack => "HACK",
            Self::Bug => "BUG",
            Self::Note => "NOTE",
        }
    }

    fn from_match(s: &str) -> Option<Self> {
        match s {
            "TODO" => Some(Self::Todo),
            "FIXME" => Some(Self::Fixme),
            "XXX" => Some(Self::Xxx),
            "HACK" => Some(Self::Hack),
            "BUG" => Some(Self::Bug),
            "NOTE" => Some(Self::Note),
            _ => None,
        }
    }
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
    ".cargo",
    "Pods",
];

/// Source extensions we scan. Kept aligned with what fileview previews so the
/// list matches user expectations of "code files".
const SOURCE_EXTS: &[&str] = &[
    "rs", "toml", "md", "txt", "ts", "tsx", "js", "jsx", "mjs", "cjs", "json", "py", "pyi", "go",
    "rb", "java", "kt", "kts", "c", "cc", "cpp", "h", "hpp", "cs", "php", "swift", "scala", "sh",
    "bash", "zsh", "fish", "yml", "yaml", "html", "css", "scss", "sass", "lua", "vue", "svelte",
];

fn is_source_file(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => SOURCE_EXTS.contains(&ext),
        None => false,
    }
}

fn should_skip_dir(name: &str) -> bool {
    SKIP_DIRS.contains(&name)
}

/// Outcome of a scan.
#[derive(Debug, Clone)]
pub struct ScanOutcome {
    pub items: Vec<TodoItem>,
    /// True when the time budget was hit before the walk completed.
    pub partial: bool,
}

/// Scan `root` for TODO-style markers, bounded by `time_budget`.
pub fn scan_repo(root: &Path, time_budget: Duration) -> ScanOutcome {
    let started = Instant::now();
    let re = compile_regex();
    let mut items: Vec<TodoItem> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    let mut partial = false;

    while let Some(dir) = stack.pop() {
        if started.elapsed() >= time_budget {
            partial = true;
            break;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            if started.elapsed() >= time_budget {
                partial = true;
                break;
            }
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if ft.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !should_skip_dir(name) {
                        stack.push(path);
                    }
                }
                continue;
            }
            if !ft.is_file() {
                continue;
            }
            if !is_source_file(&path) {
                continue;
            }
            scan_file(&re, &path, &mut items);
        }
    }

    ScanOutcome { items, partial }
}

fn compile_regex() -> Regex {
    Regex::new(r"\b(?P<tag>TODO|FIXME|XXX|HACK|BUG|NOTE)\b:?\s*(?P<msg>.*)")
        .expect("static regex compiles")
}

fn scan_file(re: &Regex, path: &Path, out: &mut Vec<TodoItem>) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    for (i, line) in text.lines().enumerate() {
        // Cheap fast path: if none of the tag substrings appear, skip the
        // regex altogether. This avoids the regex cost on typical lines.
        if !line.contains("TODO")
            && !line.contains("FIXME")
            && !line.contains("XXX")
            && !line.contains("HACK")
            && !line.contains("BUG")
            && !line.contains("NOTE")
        {
            continue;
        }
        if let Some(caps) = re.captures(line) {
            let tag_str = caps.name("tag").map(|m| m.as_str()).unwrap_or("");
            let msg = caps
                .name("msg")
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            if let Some(tag) = TodoTag::from_match(tag_str) {
                out.push(TodoItem {
                    path: path.to_path_buf(),
                    line: i + 1,
                    tag,
                    message: msg,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn extracts_basic_markers() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("a.rs");
        fs::write(
            &p,
            "// TODO: hook up logging\nfn main() {}\n// FIXME: handle error\n",
        )
        .unwrap();

        let outcome = scan_repo(dir.path(), Duration::from_secs(2));
        assert!(!outcome.partial);
        assert_eq!(outcome.items.len(), 2);
        assert_eq!(outcome.items[0].tag, TodoTag::Todo);
        assert_eq!(outcome.items[0].line, 1);
        assert!(outcome.items[0].message.contains("hook up"));
        assert_eq!(outcome.items[1].tag, TodoTag::Fixme);
        assert_eq!(outcome.items[1].line, 3);
    }

    #[test]
    fn ignores_non_source_extensions() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("notes.bin"), "TODO: this should be ignored").unwrap();
        let outcome = scan_repo(dir.path(), Duration::from_secs(2));
        assert!(outcome.items.is_empty());
    }

    #[test]
    fn skips_build_directories() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("target/release")).unwrap();
        fs::write(
            dir.path().join("target/release/buggy.rs"),
            "// TODO: must not surface\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "// TODO: surface this one\n").unwrap();

        let outcome = scan_repo(dir.path(), Duration::from_secs(2));
        assert_eq!(outcome.items.len(), 1);
        assert!(outcome.items[0].path.ends_with("src/lib.rs"));
    }

    #[test]
    fn does_not_match_substrings_inside_words() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("a.rs"),
            "fn unTODOable() {}\nfn _TODO_inside() {}\n",
        )
        .unwrap();
        let outcome = scan_repo(dir.path(), Duration::from_secs(2));
        // The word boundary in the regex prevents "unTODOable" from matching,
        // but "_TODO_" has only word characters around it which the engine
        // treats as inside a word, so it also should not match.
        assert!(outcome.items.is_empty(), "got {:?}", outcome.items);
    }

    #[test]
    fn captures_message_after_optional_colon() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("a.rs"),
            "// HACK no colon required\n// BUG: with colon\n",
        )
        .unwrap();
        let outcome = scan_repo(dir.path(), Duration::from_secs(2));
        assert_eq!(outcome.items.len(), 2);
        assert_eq!(outcome.items[0].tag, TodoTag::Hack);
        assert_eq!(outcome.items[0].message, "no colon required");
        assert_eq!(outcome.items[1].tag, TodoTag::Bug);
        assert_eq!(outcome.items[1].message, "with colon");
    }
}
