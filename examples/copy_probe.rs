//! Run with --source PATH --iterations 5 --cancel-iterations 30 --chunk-kib 1024.
//! Caller-owned input is read only. Each output is removed before the next trial.
//! Checksums are outside timings. This is a thread/controller probe, not a TUI test.

use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::Hasher;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};

#[path = "support/copy_probe.rs"]
mod copy_probe;
use copy_probe::{copy_file, CancelToken, CopyError, Progress};

fn distribution(mut values: Vec<f64>) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    values.sort_by(f64::total_cmp);
    let percentile = |p: f64| values[(values.len() as f64 * p).ceil() as usize - 1];
    json!({"samples":values.len(), "p50_ms":percentile(0.5), "p95_ms":percentile(0.95), "max_ms":values.last()})
}

fn checksum(path: &Path) -> Result<(u64, u64)> {
    let mut input = File::open(path)?;
    let mut buffer = vec![0; 1024 * 1024];
    let mut hash = DefaultHasher::new();
    let mut bytes = 0;
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.write(&buffer[..count]);
        bytes += count as u64;
    }
    Ok((bytes, hash.finish()))
}

fn median_effective_mib_per_second(bytes: u64, samples: &[f64]) -> f64 {
    let mut samples = samples.to_vec();
    samples.sort_by(f64::total_cmp);
    let median_ms = samples[(samples.len() as f64 * 0.5).ceil() as usize - 1];
    bytes as f64 / (1024.0 * 1024.0) / (median_ms / 1000.0)
}

struct BackgroundTrial {
    result: Result<u64, CopyError>,
    copy_ms: f64,
    heartbeat_ms: Vec<f64>,
    cancel_ms: Option<f64>,
    cancel_accepted: bool,
    progress_observations: usize,
}

