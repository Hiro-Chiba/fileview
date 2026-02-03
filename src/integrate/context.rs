//! Context generation for AI tools
//!
//! Outputs project context in a format optimized for AI assistants.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use crate::git::GitStatus;

/// Output project context to stdout
///
/// # Arguments
/// * `root` - Root directory path
pub fn output_context(root: &Path) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Project name (directory name)
    let project_name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    writeln!(handle, "## Project: {}", project_name)?;
    writeln!(handle)?;

    // Git information
    if let Some(git_status) = GitStatus::detect(root) {
        let branch = git_status.branch().unwrap_or("unknown");
        let is_clean = is_working_tree_clean(root);
        let clean_str = if is_clean { "(clean)" } else { "(dirty)" };
        writeln!(handle, "**Branch:** {} {}", branch, clean_str)?;

        // Recent commit
        if let Some(recent_commit) = get_recent_commit(root) {
            writeln!(handle, "**Recent:** {}", recent_commit)?;
        }

        writeln!(handle)?;
    }

    // Project structure
    writeln!(handle, "### Structure:")?;
    print_structure(&mut handle, root, "", 0, 3)?;
    writeln!(handle)?;

    // File statistics
    let stats = collect_file_stats(root);
    if !stats.is_empty() {
        let total_files: usize = stats.values().map(|(count, _)| count).sum();
        let total_lines: usize = stats.values().map(|(_, lines)| lines).sum();

        // Format file types
        let mut type_parts: Vec<String> = stats
            .iter()
            .filter(|(_, (count, _))| *count > 0)
            .map(|(ext, (count, _))| format!("{} {} files", count, ext))
            .collect();
        type_parts.sort();

        let type_str = if type_parts.len() > 5 {
            format!("{}, ...", type_parts[..5].join(", "))
        } else {
            type_parts.join(", ")
        };

        writeln!(
            handle,
            "### Stats: {} files, ~{}k lines",
            total_files,
            total_lines / 1000
        )?;
        writeln!(handle, "Types: {}", type_str)?;
    }

    handle.flush()
}

/// Check if working tree is clean
fn is_working_tree_clean(root: &Path) -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .map(|o| o.status.success() && o.stdout.is_empty())
        .unwrap_or(false)
}

/// Get recent commit message with relative time
fn get_recent_commit(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%s (%cr)"])
        .current_dir(root)
        .output()
        .ok()?;

    if output.status.success() {
        let msg = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !msg.is_empty() {
            return Some(msg);
        }
    }
    None
}

/// Print simplified project structure
fn print_structure<W: Write>(
    out: &mut W,
    path: &Path,
    prefix: &str,
    depth: usize,
    max_depth: usize,
) -> io::Result<()> {
    if depth >= max_depth {
        return Ok(());
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    // Collect and filter entries
    let mut dirs: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            // Skip hidden and common non-essential directories
            !name_str.starts_with('.')
                && !matches!(
                    name_str.as_ref(),
                    "node_modules"
                        | "target"
                        | "dist"
                        | "build"
                        | "__pycache__"
                        | "venv"
                        | ".venv"
                        | "vendor"
                )
        })
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();

    // Sort alphabetically
    dirs.sort_by_key(|a| a.file_name());

    // Limit to significant directories
    if dirs.len() > 10 {
        dirs.truncate(10);
    }

    for (i, entry) in dirs.iter().enumerate() {
        let is_last = i == dirs.len() - 1;
        let name = entry.file_name();
        let connector = if is_last { "└── " } else { "├── " };

        writeln!(out, "{}{}{}/", prefix, connector, name.to_string_lossy())?;

        let new_prefix = if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}│   ", prefix)
        };

        print_structure(out, &entry.path(), &new_prefix, depth + 1, max_depth)?;
    }

    // Show key files at root level
    if depth == 0 {
        let key_files = ["main.rs", "lib.rs", "mod.rs", "index.ts", "app.ts", "main.py"];
        let src_path = path.join("src");
        if src_path.exists() {
            for file in &key_files {
                if src_path.join(file).exists() {
                    writeln!(out, "{}└── {}", prefix, file)?;
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Collect file statistics by extension
fn collect_file_stats(root: &Path) -> HashMap<String, (usize, usize)> {
    let mut stats: HashMap<String, (usize, usize)> = HashMap::new();

    collect_stats_recursive(root, &mut stats, 0, 5);

    stats
}

fn collect_stats_recursive(
    path: &Path,
    stats: &mut HashMap<String, (usize, usize)>,
    depth: usize,
    max_depth: usize,
) {
    if depth >= max_depth {
        return;
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip hidden and vendor directories
        if name_str.starts_with('.') {
            continue;
        }
        if matches!(
            name_str.as_ref(),
            "node_modules" | "target" | "dist" | "build" | "__pycache__" | "venv" | "vendor"
        ) {
            continue;
        }

        let entry_path = entry.path();
        if entry_path.is_dir() {
            collect_stats_recursive(&entry_path, stats, depth + 1, max_depth);
        } else if entry_path.is_file() {
            if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                let ext = normalize_extension(ext);
                let line_count = count_lines(&entry_path).unwrap_or(0);

                let entry = stats.entry(ext).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += line_count;
            }
        }
    }
}

/// Normalize file extension for display
fn normalize_extension(ext: &str) -> String {
    match ext.to_lowercase().as_str() {
        "rs" => "Rust".to_string(),
        "ts" | "tsx" => "TypeScript".to_string(),
        "js" | "jsx" => "JavaScript".to_string(),
        "py" => "Python".to_string(),
        "go" => "Go".to_string(),
        "java" => "Java".to_string(),
        "kt" => "Kotlin".to_string(),
        "rb" => "Ruby".to_string(),
        "c" | "h" => "C".to_string(),
        "cpp" | "hpp" | "cc" | "cxx" => "C++".to_string(),
        "cs" => "C#".to_string(),
        "swift" => "Swift".to_string(),
        "md" => "Markdown".to_string(),
        "json" => "JSON".to_string(),
        "yaml" | "yml" => "YAML".to_string(),
        "toml" => "TOML".to_string(),
        other => other.to_uppercase(),
    }
}

/// Count lines in a file
fn count_lines(path: &Path) -> io::Result<usize> {
    let content = fs::read_to_string(path)?;
    Ok(content.lines().count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_project() -> TempDir {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("src")).unwrap();
        fs::create_dir(temp.path().join("src/app")).unwrap();
        fs::write(temp.path().join("src/main.rs"), "fn main() {\n    println!(\"Hello\");\n}").unwrap();
        fs::write(temp.path().join("src/lib.rs"), "pub mod app;").unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        temp
    }

    #[test]
    fn test_normalize_extension() {
        assert_eq!(normalize_extension("rs"), "Rust");
        assert_eq!(normalize_extension("ts"), "TypeScript");
        assert_eq!(normalize_extension("py"), "Python");
        assert_eq!(normalize_extension("unknown"), "UNKNOWN");
    }

    #[test]
    fn test_count_lines() {
        let temp = setup_test_project();
        let lines = count_lines(&temp.path().join("src/main.rs")).unwrap();
        assert_eq!(lines, 3);
    }

    #[test]
    fn test_collect_file_stats() {
        let temp = setup_test_project();
        let stats = collect_file_stats(temp.path());

        assert!(stats.contains_key("Rust"));
        assert!(stats.contains_key("TOML"));
    }

    #[test]
    fn test_print_structure() {
        let temp = setup_test_project();
        let mut output = Vec::new();
        print_structure(&mut output, temp.path(), "", 0, 3).unwrap();
        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("src/"));
    }
}
