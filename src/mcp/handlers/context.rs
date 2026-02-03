//! Context optimization handlers
//!
//! Implements get_smart_context, estimate_tokens, compress_context.

use std::fs;
use std::path::Path;

use super::{error_result, success_result, ToolCallResult};
use crate::mcp::security::validate_path;
use crate::mcp::token::{
    compress_content, estimate_file_tokens, estimate_tokens, format_file_context,
    truncate_to_tokens, TokenBudget,
};

/// Get AI-optimized context for a file
///
/// Includes:
/// 1. Main file content
/// 2. Import/dependency content (type definitions)
/// 3. Related test files
/// 4. Parent module public API
pub fn get_smart_context(
    root: &Path,
    focus_file: &str,
    max_tokens: Option<usize>,
    include_deps: bool,
    include_tests: bool,
) -> ToolCallResult {
    let canonical = match validate_path(root, focus_file) {
        Ok(p) => p,
        Err(e) => return error_result(&e.to_string()),
    };

    if canonical.is_dir() {
        return error_result("focus_file must be a file, not a directory");
    }

    let budget = TokenBudget::new(max_tokens.unwrap_or(4000));
    let mut context_parts = Vec::new();
    let mut total_tokens = 0;

    // 1. Read main file
    let main_content = match fs::read_to_string(&canonical) {
        Ok(c) => c,
        Err(e) => return error_result(&format!("Failed to read focus file: {}", e)),
    };

    let main_tokens = estimate_tokens(&main_content);
    let main_content = if main_tokens > budget.main_file_reserve {
        truncate_to_tokens(&main_content, budget.main_file_reserve)
    } else {
        main_content
    };
    let main_tokens = estimate_tokens(&main_content);
    total_tokens += main_tokens;

    context_parts.push(format!(
        "=== Main File: {} ({} tokens) ===\n{}",
        focus_file, main_tokens, main_content
    ));

    // 2. Find and include dependencies if requested
    if include_deps && budget.remaining(total_tokens) > 100 {
        let deps = find_file_dependencies(root, &canonical);
        let mut dep_content = String::new();
        let mut dep_tokens = 0;

        for dep_path in deps.iter().take(5) {
            // Limit to 5 deps
            if let Ok(content) = fs::read_to_string(dep_path) {
                let tokens = estimate_tokens(&content);
                if dep_tokens + tokens < budget.imports_reserve {
                    dep_content.push_str(&format_file_context(dep_path, &content));
                    dep_tokens += tokens;
                }
            }
        }

        if !dep_content.is_empty() {
            total_tokens += dep_tokens;
            context_parts.push(format!(
                "\n=== Dependencies ({} tokens) ===\n{}",
                dep_tokens, dep_content
            ));
        }
    }

    // 3. Find and include related tests if requested
    if include_tests && budget.remaining(total_tokens) > 100 {
        if let Some(test_path) = find_related_test(root, &canonical) {
            if let Ok(test_content) = fs::read_to_string(&test_path) {
                let test_tokens = estimate_tokens(&test_content);
                let test_content = if test_tokens > budget.tests_reserve {
                    truncate_to_tokens(&test_content, budget.tests_reserve)
                } else {
                    test_content
                };
                let test_tokens = estimate_tokens(&test_content);
                total_tokens += test_tokens;

                let rel_path = test_path
                    .strip_prefix(root)
                    .unwrap_or(&test_path)
                    .display()
                    .to_string();
                context_parts.push(format!(
                    "\n=== Related Test: {} ({} tokens) ===\n{}",
                    rel_path, test_tokens, test_content
                ));
            }
        }
    }

    // Build final output
    let mut output = String::new();
    output.push_str(&format!(
        "Smart Context for {} (Total: {} tokens)\n",
        focus_file, total_tokens
    ));
    output.push_str(&format!(
        "Budget: {}/{} tokens\n\n",
        total_tokens, budget.max_tokens
    ));
    output.push_str(&context_parts.join("\n"));

    success_result(output)
}