fn background_trial(
    source: &Path,
    destination: &Path,
    chunk: usize,
    cancel_after: Option<u64>,
) -> Result<BackgroundTrial> {
    let token = Arc::new(CancelToken::default());
    // One overwritten slot always contains the latest progress. No unbounded event queue.
    let progress = Arc::new(Mutex::new(Progress::default()));
    let worker_token = Arc::clone(&token);
    let worker_progress = Arc::clone(&progress);
    let source = source.to_path_buf();
    let destination = destination.to_path_buf();
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        let start = Instant::now();
        let result = copy_file(&source, &destination, chunk, &worker_token, |update| {
            *worker_progress
                .lock()
                .unwrap_or_else(|poison| poison.into_inner()) = update;
        });
        let finished = Instant::now();
        let _ = sender.send((result, start, finished));
    });
    let mut heartbeats = Vec::new();
    let mut previous = Instant::now();
    let mut requested = None;
    let mut observations = 0;
    let mut invalid_progress = false;
    let (result, started, finished) = loop {
        match receiver.try_recv() {
            Ok(done) => break done,
            Err(mpsc::TryRecvError::Disconnected) => {
                worker
                    .join()
                    .map_err(|_| anyhow::anyhow!("copy worker panicked"))?;
                bail!("copy worker ended without a result");
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
        let now = Instant::now();
        heartbeats.push(now.duration_since(previous).as_secs_f64() * 1000.0);
        previous = now;
        let current = *progress.lock().unwrap_or_else(|poison| poison.into_inner());
        if current.copied > current.total {
            invalid_progress = true;
            token.request_cancel();
        }
        observations += 1;
        if requested.is_none() && cancel_after.is_some_and(|bytes| current.copied >= bytes) {
            let at = Instant::now();
            if token.request_cancel() {
                requested = Some(at);
            }
        }
        thread::sleep(Duration::from_millis(1));
    };
    worker
        .join()
        .map_err(|_| anyhow::anyhow!("copy worker panicked"))?;
    ensure!(!invalid_progress, "invalid progress");
    Ok(BackgroundTrial {
        result,
        copy_ms: finished.duration_since(started).as_secs_f64() * 1000.0,
        heartbeat_ms: heartbeats,
        cancel_ms: requested
            .map(|at| finished.saturating_duration_since(at).as_secs_f64() * 1000.0),
        cancel_accepted: requested.is_some(),
        progress_observations: observations,
    })
}

fn main() -> Result<()> {
    let mut source = None;
    let mut iterations = 5;
    let mut cancel_iterations = 30;
    let mut chunk_kib = 1024usize;
    let mut output_parent = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next().context("expected option value")?;
        match arg.as_str() {
            "--source" => source = Some(PathBuf::from(value)),
            "--output-dir" => output_parent = Some(PathBuf::from(value)),
            "--iterations" => iterations = value.parse()?,
            "--cancel-iterations" => cancel_iterations = value.parse()?,
            "--chunk-kib" => chunk_kib = value.parse()?,
            _ => bail!("unknown argument {arg}"),
        }
    }
    ensure!(iterations > 0, "iterations must be positive");
    let chunk = chunk_kib.checked_mul(1024).context("chunk size overflow")?;
    ensure!(
        chunk > 0 && chunk <= 16 * 1024 * 1024,
        "chunk must be 1..=16384 KiB"
    );
    let source = source.context("--source is required")?;
    ensure!(
        source.symlink_metadata()?.file_type().is_file(),
        "source must be a regular file, not a symlink"
    );
    ensure!(
        source.metadata()?.len() <= 1024 * 1024 * 1024,
        "probe source is limited to 1 GiB"
    );
    let destination_dir = if let Some(parent) = output_parent {
        tempfile::tempdir_in(parent)?
    } else {
        tempfile::tempdir()?
    };
    let destination = destination_dir.path().join("output");
    let initial_checksum = checksum(&source)?;
    let mut baseline = Vec::new();
    let mut sync_copy = Vec::new();
    let mut background = Vec::new();
    let mut heartbeats = Vec::new();
    let mut observations = 0;
    for _ in 0..iterations {
        let start = Instant::now();
        let count = std::fs::copy(&source, &destination)?;
        baseline.push(start.elapsed().as_secs_f64() * 1000.0);
        ensure!(
            count == initial_checksum.0 && checksum(&destination)? == initial_checksum,
            "std copy content differs"
        );
        std::fs::remove_file(&destination)?;

        let start = Instant::now();
        let count = copy_file(
            &source,
            &destination,
            chunk,
            &CancelToken::default(),
            |_| {},
        )?;
        sync_copy.push(start.elapsed().as_secs_f64() * 1000.0);
        ensure!(
            count == initial_checksum.0 && checksum(&destination)? == initial_checksum,
            "stream copy content differs"
        );
        std::fs::remove_file(&destination)?;

        let trial = background_trial(&source, &destination, chunk, None)?;
        ensure!(
            trial.result? == initial_checksum.0 && checksum(&destination)? == initial_checksum,
            "background copy content differs"
        );
        background.push(trial.copy_ms);
        heartbeats.extend(trial.heartbeat_ms);
        observations += trial.progress_observations;
        std::fs::remove_file(&destination)?;
        ensure!(
            std::fs::read_dir(destination_dir.path())?.next().is_none(),
            "temporary file leaked"
        );
    }
    let mut cancellation = Vec::new();
    let mut cancelled_bytes = Vec::new();
    let mut completed_before_cancel = 0;
    let cancel_after = (8 * chunk as u64).min(initial_checksum.0 / 2);
    for _ in 0..cancel_iterations {
        let trial = background_trial(&source, &destination, chunk, Some(cancel_after))?;
        match trial.result {
            Err(CopyError::Cancelled { copied }) => {
                ensure!(trial.cancel_accepted, "cancellation was not requested");
                cancellation.push(trial.cancel_ms.context("missing cancellation timestamp")?);
                cancelled_bytes.push(copied);
                ensure!(!destination.exists(), "cancelled copy was published");
            }
            Ok(_) if !trial.cancel_accepted => {
                completed_before_cancel += 1;
                ensure!(
                    checksum(&destination)? == initial_checksum,
                    "completed race copy differs"
                );
                std::fs::remove_file(&destination)?;
            }
            other => bail!("unexpected cancellation result {other:?}"),
        }
        ensure!(
            std::fs::read_dir(destination_dir.path())?.next().is_none(),
            "cancellation leaked temporary files"
        );
    }
    ensure!(
        checksum(&source)? == initial_checksum,
        "caller-owned source changed"
    );
    let rates = json!({
        "std_logical_mib_per_second": median_effective_mib_per_second(initial_checksum.0, &baseline),
        "stream_sync_effective_mib_per_second": median_effective_mib_per_second(initial_checksum.0, &sync_copy),
        "stream_background_effective_mib_per_second": median_effective_mib_per_second(initial_checksum.0, &background),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "source_name": source.file_name(), "bytes": initial_checksum.0,
            "checksum": format!("{:016x}", initial_checksum.1),
            "checksum_method": "Rust DefaultHasher streaming checksum; noncryptographic, same-process content comparison; all reads outside timing",
            "chunk_bytes": chunk, "iterations": iterations,
            "std_fs_copy": distribution(baseline),
            "prototype_sync_including_file_sync_and_publish": distribution(sync_copy),
        "prototype_background_copy": distribution(background),
        "median_effective_rates": rates,
            "background_controller_heartbeat": distribution(heartbeats),
            "heartbeat_target_ms": 1, "progress_slots": 1, "progress_observations": observations,
            "cancel_requested_after_observed_bytes": cancel_after,
            "cancel_request_to_worker_return": distribution(cancellation),
            "cancelled_copied_bytes": cancelled_bytes, "completed_before_cancel": completed_before_cancel,
            "source_checksum_unchanged": true, "outputs_verified": true, "temporary_files_cleaned": true,
            "interpretation": "std::fs::copy may use APFS cloning and does not include explicit fsync, so its logical copy timing is not physical byte throughput. Streaming prototype includes fsync and no-clobber publication. Heartbeats measure a standalone controller thread, not actual TUI responsiveness. Synchronous handler blocking inferred from direct calls, not a TUI benchmark.",
            "limitations": "Single regular file only. No metadata preservation, directories, undo, resumable jobs, cross-device move rollback, atomic source snapshot or crash-recovery guarantees. Cancellation cannot interrupt an OS syscall and is rejected after the commit boundary. Parent-directory replacement attacks are outside this prototype."
        }))?
    );
    Ok(())
}
