//! Selection and clipboard action handlers
//!
//! Handles ToggleMark, ClearMarks, Copy, Cut, SelectAll, InvertSelection,
//! SelectGitChanged, SelectTestPair, SelectByExtension, SelectRecentCommit, SelectGitStaged

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::action::Clipboard;
use crate::core::AppState;
use crate::git::FileStatus;
use crate::handler::key::KeyAction;

use super::EntrySnapshot;

/// Common file extensions mapped to Ctrl+1..9
const EXTENSION_SLOTS: [&[&str]; 9] = [
    &["rs"],                   // Ctrl+1: Rust
    &["ts", "tsx"],            // Ctrl+2: TypeScript
    &["js", "jsx"],            // Ctrl+3: JavaScript
    &["py"],                   // Ctrl+4: Python
    &["go"],                   // Ctrl+5: Go
    &["java", "kt"],           // Ctrl+6: Java/Kotlin
    &["md", "mdx"],            // Ctrl+7: Markdown
    &["json", "yaml", "toml"], // Ctrl+8: Config files
    &["css", "scss", "sass"],  // Ctrl+9: Stylesheets
];

/// Handle selection and clipboard actions
pub fn handle(action: KeyAction, state: &mut AppState, focused_path: &Option<PathBuf>) {
    match action {
        KeyAction::ToggleMark => {
            if let Some(path) = focused_path {
                if state.selected_paths.contains(path) {
                    state.selected_paths.remove(path);
                } else {
                    state.selected_paths.insert(path.clone());
                }
            }
        }
        KeyAction::ClearMarks => {
            state.selected_paths.clear();
        }
        KeyAction::Copy => {
            let paths: Vec<PathBuf> = if state.selected_paths.is_empty() {
                focused_path.clone().into_iter().collect()
            } else {
                state.selected_paths.iter().cloned().collect()
            };
            if !paths.is_empty() {
                let mut clipboard = Clipboard::new();
                let count = paths.len();
                clipboard.copy(paths);
                state.clipboard = Some(clipboard);
                state.set_message(format!("Copied {} item(s)", count));
            }
        }
        KeyAction::Cut => {
            let paths: Vec<PathBuf> = if state.selected_paths.is_empty() {
                focused_path.clone().into_iter().collect()
            } else {
                state.selected_paths.iter().cloned().collect()
            };
            if !paths.is_empty() {
                let mut clipboard = Clipboard::new();
                let count = paths.len();
                clipboard.cut(paths);
                state.clipboard = Some(clipboard);
                state.set_message(format!("Cut {} item(s)", count));
            }
        }
        _ => {}
    }
}

/// Handle selection with entries context
pub fn handle_with_entries(action: KeyAction, state: &mut AppState, entries: &[EntrySnapshot]) {
    match action {
        KeyAction::SelectAll => {
            // Toggle: if all are selected, deselect all; otherwise select all
            let all_paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
            let all_selected = all_paths.iter().all(|p| state.selected_paths.contains(p));

            if all_selected {
                state.selected_paths.clear();
                state.set_message("Cleared all selections");
            } else {
                for path in all_paths {
                    state.selected_paths.insert(path);
                }
                state.set_message(format!("Selected {} item(s)", entries.len()));
            }
        }
        KeyAction::InvertSelection => {
            let all_paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
            let mut new_selection = std::collections::HashSet::new();

            for path in all_paths {
                if !state.selected_paths.contains(&path) {
                    new_selection.insert(path);
                }
            }

            state.selected_paths = new_selection;
            state.set_message(format!(
                "Inverted selection: {} item(s)",
                state.selected_paths.len()
            ));
        }
        _ => {}
    }
}

/// Select range of entries (for visual select mode)
pub fn select_range(
    state: &mut AppState,
    entries: &[EntrySnapshot],
    anchor: usize,
    current: usize,
) {
    let start = anchor.min(current);
    let end = anchor.max(current);

    // Clear previous selection and select the range
    state.selected_paths.clear();
    for i in start..=end {
        if let Some(entry) = entries.get(i) {
            state.selected_paths.insert(entry.path.clone());
        }
    }
}

/// Select all git changed files
pub fn select_git_changed(state: &mut AppState, entries: &[EntrySnapshot]) {
    let git_status = match &state.git_status {
        Some(status) => status,
        None => {
            state.set_message("Not in a git repository");
            return;
        }
    };

    let mut count = 0;
    for entry in entries {
        let status = git_status.get_status(&entry.path);
        if status != FileStatus::Clean && status != FileStatus::Ignored {
            state.selected_paths.insert(entry.path.clone());
            count += 1;
        }
    }

    if count > 0 {
        state.set_message(format!("Selected {} git changed file(s)", count));
    } else {
        state.set_message("No git changes in current view");
    }
}

/// Select test pair for the focused file
pub fn select_test_pair(state: &mut AppState, focused_path: &Option<PathBuf>) {
    let path = match focused_path {
        Some(p) => p,
        None => {
            state.set_message("No file focused");
            return;
        }
    };

    let test_files = find_test_pairs(path);

    if test_files.is_empty() {
        state.set_message("No test file found");
        return;
    }

    // Add the current file and its test pairs to selection
    state.selected_paths.insert(path.clone());
    let mut found_count = 0;
    for test_file in test_files {
        if test_file.exists() {
            state.selected_paths.insert(test_file);
            found_count += 1;
        }
    }

    if found_count > 0 {
        state.set_message(format!("Selected {} test pair(s)", found_count));
    } else {
        state.set_message("Test file not found");
        // Remove the source file from selection if no test found
        state.selected_paths.remove(path);
    }
}

