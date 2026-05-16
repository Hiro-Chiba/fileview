//! Tests for `--tokens <path>` non-interactive subcommand.
//!
//! Prints the estimated cl100k_base token count for the given file to stdout
//! so AI agents and shell scripts can budget context without round-tripping
//! through the TUI or the MCP server.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn fv() -> Command {
    cargo_bin_cmd!("fv")
}

#[test]
fn tokens_flag_prints_count_for_existing_file() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("hello.txt");
    fs::write(&file, "hello world\n").unwrap();

    fv().arg("--tokens")
        .arg(&file)
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^\d+\n$").unwrap());
}

#[test]
fn tokens_flag_emits_positive_count_for_nonempty_file() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("doc.md");
    fs::write(
        &file,
        "# Hello\n\nThis is a paragraph with enough text to tokenize.\n",
    )
    .unwrap();

    let output = fv()
        .arg("--tokens")
        .arg(&file)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    let count: usize = text
        .trim()
        .parse()
        .expect("stdout should be a positive integer");
    assert!(count > 0, "expected positive token count, got {}", count);
}

#[test]
fn tokens_flag_returns_zero_for_empty_file() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("empty.txt");
    fs::write(&file, "").unwrap();

    fv().arg("--tokens")
        .arg(&file)
        .assert()
        .success()
        .stdout("0\n");
}

#[test]
fn tokens_flag_fails_for_missing_file() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("does-not-exist.txt");

    fv().arg("--tokens")
        .arg(&missing)
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("does-not-exist.txt")
                .or(predicate::str::contains("not found")),
        );
}

#[test]
fn tokens_flag_requires_a_path() {
    fv().arg("--tokens")
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires").or(predicate::str::contains("value")));
}
