//! MCP tool handlers
//!
//! Implements the actual functionality for each MCP tool.

use std::fs;
use std::path::Path;
use std::process::Command;

use regex::Regex;

use super::types::{ToolCallResult, ToolContent};
use crate::git::{get_diff, DiffLine, GitStatus};

/// List directory contents
pub fn list_directory(root: &Path, path: Option<&str>) -> ToolCallResult {
    let target = match path {
        Some(p) => {
            let target = root.join(p);
            // Security: ensure path is within root
            match target.canonicalize() {
                Ok(canonical) => {
                    if !canonical.starts_with(root) {
                        return error_result("Path is outside root directory");
                    }
                    canonical
                }
                Err(e) => return error_result(&format!("Invalid path: {}", e)),
            }
        }
        None => root.to_path_buf(),
    };

    match fs::read_dir(&target) {
        Ok(entries) => {
            let mut items: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| {
                    let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    let name = e.file_name().to_string_lossy().to_string();
                    if is_dir {
                        format!("{}/", name)
                    } else {
                        name
                    }
                })
                .collect();
            items.sort();

            ToolCallResult {
                content: vec![ToolContent::text(items.join("\n"))],
                is_error: None,
            }
        }
        Err(e) => error_result(&format!("Failed to read directory: {}", e)),
    }
}

/// Get directory tree
pub fn get_tree(root: &Path, path: Option<&str>, depth: Option<usize>) -> ToolCallResult {
    let target = match path {
        Some(p) => {
            let target = root.join(p);
            match target.canonicalize() {
                Ok(canonical) => {
                    if !canonical.starts_with(root) {
                        return error_result("Path is outside root directory");
                    }
                    canonical
                }
                Err(e) => return error_result(&format!("Invalid path: {}", e)),
            }
        }
        None => root.to_path_buf(),
    };

    let mut output = Vec::new();
    if let Err(e) = write_tree(&mut output, &target, depth) {
        return error_result(&format!("Failed to generate tree: {}", e));
    }

    ToolCallResult {
        content: vec![ToolContent::text(
            String::from_utf8_lossy(&output).to_string(),
        )],
        is_error: None,
    }
}

/// Write tree to output
fn write_tree<W: std::io::Write>(
    out: &mut W,
    path: &Path,
    depth: Option<usize>,
) -> std::io::Result<()> {
    writeln!(out, "{}", path.display())?;
    crate::integrate::tree::print_tree_recursive_pub(out, path, "", depth, 0, false)
}

/// Read file content
pub fn read_file(root: &Path, path: &str) -> ToolCallResult {
    let target = root.join(path);

    // Security: ensure path is within root
    match target.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(root) {
                return error_result("Path is outside root directory");
            }

            if canonical.is_dir() {
                return error_result("Path is a directory, not a file");
            }

            match fs::read_to_string(&canonical) {
                Ok(content) => ToolCallResult {
                    content: vec![ToolContent::text(content)],
                    is_error: None,
                },
                Err(e) => error_result(&format!("Failed to read file: {}", e)),
            }
        }
        Err(e) => error_result(&format!("Invalid path: {}", e)),
    }
}

/// Create error result
fn error_result(message: &str) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent::text(message.to_string())],
        is_error: Some(true),
    }
}

