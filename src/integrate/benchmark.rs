//! Lightweight AI benchmark helpers for CLI comparison.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::mcp::token::estimate_tokens;

use super::{
    build_context_pack_with_options, collect_related_candidates, ContextPackOptions,
    ContextPackPreset,
};

#[derive(Debug, Clone)]
struct BenchmarkResult {
    scenario: String,
    iterations: usize,
    avg_ms: f64,
    bytes: usize,
    tokens: usize,
    note: String,
}

/// Run AI benchmark scenario(s) and print JSON lines.
pub fn run_ai_benchmark(root: &Path, scenario: &str, iterations: usize) -> io::Result<()> {
    let iterations = iterations.max(1);
    let mut results = Vec::new();

    match scenario {
        "context-pack" => {
            results.push(bench_context_pack(
                root,
                iterations,
                ContextPackPreset::Minimal,
            )?);
        }
        "review-pack" => {
            results.push(bench_context_pack(
                root,
                iterations,
                ContextPackPreset::Review,
            )?);
        }
        "related" => {
            results.push(bench_related(root, iterations)?);
        }
        "all" => {
            results.push(bench_context_pack(
                root,
                iterations,
                ContextPackPreset::Minimal,
            )?);
            results.push(bench_context_pack(
                root,
                iterations,
                ContextPackPreset::Review,
            )?);
            results.push(bench_related(root, iterations)?);
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown benchmark scenario: {}", scenario),
            ));
        }
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    for r in results {
        let json = serde_json::json!({
            "scenario": r.scenario,
            "iterations": r.iterations,
            "avg_ms": r.avg_ms,
            "bytes": r.bytes,
            "tokens": r.tokens,
            "note": r.note,
        });
        writeln!(handle, "{}", json)?;
    }
    handle.flush()
}

fn bench_context_pack(
    root: &Path,
    iterations: usize,
    preset: ContextPackPreset,
) -> io::Result<BenchmarkResult> {
    let mut total_ms = 0.0f64;
    let mut last = String::new();
    let options = ContextPackOptions::default();

    for _ in 0..iterations {
        let start = Instant::now();
        let out = build_context_pack_with_options(root, preset, &[], &options)?;
        total_ms += start.elapsed().as_secs_f64() * 1000.0;
        last = out;
    }

    Ok(BenchmarkResult {
        scenario: format!("context-pack-{}", preset.as_str()),
        iterations,
        avg_ms: total_ms / iterations as f64,
        bytes: last.len(),
        tokens: estimate_tokens(&last),
        note: "build_context_pack_with_options".to_string(),
    })
}

fn bench_related(root: &Path, iterations: usize) -> io::Result<BenchmarkResult> {
    let target = find_first_code_file(root, 0, 4).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "no code file found for related benchmark",
        )
    })?;

    let mut total_ms = 0.0f64;
    let mut last_count = 0usize;
    for _ in 0..iterations {
        let start = Instant::now();
        let candidates = collect_related_candidates(&target);
        total_ms += start.elapsed().as_secs_f64() * 1000.0;
        last_count = candidates.len();
    }

    Ok(BenchmarkResult {
        scenario: "related-selection".to_string(),
        iterations,
        avg_ms: total_ms / iterations as f64,
        bytes: last_count,
        tokens: 0,
        note: format!("target={}", target.display()),
    })
}

fn find_first_code_file(path: &Path, depth: usize, max_depth: usize) -> Option<PathBuf> {
    if depth > max_depth {
        return None;
    }
    let entries = fs::read_dir(path).ok()?;
    for entry in entries.flatten() {
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            if let Some(found) = find_first_code_file(&p, depth + 1, max_depth) {
                return Some(found);
            }
            continue;
        }
        if matches!(
            p.extension().and_then(|e| e.to_str()).unwrap_or_default(),
            "rs" | "go" | "py" | "ts" | "tsx" | "js" | "jsx" | "java" | "kt"
        ) {
            return Some(p);
        }
    }
    None
}
