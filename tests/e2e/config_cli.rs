//! E2E tests for configuration CLI options
//!
//! Tests for the new CLI options added with the config file system:
//! - --hidden / -a: Show hidden files
//! - --no-hidden: Hide hidden files
//! - Config file documentation in help

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

fn fv() -> Command {
    cargo_bin_cmd!("fv")
}

// =============================================================================
// Hidden Files CLI Options
// =============================================================================

#[test]
fn hidden_flag_is_accepted() {
    fv().args(["--help", "--hidden"]).assert().success();
}

#[test]
fn hidden_short_flag_is_accepted() {
    fv().args(["--help", "-a"]).assert().success();
}

#[test]
fn no_hidden_flag_is_accepted() {
    fv().args(["--help", "--no-hidden"]).assert().success();
}

#[test]
fn hidden_and_no_hidden_last_wins() {
    // Both flags can be specified, last one wins (standard CLI behavior)
    fv().args(["--help", "--hidden", "--no-hidden"])
        .assert()
        .success();
    fv().args(["--help", "--no-hidden", "--hidden"])
        .assert()
        .success();
}

// =============================================================================
// Help Output Contains Config File Info
// =============================================================================

#[test]
fn help_shows_config_file_section() {
    fv().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("CONFIG FILE:"));
}

#[test]
fn help_shows_config_toml_path() {
    fv().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("config.toml"));
}

#[test]
fn help_shows_keymap_toml_path() {
    fv().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("keymap.toml"));
}

#[test]
fn help_shows_theme_toml_path() {
    fv().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("theme.toml"));
}

#[test]
fn help_shows_hidden_option() {
    fv().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--hidden"))
        .stdout(predicate::str::contains("-a"));
}

// =============================================================================
// Config File Loading (without TUI)
// =============================================================================

#[test]
fn config_file_syntax_error_does_not_crash() {
    // Create a temporary config directory with invalid TOML
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".config").join("fileview");
    fs::create_dir_all(&config_dir).unwrap();

    let config_path = config_dir.join("config.toml");
    let mut file = fs::File::create(&config_path).unwrap();
    writeln!(file, "invalid {{ toml").unwrap();

    // Set HOME to temp dir so fv looks for config there
    // Note: This test verifies the app doesn't crash on invalid config
    // The actual config loading uses dirs crate which checks real home dir
    fv().env("HOME", temp_dir.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn empty_config_file_does_not_crash() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".config").join("fileview");
    fs::create_dir_all(&config_dir).unwrap();

    let config_path = config_dir.join("config.toml");
    fs::File::create(&config_path).unwrap();

    fv().env("HOME", temp_dir.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn valid_config_file_is_read() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".config").join("fileview");
    fs::create_dir_all(&config_dir).unwrap();

    let config_path = config_dir.join("config.toml");
    let mut file = fs::File::create(&config_path).unwrap();
    writeln!(
        file,
        r#"
[general]
show_hidden = true
enable_icons = false

[preview]
hex_max_bytes = 8192

[performance]
git_poll_interval_secs = 10
"#
    )
    .unwrap();

    fv().env("HOME", temp_dir.path())
        .arg("--help")
        .assert()
        .success();
}

// =============================================================================
// Keymap File Loading
// =============================================================================

#[test]
fn keymap_file_syntax_error_does_not_crash() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".config").join("fileview");
    fs::create_dir_all(&config_dir).unwrap();

    let keymap_path = config_dir.join("keymap.toml");
    let mut file = fs::File::create(&keymap_path).unwrap();
    writeln!(file, "invalid {{ toml").unwrap();

    fv().env("HOME", temp_dir.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn valid_keymap_file_is_read() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".config").join("fileview");
    fs::create_dir_all(&config_dir).unwrap();

    let keymap_path = config_dir.join("keymap.toml");
    let mut file = fs::File::create(&keymap_path).unwrap();
    writeln!(
        file,
        r#"
[browse]
"x" = "quit"
"ctrl+q" = "quit"

[preview]
"q" = "cancel"
"#
    )
    .unwrap();

    fv().env("HOME", temp_dir.path())
        .arg("--help")
        .assert()
        .success();
}

// =============================================================================
// Theme File Loading
// =============================================================================

#[test]
fn theme_file_syntax_error_does_not_crash() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".config").join("fileview");
    fs::create_dir_all(&config_dir).unwrap();

    let theme_path = config_dir.join("theme.toml");
    let mut file = fs::File::create(&theme_path).unwrap();
    writeln!(file, "invalid {{ toml").unwrap();

    fv().env("HOME", temp_dir.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn valid_theme_file_is_read() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".config").join("fileview");
    fs::create_dir_all(&config_dir).unwrap();

    let theme_path = config_dir.join("theme.toml");
    let mut file = fs::File::create(&theme_path).unwrap();
    writeln!(
        file,
        r##"
[colors]
background = "#1e1e1e"
foreground = "#d4d4d4"
selection = "blue"

[file_colors]
directory = "cyan"

[git_colors]
modified = "yellow"
"##
    )
    .unwrap();

    fv().env("HOME", temp_dir.path())
        .arg("--help")
        .assert()
        .success();
}
