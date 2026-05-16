//! Tests for `--snapshot-create <name>` / `--snapshot-diff <name>` subcommands.
//!
//! Lets an AI session capture the working tree at a point in time and ask
//! "what changed since then" without leaning on git. Useful when the user
//! hasn't committed yet, or for tracking ad-hoc experiments.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn fv() -> Command {
    cargo_bin_cmd!("fv")
}

fn write_file(dir: &Path, rel: &str, contents: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

#[test]
fn snapshot_create_in_empty_dir_succeeds() {
    let tmp = TempDir::new().unwrap();
    fv().arg("--snapshot-create")
        .arg("alpha")
        .current_dir(tmp.path())
        .assert()
        .success();
    assert!(
        tmp.path().join(".fileview/snapshots/alpha.json").exists(),
        "snapshot file should be created"
    );
}

#[test]
fn snapshot_create_requires_a_name() {
    let tmp = TempDir::new().unwrap();
    fv().arg("--snapshot-create")
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires").or(predicate::str::contains("value")));
}

#[test]
fn snapshot_diff_with_no_changes_outputs_nothing() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "src/lib.rs", "fn lib() {}\n");
    fv().arg("--snapshot-create")
        .arg("base")
        .current_dir(tmp.path())
        .assert()
        .success();

    fv().arg("--snapshot-diff")
        .arg("base")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout("");
}

#[test]
fn snapshot_diff_shows_added_file() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "src/lib.rs", "fn lib() {}\n");
    fv().arg("--snapshot-create")
        .arg("base")
        .current_dir(tmp.path())
        .assert()
        .success();

    write_file(tmp.path(), "src/new.rs", "fn new() {}\n");

    fv().arg("--snapshot-diff")
        .arg("base")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("+ src/new.rs"));
}

#[test]
fn snapshot_diff_shows_removed_file() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "keep.txt", "stays\n");
    write_file(tmp.path(), "drop.txt", "will be deleted\n");
    fv().arg("--snapshot-create")
        .arg("base")
        .current_dir(tmp.path())
        .assert()
        .success();

    fs::remove_file(tmp.path().join("drop.txt")).unwrap();

    fv().arg("--snapshot-diff")
        .arg("base")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("- drop.txt"))
        .stdout(predicate::str::contains("+ ").not())
        .stdout(predicate::str::contains("M ").not());
}

#[test]
fn snapshot_diff_shows_modified_file() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "doc.md", "first version\n");
    fv().arg("--snapshot-create")
        .arg("base")
        .current_dir(tmp.path())
        .assert()
        .success();

    // Sleep briefly so mtime resolution catches the change on all platforms.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    write_file(tmp.path(), "doc.md", "second version with more content\n");

    fv().arg("--snapshot-diff")
        .arg("base")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("M doc.md"));
}

#[test]
fn snapshot_diff_skips_dotfiles_and_dot_dirs() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "src/lib.rs", "fn lib() {}\n");
    fv().arg("--snapshot-create")
        .arg("base")
        .current_dir(tmp.path())
        .assert()
        .success();

    write_file(tmp.path(), ".hidden", "x\n");
    write_file(tmp.path(), ".cache/index", "x\n");

    fv().arg("--snapshot-diff")
        .arg("base")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(".hidden").not())
        .stdout(predicate::str::contains(".cache").not());
}

#[test]
fn snapshot_diff_for_missing_snapshot_fails() {
    let tmp = TempDir::new().unwrap();
    fv().arg("--snapshot-diff")
        .arg("nonexistent")
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("nonexistent").or(predicate::str::contains("not found")));
}
