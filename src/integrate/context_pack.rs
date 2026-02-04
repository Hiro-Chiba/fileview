//! Context pack generation for AI workflows.

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use super::context::build_project_context;

/// Built-in context pack presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextPackPreset {
    Minimal,
    Review,
    Debug,
}

impl ContextPackPreset {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Review => "review",
            Self::Debug => "debug",
        }
    }
}

impl FromStr for ContextPackPreset {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "minimal" => Ok(Self::Minimal),
            "review" => Ok(Self::Review),
            "debug" => Ok(Self::Debug),
            _ => Err(()),
        }
    }
}

/// Build a context pack as markdown text.
pub fn build_context_pack(
    root: &Path,
    preset: ContextPackPreset,
    selected_paths: &[PathBuf],
) -> io::Result<String> {
    let mut out = Vec::new();
    writeln!(&mut out, "## Context Pack: {}", preset.as_str())?;
    writeln!(&mut out)?;
    writeln!(&mut out, "{}", build_project_context(root)?)?;

    if !selected_paths.is_empty() {
        writeln!(&mut out, "### Selected Files")?;
        for path in selected_paths {
            writeln!(&mut out, "- {}", path.display())?;
        }
        writeln!(&mut out)?;
    }

    match preset {
        ContextPackPreset::Minimal => {
            append_git_changed_files(root, &mut out)?;
        }
        ContextPackPreset::Review => {
            append_git_changed_files(root, &mut out)?;
            append_git_diff_stat(root, &mut out)?;
        }
        ContextPackPreset::Debug => {
            append_git_changed_files(root, &mut out)?;
            append_error_log_candidates(root, &mut out)?;
        }
    }

    Ok(String::from_utf8_lossy(&out).to_string())
}

/// Output a context pack to stdout.
pub fn output_context_pack(root: &Path, preset: ContextPackPreset) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let text = build_context_pack(root, preset, &[])?;
    write!(handle, "{}", text)?;
    handle.flush()
}

fn append_git_changed_files(root: &Path, out: &mut Vec<u8>) -> io::Result<()> {
    let changed = git_stdout(root, &["status", "--porcelain"])?;
    if changed.trim().is_empty() {
        return Ok(());
    }

    writeln!(out, "### Git Changed Files")?;
    for line in changed.lines().take(30) {
        let path = line.get(3..).unwrap_or(line).trim();
        if !path.is_empty() {
            writeln!(out, "- {}", path)?;
        }
    }
    writeln!(out)?;
    Ok(())
}

fn append_git_diff_stat(root: &Path, out: &mut Vec<u8>) -> io::Result<()> {
    let diff = git_stdout(root, &["diff", "--stat"])?;
    if diff.trim().is_empty() {
        return Ok(());
    }

    writeln!(out, "### Diff Summary")?;
    for line in diff.lines().take(40) {
        writeln!(out, "{}", line)?;
    }
    writeln!(out)?;
    Ok(())
}

fn append_error_log_candidates(root: &Path, out: &mut Vec<u8>) -> io::Result<()> {
    let mut candidates = BTreeSet::new();
    collect_error_candidates(root, 0, 2, &mut candidates);
    if candidates.is_empty() {
        return Ok(());
    }

    writeln!(out, "### Error Log Candidates")?;
    for path in candidates.iter().take(20) {
        writeln!(out, "- {}", path.display())?;
    }
    writeln!(out)?;
    Ok(())
}

fn collect_error_candidates(
    path: &Path,
    depth: usize,
    max_depth: usize,
    out: &mut BTreeSet<PathBuf>,
) {
    if depth > max_depth {
        return;
    }

    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if p.is_dir() {
            if !name.starts_with('.') && name != "target" && name != "node_modules" {
                collect_error_candidates(&p, depth + 1, max_depth, out);
            }
            continue;
        }

        if name.contains("error")
            || name.contains("stderr")
            || name.contains("panic")
            || name.ends_with(".log")
        {
            out.insert(p);
        }
    }
}

fn git_stdout(root: &Path, args: &[&str]) -> io::Result<String> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() {
        return Ok(String::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
