//! Code analysis handlers
//!
//! Implements get_file_symbols, get_definitions, get_references, get_diagnostics.

use std::fs;
use std::path::Path;
use std::process::Command;

use regex::Regex;

use super::{error_result, success_result, ToolCallResult};
use crate::mcp::security::validate_path;

/// Code symbol type
#[derive(Debug, Clone, Copy)]
pub enum SymbolKind {
    Function,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Const,
    Type,
    Module,
    Variable,
}

impl SymbolKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Const => "const",
            Self::Type => "type",
            Self::Module => "module",
            Self::Variable => "variable",
        }
    }
}

/// Extracted code symbol
#[derive(Debug)]
pub struct CodeSymbol {
    pub kind: SymbolKind,
    pub name: String,
    pub line: usize,
    pub signature: Option<String>,
}

/// Get file symbols (functions, classes, etc.)
pub fn get_file_symbols(root: &Path, path: &str) -> ToolCallResult {
    let canonical = match validate_path(root, path) {
        Ok(p) => p,
        Err(e) => return error_result(&e.to_string()),
    };

    if canonical.is_dir() {
        return error_result("Path is a directory, not a file");
    }

    let content = match fs::read_to_string(&canonical) {
        Ok(c) => c,
        Err(e) => return error_result(&format!("Failed to read file: {}", e)),
    };

    let ext = canonical.extension().and_then(|e| e.to_str()).unwrap_or("");
    let symbols = extract_symbols(&content, ext);

    if symbols.is_empty() {
        return success_result(format!("No symbols found in {}", path));
    }

    let mut result = String::new();
    result.push_str(&format!(
        "Symbols in {} ({} found):\n\n",
        path,
        symbols.len()
    ));

    for symbol in symbols {
        if let Some(sig) = &symbol.signature {
            result.push_str(&format!(
                "  L{}: {} {} - {}\n",
                symbol.line,
                symbol.kind.as_str(),
                symbol.name,
                sig
            ));
        } else {
            result.push_str(&format!(
                "  L{}: {} {}\n",
                symbol.line,
                symbol.kind.as_str(),
                symbol.name
            ));
        }
    }

    success_result(result)
}

/// Get definitions from a file
pub fn get_definitions(
    root: &Path,
    path: &str,
    line: Option<u32>,
    _column: Option<u32>,
) -> ToolCallResult {
    let canonical = match validate_path(root, path) {
        Ok(p) => p,
        Err(e) => return error_result(&e.to_string()),
    };

    if canonical.is_dir() {
        return error_result("Path is a directory, not a file");
    }

    let content = match fs::read_to_string(&canonical) {
        Ok(c) => c,
        Err(e) => return error_result(&format!("Failed to read file: {}", e)),
    };

    let ext = canonical.extension().and_then(|e| e.to_str()).unwrap_or("");
    let all_symbols = extract_symbols(&content, ext);

    // If a specific line is requested, filter to symbols at or near that line
    let symbols: Vec<_> = if let Some(target_line) = line {
        all_symbols
            .into_iter()
            .filter(|s| {
                let diff = (s.line as i64 - target_line as i64).abs();
                diff <= 3 // Within 3 lines
            })
            .collect()
    } else {
        // Return all definitions (functions, classes, etc.)
        all_symbols
            .into_iter()
            .filter(|s| {
                matches!(
                    s.kind,
                    SymbolKind::Function
                        | SymbolKind::Class
                        | SymbolKind::Struct
                        | SymbolKind::Trait
                        | SymbolKind::Interface
                )
            })
            .collect()
    };

    if symbols.is_empty() {
        return success_result(format!("No definitions found in {}", path));
    }

    let mut result = String::new();
    result.push_str(&format!("Definitions in {}:\n\n", path));

    for symbol in symbols {
        // Get the line content for context
        if let Some(line_content) = content.lines().nth(symbol.line.saturating_sub(1)) {
            result.push_str(&format!(
                "L{}: {} {}\n",
                symbol.line,
                symbol.kind.as_str(),
                symbol.name
            ));
            result.push_str(&format!("  {}\n\n", line_content.trim()));
        }
    }

    success_result(result)
}