/// Get git status for the repository
///
/// Returns a list of changed/staged files with their status.
pub fn get_git_status(root: &Path) -> ToolCallResult {
    let git_status = match GitStatus::detect(root) {
        Some(status) => status,
        None => return error_result("Not a git repository"),
    };

    let mut output = String::new();

    // Add branch info
    if let Some(branch) = git_status.branch() {
        output.push_str(&format!("Branch: {}\n\n", branch));
    }

    // Get status output using git command
    let git_output = Command::new("git")
        .args(["status", "--porcelain=v1", "-uall"])
        .current_dir(root)
        .output();

    match git_output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.trim().is_empty() {
                output.push_str("Working tree clean");
            } else {
                output.push_str("Changes:\n");
                for line in stdout.lines() {
                    if line.len() >= 3 {
                        let index_status = line.chars().next().unwrap_or(' ');
                        let worktree_status = line.chars().nth(1).unwrap_or(' ');
                        let path = &line[3..];

                        let status_str = match (index_status, worktree_status) {
                            // Conflicts (check first due to overlap)
                            ('U', _) | (_, 'U') => "conflict",
                            ('M', _) | (_, 'M') => "modified",
                            ('A', _) => "added",
                            ('D', _) | (_, 'D') => "deleted",
                            ('R', _) => "renamed",
                            ('?', '?') => "untracked",
                            ('!', _) => "ignored",
                            _ => "unknown",
                        };

                        let staged = matches!(index_status, 'M' | 'A' | 'D' | 'R' | 'C');
                        let staged_marker = if staged { " [staged]" } else { "" };

                        output.push_str(&format!("  {} {}{}\n", status_str, path, staged_marker));
                    }
                }
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            return error_result(&format!("git status failed: {}", stderr));
        }
        Err(e) => {
            return error_result(&format!("Failed to run git: {}", e));
        }
    }

    ToolCallResult {
        content: vec![ToolContent::text(output)],
        is_error: None,
    }
}

/// Get git diff for a file
///
/// # Arguments
/// * `root` - The repository root directory
/// * `path` - Relative path to the file
/// * `staged` - If true, show staged changes (--cached)
pub fn get_git_diff(root: &Path, path: &str, staged: bool) -> ToolCallResult {
    let target = root.join(path);

    // Security: ensure path is within root
    match target.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(root) {
                return error_result("Path is outside root directory");
            }

            match get_diff(root, &canonical, staged) {
                Some(diff) => {
                    let mut output = String::new();
                    output.push_str(&format!("File: {}\n", path));
                    output.push_str(&format!(
                        "Changes: +{} -{}\n\n",
                        diff.additions, diff.deletions
                    ));

                    for line in &diff.lines {
                        match line {
                            DiffLine::Added(content) => {
                                output.push_str(&format!("+{}\n", content));
                            }
                            DiffLine::Removed(content) => {
                                output.push_str(&format!("-{}\n", content));
                            }
                            DiffLine::Context(content) => {
                                output.push_str(&format!(" {}\n", content));
                            }
                            DiffLine::HunkHeader(header) => {
                                output.push_str(&format!("{}\n", header));
                            }
                            DiffLine::Other(other) => {
                                output.push_str(&format!("{}\n", other));
                            }
                        }
                    }

                    ToolCallResult {
                        content: vec![ToolContent::text(output)],
                        is_error: None,
                    }
                }
                None => ToolCallResult {
                    content: vec![ToolContent::text(format!(
                        "No {} changes for: {}",
                        if staged { "staged" } else { "unstaged" },
                        path
                    ))],
                    is_error: None,
                },
            }
        }
        Err(_) => {
            // File might not exist yet (new file), try getting diff anyway
            match get_diff(root, &target, staged) {
                Some(diff) => {
                    let mut output = String::new();
                    output.push_str(&format!("File: {}\n", path));
                    output.push_str(&format!(
                        "Changes: +{} -{}\n\n",
                        diff.additions, diff.deletions
                    ));

                    for line in &diff.lines {
                        match line {
                            DiffLine::Added(content) => output.push_str(&format!("+{}\n", content)),
                            DiffLine::Removed(content) => {
                                output.push_str(&format!("-{}\n", content))
                            }
                            DiffLine::Context(content) => {
                                output.push_str(&format!(" {}\n", content))
                            }
                            DiffLine::HunkHeader(header) => {
                                output.push_str(&format!("{}\n", header))
                            }
                            DiffLine::Other(other) => output.push_str(&format!("{}\n", other)),
                        }
                    }

                    ToolCallResult {
                        content: vec![ToolContent::text(output)],
                        is_error: None,
                    }
                }
                None => error_result(&format!("File not found or no changes: {}", path)),
            }
        }
    }
}

