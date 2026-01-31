//! Pick mode tests for fv
//!
//! Tests for the --pick flag behavior and output formats.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn fv() -> Command {
    Command::cargo_bin("fv").unwrap()
}

// =============================================================================
// Pick Flag Acceptance
// =============================================================================

#[test]
fn pick_flag_is_accepted() {
    fv().args(["--help", "--pick"]).assert().success();
}

#[test]
fn pick_short_flag_is_accepted() {
    fv().args(["--help", "-p"]).assert().success();
}

#[test]
fn pick_flag_documented_in_help() {
    fv().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--pick"))
        .stdout(predicate::str::contains("-p"));
}

// =============================================================================
// Pick Mode with Format Options
// =============================================================================

#[test]
fn pick_with_format_lines() {
    let temp_dir = TempDir::new().unwrap();
    fv().args(["--help", "--pick", "--format", "lines"])
        .arg(temp_dir.path())
        .assert()
        .success();
}

#[test]
fn pick_with_format_null() {
    let temp_dir = TempDir::new().unwrap();
    fv().args(["--help", "--pick", "--format", "null"])
        .arg(temp_dir.path())
        .assert()
        .success();
}

#[test]
fn pick_with_format_json() {
    let temp_dir = TempDir::new().unwrap();
    fv().args(["--help", "--pick", "--format", "json"])
        .arg(temp_dir.path())
        .assert()
        .success();
}

// =============================================================================
// Pick Mode with on-select
// =============================================================================

#[test]
fn pick_with_on_select_command() {
    let temp_dir = TempDir::new().unwrap();
    fv().args(["--help", "--pick", "--on-select", "echo {}"])
        .arg(temp_dir.path())
        .assert()
        .success();
}

#[test]
fn on_select_documented_in_help() {
    fv().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--on-select"));
}

// =============================================================================
// Pick Mode Combinations
// =============================================================================

#[test]
fn pick_with_icons() {
    fv().args(["--help", "--pick", "--icons"])
        .assert()
        .success();
}

#[test]
fn pick_with_no_icons() {
    fv().args(["--help", "--pick", "--no-icons"])
        .assert()
        .success();
}

#[test]
fn pick_with_multiple_options() {
    let temp_dir = TempDir::new().unwrap();
    fv().args([
        "--help",
        "--pick",
        "--format",
        "json",
        "--icons",
        "--on-select",
        "echo {}",
    ])
    .arg(temp_dir.path())
    .assert()
    .success();
}
