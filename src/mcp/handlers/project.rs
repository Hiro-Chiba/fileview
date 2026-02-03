//! Project management handlers
//!
//! Implements run_build, run_test, run_lint, get_project_stats.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use super::{error_result, success_result, ToolCallResult};
use crate::mcp::security::validate_path;

/// Detected project type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
    Java,
    Unknown,
}

impl ProjectType {
    /// Detect project type from root directory
    pub fn detect(root: &Path) -> Self {
        if root.join("Cargo.toml").exists() {
            Self::Rust
        } else if root.join("package.json").exists() {
            Self::Node
        } else if root.join("pyproject.toml").exists()
            || root.join("setup.py").exists()
            || root.join("requirements.txt").exists()
        {
            Self::Python
        } else if root.join("go.mod").exists() {
            Self::Go
        } else if root.join("pom.xml").exists() || root.join("build.gradle").exists() {
            Self::Java
        } else {
            Self::Unknown
        }
    }

    /// Get default build command
    pub fn build_command(&self) -> Option<(&str, Vec<&str>)> {
        match self {
            Self::Rust => Some(("cargo", vec!["build"])),
            Self::Node => Some(("npm", vec!["run", "build"])),
            Self::Python => Some(("python", vec!["-m", "build"])),
            Self::Go => Some(("go", vec!["build", "./..."])),
            Self::Java => Some(("mvn", vec!["compile"])),
            Self::Unknown => None,
        }
    }

    /// Get default test command
    pub fn test_command(&self) -> Option<(&str, Vec<&str>)> {
        match self {
            Self::Rust => Some(("cargo", vec!["test"])),
            Self::Node => Some(("npm", vec!["test"])),
            Self::Python => Some(("pytest", vec![])),
            Self::Go => Some(("go", vec!["test", "./..."])),
            Self::Java => Some(("mvn", vec!["test"])),
            Self::Unknown => None,
        }
    }

