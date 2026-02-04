//! Context pack generation for AI workflows.

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use crate::mcp::token::estimate_tokens;

use super::context::build_project_context;

/// Built-in context pack presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextPackPreset {
    Minimal,
    Review,
    Debug,
    Refactor,
    Incident,
    Onboarding,
}

impl ContextPackPreset {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Review => "review",
            Self::Debug => "debug",
            Self::Refactor => "refactor",
            Self::Incident => "incident",
            Self::Onboarding => "onboarding",
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
            "refactor" => Ok(Self::Refactor),
            "incident" => Ok(Self::Incident),
            "onboarding" => Ok(Self::Onboarding),
            _ => Err(()),
        }
    }
}

/// Target agent profile for context shaping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContextAgent {
    #[default]
    Claude,
    Codex,
    Cursor,
}

impl ContextAgent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
        }
    }
}

impl FromStr for ContextAgent {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            "cursor" => Ok(Self::Cursor),
            _ => Err(()),
        }
    }
}

/// Output format for context-pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContextPackFormat {
    #[default]
    AiMarkdown,
    Jsonl,
}

impl ContextPackFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AiMarkdown => "ai-md",
            Self::Jsonl => "jsonl",
        }
    }
}

impl FromStr for ContextPackFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ai-md" | "aimd" | "markdown" => Ok(Self::AiMarkdown),
            "jsonl" => Ok(Self::Jsonl),
            _ => Err(()),
        }
    }
}

/// Context pack generation options.
#[derive(Debug, Clone)]
pub struct ContextPackOptions {
    pub token_budget: usize,
    pub include_git_diff: bool,
    pub include_tests: bool,
    pub depth: usize,
    pub format: ContextPackFormat,
    pub agent: ContextAgent,
}

impl Default for ContextPackOptions {
    fn default() -> Self {
        Self {
            token_budget: 4000,
            include_git_diff: false,
            include_tests: false,
            depth: 2,
            format: ContextPackFormat::AiMarkdown,
            agent: ContextAgent::default(),
        }
    }
}

impl ContextPackOptions {
    /// Fill preset-driven defaults for unset options.
    pub fn with_preset_defaults(mut self, preset: ContextPackPreset) -> Self {
        match preset {
            ContextPackPreset::Minimal => {}
            ContextPackPreset::Review => {
                self.include_git_diff = true;
                self.include_tests = true;
            }
            ContextPackPreset::Debug => {
                self.include_git_diff = true;
            }
            ContextPackPreset::Refactor => {
                self.include_git_diff = true;
                self.include_tests = true;
                self.token_budget = self.token_budget.max(6000);
            }
            ContextPackPreset::Incident => {
                self.include_git_diff = true;
                self.token_budget = self.token_budget.max(6000);
            }
            ContextPackPreset::Onboarding => {
                self.token_budget = self.token_budget.max(5000);
            }
        }
        self
    }

    /// Apply profile-specific defaults while preserving explicit options.
    pub fn with_agent_defaults(mut self) -> Self {
        match self.agent {
            ContextAgent::Claude => {}
            ContextAgent::Codex => {
                self.token_budget = self.token_budget.max(5000);
            }
            ContextAgent::Cursor => {
                self.token_budget = self.token_budget.max(4500);
                self.include_tests = true;
            }
        }
        self
    }
}

/// Build a context pack as text with default options.
pub fn build_context_pack(
    root: &Path,
    preset: ContextPackPreset,
    selected_paths: &[PathBuf],
) -> io::Result<String> {
    build_context_pack_with_options(root, preset, selected_paths, &ContextPackOptions::default())
}

/// Build a context pack with explicit options.
pub fn build_context_pack_with_options(
    root: &Path,
    preset: ContextPackPreset,
    selected_paths: &[PathBuf],
    options: &ContextPackOptions,
) -> io::Result<String> {
    let options = options
        .clone()
        .with_preset_defaults(preset)
        .with_agent_defaults();
    let candidates =
        collect_candidate_files(root, selected_paths, options.depth, options.include_tests);
    let snippets = collect_snippets(&candidates, options.token_budget);

    match options.format {
        ContextPackFormat::AiMarkdown => {
            let mut out = Vec::new();
            writeln!(&mut out, "## Context Pack: {}", preset.as_str())?;
            writeln!(&mut out, "- agent: {}", options.agent.as_str())?;
            writeln!(&mut out, "- token_budget: {}", options.token_budget)?;
            writeln!(&mut out, "- format: {}", options.format.as_str())?;
            writeln!(&mut out)?;
            writeln!(&mut out, "{}", build_project_context(root)?)?;

            if options.include_git_diff {
                append_git_diff_stat(root, &mut out)?;
            }
            if matches!(
                preset,
                ContextPackPreset::Debug | ContextPackPreset::Incident
            ) {
                append_error_log_candidates(root, &mut out)?;
            }

            if !snippets.is_empty() {
                writeln!(&mut out, "### Files")?;
                for (path, content, tokens) in snippets {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    match options.agent {
                        ContextAgent::Claude => {
                            writeln!(&mut out, "### File: {}", path.display())?;
                        }
                        ContextAgent::Codex => {
                            writeln!(&mut out, "#### {}", path.display())?;
                        }
                        ContextAgent::Cursor => {
                            writeln!(&mut out, "#### Context: {}", path.display())?;
                        }
                    }
                    writeln!(&mut out, "- est_tokens: {}", tokens)?;
                    writeln!(&mut out, "```{}", ext)?;
                    writeln!(&mut out, "{}", content)?;
                    writeln!(&mut out, "```")?;
                    writeln!(&mut out)?;
                }
            }

            Ok(String::from_utf8_lossy(&out).to_string())
        }
        ContextPackFormat::Jsonl => {
            let mut lines = Vec::new();
            lines.push(serde_json::json!({
                "type": "meta",
                "agent": options.agent.as_str(),
                "preset": preset.as_str(),
                "token_budget": options.token_budget,
                "format": options.format.as_str()
            }));
            for (path, content, tokens) in snippets {
                lines.push(serde_json::json!({
                    "type": "file",
                    "path": path.display().to_string(),
                    "tokens": tokens,
                    "content": content
                }));
            }
            let text = lines
                .into_iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            Ok(text)
        }
    }
}