/// Estimate token counts for files
pub fn estimate_tokens_handler(root: &Path, paths: &[&str]) -> ToolCallResult {
    let mut results = Vec::new();
    let mut total = 0;

    for path in paths {
        match validate_path(root, path) {
            Ok(canonical) => {
                if canonical.is_dir() {
                    results.push(format!("{}: (directory)", path));
                    continue;
                }

                match estimate_file_tokens(&canonical) {
                    Ok(tokens) => {
                        results.push(format!("{}: {} tokens", path, tokens));
                        total += tokens;
                    }
                    Err(e) => {
                        results.push(format!("{}: error - {}", path, e));
                    }
                }
            }
            Err(e) => {
                results.push(format!("{}: error - {}", path, e));
            }
        }
    }

    let mut output = String::new();
    output.push_str(&format!("Token Estimates ({} files):\n\n", paths.len()));
    for result in &results {
        output.push_str(result);
        output.push('\n');
    }
    output.push_str(&format!("\nTotal: {} tokens", total));

    success_result(output)
}

/// Compress file content by removing comments and whitespace
pub fn compress_context_handler(
    root: &Path,
    path: &str,
    max_tokens: Option<usize>,
) -> ToolCallResult {
    let canonical = match validate_path(root, path) {
        Ok(p) => p,
        Err(e) => return error_result(&e.to_string()),
    };

    if canonical.is_dir() {
        return error_result("Path must be a file, not a directory");
    }

    let content = match fs::read_to_string(&canonical) {
        Ok(c) => c,
        Err(e) => return error_result(&format!("Failed to read file: {}", e)),
    };

    let original_tokens = estimate_tokens(&content);

    // Compress content
    let compressed = compress_content(&content);
    let compressed_tokens = estimate_tokens(&compressed);

    // Optionally truncate to max tokens
    let final_content = match max_tokens {
        Some(max) if compressed_tokens > max => truncate_to_tokens(&compressed, max),
        _ => compressed,
    };
    let final_tokens = estimate_tokens(&final_content);

    let mut output = String::new();
    output.push_str(&format!("Compressed: {}\n", path));
    output.push_str(&format!(
        "Original: {} tokens → Compressed: {} tokens",
        original_tokens, final_tokens
    ));
    if original_tokens > 0 {
        let ratio = (final_tokens as f64 / original_tokens as f64) * 100.0;
        output.push_str(&format!(" ({:.1}%)\n\n", ratio));
    } else {
        output.push_str("\n\n");
    }
    output.push_str(&final_content);

    success_result(output)
}

/// Find file dependencies by parsing imports/use statements
fn find_file_dependencies(root: &Path, file: &Path) -> Vec<std::path::PathBuf> {
    let mut deps = Vec::new();

    let content = match fs::read_to_string(file) {
        Ok(c) => c,
        Err(_) => return deps,
    };

    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "rs" => {
            // Rust: parse use/mod statements
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("use ") || trimmed.starts_with("mod ") {
                    // Extract module path and try to find the file
                    if let Some(module) = extract_rust_module(trimmed) {
                        if let Some(dep_path) = resolve_rust_module(root, file, &module) {
                            deps.push(dep_path);
                        }
                    }
                }
            }
        }
        "ts" | "tsx" | "js" | "jsx" => {
            // TypeScript/JavaScript: parse import statements
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("import ") {
                    if let Some(import_path) = extract_js_import(trimmed) {
                        if let Some(dep_path) = resolve_js_import(root, file, &import_path) {
                            deps.push(dep_path);
                        }
                    }
                }
            }
        }
        "py" => {
            // Python: parse import/from statements
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                    if let Some(module) = extract_python_import(trimmed) {
                        if let Some(dep_path) = resolve_python_import(root, file, &module) {
                            deps.push(dep_path);
                        }
                    }
                }
            }
        }
        _ => {}
    }

    deps
}

