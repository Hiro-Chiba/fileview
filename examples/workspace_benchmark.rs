//! Reproducible local search benchmark, with real files and a warm filesystem cache.
//!
//! Run with `cargo run --profile release-fast --example workspace_benchmark --
//! --files 100000 --iterations 40`. The temporary fixture is deleted automatically.
//! Use `--release` instead for the shipping size-optimized profile. Heavy phases
//! stop above 2 GiB current process RSS when the platform's `ps` reports it.
//! `--variant baseline|engine|path|vec|typing` isolates one alternative in a fresh process.
//! `--cache-experiment` additionally measures optional disk persistence costs.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    hint::black_box,
    path::Path,
    time::Instant,
};

use anyhow::{bail, Context, Result};
use fileview::render::fuzzy::{collect_paths, fuzzy_match_incremental, FuzzyState};
use fileview::workspace::WorkspaceIndex;
use serde_json::{json, Value};

#[path = "support/path_index.rs"]
mod path_index;

const QUERIES: [&str; 8] = [
    "file",
    "rs",
    "png",
    "module_042",
    "file_000420",
    "m42rs",
    "zzzz_not_found",
    "md",
];

fn distribution(mut samples: Vec<f64>) -> Value {
    samples.sort_by(f64::total_cmp);
    let percentile = |p: f64| samples[(samples.len() as f64 * p).ceil() as usize - 1];
    json!({"samples": samples.len(), "p50_ms": percentile(0.50), "p95_ms": percentile(0.95), "max_ms": samples.last()})
}

fn measure<T>(operation: impl FnOnce() -> T) -> (T, f64) {
    let start = Instant::now();
    let result = black_box(operation());
    (result, start.elapsed().as_secs_f64() * 1000.0)
}

fn current_rss_kib() -> Option<u64> {
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    String::from_utf8(output.stdout).ok()?.trim().parse().ok()
}

fn check_memory_bound() -> Result<()> {
    if current_rss_kib().is_some_and(|kib| kib > 2 * 1024 * 1024) {
        bail!("benchmark stopped at the 2 GiB process RSS safety bound");
    }
    Ok(())
}

fn fingerprint(index: &WorkspaceIndex) -> Result<(usize, u64)> {
    let entries = index.search("", true, usize::MAX, &|| false)?;
    let count = entries.len();
    let mut hasher = DefaultHasher::new();
    for entry in entries {
        (
            entry.path,
            entry.is_dir,
            entry.size,
            entry.modified_unix_secs,
        )
            .hash(&mut hasher);
    }
    Ok((count, hasher.finish()))
}

fn fixture(root: &Path, files: usize) -> Result<()> {
    for directory in 0..100 {
        std::fs::create_dir(root.join(format!("module_{directory:03}")))?;
    }
    let extensions = ["rs", "md", "png", "toml", "txt"];
    for i in 0..files {
        std::fs::write(
            root.join(format!(
                "module_{:03}/file_{i:06}.{}",
                i % 100,
                extensions[i % extensions.len()]
            )),
            b"benchmark fixture\n",
        )?;
    }
    Ok(())
}

