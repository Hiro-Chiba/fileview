//! Empirical literal-content search probe. No production commands or APIs change.
//!
//! cargo run --profile release-fast --example content_search_probe -- \
//!   --file /path/to/log --needle FV_NEAR_END --iterations 10 --compare-rg
//! --root DIR uses local ignore rules. --cancel-ms N measures cooperative abort.
//! --whole-read adds a deliberately whole-file baseline, capped at 100 MiB.

#[path = "support/content_probe.rs"]
mod content_probe;
#[path = "support/viewport_probe.rs"]
#[allow(dead_code)]
mod viewport_probe;

use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::{json, Value};

use content_probe::{Hit, Options};

const WHOLE_READ_LIMIT: u64 = 100 * 1024 * 1024;
const RG_RECORD_LIMIT: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct MatchLocation {
    line_number: u64,
    line_start: u64,
    match_offset: u64,
}

impl From<&Hit> for MatchLocation {
    fn from(hit: &Hit) -> Self {
        Self {
            line_number: hit.line_number,
            line_start: hit.line_start,
            match_offset: hit.match_offset,
        }
    }
}

#[derive(Default)]
struct Arguments {
    path: Option<PathBuf>,
    needle: String,
    iterations: usize,
    cancel_ms: Option<u64>,
    compare_rg: bool,
    whole_read: bool,
    options: Options,
}

fn parse() -> Result<Arguments> {
    let mut config = Arguments {
        iterations: 1,
        needle: "FV_NEAR_END".into(),
        ..Default::default()
    };
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || -> Result<OsString> { args.next().context("missing option value") };
        match arg.to_str() {
            Some("--file" | "--root") => config.path = Some(PathBuf::from(value()?)),
            Some("--needle") => {
                config.needle = value()?
                    .into_string()
                    .map_err(|_| anyhow::anyhow!("needle must be UTF-8"))?
            }
            Some("--iterations") => config.iterations = value()?.to_string_lossy().parse()?,
            Some("--cancel-ms") => config.cancel_ms = Some(value()?.to_string_lossy().parse()?),
            Some("--max-hits") => config.options.max_hits = value()?.to_string_lossy().parse()?,
            Some("--chunk-bytes") => {
                config.options.chunk_bytes = value()?.to_string_lossy().parse()?
            }
            Some("--max-file-bytes") => {
                config.options.max_file_bytes = Some(value()?.to_string_lossy().parse()?)
            }
            Some("--hidden") => config.options.show_hidden = true,
            Some("--compare-rg") => config.compare_rg = true,
            Some("--whole-read") => config.whole_read = true,
            Some("--help") => {
                println!("content_search_probe --file FILE|--root DIR [--needle TEXT] [--iterations N] [--cancel-ms N] [--max-hits N] [--chunk-bytes N] [--max-file-bytes N] [--hidden] [--compare-rg] [--whole-read]");
                std::process::exit(0);
            }
            _ => bail!("unknown option: {}", arg.to_string_lossy()),
        }
    }
    anyhow::ensure!(config.path.is_some(), "--file or --root is required");
    anyhow::ensure!(
        (1..=1000).contains(&config.iterations),
        "iterations must be 1..1000"
    );
    Ok(config)
}

fn distribution(mut values: Vec<f64>) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    values.sort_by(f64::total_cmp);
    let percentile = |fraction: f64| values[(fraction * values.len() as f64).ceil() as usize - 1];
    json!({"samples":values.len(),"p50_ms":percentile(0.5),"p95_ms":percentile(0.95),"max_ms":values.last()})
}

fn current_rss_kib() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let text = std::fs::read_to_string("/proc/self/status").ok()?;
        text.lines()
            .find(|line| line.starts_with("VmRSS:"))?
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let output = Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()?;
        String::from_utf8(output.stdout).ok()?.trim().parse().ok()
    }
}

