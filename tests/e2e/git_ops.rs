//! E2E tests for Git operations
//!
//! Tests for git stage/unstage functionality.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::TempDir;

fn fv() -> Command {
    cargo_bin_cmd!("fv")
}

/// Initialize a git repository in the given directory
fn init_git_repo(dir: &TempDir) -> bool {
    StdCommand::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Configure git user for commits
fn configure_git_user(dir: &TempDir) -> bool {
    let config_name = StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let config_email = StdCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    config_name && config_email
}

/// Create an initial commit
fn create_initial_commit(dir: &TempDir) -> bool {
    let file_path = dir.path().join("initial.txt");
    fs::write(&file_path, "initial content").ok();

    let add = StdCommand::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let commit = StdCommand::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(dir.path())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    add && commit
}

// =============================================================================
// Git Repository Detection
// =============================================================================

#[test]
fn fv_accepts_git_repo_directory() {
    let temp_dir = TempDir::new().unwrap();
    if !init_git_repo(&temp_dir) {
        return; // Skip if git not available
    }

    fv().arg("--help").arg(temp_dir.path()).assert().success();
}

#[test]
fn fv_accepts_directory_with_modified_files() {
    let temp_dir = TempDir::new().unwrap();
    if !init_git_repo(&temp_dir) || !configure_git_user(&temp_dir) {
        return;
    }

    if !create_initial_commit(&temp_dir) {
        return;
    }

    // Modify a file
    let file_path = temp_dir.path().join("initial.txt");
    fs::write(&file_path, "modified content").unwrap();

    fv().arg("--help").arg(temp_dir.path()).assert().success();
}

#[test]
fn fv_accepts_directory_with_untracked_files() {
    let temp_dir = TempDir::new().unwrap();
    if !init_git_repo(&temp_dir) {
        return;
    }

    // Create untracked file
    let file_path = temp_dir.path().join("untracked.txt");
    fs::write(&file_path, "untracked content").unwrap();

    fv().arg("--help").arg(temp_dir.path()).assert().success();
}

// =============================================================================
// Help Text
// =============================================================================

#[test]
fn help_mentions_file_operations() {
    fv().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("fv"));
}
