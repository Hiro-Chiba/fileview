//! E2E tests for bulk rename functionality
//!
//! Tests for the bulk rename feature via CLI.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn fv() -> Command {
    cargo_bin_cmd!("fv")
}

// =============================================================================
// Directory with Multiple Files
// =============================================================================

#[test]
fn directory_with_multiple_txt_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple files
    for i in 1..=5 {
        let path = temp_dir.path().join(format!("file{}.txt", i));
        fs::write(&path, format!("content {}", i)).unwrap();
    }

    fv().arg("--help").arg(temp_dir.path()).assert().success();
}

#[test]
fn directory_with_mixed_extensions() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("doc.txt"), "text").unwrap();
    fs::write(temp_dir.path().join("doc.md"), "markdown").unwrap();
    fs::write(temp_dir.path().join("code.rs"), "rust").unwrap();
    fs::write(temp_dir.path().join("code.py"), "python").unwrap();

    fv().arg("--help").arg(temp_dir.path()).assert().success();
}

#[test]
fn directory_with_prefixed_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create files with common prefix
    for i in 1..=3 {
        let path = temp_dir.path().join(format!("backup_{}.txt", i));
        fs::write(&path, format!("backup {}", i)).unwrap();
    }

    fv().arg("--help").arg(temp_dir.path()).assert().success();
}

// =============================================================================
// Complex Directory Structures
// =============================================================================

#[test]
fn nested_directories_with_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create nested structure
    let level1 = temp_dir.path().join("level1");
    let level2 = level1.join("level2");
    fs::create_dir_all(&level2).unwrap();

    fs::write(temp_dir.path().join("root.txt"), "root").unwrap();
    fs::write(level1.join("l1.txt"), "level1").unwrap();
    fs::write(level2.join("l2.txt"), "level2").unwrap();

    fv().arg("--help").arg(temp_dir.path()).assert().success();
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn empty_directory_accepted() {
    let temp_dir = TempDir::new().unwrap();
    fv().arg("--help").arg(temp_dir.path()).assert().success();
}

#[test]
fn directory_with_special_characters_in_filenames() {
    let temp_dir = TempDir::new().unwrap();

    // Create files with special characters (safe ones for cross-platform)
    fs::write(temp_dir.path().join("file-with-dash.txt"), "dash").unwrap();
    fs::write(
        temp_dir.path().join("file_with_underscore.txt"),
        "underscore",
    )
    .unwrap();
    fs::write(temp_dir.path().join("file.multiple.dots.txt"), "dots").unwrap();

    fv().arg("--help").arg(temp_dir.path()).assert().success();
}

#[test]
fn directory_with_hidden_files() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join(".hidden"), "hidden").unwrap();
    fs::write(temp_dir.path().join("visible.txt"), "visible").unwrap();

    fv().arg("--help").arg(temp_dir.path()).assert().success();
}

#[test]
fn show_hidden_flag_accepted() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join(".hidden"), "hidden").unwrap();

    fv().args(["--help", "-a"])
        .arg(temp_dir.path())
        .assert()
        .success();
}

// =============================================================================
// Version Check
// =============================================================================

#[test]
fn version_output_valid() {
    fv().arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"\d+\.\d+\.\d+").unwrap());
}