fn engine_benchmark(root: &Path, iterations: usize, cache_experiment: bool) -> Result<Value> {
    let rss_before_build = current_rss_kib();
    let mut index = WorkspaceIndex::new(root)?;
    let mut builds = Vec::new();
    let mut rss_after_first_build = None;
    for iteration in 0..iterations {
        let (result, ms) = measure(|| index.rebuild(&|| false));
        result?;
        builds.push(ms);
        check_memory_bound()?;
        if iteration == 0 {
            rss_after_first_build = current_rss_kib();
        }
    }
    let rss_after_rebuilds = current_rss_kib();
    let mut fresh_initializations = Vec::new();
    for _ in 0..iterations {
        let (fresh, ms) = measure(|| -> Result<WorkspaceIndex> {
            let mut fresh = WorkspaceIndex::new(root)?;
            fresh.rebuild(&|| false)?;
            Ok(fresh)
        });
        black_box(fresh?);
        fresh_initializations.push(ms);
        check_memory_bound()?;
    }
    let cache_dir = tempfile::tempdir()?;
    let cache_path = cache_dir.path().join("workspace.json");
    let mut saves = Vec::new();
    let mut loads = Vec::new();
    let mut cache_error = None;
    for _ in 0..if cache_experiment { iterations } else { 0 } {
        let (result, ms) = measure(|| index.save_cache(&cache_path));
        if let Err(error) = result {
            cache_error = Some(error.to_string());
            break;
        }
        saves.push(ms);
        let (result, ms) = measure(|| WorkspaceIndex::load_cache(root, &cache_path));
        black_box(result?);
        loads.push(ms);
        check_memory_bound()?;
    }
    let cache_save = (!saves.is_empty()).then(|| distribution(saves));
    let cache_load = (!loads.is_empty()).then(|| distribution(loads));
    let rss_after_caches = current_rss_kib();
    let cache_initialize = if cache_experiment && cache_error.is_none() {
        let mut samples = Vec::new();
        for _ in 0..iterations {
            let (loaded, ms) = measure(|| -> Result<WorkspaceIndex> {
                let mut loaded = WorkspaceIndex::load_cache(root, &cache_path)?;
                loaded.rebuild(&|| false)?;
                Ok(loaded)
            });
            black_box(loaded?);
            samples.push(ms);
            check_memory_bound()?;
        }
        Some(distribution(samples))
    } else {
        None
    };
    let mut queries = serde_json::Map::new();
    let mut all = Vec::new();
    let paths = collect_paths(&root.to_path_buf(), false);
    for query in QUERIES {
        let matches = index.search(query, false, 15, &|| false)?;
        let baseline =
            fuzzy_match_incremental(query, &paths, &root.to_path_buf(), &mut FuzzyState::new());
        // Equal-score ordering differs, so compare ranked scores, not arbitrary tie order.
        anyhow::ensure!(
            matches.iter().map(|entry| entry.score).collect::<Vec<_>>()
                == baseline.iter().map(|entry| entry.score).collect::<Vec<_>>(),
            "top search scores differ from baseline for {query}"
        );
        check_memory_bound()?;
        let mut samples = Vec::new();
        for _ in 0..iterations {
            let (result, ms) = measure(|| index.search(query, false, 15, &|| false));
            black_box(result?);
            samples.push(ms);
            all.push(ms);
        }
        queries.insert(query.to_owned(), distribution(samples));
    }
    drop(paths);
    let mut singles = Vec::new();
    let single = root.join("single_delta.rs");
    for iteration in 0..iterations {
        if iteration % 2 == 0 {
            std::fs::write(&single, b"single delta\n")?;
        } else {
            std::fs::remove_file(&single)?;
        }
        let (result, ms) = measure(|| index.reconcile(std::slice::from_ref(&single), &|| false));
        result?;
        singles.push(ms);
    }
    let batch = root.join("batch_delta");
    let mut batches = Vec::new();
    for iteration in 0..iterations {
        if iteration % 2 == 0 {
            std::fs::create_dir(&batch)?;
            for i in 0..1000 {
                std::fs::write(batch.join(format!("batch_{i:04}.rs")), b"batch delta\n")?;
            }
        } else {
            std::fs::remove_dir_all(&batch)?;
        }
        let (result, ms) = measure(|| index.reconcile(std::slice::from_ref(&batch), &|| false));
        result?;
        batches.push(ms);
    }
    // Both index contents and metadata must agree with an independent rebuild.
    let incremental = fingerprint(&index)?;
    drop(index);
    let mut reference = WorkspaceIndex::new(root)?;
    reference.rebuild(&|| false)?;
    anyhow::ensure!(
        incremental == fingerprint(&reference)?,
        "delta index differs from fresh rebuild"
    );
    Ok(json!({
        "rebuild": distribution(builds),
        "initialize_without_cache": distribution(fresh_initializations),
        "cache_save": cache_save,
        "cache_load_warm_os_cache": cache_load,
        "cache_bytes": std::fs::metadata(cache_path).ok().map(|metadata| metadata.len()),
        "cache_error": cache_error,
        "cache_experiment": cache_experiment,
        "cache_load_and_revalidate": cache_initialize,
        "process_rss_kib": {
            "before_build": rss_before_build,
            "after_first_build": rss_after_first_build,
            "after_repeated_rebuilds": rss_after_rebuilds,
            "after_repeated_cache_loads": rss_after_caches,
            "scope": "whole process current RSS; includes allocator retention and baseline work"
        },
        "search": distribution(all),
        "queries": queries,
        "single_file_delta": distribution(singles),
        "directory_1000_file_delta": distribution(batches),
        "delta_workload": "alternating create/delete; filesystem mutations excluded from timings",
        "delta_matches_fresh_rebuild": true,
        "top_scores_match_baseline": true,
    }))
}

