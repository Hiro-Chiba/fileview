use std::process::Command;

fn command(home: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fv"));
    command
        .arg("search")
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_CACHE_HOME", home.join("cache"));
    command
}

#[test]
fn search_uses_current_or_explicit_root_without_creating_state() {
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("first.rs"), "").unwrap();
    std::fs::write(root.path().join("text.txt"), "").unwrap();
    std::fs::write(other.path().join("other.rs"), "").unwrap();
    let output = command(home.path())
        .current_dir(root.path())
        .arg("first")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "first.rs");

    std::fs::write(root.path().join("second.rs"), "new").unwrap();
    let output = command(home.path())
        .current_dir(other.path())
        .arg("ext:rs type:file")
        .arg(root.path())
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let matches: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(matches.as_array().unwrap().len(), 2);
    assert_eq!(matches[0]["display"], "first.rs");
    assert_eq!(matches[1]["display"], "second.rs");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 3);
    assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
}

#[test]
fn search_obeys_visibility_ignore_rules_and_result_limit() {
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    for name in ["a.rs", "b.rs", ".hidden.rs", "ignored.rs"] {
        std::fs::write(root.path().join(name), "").unwrap();
    }
    std::fs::write(root.path().join(".ignore"), "ignored.rs\n").unwrap();
    for (flags, expected) in [
        (vec![], 2),
        (vec!["--hidden"], 3),
        (vec!["-a", "--limit", "1"], 1),
    ] {
        let output = command(home.path())
            .arg("ext:rs")
            .arg(root.path())
            .arg("--json")
            .args(flags)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let matches: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let matches = matches.as_array().unwrap();
        assert_eq!(matches.len(), expected);
        assert!(matches.iter().all(|entry| entry["display"] != "ignored.rs"));
    }
}

#[test]
fn search_rejects_invalid_queries_and_arguments_with_clear_errors() {
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    for (args, message) in [
        (vec![], "Expected QUERY"),
        (vec!["a", "b", "c"], "Expected QUERY"),
        (vec!["type:invalid"], "type:"),
        (vec!["", "--limit", "0"], "--limit"),
        (vec!["", "--limit", "1001"], "--limit"),
        (vec!["", "--limit"], "--limit"),
        (vec!["", "--limit", "abc"], "--limit"),
        (vec!["", "--unknown"], "Unknown search option"),
    ] {
        let output = command(home.path())
            .current_dir(root.path())
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(message),
            "{output:?}"
        );
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn search_help_and_option_terminator_are_unambiguous() {
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("--help.rs"), "").unwrap();
    // Former management verbs are ordinary search terms, not new subcommands.
    std::fs::write(root.path().join("save.rs"), "").unwrap();
    let output = command(home.path()).arg("--help").output().unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("fv search QUERY [PATH]"));
    assert!(!help.contains("fv workspace"));
    for (query, expected) in [("--help", "--help.rs"), ("save", "save.rs")] {
        let output = command(home.path())
            .current_dir(root.path())
            .args(["--json", "--", query])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let matches: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(matches[0]["display"], expected);
    }
}

#[cfg(unix)]
#[test]
fn json_search_reports_non_utf8_paths_without_panicking() {
    use std::os::unix::ffi::OsStringExt;
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let name = std::ffi::OsString::from_vec(vec![b'a', 0xff]);
    if std::fs::write(root.path().join(name), "").is_err() {
        return;
    }
    let output = command(home.path())
        .args(["", "--json"])
        .arg(root.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("non-UTF8"), "{stderr}");
    assert!(!stderr.contains("panicked"));
}

#[test]
fn content_search_returns_literal_unicode_line_offsets_with_existing_command() {
    if Command::new("rg").arg("--version").output().is_err() {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("sample.txt"), "first\r\n日本語 TODO.*\n").unwrap();
    std::fs::write(root.path().join(".hidden"), "TODO.*").unwrap();
    std::fs::write(root.path().join("ignored"), "TODO.*").unwrap();
    std::fs::write(root.path().join(".ignore"), "ignored\n").unwrap();
    let output = command(home.path())
        .arg("text:TODO.*")
        .arg(root.path())
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let hits: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(hits.as_array().unwrap().len(), 1);
    assert_eq!(hits[0]["line"], 2);
    assert_eq!(hits[0]["byte_offset"], 17);
    assert_eq!(hits[0]["text"], "TODO.*");
    let output = command(home.path())
        .arg("text:")
        .arg(root.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("1 to 4096"));
}

#[test]
fn ripgrep_is_optional_for_path_search_and_required_only_for_contents() {
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("sample.txt"), "needle").unwrap();
    let output = command(home.path())
        .env("PATH", home.path())
        .arg("sample")
        .arg(root.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let output = command(home.path())
        .env("PATH", home.path())
        .arg("text:needle")
        .arg(root.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires ripgrep"));
}