/// Find references to a symbol
pub fn get_references(root: &Path, path: &str, symbol: &str) -> ToolCallResult {
    // Validate the starting file
    let canonical = match validate_path(root, path) {
        Ok(p) => p,
        Err(e) => return error_result(&e.to_string()),
    };

    if canonical.is_dir() {
        return error_result("Path is a directory, not a file");
    }

    // Use ripgrep or grep to find references
    let (cmd, args) = if Command::new("rg").arg("--version").output().is_ok() {
        ("rg", vec!["-n", "--no-heading", "-w", symbol])
    } else {
        ("grep", vec!["-rn", "-w", symbol])
    };

    let output = Command::new(cmd).args(&args).current_dir(root).output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);

            if stdout.is_empty() || !o.status.success() {
                return success_result(format!("No references found for '{}'", symbol));
            }

            let lines: Vec<&str> = stdout.lines().take(50).collect();
            let total = stdout.lines().count();

            let mut result = String::new();
            result.push_str(&format!(
                "References to '{}' ({} found):\n\n",
                symbol,
                total.min(50)
            ));

            for line in lines {
                result.push_str(line);
                result.push('\n');
            }

            if total > 50 {
                result.push_str(&format!("\n... and {} more references", total - 50));
            }

            success_result(result)
        }
        Err(e) => error_result(&format!("Failed to search for references: {}", e)),
    }
}

/// Get diagnostics (errors/warnings) for a file
///
/// This is a best-effort implementation that tries to use language-specific tools.
pub fn get_diagnostics(root: &Path, path: &str) -> ToolCallResult {
    let canonical = match validate_path(root, path) {
        Ok(p) => p,
        Err(e) => return error_result(&e.to_string()),
    };

    if canonical.is_dir() {
        return error_result("Path is a directory, not a file");
    }

    let ext = canonical.extension().and_then(|e| e.to_str()).unwrap_or("");

    // Try language-specific diagnostic tools
    let output = match ext {
        "rs" => {
            // Rust: use cargo check
            Command::new("cargo")
                .args(["check", "--message-format=short"])
                .current_dir(root)
                .output()
        }
        "py" => {
            // Python: try ruff, then flake8, then pylint
            if Command::new("ruff").arg("--version").output().is_ok() {
                Command::new("ruff")
                    .args(["check", path])
                    .current_dir(root)
                    .output()
            } else if Command::new("flake8").arg("--version").output().is_ok() {
                Command::new("flake8").arg(path).current_dir(root).output()
            } else {
                return success_result(
                    "No Python linter found (install ruff or flake8)".to_string(),
                );
            }
        }
        "ts" | "tsx" => {
            // TypeScript: use tsc
            Command::new("npx")
                .args(["tsc", "--noEmit", path])
                .current_dir(root)
                .output()
        }
        "js" | "jsx" => {
            // JavaScript: try eslint
            if Command::new("npx")
                .arg("eslint")
                .arg("--version")
                .output()
                .is_ok()
            {
                Command::new("npx")
                    .args(["eslint", "--format", "compact", path])
                    .current_dir(root)
                    .output()
            } else {
                return success_result("ESLint not found".to_string());
            }
        }
        _ => {
            return success_result(format!("No diagnostic tool available for .{} files", ext));
        }
    };

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);

            let combined = if !stdout.is_empty() {
                stdout.to_string()
            } else {
                stderr.to_string()
            };

            if combined.trim().is_empty() {
                success_result(format!("No diagnostics for {}", path))
            } else {
                let mut result = String::new();
                result.push_str(&format!("Diagnostics for {}:\n\n", path));
                result.push_str(&combined);
                success_result(result)
            }
        }
        Err(e) => error_result(&format!("Failed to run diagnostic tool: {}", e)),
    }
}

/// Extract symbols from code based on file extension
pub fn extract_symbols(content: &str, ext: &str) -> Vec<CodeSymbol> {
    match ext {
        "rs" => extract_rust_symbols(content),
        "py" => extract_python_symbols(content),
        "ts" | "tsx" | "js" | "jsx" => extract_typescript_symbols(content),
        "go" => extract_go_symbols(content),
        "java" | "kt" => extract_java_symbols(content),
        _ => extract_generic_symbols(content),
    }
}

/// Generic symbol extraction with pre-compiled regexes
fn extract_symbols_with_patterns(
    content: &str,
    patterns: &[(&str, SymbolKind)],
    skip_dunder: bool,
) -> Vec<CodeSymbol> {
    // Pre-compile regexes once
    let compiled: Vec<_> = patterns
        .iter()
        .filter_map(|(pattern, kind)| Regex::new(pattern).ok().map(|re| (re, *kind)))
        .collect();

    let mut symbols = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        for (re, kind) in &compiled {
            if let Some(caps) = re.captures(line) {
                if let Some(name) = caps.get(1) {
                    let name_str = name.as_str();
                    // Skip dunder methods if requested
                    if skip_dunder && name_str.starts_with("__") && name_str.ends_with("__") {
                        continue;
                    }
                    symbols.push(CodeSymbol {
                        kind: *kind,
                        name: name_str.to_string(),
                        line: line_num + 1,
                        signature: Some(line.trim().to_string()),
                    });
                    break;
                }
            }
        }
    }

    symbols
}