fn baseline_benchmark(root: &Path, iterations: usize) -> Result<Value> {
    let root = root.to_path_buf();
    // Warmup is explicit. This does not claim cold disk performance.
    let paths = collect_paths(&root, false);
    for query in QUERIES {
        black_box(fuzzy_match_incremental(
            query,
            &paths,
            &root,
            &mut FuzzyState::new(),
        ));
    }
    let mut scans = Vec::new();
    let mut open_and_search = Vec::new();
    for iteration in 0..iterations {
        let start = Instant::now();
        let (collected, ms) = measure(|| collect_paths(&root, false));
        scans.push(ms);
        black_box(fuzzy_match_incremental(
            QUERIES[iteration % QUERIES.len()],
            &collected,
            &root,
            &mut FuzzyState::new(),
        ));
        open_and_search.push(start.elapsed().as_secs_f64() * 1000.0);
        check_memory_bound()?;
    }
    let mut queries = serde_json::Map::new();
    let mut all = Vec::new();
    for query in QUERIES {
        let mut samples = Vec::new();
        for _ in 0..iterations {
            let (_, ms) =
                measure(|| fuzzy_match_incremental(query, &paths, &root, &mut FuzzyState::new()));
            samples.push(ms);
            all.push(ms);
            check_memory_bound()?;
        }
        queries.insert(query.to_owned(), distribution(samples));
    }
    let indexed_paths = paths.len();
    drop(paths);
    Ok(json!({
        "indexed_paths": indexed_paths,
        "scan": distribution(scans),
        "search": distribution(all),
        "queries": queries,
        "open_and_search": distribution(open_and_search),
    }))
}

fn typing_benchmark(root: &Path, iterations: usize) -> Result<Value> {
    let trace = [
        "m",
        "mo",
        "mod",
        "module",
        "module_",
        "module_0",
        "module_04",
        "module_042",
        "module_042/",
        "module_042/f",
        "module_042/fi",
        "module_042/file_000",
        "module_042/file_0000",
        "module_042/file_000",
        "module_042/file_",
        "",
        "r",
        "rs",
        "png",
        "zzzz_not_found",
    ];
    let root = root.to_path_buf();
    let paths = collect_paths(&root, false);
    let mut index = WorkspaceIndex::new(&root)?;
    index.rebuild(&|| false)?;
    let mut baseline_samples = vec![Vec::new(); trace.len()];
    let mut engine_samples = vec![Vec::new(); trace.len()];
    let mut baseline_totals = Vec::new();
    let mut engine_totals = Vec::new();
    // The first complete trace warms both algorithms and is excluded from results.
    for trial in 0..=iterations {
        let mut state = FuzzyState::new();
        let mut baseline_total = 0.0;
        let mut engine_total = 0.0;
        for (step, query) in trace.iter().enumerate() {
            let (baseline, baseline_ms);
            let (engine, engine_ms);
            // Alternate measurement order to reduce systematic first-run bias.
            if trial % 2 == 0 {
                (baseline, baseline_ms) =
                    measure(|| fuzzy_match_incremental(query, &paths, &root, &mut state));
                (engine, engine_ms) = measure(|| index.search(query, false, 15, &|| false));
            } else {
                (engine, engine_ms) = measure(|| index.search(query, false, 15, &|| false));
                (baseline, baseline_ms) =
                    measure(|| fuzzy_match_incremental(query, &paths, &root, &mut state));
            }
            let engine = engine?;
            anyhow::ensure!(
                baseline.iter().map(|entry| entry.score).collect::<Vec<_>>()
                    == engine.iter().map(|entry| entry.score).collect::<Vec<_>>(),
                "typing scores differ at step {step}: {query}"
            );
            if trial > 0 {
                baseline_samples[step].push(baseline_ms);
                engine_samples[step].push(engine_ms);
                baseline_total += baseline_ms;
                engine_total += engine_ms;
            }
        }
        if trial > 0 {
            baseline_totals.push(baseline_total);
            engine_totals.push(engine_total);
        }
        check_memory_bound()?;
    }
    let baseline_all = baseline_samples.iter().flatten().copied().collect();
    let engine_all = engine_samples.iter().flatten().copied().collect();
    let steps: Vec<_> = trace
        .iter()
        .enumerate()
        .map(|(step, query)| {
            json!({
                "step": step, "query": query,
                "baseline": distribution(std::mem::take(&mut baseline_samples[step])),
                "engine": distribution(std::mem::take(&mut engine_samples[step])),
            })
        })
        .collect();
    let mut structured = serde_json::Map::new();
    for query in ["", "type:dir", "ext:rs"] {
        let expected: Vec<_> = index
            .search(query, false, usize::MAX, &|| false)?
            .into_iter()
            .take(15)
            .collect();
        let actual = index.search(query, false, 15, &|| false)?;
        anyhow::ensure!(
            serde_json::to_value(actual)? == serde_json::to_value(expected)?,
            "bounded filter differs from full-result prefix: {query}"
        );
        let mut samples = Vec::new();
        for _ in 0..iterations {
            let (result, ms) = measure(|| index.search(query, false, 15, &|| false));
            black_box(result?);
            samples.push(ms);
        }
        structured.insert(query.to_owned(), distribution(samples));
    }
    Ok(json!({
        "trace": steps,
        "baseline_keystrokes": distribution(baseline_all),
        "engine_keystrokes": distribution(engine_all),
        "baseline_trace_compute_sum": distribution(baseline_totals),
        "engine_trace_compute_sum": distribution(engine_totals),
        "top_scores_match_baseline": true,
        "filter_only_queries": structured,
        "filter_only_matches_unbounded_prefix": true,
        "condition": "warm index and path list; legacy FuzzyState retained across each prefix/backspace/query-switch trace; checks outside timings; first trace excluded; no UI scheduling or typing delays",
    }))
}