/// Select files by extension index (Ctrl+1..9)
pub fn select_by_extension(state: &mut AppState, entries: &[EntrySnapshot], index: u8) {
    if !(1..=9).contains(&index) {
        state.set_message("Invalid extension slot");
        return;
    }

    let extensions = EXTENSION_SLOTS[(index - 1) as usize];

    let mut count = 0;
    for entry in entries {
        if let Some(ext) = entry.path.extension().and_then(|e| e.to_str()) {
            if extensions.contains(&ext) {
                state.selected_paths.insert(entry.path.clone());
                count += 1;
            }
        }
    }

    if count > 0 {
        let ext_list = extensions.join("/");
        state.set_message(format!("Selected {} .{} file(s)", count, ext_list));
    } else {
        let ext_list = extensions.join("/");
        state.set_message(format!("No .{} files in view", ext_list));
    }
}

/// Select files changed in the most recent git commit
pub fn select_recent_commit(state: &mut AppState, entries: &[EntrySnapshot]) {
    // Get files from the last commit using git
    let output = Command::new("git")
        .args(["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"])
        .current_dir(&state.root)
        .output();

    let changed_files: Vec<String> = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect(),
        _ => {
            state.set_message("Failed to get recent commit files");
            return;
        }
    };

    if changed_files.is_empty() {
        state.set_message("No files in recent commit");
        return;
    }

    let mut count = 0;
    for entry in entries {
        // Get relative path
        if let Ok(rel_path) = entry.path.strip_prefix(&state.root) {
            let rel_str = rel_path.display().to_string();
            if changed_files.contains(&rel_str) {
                state.selected_paths.insert(entry.path.clone());
                count += 1;
            }
        }
    }

    if count > 0 {
        state.set_message(format!("Selected {} file(s) from recent commit", count));
    } else {
        state.set_message("Recent commit files not in view");
    }
}

/// Select only git staged files
pub fn select_git_staged(state: &mut AppState, entries: &[EntrySnapshot]) {
    let git_status = match &state.git_status {
        Some(status) => status,
        None => {
            state.set_message("Not in a git repository");
            return;
        }
    };

    let mut count = 0;
    for entry in entries {
        if git_status.is_staged(&entry.path) {
            state.selected_paths.insert(entry.path.clone());
            count += 1;
        }
    }

    if count > 0 {
        state.set_message(format!("Selected {} staged file(s)", count));
    } else {
        state.set_message("No staged files in current view");
    }
}

/// Find potential test file paths for a given source file
fn find_test_pairs(path: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return candidates,
    };

    let stem = match path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => s,
        None => return candidates,
    };

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let parent = path.parent();

    // Detect if this is already a test file (reverse lookup)
    let is_test_file = stem.ends_with("_test")
        || stem.starts_with("test_")
        || stem.ends_with(".test")
        || stem.ends_with(".spec")
        || file_name.contains("_test.")
        || file_name.contains(".test.")
        || file_name.contains(".spec.");

    if is_test_file {
        // Find the source file from a test file
        let source_stem = stem
            .trim_end_matches("_test")
            .trim_start_matches("test_")
            .trim_end_matches(".test")
            .trim_end_matches(".spec");

        if let Some(p) = parent {
            // Same directory
            candidates.push(p.join(format!("{}.{}", source_stem, ext)));

            // Parent directory (for tests/ directory structure)
            if let Some(grandparent) = p.parent() {
                if p.file_name().map(|n| n.to_str()) == Some(Some("tests")) {
                    candidates.push(grandparent.join(format!("{}.{}", source_stem, ext)));
                }
            }
        }
    } else {
        // Find test files from a source file
        if let Some(p) = parent {
            match ext {
                // Rust patterns
                "rs" => {
                    candidates.push(p.join(format!("{}_test.rs", stem)));
                    candidates.push(p.join(format!("test_{}.rs", stem)));
                    candidates.push(p.join("tests").join(format!("{}.rs", stem)));
                    candidates.push(p.join("tests").join(format!("{}_test.rs", stem)));
                }
                // TypeScript/JavaScript patterns
                "ts" | "tsx" | "js" | "jsx" => {
                    candidates.push(p.join(format!("{}.test.{}", stem, ext)));
                    candidates.push(p.join(format!("{}.spec.{}", stem, ext)));
                    candidates.push(p.join("__tests__").join(format!("{}.{}", stem, ext)));
                    candidates.push(p.join("__tests__").join(format!("{}.test.{}", stem, ext)));
                }
                // Python patterns
                "py" => {
                    candidates.push(p.join(format!("test_{}.py", stem)));
                    candidates.push(p.join(format!("{}_test.py", stem)));
                    candidates.push(p.join("tests").join(format!("test_{}.py", stem)));
                    candidates.push(p.join("tests").join(format!("{}_test.py", stem)));
                }
                // Go patterns
                "go" => {
                    candidates.push(p.join(format!("{}_test.go", stem)));
                }
                // Java/Kotlin patterns
                "java" | "kt" => {
                    candidates.push(p.join(format!("{}Test.{}", stem, ext)));
                    // Also check in test source tree
                    let test_path = path
                        .to_string_lossy()
                        .replace("/main/", "/test/")
                        .replace("\\main\\", "\\test\\");
                    if test_path != path.to_string_lossy() {
                        candidates.push(PathBuf::from(test_path));
                    }
                }
                // Default: try common patterns
                _ => {
                    candidates.push(p.join(format!("{}_test.{}", stem, ext)));
                    candidates.push(p.join(format!("test_{}.{}", stem, ext)));
                    candidates.push(p.join(format!("{}.test.{}", stem, ext)));
                }
            }
        }
    }

    candidates
}
