//! Dependency analysis handlers
//!
//! Implements get_dependency_graph, get_import_tree, find_circular_deps.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use petgraph::algo::tarjan_scc;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;

use super::{error_result, success_result, ToolCallResult};
use crate::mcp::security::validate_path;

/// Get dependency graph for a path
pub fn get_dependency_graph(root: &Path, path: &str, depth: Option<usize>) -> ToolCallResult {
    let canonical = match validate_path(root, path) {
        Ok(p) => p,
        Err(e) => return error_result(&e.to_string()),
    };

    let max_depth = depth.unwrap_or(2);

    // Build dependency graph
    let (graph, node_indices, _) = build_dependency_graph(root, &canonical, max_depth);

    if graph.node_count() == 0 {
        return success_result(format!("No dependencies found for {}", path));
    }

    let mut result = String::new();
    result.push_str(&format!(
        "Dependency Graph for {} (depth: {})\n",
        path, max_depth
    ));
    result.push_str(&format!(
        "Nodes: {}, Edges: {}\n\n",
        graph.node_count(),
        graph.edge_count()
    ));

    // Format as adjacency list
    result.push_str("Dependencies:\n");
    for (path, &node_idx) in &node_indices {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();

        let deps: Vec<String> = graph
            .neighbors_directed(node_idx, Direction::Outgoing)
            .filter_map(|neighbor| {
                graph
                    .node_weight(neighbor)
                    .map(|p| p.strip_prefix(root).unwrap_or(p).display().to_string())
            })
            .collect();

        if !deps.is_empty() {
            result.push_str(&format!("  {} -> {}\n", rel_path, deps.join(", ")));
        }
    }

    success_result(result)
}

/// Get import tree for a file
pub fn get_import_tree(root: &Path, path: &str) -> ToolCallResult {
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

    let ext = canonical.extension().and_then(|e| e.to_str()).unwrap_or("");
    let imports = extract_imports(&content, ext);

    if imports.is_empty() {
        return success_result(format!("No imports found in {}", path));
    }

    let mut result = String::new();
    result.push_str(&format!("Import Tree for {}:\n\n", path));

    for import in &imports {
        result.push_str(&format!("  {}\n", import));

        // Try to resolve the import
        if let Some(resolved) = resolve_import(root, &canonical, import, ext) {
            let rel_path = resolved
                .strip_prefix(root)
                .unwrap_or(&resolved)
                .display()
                .to_string();
            result.push_str(&format!("    -> {}\n", rel_path));
        }
    }

    success_result(result)
}

/// Find circular dependencies
pub fn find_circular_deps(root: &Path, path: Option<&str>) -> ToolCallResult {
    let start_path = match path {
        Some(p) => match validate_path(root, p) {
            Ok(path) => path,
            Err(e) => return error_result(&e.to_string()),
        },
        None => root.to_path_buf(),
    };

    // Build a comprehensive dependency graph
    let (graph, _node_indices, index_to_path) = build_comprehensive_graph(root, &start_path);

    if graph.node_count() == 0 {
        return success_result("No files found to analyze".to_string());
    }

    // Find strongly connected components (cycles)
    let sccs = tarjan_scc(&graph);

    // Filter to components with more than one node (actual cycles)
    let cycles: Vec<_> = sccs.into_iter().filter(|scc| scc.len() > 1).collect();

    if cycles.is_empty() {
        return success_result(format!(
            "No circular dependencies found (analyzed {} files)",
            graph.node_count()
        ));
    }

    let mut result = String::new();
    result.push_str(&format!(
        "Circular Dependencies Found: {} cycle(s)\n\n",
        cycles.len()
    ));

    for (i, cycle) in cycles.iter().enumerate() {
        result.push_str(&format!("Cycle {}:\n", i + 1));

        let paths: Vec<String> = cycle
            .iter()
            .filter_map(|&idx| index_to_path.get(&idx))
            .map(|p| p.strip_prefix(root).unwrap_or(p).display().to_string())
            .collect();

        for path in &paths {
            result.push_str(&format!("  -> {}\n", path));
        }
        if !paths.is_empty() {
            result.push_str(&format!("  -> {} (back to start)\n", paths[0]));
        }
        result.push('\n');
    }

    success_result(result)
}