/// Write content to a file
///
/// # Arguments
/// * `root` - The repository root directory
/// * `path` - Relative path to the file
/// * `content` - Content to write
/// * `create_dirs` - If true, create parent directories if they don't exist
pub fn write_file(root: &Path, path: &str, content: &str, create_dirs: bool) -> ToolCallResult {
    let target = root.join(path);

    // Security: ensure path is within root
    // For new files, we need to check the parent directory
    let parent = target.parent().unwrap_or(root);
    if !parent.starts_with(root) && parent != root {
        // Check if resolved parent would be outside root
        if let Ok(canonical_parent) = parent.canonicalize() {
            if !canonical_parent.starts_with(root) {
                return error_result("Path is outside root directory");
            }
        }
    }

    // Create parent directories if requested
    if create_dirs {
        if let Some(parent) = target.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return error_result(&format!("Failed to create directories: {}", e));
            }
        }
    }

    // Write the file
    match fs::write(&target, content) {
        Ok(_) => {
            let bytes = content.len();
            ToolCallResult {
                content: vec![ToolContent::text(format!(
                    "Successfully wrote {} bytes to {}",
                    bytes, path
                ))],
                is_error: None,
            }
        }
        Err(e) => error_result(&format!("Failed to write file: {}", e)),
    }
}

/// Delete a file or directory
///
/// # Arguments
/// * `root` - The repository root directory
/// * `path` - Relative path to the file or directory
/// * `recursive` - If true, delete directories recursively
/// * `use_trash` - If true, move to trash instead of permanent deletion
pub fn delete_file(root: &Path, path: &str, recursive: bool, use_trash: bool) -> ToolCallResult {
    let target = root.join(path);

    // Security: ensure path is within root
    match target.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(root) {
                return error_result("Path is outside root directory");
            }

            // Don't allow deleting the root itself
            if canonical == root.canonicalize().unwrap_or_else(|_| root.to_path_buf()) {
                return error_result("Cannot delete root directory");
            }

            let is_dir = canonical.is_dir();

            if use_trash {
                // Use trash crate for safe deletion
                match trash::delete(&canonical) {
                    Ok(_) => ToolCallResult {
                        content: vec![ToolContent::text(format!("Moved to trash: {}", path))],
                        is_error: None,
                    },
                    Err(e) => error_result(&format!("Failed to move to trash: {}", e)),
                }
            } else if is_dir {
                if recursive {
                    match fs::remove_dir_all(&canonical) {
                        Ok(_) => ToolCallResult {
                            content: vec![ToolContent::text(format!(
                                "Deleted directory: {}",
                                path
                            ))],
                            is_error: None,
                        },
                        Err(e) => error_result(&format!("Failed to delete directory: {}", e)),
                    }
                } else {
                    match fs::remove_dir(&canonical) {
                        Ok(_) => ToolCallResult {
                            content: vec![ToolContent::text(format!(
                                "Deleted empty directory: {}",
                                path
                            ))],
                            is_error: None,
                        },
                        Err(e) => {
                            error_result(&format!("Failed to delete directory (not empty?): {}", e))
                        }
                    }
                }
            } else {
                match fs::remove_file(&canonical) {
                    Ok(_) => ToolCallResult {
                        content: vec![ToolContent::text(format!("Deleted file: {}", path))],
                        is_error: None,
                    },
                    Err(e) => error_result(&format!("Failed to delete file: {}", e)),
                }
            }
        }
        Err(e) => error_result(&format!("Invalid path: {}", e)),
    }
}

/// Read multiple files at once
///
/// # Arguments
/// * `root` - The repository root directory
/// * `paths` - List of relative paths to read
pub fn read_files(root: &Path, paths: &[&str]) -> ToolCallResult {
    let mut results = Vec::new();
    let mut success_count = 0;
    let mut error_count = 0;

    for path in paths {
        let target = root.join(path);

        match target.canonicalize() {
            Ok(canonical) => {
                if !canonical.starts_with(root) {
                    results.push(format!(
                        "--- {} ---\nError: Path is outside root directory\n",
                        path
                    ));
                    error_count += 1;
                    continue;
                }

                if canonical.is_dir() {
                    results.push(format!("--- {} ---\nError: Path is a directory\n", path));
                    error_count += 1;
                    continue;
                }

                match fs::read_to_string(&canonical) {
                    Ok(content) => {
                        results.push(format!("--- {} ---\n{}\n", path, content));
                        success_count += 1;
                    }
                    Err(e) => {
                        results.push(format!("--- {} ---\nError: {}\n", path, e));
                        error_count += 1;
                    }
                }
            }
            Err(e) => {
                results.push(format!("--- {} ---\nError: {}\n", path, e));
                error_count += 1;
            }
        }
    }

    let summary = format!(
        "Read {} file(s) successfully, {} error(s)\n\n",
        success_count, error_count
    );

    ToolCallResult {
        content: vec![ToolContent::text(format!(
            "{}{}",
            summary,
            results.join("\n")
        ))],
        is_error: if error_count > 0 && success_count == 0 {
            Some(true)
        } else {
            None
        },
    }
}