    /// Get default lint command
    pub fn lint_command(&self) -> Option<(&str, Vec<&str>)> {
        match self {
            Self::Rust => Some(("cargo", vec!["clippy"])),
            Self::Node => Some(("npx", vec!["eslint", "."])),
            Self::Python => Some(("ruff", vec!["check", "."])),
            Self::Go => Some(("golangci-lint", vec!["run"])),
            Self::Java => Some(("mvn", vec!["checkstyle:check"])),
            Self::Unknown => None,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &str {
        match self {
            Self::Rust => "Rust (Cargo)",
            Self::Node => "Node.js (npm)",
            Self::Python => "Python",
            Self::Go => "Go",
            Self::Java => "Java (Maven)",
            Self::Unknown => "Unknown",
        }
    }
}

/// Allowed build commands (whitelist for security)
const ALLOWED_BUILD_COMMANDS: &[&str] = &[
    "cargo", "npm", "yarn", "pnpm", "make", "cmake", "go", "mvn", "gradle", "pip", "poetry",
];

/// Run project build
pub fn run_build(root: &Path, custom_command: Option<&str>) -> ToolCallResult {
    if let Some(custom) = custom_command {
        // Parse custom command with security validation
        let parts: Vec<&str> = custom.split_whitespace().collect();
        if parts.is_empty() {
            return error_result("Empty command");
        }

        let cmd = parts[0];

        // Security: Whitelist allowed commands to prevent command injection
        if !ALLOWED_BUILD_COMMANDS.contains(&cmd) {
            return error_result(&format!(
                "Command '{}' is not in the allowed list. Allowed: {}",
                cmd,
                ALLOWED_BUILD_COMMANDS.join(", ")
            ));
        }

        // Security: Reject arguments containing shell metacharacters
        const FORBIDDEN_CHARS: &[char] = &[
            ';', '&', '|', '$', '`', '(', ')', '{', '}', '<', '>', '\'', '"', '\\', '\n',
        ];
        for arg in &parts[1..] {
            if arg.contains(FORBIDDEN_CHARS) {
                return error_result("Arguments contain forbidden shell metacharacters");
            }
        }

        return run_command(root, cmd, &parts[1..], "Build");
    }

    // Auto-detect project type
    let project_type = ProjectType::detect(root);
    match project_type.build_command() {
        Some((cmd, args)) => run_command(root, cmd, &args, "Build"),
        None => error_result(&format!(
            "Could not detect project type. No Cargo.toml, package.json, or similar found.\nDetected: {}",
            project_type.display_name()
        )),
    }
}

/// Run project tests
pub fn run_test(root: &Path, path: Option<&str>, filter: Option<&str>) -> ToolCallResult {
    let project_type = ProjectType::detect(root);

    let (cmd, mut args) = match project_type.test_command() {
        Some((cmd, args)) => (cmd, args.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
        None => {
            return error_result(&format!(
                "Could not detect test framework for project type: {}",
                project_type.display_name()
            ));
        }
    };

    // Add path filter if specified
    if let Some(p) = path {
        match project_type {
            ProjectType::Rust => {
                // Cargo test with path
                args.push("--".to_string());
                args.push(p.to_string());
            }
            ProjectType::Node => {
                args.push("--".to_string());
                args.push(p.to_string());
            }
            ProjectType::Python => {
                args.push(p.to_string());
            }
            ProjectType::Go => {
                args.clear();
                args.push("test".to_string());
                args.push(format!("./{}", p));
            }
            _ => {}
        }
    }

    // Add name filter if specified
    if let Some(f) = filter {
        match project_type {
            ProjectType::Rust => {
                if !args.contains(&"--".to_string()) {
                    args.push("--".to_string());
                }
                args.push(f.to_string());
            }
            ProjectType::Python => {
                args.push("-k".to_string());
                args.push(f.to_string());
            }
            ProjectType::Go => {
                args.push("-run".to_string());
                args.push(f.to_string());
            }
            _ => {}
        }
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_command(root, cmd, &args_refs, "Test")
}

/// Run project linter
pub fn run_lint(root: &Path, path: Option<&str>, fix: bool) -> ToolCallResult {
    let project_type = ProjectType::detect(root);

    let (cmd, mut args) = match project_type.lint_command() {
        Some((cmd, args)) => (cmd, args.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
        None => {
            return error_result(&format!(
                "Could not detect linter for project type: {}",
                project_type.display_name()
            ));
        }
    };

    // Add fix flag if requested
    if fix {
        match project_type {
            ProjectType::Rust => {
                args.push("--fix".to_string());
            }
            ProjectType::Node => {
                args.push("--fix".to_string());
            }
            ProjectType::Python => {
                // ruff check --fix
                let fix_idx = args.iter().position(|a| a == "check");
                if let Some(idx) = fix_idx {
                    args.insert(idx + 1, "--fix".to_string());
                }
            }
            ProjectType::Go => {
                // golangci-lint run --fix
                args.push("--fix".to_string());
            }
            _ => {}
        }
    }

    // Add path filter if specified
    if let Some(p) = path {
        match validate_path(root, p) {
            Ok(_) => {
                // Replace "." with specific path
                if let Some(pos) = args.iter().position(|a| a == ".") {
                    args[pos] = p.to_string();
                } else {
                    args.push(p.to_string());
                }
            }
            Err(e) => return error_result(&e.to_string()),
        }
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_command(root, cmd, &args_refs, "Lint")
}

/// Get project statistics
pub fn get_project_stats(root: &Path, path: Option<&str>) -> ToolCallResult {
    let start_path = match path {
        Some(p) => match validate_path(root, p) {
            Ok(path) => path,
            Err(e) => return error_result(&e.to_string()),
        },
        None => root.to_path_buf(),
    };

    let project_type = ProjectType::detect(root);

    // Collect statistics (with depth limit for security)
    let mut stats = ProjectStats::default();
    collect_stats(&start_path, &mut stats, 0);

    let mut result = String::new();
    result.push_str(&format!(
        "Project Statistics for: {}\n",
        start_path.display()
    ));
    result.push_str(&format!(
        "Project Type: {}\n\n",
        project_type.display_name()
    ));

    result.push_str("Files by Type:\n");
    let mut sorted_types: Vec<_> = stats.files_by_type.iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(a.1));
    for (ext, count) in sorted_types.iter().take(15) {
        result.push_str(&format!("  .{}: {} files\n", ext, count));
    }
    if sorted_types.len() > 15 {
        result.push_str(&format!(
            "  ... and {} more types\n",
            sorted_types.len() - 15
        ));
    }

    result.push_str(&format!("\nTotal Files: {}\n", stats.total_files));
    result.push_str(&format!("Total Directories: {}\n", stats.total_dirs));
    result.push_str(&format!("Total Lines of Code: {}\n", stats.total_lines));
    result.push_str(&format!("Total Size: {}\n", format_size(stats.total_size)));

    if !stats.lines_by_type.is_empty() {
        result.push_str("\nLines by Language:\n");
        let mut sorted_lines: Vec<_> = stats.lines_by_type.iter().collect();
        sorted_lines.sort_by(|a, b| b.1.cmp(a.1));
        for (ext, lines) in sorted_lines.iter().take(10) {
            result.push_str(&format!("  .{}: {} lines\n", ext, lines));
        }
    }

    success_result(result)
}

/// Run a command and return formatted result
fn run_command(root: &Path, cmd: &str, args: &[&str], operation: &str) -> ToolCallResult {
    let output = Command::new(cmd).args(args).current_dir(root).output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);

            let mut result = String::new();
            result.push_str(&format!(
                "{} Command: {} {}\n",
                operation,
                cmd,
                args.join(" ")
            ));
            result.push_str(&format!("Exit Status: {}\n\n", o.status));

            if !stdout.is_empty() {
                result.push_str("Output:\n");
                // Limit output to prevent huge responses
                let lines: Vec<&str> = stdout.lines().take(100).collect();
                result.push_str(&lines.join("\n"));
                if stdout.lines().count() > 100 {
                    result.push_str(&format!(
                        "\n... ({} more lines)",
                        stdout.lines().count() - 100
                    ));
                }
                result.push('\n');
            }

            if !stderr.is_empty() {
                result.push_str("\nErrors/Warnings:\n");
                let lines: Vec<&str> = stderr.lines().take(50).collect();
                result.push_str(&lines.join("\n"));
                if stderr.lines().count() > 50 {
                    result.push_str(&format!(
                        "\n... ({} more lines)",
                        stderr.lines().count() - 50
                    ));
                }
            }

            if o.status.success() {
                success_result(result)
            } else {
                ToolCallResult {
                    content: vec![super::ToolContent::text(result)],
                    is_error: Some(true),
                }
            }
        }
        Err(e) => error_result(&format!("Failed to run {}: {}", cmd, e)),
    }
}

/// Project statistics
#[derive(Default)]
struct ProjectStats {
    total_files: usize,
    total_dirs: usize,
    total_lines: usize,
    total_size: u64,
    files_by_type: HashMap<String, usize>,
    lines_by_type: HashMap<String, usize>,
}

/// Maximum recursion depth for stats collection (security: prevent DoS)
const MAX_STATS_DEPTH: usize = 50;

/// Maximum file size to read for line counting (10 MB)
const MAX_FILE_SIZE_FOR_LINES: u64 = 10 * 1024 * 1024;

/// Collect statistics recursively with depth limit
fn collect_stats(path: &Path, stats: &mut ProjectStats, depth: usize) {
    // Security: Prevent unbounded recursion
    if depth > MAX_STATS_DEPTH {
        return;
    }

    // Security: Skip symlinks to prevent escape attacks
    if path.is_symlink() {
        return;
    }

    if path.is_dir() {
        // Skip common non-source directories
        let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if matches!(
            dir_name,
            "node_modules"
                | "target"
                | ".git"
                | "__pycache__"
                | "venv"
                | ".venv"
                | "dist"
                | "build"
                | ".next"
        ) {
            return;
        }

        stats.total_dirs += 1;

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                collect_stats(&entry.path(), stats, depth + 1);
            }
        }
    } else if path.is_file() {
        stats.total_files += 1;

        let file_size = path.metadata().map(|m| m.len()).unwrap_or(0);
        stats.total_size += file_size;

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("(none)")
            .to_lowercase();

        *stats.files_by_type.entry(ext.clone()).or_insert(0) += 1;

        // Count lines for source files (with size limit for security)
        if is_source_extension(&ext) && file_size <= MAX_FILE_SIZE_FOR_LINES {
            if let Ok(content) = std::fs::read_to_string(path) {
                let lines = content.lines().count();
                stats.total_lines += lines;
                *stats.lines_by_type.entry(ext).or_insert(0) += lines;
            }
        }
    }
}

/// Check if extension is a source code file
fn is_source_extension(ext: &str) -> bool {
    matches!(
        ext,
        "rs" | "py"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "go"
            | "java"
            | "kt"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "rb"
            | "php"
            | "swift"
            | "scala"
            | "cs"
            | "vue"
            | "svelte"
    )
}

/// Format byte size
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