/// Build a dependency graph starting from a path
fn build_dependency_graph(
    root: &Path,
    start: &Path,
    max_depth: usize,
) -> (
    DiGraph<PathBuf, ()>,
    HashMap<PathBuf, NodeIndex>,
    HashMap<NodeIndex, PathBuf>,
) {
    let mut graph = DiGraph::new();
    let mut node_indices: HashMap<PathBuf, NodeIndex> = HashMap::new();
    let mut index_to_path: HashMap<NodeIndex, PathBuf> = HashMap::new();
    let mut visited: HashSet<PathBuf> = HashSet::new();

    #[allow(clippy::too_many_arguments)]
    fn add_deps(
        graph: &mut DiGraph<PathBuf, ()>,
        node_indices: &mut HashMap<PathBuf, NodeIndex>,
        index_to_path: &mut HashMap<NodeIndex, PathBuf>,
        visited: &mut HashSet<PathBuf>,
        root: &Path,
        path: &Path,
        depth: usize,
        max_depth: usize,
    ) {
        if depth > max_depth || visited.contains(path) {
            return;
        }
        visited.insert(path.to_path_buf());

        // Get or create node for this path
        let node_idx = *node_indices.entry(path.to_path_buf()).or_insert_with(|| {
            let idx = graph.add_node(path.to_path_buf());
            index_to_path.insert(idx, path.to_path_buf());
            idx
        });

        // If it's a directory, process all source files
        if path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if is_source_file(&entry_path) {
                        add_deps(
                            graph,
                            node_indices,
                            index_to_path,
                            visited,
                            root,
                            &entry_path,
                            depth,
                            max_depth,
                        );
                    }
                }
            }
            return;
        }

        // Read file and extract imports
        if let Ok(content) = fs::read_to_string(path) {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let imports = extract_imports(&content, ext);

            for import in imports {
                if let Some(resolved) = resolve_import(root, path, &import, ext) {
                    if resolved.exists() && resolved.starts_with(root) {
                        let dep_idx = *node_indices.entry(resolved.clone()).or_insert_with(|| {
                            let idx = graph.add_node(resolved.clone());
                            index_to_path.insert(idx, resolved.clone());
                            idx
                        });

                        // Add edge
                        if !graph.contains_edge(node_idx, dep_idx) {
                            graph.add_edge(node_idx, dep_idx, ());
                        }

                        // Recursively process dependency
                        add_deps(
                            graph,
                            node_indices,
                            index_to_path,
                            visited,
                            root,
                            &resolved,
                            depth + 1,
                            max_depth,
                        );
                    }
                }
            }
        }
    }

    add_deps(
        &mut graph,
        &mut node_indices,
        &mut index_to_path,
        &mut visited,
        root,
        start,
        0,
        max_depth,
    );

    (graph, node_indices, index_to_path)
}

/// Build a comprehensive graph of all source files
fn build_comprehensive_graph(
    root: &Path,
    start: &Path,
) -> (
    DiGraph<PathBuf, ()>,
    HashMap<PathBuf, NodeIndex>,
    HashMap<NodeIndex, PathBuf>,
) {
    // Use a larger depth for comprehensive analysis
    build_dependency_graph(root, start, 10)
}

/// Check if a path is a source file we should analyze
fn is_source_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(
        ext,
        "rs" | "py" | "ts" | "tsx" | "js" | "jsx" | "go" | "java" | "kt" | "rb"
    )
}