/// Get git commit log
///
/// # Arguments
/// * `root` - The repository root directory
/// * `limit` - Maximum number of commits to return
/// * `path` - Optional path to filter commits
pub fn git_log(root: &Path, limit: Option<usize>, path: Option<&str>) -> ToolCallResult {
    let limit = limit.unwrap_or(10);

    let mut args = vec![
        "log".to_string(),
        format!("-{}", limit),
        "--pretty=format:%h|%an|%ar|%s".to_string(),
    ];

    if let Some(p) = path {
        args.push("--".to_string());
        args.push(p.to_string());
    }

    let output = Command::new("git").args(&args).current_dir(root).output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.trim().is_empty() {
                return ToolCallResult {
                    content: vec![ToolContent::text("No commits found".to_string())],
                    is_error: None,
                };
            }

            let mut result = String::new();
            result.push_str("Commit History:\n\n");

            for line in stdout.lines() {
                let parts: Vec<&str> = line.splitn(4, '|').collect();
                if parts.len() == 4 {
                    result.push_str(&format!(
                        "{} - {} ({}) - {}\n",
                        parts[0], parts[1], parts[2], parts[3]
                    ));
                }
            }

            ToolCallResult {
                content: vec![ToolContent::text(result)],
                is_error: None,
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            error_result(&format!("git log failed: {}", stderr))
        }
        Err(e) => error_result(&format!("Failed to run git: {}", e)),
    }
}

/// Stage files for git commit
///
/// # Arguments
/// * `root` - The repository root directory
/// * `paths` - List of paths to stage (empty for all)
pub fn stage_files(root: &Path, paths: &[&str]) -> ToolCallResult {
    let args: Vec<&str> = if paths.is_empty() {
        vec!["add", "-A"]
    } else {
        let mut a = vec!["add"];
        a.extend(paths);
        a
    };

    let output = Command::new("git").args(&args).current_dir(root).output();

    match output {
        Ok(o) if o.status.success() => {
            let count = if paths.is_empty() {
                "all changes".to_string()
            } else {
                format!("{} file(s)", paths.len())
            };
            ToolCallResult {
                content: vec![ToolContent::text(format!("Staged {}", count))],
                is_error: None,
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            error_result(&format!("git add failed: {}", stderr))
        }
        Err(e) => error_result(&format!("Failed to run git: {}", e)),
    }
}

/// Create a git commit
///
/// # Arguments
/// * `root` - The repository root directory
/// * `message` - Commit message
pub fn create_commit(root: &Path, message: &str) -> ToolCallResult {
    if message.trim().is_empty() {
        return error_result("Commit message cannot be empty");
    }

    let output = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(root)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            // Extract commit hash from output
            let commit_info = stdout.lines().next().unwrap_or("Commit created");
            ToolCallResult {
                content: vec![ToolContent::text(commit_info.to_string())],
                is_error: None,
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            let stdout = String::from_utf8_lossy(&o.stdout);
            // Git might report "nothing to commit" on stdout
            if stdout.contains("nothing to commit") || stderr.contains("nothing to commit") {
                error_result("Nothing to commit (no staged changes)")
            } else {
                error_result(&format!("git commit failed: {}", stderr))
            }
        }
        Err(e) => error_result(&format!("Failed to run git: {}", e)),
    }
}