/// Extract symbols from Rust code
fn extract_rust_symbols(content: &str) -> Vec<CodeSymbol> {
    let patterns: &[(&str, SymbolKind)] = &[
        (r"^\s*(?:pub\s+)?fn\s+(\w+)", SymbolKind::Function),
        (r"^\s*(?:pub\s+)?struct\s+(\w+)", SymbolKind::Struct),
        (r"^\s*(?:pub\s+)?enum\s+(\w+)", SymbolKind::Enum),
        (r"^\s*(?:pub\s+)?trait\s+(\w+)", SymbolKind::Trait),
        (r"^\s*(?:pub\s+)?type\s+(\w+)", SymbolKind::Type),
        (r"^\s*(?:pub\s+)?const\s+(\w+)", SymbolKind::Const),
        (r"^\s*(?:pub\s+)?mod\s+(\w+)", SymbolKind::Module),
    ];
    extract_symbols_with_patterns(content, patterns, false)
}

/// Extract symbols from Python code
fn extract_python_symbols(content: &str) -> Vec<CodeSymbol> {
    let patterns: &[(&str, SymbolKind)] = &[
        (r"^def\s+(\w+)", SymbolKind::Function),
        (r"^class\s+(\w+)", SymbolKind::Class),
        (r"^\s{4}def\s+(\w+)", SymbolKind::Function), // Method
        (r"^(\w+)\s*=", SymbolKind::Variable),        // Top-level variable
    ];
    extract_symbols_with_patterns(content, patterns, true) // skip dunder
}

/// Extract symbols from TypeScript/JavaScript code
fn extract_typescript_symbols(content: &str) -> Vec<CodeSymbol> {
    let patterns: &[(&str, SymbolKind)] = &[
        (r"^\s*(?:export\s+)?function\s+(\w+)", SymbolKind::Function),
        (r"^\s*(?:export\s+)?class\s+(\w+)", SymbolKind::Class),
        (r"^\s*(?:export\s+)?interface\s+(\w+)", SymbolKind::Interface),
        (r"^\s*(?:export\s+)?type\s+(\w+)", SymbolKind::Type),
        (r"^\s*(?:export\s+)?enum\s+(\w+)", SymbolKind::Enum),
        (
            r"^\s*(?:export\s+)?const\s+(\w+)\s*=\s*(?:async\s+)?\(",
            SymbolKind::Function,
        ),
    ];
    extract_symbols_with_patterns(content, patterns, false)
}

/// Extract symbols from Go code
fn extract_go_symbols(content: &str) -> Vec<CodeSymbol> {
    let patterns: &[(&str, SymbolKind)] = &[
        (r"^func\s+(?:\([^)]+\)\s+)?(\w+)", SymbolKind::Function),
        (r"^type\s+(\w+)\s+struct", SymbolKind::Struct),
        (r"^type\s+(\w+)\s+interface", SymbolKind::Interface),
        (r"^const\s+(\w+)", SymbolKind::Const),
        (r"^var\s+(\w+)", SymbolKind::Variable),
    ];
    extract_symbols_with_patterns(content, patterns, false)
}

/// Extract symbols from Java/Kotlin code
fn extract_java_symbols(content: &str) -> Vec<CodeSymbol> {
    let patterns: &[(&str, SymbolKind)] = &[
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
        (
            r"^\s*(?:public|private|protected)?\s*(?:static)?\s*(?:final)?\s*\w+\s+(\w+)\s*\([^)]*\)\s*\{",
            SymbolKind::Function,
        ),
    ];
    extract_symbols_with_patterns(content, patterns, false)
}

/// Extract symbols from generic code (fallback)
fn extract_generic_symbols(content: &str) -> Vec<CodeSymbol> {
    let patterns: &[(&str, SymbolKind)] = &[
        (r"^\s*(?:function|def|fn|func)\s+(\w+)", SymbolKind::Function),
        (r"^\s*class\s+(\w+)", SymbolKind::Class),
    ];
    extract_symbols_with_patterns(content, patterns, false)
}