/// Extract imports from source code
fn extract_imports(content: &str, ext: &str) -> Vec<String> {
    let mut imports = Vec::new();

    match ext {
        "rs" => {
            // Rust: use and mod statements
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("use ") || trimmed.starts_with("mod ") {
                    imports.push(trimmed.trim_end_matches(';').to_string());
                }
            }
        }
        "py" => {
            // Python: import and from statements
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                    imports.push(trimmed.to_string());
                }
            }
        }
        "ts" | "tsx" | "js" | "jsx" => {
            // TypeScript/JavaScript: import statements
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("import ") {
                    imports.push(trimmed.to_string());
                }
            }
        }
        "go" => {
            // Go: import statements
            let mut in_import_block = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("import (") {
                    in_import_block = true;
                } else if in_import_block && trimmed == ")" {
                    in_import_block = false;
                } else if in_import_block && !trimmed.is_empty() {
                    imports.push(format!("import {}", trimmed));
                } else if trimmed.starts_with("import ") {
                    imports.push(trimmed.to_string());
                }
            }
        }
        "java" | "kt" => {
            // Java/Kotlin: import statements
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("import ") {
                    imports.push(trimmed.trim_end_matches(';').to_string());
                }
            }
        }
        _ => {}
    }

    imports
}

/// Resolve an import statement to a file path
fn resolve_import(root: &Path, source_file: &Path, import: &str, ext: &str) -> Option<PathBuf> {
    match ext {
        "rs" => resolve_rust_import(root, source_file, import),
        "py" => resolve_python_import(root, import),
        "ts" | "tsx" | "js" | "jsx" => resolve_js_import(root, source_file, import),
        "go" => resolve_go_import(root, import),
        _ => None,
    }
}

fn resolve_rust_import(root: &Path, _source: &Path, import: &str) -> Option<PathBuf> {
    // Parse "use crate::module::item" or "mod module"
    let parts: Vec<&str> = import.split_whitespace().collect();

    if parts.is_empty() {
        return None;
    }

    if parts[0] == "mod" && parts.len() >= 2 {
        let module = parts[1].trim_end_matches(';');
        let candidates = [
            root.join("src").join(format!("{}.rs", module)),
            root.join("src").join(module).join("mod.rs"),
        ];
        for c in &candidates {
            if c.exists() {
                return Some(c.clone());
            }
        }
    } else if parts[0] == "use" && parts.len() >= 2 {
        let path = parts[1].trim_end_matches(';');
        if path.starts_with("crate::") {
            let module_path = path.trim_start_matches("crate::");
            let module_parts: Vec<&str> = module_path.split("::").collect();
            if let Some(first) = module_parts.first() {
                let candidates = [
                    root.join("src").join(format!("{}.rs", first)),
                    root.join("src").join(first).join("mod.rs"),
                ];
                for c in &candidates {
                    if c.exists() {
                        return Some(c.clone());
                    }
                }
            }
        }
    }

    None
}

fn resolve_python_import(root: &Path, import: &str) -> Option<PathBuf> {
    // Parse "from module import item" or "import module"
    let parts: Vec<&str> = import.split_whitespace().collect();

    let module = if parts.first() == Some(&"from") {
        parts.get(1)?
    } else if parts.first() == Some(&"import") {
        parts.get(1)?.split(',').next()?
    } else {
        return None;
    };

    let path_str = module.replace('.', "/");
    let candidates = [
        root.join(format!("{}.py", path_str)),
        root.join(&path_str).join("__init__.py"),
    ];

    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }

    None
}

fn resolve_js_import(root: &Path, source: &Path, import: &str) -> Option<PathBuf> {
    // Parse "import ... from 'path'"
    let from_idx = import.find("from ")?;
    let path_part = import[from_idx + 5..]
        .trim()
        .trim_matches(|c| c == '\'' || c == '"' || c == ';');

    if !path_part.starts_with('.') {
        return None; // External dependency
    }

    let base_dir = source.parent()?;
    let resolved = base_dir.join(path_part);

    let extensions = ["", ".ts", ".tsx", ".js", ".jsx", "/index.ts", "/index.js"];
    for ext in &extensions {
        let candidate = PathBuf::from(format!("{}{}", resolved.display(), ext));
        if candidate.exists() && candidate.starts_with(root) {
            return Some(candidate);
        }
    }

    None
}

fn resolve_go_import(_root: &Path, _import: &str) -> Option<PathBuf> {
    // Go imports are typically external packages
    // Local imports would need module path resolution
    None
}