fn peak_rss_kib() -> Option<u64> {
    #[cfg(unix)]
    {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        // The OS initializes rusage on a successful call.
        if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
            return None;
        }
        let usage = unsafe { usage.assume_init() };
        let value = usage.ru_maxrss as u64;
        Some(if cfg!(target_os = "macos") {
            value / 1024
        } else {
            value
        })
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Reference rg JSON stream, bounded per record and stopped at the same hit cap.
/// Used only with explicit regular text files to avoid different ignore policies.
fn rg_reference(path: &Path, needle: &str, limit: usize) -> Result<Value> {
    let start = Instant::now();
    let mut child = Command::new("rg")
        .args([
            "--no-config",
            "--fixed-strings",
            "--json",
            "--line-number",
            "--",
            needle,
        ])
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdout = child.stdout.take().context("rg stdout unavailable")?;
    let mut reader = BufReader::new(stdout);
    let mut matches = Vec::new();
    let mut first_hit_ms = None;
    let result = (|| -> Result<()> {
        loop {
            let mut record = Vec::new();
            Read::by_ref(&mut reader)
                .take(RG_RECORD_LIMIT + 1)
                .read_until(b'\n', &mut record)?;
            if record.is_empty() {
                break;
            }
            if record.len() as u64 > RG_RECORD_LIMIT {
                bail!("rg JSON record exceeds bounded 1 MiB reference limit");
            }
            let value: Value = serde_json::from_slice(&record)?;
            if value["type"] != "match" {
                continue;
            }
            first_hit_ms.get_or_insert_with(|| start.elapsed().as_secs_f64() * 1000.0);
            let data = &value["data"];
            let line_number = data["line_number"]
                .as_u64()
                .context("rg omitted line number")?;
            let line_start = data["absolute_offset"]
                .as_u64()
                .context("rg omitted offset")?;
            for found in data["submatches"]
                .as_array()
                .context("rg omitted submatches")?
            {
                matches.push(MatchLocation {
                    line_number,
                    line_start,
                    match_offset: line_start
                        + found["start"].as_u64().context("rg omitted match offset")?,
                });
                if matches.len() == limit {
                    return Ok(());
                }
            }
        }
        Ok(())
    })();
    let limit_reached = matches.len() == limit;
    if result.is_err() || limit_reached {
        let _ = child.kill();
    }
    let status = child.wait()?;
    result?;
    if !limit_reached && !matches!(status.code(), Some(0 | 1)) {
        bail!("rg reference failed: {status}");
    }
    Ok(json!({"matches": matches, "first_hit_ms":first_hit_ms,
        "total_ms":start.elapsed().as_secs_f64()*1000.0,"hit_limit_reached":limit_reached}))
}

fn whole_read_reference(path: &Path, needle: &str, limit: usize) -> Result<Value> {
    anyhow::ensure!(
        path.metadata()?.len() <= WHOLE_READ_LIMIT,
        "whole-read reference is limited to 100 MiB"
    );
    let start = Instant::now();
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(WHOLE_READ_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    anyhow::ensure!(
        bytes.len() as u64 <= WHOLE_READ_LIMIT,
        "file grew above whole-read reference bound"
    );
    let regex = regex::bytes::Regex::new(&regex::escape(needle))?;
    let mut matches = Vec::new();
    let mut first_hit_ms = None;
    let mut line_number = 1;
    let mut line_start = 0;
    let mut cursor = 0;
    for found in regex.find_iter(&bytes).take(limit) {
        for (position, byte) in bytes[cursor..found.start()].iter().enumerate() {
            if *byte == b'\n' {
                line_number += 1;
                line_start = (cursor + position + 1) as u64;
            }
        }
        cursor = found.start();
        first_hit_ms.get_or_insert_with(|| start.elapsed().as_secs_f64() * 1000.0);
        matches.push(MatchLocation {
            line_number,
            line_start,
            match_offset: found.start() as u64,
        });
    }
    Ok(
        json!({"matches":matches,"first_hit_ms":first_hit_ms,"total_ms":start.elapsed().as_secs_f64()*1000.0}),
    )
}

fn main() -> Result<()> {
    let config = parse()?;
    let path = config.path.as_ref().context("missing path")?;
    let is_file = path.symlink_metadata()?.is_file();
    anyhow::ensure!(
        !(config.compare_rg || config.whole_read) || is_file,
        "references require an explicit regular text file"
    );
    let rss_before = current_rss_kib();
    let mut trials = Vec::new();
    let mut locations = Vec::new();
    let mut first_hits = Vec::new();
    let mut totals = Vec::new();
    let mut cancellations = Vec::new();
    for iteration in 0..config.iterations {
        let cancelled = Arc::new(AtomicBool::new(false));
        let fired_at = Arc::new(Mutex::new(None));
        let timer = config.cancel_ms.map(|delay| {
            let flag = cancelled.clone();
            let fired_at = fired_at.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(delay));
                *fired_at.lock().unwrap_or_else(|error| error.into_inner()) = Some(Instant::now());
                flag.store(true, Ordering::Release);
            })
        });
        let mut found = Vec::new();
        let stats = content_probe::search(
            path,
            &config.needle,
            &config.options,
            &|| cancelled.load(Ordering::Acquire),
            &mut |hit| {
                found.push(MatchLocation::from(hit));
                true
            },
        )?;
        let completed = Instant::now();
        if let Some(timer) = timer {
            let _ = timer.join();
        }
        let cancel_ack_ms = if stats.cancelled {
            fired_at
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .map(|fired| completed.duration_since(fired).as_secs_f64() * 1000.0)
        } else {
            None
        };
        if let Some(value) = cancel_ack_ms {
            cancellations.push(value);
        }
        if let Some(value) = stats.first_hit_ms {
            first_hits.push(value);
        }
        totals.push(stats.total_ms);
        if iteration == 0 {
            locations = found;
        }
        trials.push(json!({"stats":stats,"cancel_ack_ms":cancel_ack_ms}));
        anyhow::ensure!(
            current_rss_kib().is_none_or(|rss| rss < 1024 * 1024),
            "probe exceeds 1 GiB RSS safety bound"
        );
    }
    let stream_peak = peak_rss_kib();
    let rss_after_stream = current_rss_kib();
    let rg_version = if config.compare_rg {
        Some(
            String::from_utf8_lossy(&Command::new("rg").arg("--version").output()?.stdout)
                .lines()
                .next()
                .unwrap_or("unknown")
                .to_owned(),
        )
    } else {
        None
    };
    let rg = if config.compare_rg {
        Some(rg_reference(path, &config.needle, config.options.max_hits)?)
    } else {
        None
    };
    let whole = if config.whole_read {
        Some(whole_read_reference(
            path,
            &config.needle,
            config.options.max_hits,
        )?)
    } else {
        None
    };
    if config.cancel_ms.is_none() && config.options.max_file_bytes.is_none() {
        for reference in [&rg, &whole].into_iter().flatten() {
            anyhow::ensure!(
                reference["matches"] == serde_json::to_value(&locations)?,
                "streamed locations differ from reference"
            );
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "experimental":true,"input":path.to_string_lossy(),"needle":config.needle,
            "input_bytes":if is_file {Some(path.metadata()?.len())} else {None},
            "streaming":{"first_hit":distribution(first_hits),"total":distribution(totals),"cancel_ack":distribution(cancellations),"trials":trials,"locations":locations},
            "memory":{"rss_before_kib":rss_before,"rss_after_stream_kib":rss_after_stream,"peak_before_references_kib":stream_peak,"peak_after_references_kib":peak_rss_kib()},
            "rg_version":rg_version,"rg_reference":rg,"whole_read_reference":whole,
            "policy":"literal case-sensitive single-line UTF-8 query; skip NUL in first chunk, stop on later NUL with earlier hits retained; skip symlinks; bounded preceding-context snippets; changed stamp requires re-search before navigation",
            "measurement":"regular OS reads, cooperative cancellation between bounded chunks; warm filesystem cache if fixture was recently written; no production integration"
        }))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streamed_locations_match_installed_rg_json_and_whole_read_reference() {
        if Command::new("rg").arg("--version").output().is_err() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("sample.txt");
        std::fs::write(&path, "αβ\r\n日本語needle\r\n終needle\nneedle").unwrap();
        let mut locations = Vec::new();
        content_probe::search(
            &path,
            "needle",
            &Options {
                chunk_bytes: 7,
                ..Default::default()
            },
            &|| false,
            &mut |hit| {
                locations.push(MatchLocation::from(hit));
                true
            },
        )
        .unwrap();
        let expected = serde_json::to_value(locations).unwrap();
        assert_eq!(
            rg_reference(&path, "needle", 100).unwrap()["matches"],
            expected
        );
        assert_eq!(
            whole_read_reference(&path, "needle", 100).unwrap()["matches"],
            expected
        );
    }

    #[test]
    fn emitted_line_anchor_opens_the_actual_match_line_and_rejects_later_changes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("sample.txt");
        std::fs::write(&path, "first\r\n日本語 needle here\r\nlast\n").unwrap();
        let mut found = None;
        content_probe::search(
            &path,
            "needle",
            &Options::default(),
            &|| false,
            &mut |hit| {
                found = Some(hit.clone());
                false
            },
        )
        .unwrap();
        let hit = found.unwrap();
        let anchor = viewport_probe::ViewportAnchor {
            line_number: hit.line_number,
            byte_offset: hit.line_start,
            stamp: hit.stamp,
        };
        let page = viewport_probe::read_match_viewport(
            &path,
            &anchor,
            hit.match_offset,
            2,
            64 * 1024,
            &|| false,
        )
        .unwrap();
        assert!(page.text.starts_with("日本語 needle here\r\n"));
        assert_eq!(page.first_line, 2);
        std::fs::write(&path, "changed").unwrap();
        assert!(viewport_probe::read_match_viewport(
            &path,
            &anchor,
            hit.match_offset,
            2,
            64 * 1024,
            &|| false
        )
        .is_err());
    }
}