/// Search code in the repository
///
/// # Arguments
/// * `root` - The repository root directory
/// * `pattern` - The search pattern (regex)
/// * `path` - Optional path to limit search scope
pub fn search_code(root: &Path, pattern: &str, path: Option<&str>) -> ToolCallResult {
    // Try ripgrep first, fall back to grep
    let (cmd, args) = if Command::new("rg").arg("--version").output().is_ok() {
        ("rg", vec!["-n", "--no-heading", pattern])
    } else {
        ("grep", vec!["-rn", pattern])
    };

    let search_path = match path {
        Some(p) => {
            let target = root.join(p);
            // Security: ensure path is within root
            match target.canonicalize() {
                Ok(canonical) => {
                    if !canonical.starts_with(root) {
                        return error_result("Path is outside root directory");
                    }
                    canonical
                }
                Err(e) => return error_result(&format!("Invalid path: {}", e)),
            }
        }
        None => root.to_path_buf(),
    };

    let output = Command::new(cmd)
        .args(&args)
        .arg(&search_path)
        .current_dir(root)
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);

            if !o.status.success() && stdout.is_empty() {
                if stderr.contains("No such file") || stderr.contains("not found") {
                    return error_result(&format!("Search error: {}", stderr));
                }
                // No matches found is not an error
                return ToolCallResult {
                    content: vec![ToolContent::text(format!(
                        "No matches found for pattern: {}",
                        pattern
                    ))],
                    is_error: None,
                };
            }

            // Limit output to first 100 matches to avoid overwhelming the response
            let lines: Vec<&str> = stdout.lines().take(100).collect();
            let total_matches = stdout.lines().count();

            let mut result = String::new();
            result.push_str(&format!(
                "Search results for '{}' ({} matches):\n\n",
                pattern,
                total_matches.min(100)
            ));

            for line in &lines {
                result.push_str(line);
                result.push('\n');
            }

            if total_matches > 100 {
                result.push_str(&format!(
                    "\n... and {} more matches (showing first 100)",
                    total_matches - 100
                ));
            }

            ToolCallResult {
                content: vec![ToolContent::text(result)],
                is_error: None,
            }
        }
        Err(e) => error_result(&format!("Failed to run search: {}", e)),
    }
}

/// Code symbol type
#[derive(Debug)]
enum SymbolKind {
    Function,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Const,
    Type,
    Module,
}

impl SymbolKind {
    fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Class => "class",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Interface => "interface",
            SymbolKind::Trait => "trait",
            SymbolKind::Const => "const",
            SymbolKind::Type => "type",
            SymbolKind::Module => "module",
        }
    }
}

/// Extracted code symbol
#[derive(Debug)]
struct CodeSymbol {
    kind: SymbolKind,
    name: String,
    line: usize,
}

/// Get file symbols (functions, classes, etc.)
///
/// # Arguments
/// * `root` - The repository root directory
/// * `path` - Relative path to the file
pub fn get_file_symbols(root: &Path, path: &str) -> ToolCallResult {
    let target = root.join(path);

    // Security: ensure path is within root
    match target.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(root) {
                return error_result("Path is outside root directory");
            }

            if canonical.is_dir() {
                return error_result("Path is a directory, not a file");
            }

            let content = match fs::read_to_string(&canonical) {
                Ok(c) => c,
                Err(e) => return error_result(&format!("Failed to read file: {}", e)),
            };

            // Detect language from extension
            let ext = canonical.extension().and_then(|e| e.to_str()).unwrap_or("");

            let symbols = match ext {
                "rs" => extract_rust_symbols(&content),
                "py" => extract_python_symbols(&content),
                "ts" | "tsx" | "js" | "jsx" => extract_typescript_symbols(&content),
                "go" => extract_go_symbols(&content),
                "java" | "kt" => extract_java_symbols(&content),
                _ => extract_generic_symbols(&content),
            };

            if symbols.is_empty() {
                return ToolCallResult {
                    content: vec![ToolContent::text(format!("No symbols found in {}", path))],
                    is_error: None,
                };
            }

            let mut result = String::new();
            result.push_str(&format!(
                "Symbols in {} ({} found):\n\n",
                path,
                symbols.len()
            ));

            for symbol in symbols {
                result.push_str(&format!(
                    "  L{}: {} {}\n",
                    symbol.line,
                    symbol.kind.as_str(),
                    symbol.name
                ));
            }

            ToolCallResult {
                content: vec![ToolContent::text(result)],
                is_error: None,
            }
        }
        Err(e) => error_result(&format!("Invalid path: {}", e)),
    }
}

