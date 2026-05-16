//! Tests for `--watch <path> [--watch-timeout-secs N]` subcommand.
//!
//! Blocks until the path is modified externally, then prints the changed
//! path and exits. With a timeout, exits with the CANCELLED code when
//! nothing happens. Lets AI sessions detect stale reads before applying
//! patches.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn fv() -> Command {
    cargo_bin_cmd!("fv")
}

#[test]
fn watch_with_short_timeout_and_no_change_exits_cancelled() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("idle.txt");
    fs::write(&file, "stable\n").unwrap();

    // Give macOS FSEvents a moment to settle so the watch does not pick
    // up the file-creation event itself as a "change".
    std::thread::sleep(std::time::Duration::from_millis(500));

    fv().arg("--watch")
        .arg(&file)
        .arg("--watch-timeout-secs")
        .arg("1")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("timeout").or(predicate::str::contains("no change")));
}

#[test]
fn watch_requires_a_path() {
    fv().arg("--watch")
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires").or(predicate::str::contains("value")));
}

#[test]
fn watch_missing_file_returns_runtime_error() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("nope.txt");
    fv().arg("--watch")
        .arg(&missing)
        .arg("--watch-timeout-secs")
        .arg("1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("nope.txt").or(predicate::str::contains("not found")));
}

#[test]
fn watch_timeout_secs_requires_a_value() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("x.txt");
    fs::write(&file, "x").unwrap();
    fv().arg("--watch")
        .arg(&file)
        .arg("--watch-timeout-secs")
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires").or(predicate::str::contains("value")));
}
