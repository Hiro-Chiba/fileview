//! stdin mode tests for fv
//!
//! Tests for the --stdin flag behavior.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;

fn fv() -> Command {
    cargo_bin_cmd!("fv")
}

// =============================================================================
// stdin Mode Error Cases
// =============================================================================

#[test]
fn stdin_flag_without_pipe_returns_error() {
    // When --stdin is used without actual piped input (TTY), it should error
    fv().arg("--stdin")
        .assert()
        .failure()
        .stderr(predicate::str::contains("stdin").or(predicate::str::contains("input")));
}

#[test]
fn stdin_with_empty_input_returns_error() {
    fv().arg("--stdin")
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No valid paths provided"));
}

#[test]
fn stdin_help_is_documented() {
    fv().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("stdin"));
}

// =============================================================================
// stdin with valid input (basic check)
// =============================================================================

#[test]
fn stdin_with_nonexistent_paths_returns_error() {
    fv().arg("--stdin")
        .write_stdin("/nonexistent/path/1\n/nonexistent/path/2\n")
        .assert()
        .failure();
}