/// Extract symbols from Rust code
fn extract_rust_symbols(content: &str) -> Vec<CodeSymbol> {
    let mut symbols = Vec::new();

    let patterns = [
        (r"^\s*(?:pub\s+)?fn\s+(\w+)", SymbolKind::Function),
        (r"^\s*(?:pub\s+)?struct\s+(\w+)", SymbolKind::Struct),
        (r"^\s*(?:pub\s+)?enum\s+(\w+)", SymbolKind::Enum),
        (r"^\s*(?:pub\s+)?trait\s+(\w+)", SymbolKind::Trait),
        (r"^\s*(?:pub\s+)?type\s+(\w+)", SymbolKind::Type),
        (r"^\s*(?:pub\s+)?const\s+(\w+)", SymbolKind::Const),
        (r"^\s*(?:pub\s+)?mod\s+(\w+)", SymbolKind::Module),
        (
            r"^\s*impl(?:<[^>]*>)?\s+(?:\w+\s+for\s+)?(\w+)",
            SymbolKind::Struct,
        ),
    ];

    for (line_num, line) in content.lines().enumerate() {
        for (pattern, kind) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(line) {
                    if let Some(name) = caps.get(1) {
                        symbols.push(CodeSymbol {
                            kind: match kind {
                                SymbolKind::Function => SymbolKind::Function,
                                SymbolKind::Struct => SymbolKind::Struct,
                                SymbolKind::Enum => SymbolKind::Enum,
                                SymbolKind::Trait => SymbolKind::Trait,
                                SymbolKind::Type => SymbolKind::Type,
                                SymbolKind::Const => SymbolKind::Const,
                                SymbolKind::Module => SymbolKind::Module,
                                _ => SymbolKind::Function,
                            },
                            name: name.as_str().to_string(),
                            line: line_num + 1,
                        });
                        break;
                    }
                }
            }
        }
    }

    symbols
}

/// Extract symbols from Python code
fn extract_python_symbols(content: &str) -> Vec<CodeSymbol> {
    let mut symbols = Vec::new();

    let patterns = [
        (r"^def\s+(\w+)", SymbolKind::Function),
        (r"^class\s+(\w+)", SymbolKind::Class),
        (r"^\s{4}def\s+(\w+)", SymbolKind::Function), // Method
    ];

    for (line_num, line) in content.lines().enumerate() {
        for (pattern, kind) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(line) {
                    if let Some(name) = caps.get(1) {
                        symbols.push(CodeSymbol {
                            kind: match kind {
                                SymbolKind::Function => SymbolKind::Function,
                                SymbolKind::Class => SymbolKind::Class,
                                _ => SymbolKind::Function,
                            },
                            name: name.as_str().to_string(),
                            line: line_num + 1,
                        });
                        break;
                    }
                }
            }
        }
    }

    symbols
}

/// Extract symbols from TypeScript/JavaScript code
fn extract_typescript_symbols(content: &str) -> Vec<CodeSymbol> {
    let mut symbols = Vec::new();

    let patterns = [
        (r"^\s*(?:export\s+)?function\s+(\w+)", SymbolKind::Function),
        (r"^\s*(?:export\s+)?class\s+(\w+)", SymbolKind::Class),
        (
            r"^\s*(?:export\s+)?interface\s+(\w+)",
            SymbolKind::Interface,
        ),
        (r"^\s*(?:export\s+)?type\s+(\w+)", SymbolKind::Type),
        (r"^\s*(?:export\s+)?enum\s+(\w+)", SymbolKind::Enum),
        (
            r"^\s*(?:export\s+)?const\s+(\w+)\s*=\s*(?:async\s+)?\(",
            SymbolKind::Function,
        ), // Arrow function
        (
            r"^\s*(?:export\s+)?const\s+(\w+)\s*=\s*function",
            SymbolKind::Function,
        ),
    ];

    for (line_num, line) in content.lines().enumerate() {
        for (pattern, kind) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(line) {
                    if let Some(name) = caps.get(1) {
                        symbols.push(CodeSymbol {
                            kind: match kind {
                                SymbolKind::Function => SymbolKind::Function,
                                SymbolKind::Class => SymbolKind::Class,
                                SymbolKind::Interface => SymbolKind::Interface,
                                SymbolKind::Type => SymbolKind::Type,
                                SymbolKind::Enum => SymbolKind::Enum,
                                _ => SymbolKind::Function,
                            },
                            name: name.as_str().to_string(),
                            line: line_num + 1,
                        });
                        break;
                    }
                }
            }
        }
    }

    symbols
}

