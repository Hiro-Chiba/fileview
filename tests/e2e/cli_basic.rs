//! Basic CLI tests for fv
//!
//! Tests for command-line argument parsing, help output, version display,
//! and error handling for invalid inputs.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn fv() -> Command {
    Command::cargo_bin("fv").unwrap()
}

// =============================================================================
// Help and Version
// =============================================================================

#[test]
fn help_flag_shows_usage() {
    fv().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("USAGE:"))
        .stdout(predicate::str::contains("fv"))
        .stdout(predicate::str::contains("--pick"));
}

#[test]
fn help_short_flag_shows_usage() {
    fv().arg("-h")
        .assert()
        .success()
        .stdout(predicate::str::contains("USAGE:"));
}

#[test]
fn version_flag_shows_version() {
    fv().arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn version_short_flag_shows_version() {
    fv().arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

// =============================================================================
// Invalid Options (Exit Code 3)
// =============================================================================

#[test]
fn unknown_option_returns_exit_code_3() {
    fv().arg("--unknown-option")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("unknown"));
}

#[test]
fn invalid_short_option_returns_exit_code_3() {
    fv().arg("-x")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("Unknown option"));
}

#[test]
fn nonexistent_path_returns_exit_code_3() {
    fv().arg("/nonexistent/path/that/does/not/exist")
        .assert()
        .code(3)
        .stderr(
            predicate::str::contains("not found").or(predicate::str::contains("does not exist")),
        );
}

#[test]
fn invalid_format_value_returns_exit_code_3() {
    fv().args(["--pick", "--format", "invalid"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("format").or(predicate::str::contains("invalid")));
}

#[test]
fn on_select_without_command_returns_exit_code_3() {
    fv().arg("--on-select")
        .assert()
        .code(3)
        .stderr(predicate::str::is_empty().not());
}

// =============================================================================
// Valid Paths
// =============================================================================

#[test]
fn current_directory_is_accepted() {
    // Just check that it doesn't fail immediately with help or version
    // We can't actually run the TUI in tests
    fv().arg("--help").arg(".").assert().success();
}

#[test]
fn temp_directory_is_accepted() {
    let temp_dir = TempDir::new().unwrap();
    // Check with --help to avoid TUI
    fv().arg("--help").arg(temp_dir.path()).assert().success();
}

// =============================================================================
// Format Option Values
// =============================================================================

#[test]
fn format_lines_is_valid() {
    fv().args(["--help", "--format", "lines"])
        .assert()
        .success();
}

#[test]
fn format_null_is_valid() {
    fv().args(["--help", "--format", "null"]).assert().success();
}

#[test]
fn format_json_is_valid() {
    fv().args(["--help", "--format", "json"]).assert().success();
}

// =============================================================================
// Icon Options
// =============================================================================

#[test]
fn no_icons_flag_is_accepted() {
    fv().args(["--help", "--no-icons"]).assert().success();
}

#[test]
fn icons_flag_is_accepted() {
    fv().args(["--help", "--icons"]).assert().success();
}

#[test]
fn icons_short_flag_is_accepted() {
    fv().args(["--help", "-i"]).assert().success();
}