fn main() -> Result<()> {
    let mut files = 100_000;
    let mut iterations = 40;
    let mut variant = String::from("all");
    let mut cache_experiment = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--cache-experiment" {
            cache_experiment = true;
            continue;
        }
        let value = args.next().context("expected option value")?;
        match arg.as_str() {
            "--files" => files = value.parse()?,
            "--iterations" => iterations = value.parse()?,
            "--variant" => variant = value,
            _ => bail!("unknown option {arg}"),
        }
    }
    if files == 0 || iterations == 0 {
        bail!("files and iterations must be positive");
    }
    if !matches!(
        variant.as_str(),
        "all" | "baseline" | "engine" | "path" | "vec" | "typing"
    ) {
        bail!("variant must be all, baseline, engine, path, vec, or typing");
    }
    let temp = tempfile::tempdir()?;
    let root = temp.path().canonicalize()?;
    fixture(&root, files)?;
    let mut report = json!({
        "files": files,
        "directories": 100,
        "variant": variant,
        "cache_experiment": cache_experiment,
        "iterations_per_query": iterations,
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "cache_condition": "warm filesystem cache after fixture creation and explicit warmup",
        "query_condition": "independent full query each time; baseline incremental state reset",
        "timing_scope": "in-process operations only; fixture creation and deletion excluded",
    });
    let result = (|| -> Result<()> {
        if matches!(variant.as_str(), "all" | "baseline") {
            report["baseline"] = baseline_benchmark(&root, iterations).context("baseline phase")?;
        }
        if matches!(variant.as_str(), "all" | "engine") {
            report["engine"] = engine_benchmark(&root, iterations, cache_experiment)
                .context("production index phase")?;
        }
        // Restore the fixture after odd delta iteration counts, outside timings.
        let single = root.join("single_delta.rs");
        if single.exists() {
            std::fs::remove_file(single)?;
        }
        let batch = root.join("batch_delta");
        if batch.exists() {
            std::fs::remove_dir_all(batch)?;
        }
        if matches!(variant.as_str(), "all" | "path") {
            report["path_index"] = path_index::benchmark(&root, iterations, &QUERIES)
                .context("path BTreeMap phase")?;
        }
        if matches!(variant.as_str(), "all" | "vec") {
            report["path_index_vec"] =
                path_index::benchmark_vec(&root, iterations, &QUERIES).context("path Vec phase")?;
        }
        if variant == "typing" {
            report["typing"] = typing_benchmark(&root, iterations).context("typing trace phase")?;
            report["query_condition"] =
                json!("incremental typing trace; baseline state retained within each trace");
        }
        Ok(())
    })();
    if let Err(error) = &result {
        report["error"] = json!(format!("{error:#}"));
    }
    report["process_rss_at_completion_kib"] = json!(current_rss_kib());
    // Emit partial completed phases even on a memory-bound failure.
    println!("{}", serde_json::to_string_pretty(&report)?);
    result
}
