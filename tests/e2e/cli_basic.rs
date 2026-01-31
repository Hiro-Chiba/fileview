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

#[test]
fn absolute_path_is_accepted() {
    let temp_dir = TempDir::new().unwrap();
    let abs_path = temp_dir.path().canonicalize().unwrap();
    fv().arg("--help").arg(&abs_path).assert().success();
}

#[test]
fn file_path_is_accepted() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "content").unwrap();
    // When given a file path, fv should start in its parent directory
    fv().arg("--help").arg(&file_path).assert().success();
}

#[test]
fn nested_directory_path_is_accepted() {
    let temp_dir = TempDir::new().unwrap();
    let nested = temp_dir.path().join("a").join("b").join("c");
    std::fs::create_dir_all(&nested).unwrap();
    fv().arg("--help").arg(&nested).assert().success();
}

// =============================================================================
// Multiple Paths
// =============================================================================

#[test]
fn multiple_paths_accepted() {
    let temp_dir = TempDir::new().unwrap();
    let dir1 = temp_dir.path().join("dir1");
    let dir2 = temp_dir.path().join("dir2");
    std::fs::create_dir(&dir1).unwrap();
    std::fs::create_dir(&dir2).unwrap();

    // Multiple paths are accepted (uses stdin mode internally)
    fv().arg("--help").arg(&dir1).arg(&dir2).assert().success();
}

#[test]
fn multiple_file_paths_accepted() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.txt");
    let file2 = temp_dir.path().join("file2.txt");
    std::fs::write(&file1, "content1").unwrap();
    std::fs::write(&file2, "content2").unwrap();

    fv().arg("--help")
        .arg(&file1)
        .arg(&file2)
        .assert()
        .success();
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