/// Find related test file for a source file
fn find_related_test(root: &Path, file: &Path) -> Option<std::path::PathBuf> {
    let file_name = file.file_stem()?.to_str()?;
    let ext = file.extension()?.to_str()?;

    // Common test file patterns
    let test_patterns = [
        format!("{}_test.{}", file_name, ext),
        format!("test_{}.{}", file_name, ext),
        format!("{}.test.{}", file_name, ext),
        format!("{}.spec.{}", file_name, ext),
    ];

    // Check in same directory
    if let Some(parent) = file.parent() {
        for pattern in &test_patterns {
            let test_path = parent.join(pattern);
            if test_path.exists() {
                return Some(test_path);
            }
        }
    }

    // Check in tests/ directory at root
    let tests_dir = root.join("tests");
    if tests_dir.exists() {
        for pattern in &test_patterns {
            let test_path = tests_dir.join(pattern);
            if test_path.exists() {
                return Some(test_path);
            }
        }
    }

    // Check in __tests__/ directory (common in JS projects)
    if let Some(parent) = file.parent() {
        let tests_dir = parent.join("__tests__");
        if tests_dir.exists() {
            for pattern in &test_patterns {
                let test_path = tests_dir.join(pattern);
                if test_path.exists() {
                    return Some(test_path);
                }
            }
        }
    }

    None
}

// Helper functions for parsing imports

fn extract_rust_module(line: &str) -> Option<String> {
    // Parse "use crate::foo::bar" or "mod foo"
    if line.starts_with("use ") {
        let path = line
            .trim_start_matches("use ")
            .trim_end_matches(';')
            .split("::")
            .next()?;
        Some(path.to_string())
    } else if line.starts_with("mod ") {
        let name = line.trim_start_matches("mod ").trim_end_matches(';').trim();
        Some(name.to_string())
    } else {
        None
    }
}

fn resolve_rust_module(root: &Path, _file: &Path, module: &str) -> Option<std::path::PathBuf> {
    // Try to find module file
    let candidates = [
        root.join("src").join(format!("{}.rs", module)),
        root.join("src").join(module).join("mod.rs"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Some(candidate.clone());
        }
    }

    None
}

fn extract_js_import(line: &str) -> Option<String> {
    // Parse "import ... from 'path'" or "import 'path'"
    if let Some(from_idx) = line.find("from ") {
        let rest = &line[from_idx + 5..];
        let path = rest
            .trim()
            .trim_matches(|c| c == '\'' || c == '"' || c == ';');
        Some(path.to_string())
    } else {
        None
    }
}

fn resolve_js_import(root: &Path, file: &Path, import_path: &str) -> Option<std::path::PathBuf> {
    if !import_path.starts_with('.') {
        return None; // Skip node_modules imports
    }

    let base_dir = file.parent()?;
    let resolved = base_dir.join(import_path);

    // Try various extensions
    let extensions = ["", ".ts", ".tsx", ".js", ".jsx", "/index.ts", "/index.js"];
    for ext in &extensions {
        let candidate = std::path::PathBuf::from(format!("{}{}", resolved.display(), ext));
        if candidate.exists() && candidate.starts_with(root) {
            return Some(candidate);
        }
    }

    None
}

fn extract_python_import(line: &str) -> Option<String> {
    // Parse "from foo import bar" or "import foo"
    if line.starts_with("from ") {
        let module = line.trim_start_matches("from ").split_whitespace().next()?;
        Some(module.to_string())
    } else if line.starts_with("import ") {
        let module = line
            .trim_start_matches("import ")
            .split_whitespace()
            .next()?
            .trim_end_matches(',');
        Some(module.to_string())
    } else {
        None
    }
}

fn resolve_python_import(root: &Path, _file: &Path, module: &str) -> Option<std::path::PathBuf> {
    // Convert module path to file path
    let path_str = module.replace('.', "/");

    let candidates = [
        root.join(format!("{}.py", path_str)),
        root.join(path_str).join("__init__.py"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Some(candidate.clone());
        }
    }

    None
}