/// Output a context pack to stdout with default options.
pub fn output_context_pack(root: &Path, preset: ContextPackPreset) -> io::Result<()> {
    output_context_pack_with_options(root, preset, &ContextPackOptions::default())
}

/// Output a context pack to stdout with explicit options.
pub fn output_context_pack_with_options(
    root: &Path,
    preset: ContextPackPreset,
    options: &ContextPackOptions,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let text = build_context_pack_with_options(root, preset, &[], options)?;
    write!(handle, "{}", text)?;
    handle.flush()
}

fn collect_candidate_files(
    root: &Path,
    selected_paths: &[PathBuf],
    max_depth: usize,
    include_tests: bool,
) -> Vec<PathBuf> {
    if !selected_paths.is_empty() {
        return selected_paths
            .iter()
            .filter(|p| p.is_file())
            .cloned()
            .collect();
    }

    let mut out = BTreeSet::new();
    let changed = git_stdout(root, &["status", "--porcelain"]).unwrap_or_default();
    for line in changed.lines().take(50) {
        let rel = line.get(3..).unwrap_or("").trim();
        if rel.is_empty() {
            continue;
        }
        let path = root.join(rel);
        if path.is_file() {
            out.insert(path.clone());
            if include_tests {
                for test_path in infer_test_paths(&path) {
                    if test_path.exists() {
                        out.insert(test_path);
                    }
                }
            }
        }
    }

    if out.is_empty() {
        collect_code_files_recursive(root, 0, max_depth, &mut out);
    }
    out.into_iter().collect()
}

fn collect_snippets(files: &[PathBuf], token_budget: usize) -> Vec<(PathBuf, String, usize)> {
    let mut used = 0usize;
    let mut out = Vec::new();

    for path in files {
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        let clipped = content.lines().take(120).collect::<Vec<_>>().join("\n");
        let tokens = estimate_tokens(&clipped);
        if used + tokens > token_budget {
            continue;
        }
        used += tokens;
        out.push((path.clone(), clipped, tokens));
    }
    out
}

fn infer_test_paths(path: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    if let Some(parent) = path.parent() {
        out.push(parent.join(format!("{}_test.{}", stem, ext)));
        out.push(parent.join(format!("test_{}.{}", stem, ext)));
        out.push(parent.join(format!("{}.test.{}", stem, ext)));
        out.push(parent.join(format!("{}.spec.{}", stem, ext)));
        out.push(parent.join("tests").join(format!("{}.{}", stem, ext)));
    }
    out
}

fn collect_code_files_recursive(
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
            if !name.starts_with('.')
                && !matches!(
                    name.as_str(),
                    "target" | "node_modules" | "dist" | "build" | ".git"
                )
            {
                collect_code_files_recursive(&p, depth + 1, max_depth, out);
            }
        } else if is_code_like(&p) {
            out.insert(p);
        }
    }
}

fn is_code_like(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default(),
        "rs" | "go" | "py" | "ts" | "tsx" | "js" | "jsx" | "java" | "kt"
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_new_presets() {
        assert_eq!(
            ContextPackPreset::from_str("refactor").ok(),
            Some(ContextPackPreset::Refactor)
        );
        assert_eq!(
            ContextPackPreset::from_str("incident").ok(),
            Some(ContextPackPreset::Incident)
        );
        assert_eq!(
            ContextPackPreset::from_str("onboarding").ok(),
            Some(ContextPackPreset::Onboarding)
        );
    }

    #[test]
    fn test_parse_context_agent() {
        assert_eq!(
            ContextAgent::from_str("claude").ok(),
            Some(ContextAgent::Claude)
        );
        assert_eq!(
            ContextAgent::from_str("codex").ok(),
            Some(ContextAgent::Codex)
        );
        assert_eq!(
            ContextAgent::from_str("cursor").ok(),
            Some(ContextAgent::Cursor)
        );
    }
}