/// Extract symbols from Go code
fn extract_go_symbols(content: &str) -> Vec<CodeSymbol> {
    let mut symbols = Vec::new();

    let patterns = [
        (r"^func\s+(?:\([^)]+\)\s+)?(\w+)", SymbolKind::Function),
        (r"^type\s+(\w+)\s+struct", SymbolKind::Struct),
        (r"^type\s+(\w+)\s+interface", SymbolKind::Interface),
        (r"^const\s+(\w+)", SymbolKind::Const),
    ];

    for (line_num, line) in content.lines().enumerate() {
        for (pattern, kind) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(line) {
                    if let Some(name) = caps.get(1) {
                        symbols.push(CodeSymbol {
                            kind: match kind {
                                SymbolKind::Function => SymbolKind::Function,
                                SymbolKind::Struct => SymbolKind::Struct,
                                SymbolKind::Interface => SymbolKind::Interface,
                                SymbolKind::Const => SymbolKind::Const,
                                _ => SymbolKind::Function,
                            },
                            name: name.as_str().to_string(),
                            line: line_num + 1,
                        });
                        break;
                    }
                }
            }
        }
    }

    symbols
}

/// Extract symbols from Java/Kotlin code
fn extract_java_symbols(content: &str) -> Vec<CodeSymbol> {
    let mut symbols = Vec::new();

    let patterns = [
        (
            r"^\s*(?:public|private|protected)?\s*(?:static)?\s*(?:final)?\s*(?:\w+\s+)?(\w+)\s*\([^)]*\)\s*\{",
            SymbolKind::Function,
        ),
        (
            r"^\s*(?:public|private|protected)?\s*class\s+(\w+)",
            SymbolKind::Class,
        ),
        (
            r"^\s*(?:public|private|protected)?\s*interface\s+(\w+)",
            SymbolKind::Interface,
        ),
        (
            r"^\s*(?:public|private|protected)?\s*enum\s+(\w+)",
            SymbolKind::Enum,
        ),
    ];

    for (line_num, line) in content.lines().enumerate() {
        for (pattern, kind) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(line) {
                    if let Some(name) = caps.get(1) {
                        symbols.push(CodeSymbol {
                            kind: match kind {
                                SymbolKind::Function => SymbolKind::Function,
                                SymbolKind::Class => SymbolKind::Class,
                                SymbolKind::Interface => SymbolKind::Interface,
                                SymbolKind::Enum => SymbolKind::Enum,
                                _ => SymbolKind::Function,
                            },
                            name: name.as_str().to_string(),
                            line: line_num + 1,
                        });
                        break;
                    }
                }
            }
        }
    }

    symbols
}

/// Extract symbols from generic code (fallback)
fn extract_generic_symbols(content: &str) -> Vec<CodeSymbol> {
    let mut symbols = Vec::new();

    // Try common patterns
    let patterns = [
        (
            r"^\s*(?:function|def|fn|func)\s+(\w+)",
            SymbolKind::Function,
        ),
        (r"^\s*class\s+(\w+)", SymbolKind::Class),
    ];

    for (line_num, line) in content.lines().enumerate() {
        for (pattern, kind) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(line) {
                    if let Some(name) = caps.get(1) {
                        symbols.push(CodeSymbol {
                            kind: match kind {
                                SymbolKind::Function => SymbolKind::Function,
                                SymbolKind::Class => SymbolKind::Class,
                                _ => SymbolKind::Function,
                            },
                            name: name.as_str().to_string(),
                            line: line_num + 1,
                        });
                        break;
                    }
                }
            }
        }
    }

    symbols
}
