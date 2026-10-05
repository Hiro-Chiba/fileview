//! Bounded plain-text viewport experiment. No production behavior is changed.
//! Example: cargo run --release --example viewport_probe -- /tmp/log.log
//! --offset 1046528 --line 8177 --iterations 40 --whole --render

use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, ensure, Context, Result};
use fileview::render::TextPreview;
use serde_json::{json, Value};

#[path = "support/viewport_probe.rs"]
mod viewport_probe;
use viewport_probe::{read_match_viewport, read_viewport, FileStamp, ViewportAnchor};

fn rss_kib() -> Option<u64> {
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    String::from_utf8(output.stdout).ok()?.trim().parse().ok()
}

fn distribution(mut samples: Vec<f64>) -> Value {
    samples.sort_by(f64::total_cmp);
    let percentile = |fraction: f64| samples[(samples.len() as f64 * fraction).ceil() as usize - 1];
    json!({"samples":samples.len(),"p50_ms":percentile(0.5),"p95_ms":percentile(0.95),"max_ms":samples.last()})
}

fn benchmark<T>(operation: impl Fn() -> Result<T>, iterations: usize) -> Result<Value> {
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        ensure!(
            rss_kib().is_none_or(|rss| rss < 512 * 1024),
            "512 MiB RSS guard exceeded"
        );
        let start = Instant::now();
        let result = operation()?;
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        black_box(result);
    }
    Ok(distribution(samples))
}

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let path = PathBuf::from(args.next().context("usage: viewport_probe PATH [--offset N --line N --match-offset N --iterations N --max-bytes N --whole --render]")?);
    let mut offset = 0;
    let mut line = 1;
    let mut match_offset = None;
    let mut iterations = 40;
    let mut budget = 65536;
    let mut whole = false;
    let mut render = false;
    while let Some(argument) = args.next() {
        let number = |args: &mut std::iter::Skip<std::env::ArgsOs>| -> Result<u64> {
            args.next()
                .context("option requires a number")?
                .to_str()
                .context("invalid number")?
                .parse()
                .context("invalid number")
        };
        match argument.to_str() {
            Some("--offset") => offset = number(&mut args)?,
            Some("--line") => line = number(&mut args)?,
            Some("--match-offset") => match_offset = Some(number(&mut args)?),
            Some("--iterations") => iterations = usize::try_from(number(&mut args)?)?,
            Some("--max-bytes") => budget = usize::try_from(number(&mut args)?)?,
            Some("--whole") => whole = true,
            Some("--render") => render = true,
            _ => bail!(
                "unknown viewport probe option: {}",
                argument.to_string_lossy()
            ),
        }
    }
    ensure!(
        (1..=1000).contains(&iterations),
        "iterations must be between 1 and 1000"
    );
    let stamp = FileStamp::capture(&path)?;
    let first = ViewportAnchor {
        line_number: 1,
        byte_offset: 0,
        stamp: stamp.clone(),
    };
    let target = ViewportAnchor {
        line_number: line,
        byte_offset: offset,
        stamp,
    };
    let before_rss = rss_kib();
    let first_result = benchmark(
        || read_viewport(&path, &first, 40, budget, &|| false),
        iterations,
    )?;
    let target_window = || match match_offset {
        Some(offset) => read_match_viewport(&path, &target, offset, 40, budget, &|| false),
        None => read_viewport(&path, &target, 40, budget, &|| false),
    };
    let target_result = benchmark(target_window, iterations)?;
    let cancellation = benchmark(
        || {
            let calls = std::cell::Cell::new(0);
            let result = read_viewport(&path, &first, 1000, budget, &|| {
                calls.set(calls.get() + 1);
                calls.get() >= 3
            });
            ensure!(
                result
                    .err()
                    .is_some_and(|error| error.to_string().contains("cancelled")),
                "cancellation was not observed"
            );
            Ok(())
        },
        iterations,
    )?;
    let page = target_window()?;
    let window_rss = rss_kib();
    let mut result = json!({
        "path": path, "file_bytes":std::fs::metadata(&path)?.len(), "iterations":iterations,
        "window_budget_bytes":budget, "first_screen":first_result,"known_line_offset":target_result,
        "cancellation_after_first_chunk":cancellation,"rss_before_kib":before_rss,"rss_after_windows_kib":window_rss,
        "target_page":{"first_line":page.first_line,"byte_offset":page.byte_offset,"next_offset":page.next_offset,"bytes_read":page.bytes_read,"shown_lines":page.text.lines().count(),"has_more":page.has_more,"partial_last_line":page.partial_last_line,"prefix_clipped":page.prefix_clipped},
        "scope":"plain UTF-8 text with trusted scanner line/match offsets; no syntax context, arbitrary line index, cold-storage guarantee, or full TUI latency claim"
    });
    if render {
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 42))?;
        terminal.draw(|frame| {
            let text = page
                .text
                .lines()
                .enumerate()
                .map(|(index, text)| format!("{:>9} {text}", page.first_line + index as u64))
                .collect::<Vec<_>>()
                .join("\n");
            frame.render_widget(ratatui::widgets::Paragraph::new(text), frame.area());
        })?;
        let buffer = terminal.backend().buffer();
        let rows: Vec<_> = (0..42)
            .map(|y| {
                (0..120)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect();
        result["offscreen_render_rows"] = json!(rows);
    }
    if whole {
        ensure!(
            std::fs::metadata(&path)?.len() <= 10 * 1024 * 1024,
            "whole-file comparison is restricted to 10 MiB"
        );
        fileview::render::preview::text::warmup();
        result["whole_baseline_warm_rss_kib"] = json!(rss_kib());
        let mut baseline_times = Vec::new();
        let mut baseline_live_rss = None;
        for _ in 0..iterations.min(10) {
            ensure!(
                rss_kib().is_none_or(|rss| rss < 512 * 1024),
                "512 MiB RSS guard exceeded"
            );
            let start = Instant::now();
            let content = std::fs::read_to_string(&path)?;
            let preview = TextPreview::with_highlighting(&content, &path);
            baseline_times.push(start.elapsed().as_secs_f64() * 1000.0);
            // Measure live objects after stopping the latency timer.
            baseline_live_rss = baseline_live_rss.max(rss_kib());
            black_box((&content, &preview));
        }
        result["whole_read_plus_preview"] = distribution(baseline_times);
        result["whole_baseline_max_observed_live_rss_kib"] = json!(baseline_live_rss);
        result["rss_after_whole_baseline_kib"] = json!(rss_kib());
        result["baseline_note"] = json!("syntect caches warmed before measurement; syntax highlighting depends on filename; input content and preview line copies are allocated in full; live RSS sampled after construction, not a continuous peak measurement");
    }
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
