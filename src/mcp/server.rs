//! MCP server implementation
//!
//! JSON-RPC server communicating over stdin/stdout.

use std::io::{self, BufRead, Write};
use std::path::Path;

use serde_json::json;

use super::handlers::{analysis, context, dependency, file, git, project};
use super::registry;
use super::types::*;

/// Run the MCP server
pub fn run_server(root: &Path) -> anyhow::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();

    let reader = stdin.lock();
    let mut writer = stdout.lock();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading stdin: {}", e);
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let response = handle_request(root, &line);
        let response_json = serde_json::to_string(&response)?;
        writeln!(writer, "{}", response_json)?;
        writer.flush()?;
    }

    Ok(())
}

/// Handle a single JSON-RPC request
fn handle_request(root: &Path, request_str: &str) -> JsonRpcResponse {
    let request: JsonRpcRequest = match serde_json::from_str(request_str) {
        Ok(r) => r,
        Err(e) => {
            return JsonRpcResponse::error(
                None,
                error_codes::PARSE_ERROR,
                format!("Parse error: {}", e),
            );
        }
    };

    match request.method.as_str() {
        "initialize" => handle_initialize(request.id),
        "initialized" => JsonRpcResponse::success(request.id, json!({})),
        "tools/list" => handle_tools_list(request.id),
        "tools/call" => handle_tools_call(root, request.id, request.params),
        "ping" => JsonRpcResponse::success(request.id, json!({})),
        _ => JsonRpcResponse::error(
            request.id,
            error_codes::METHOD_NOT_FOUND,
            format!("Method not found: {}", request.method),
        ),
    }
}

/// Handle initialize request
fn handle_initialize(id: Option<serde_json::Value>) -> JsonRpcResponse {
    let result = InitializeResult {
        protocol_version: "2024-11-05".to_string(),
        capabilities: ServerCapabilities {
            tools: ToolsCapability {
                list_changed: false,
            },
        },
        server_info: ServerInfo {
            name: "fileview".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };

    match serde_json::to_value(result) {
        Ok(v) => JsonRpcResponse::success(id, v),
        Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
    }
}

/// Handle tools/list request
fn handle_tools_list(id: Option<serde_json::Value>) -> JsonRpcResponse {
    // Use registry to get all tools
    let tools = registry::to_mcp_tools();
    let result = ToolListResult { tools };
    match serde_json::to_value(result) {
        Ok(v) => JsonRpcResponse::success(id, v),
        Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
    }
}

/// Handle tools/call request
fn handle_tools_call(
    root: &Path,
    id: Option<serde_json::Value>,
    params: serde_json::Value,
) -> JsonRpcResponse {
    let call_params: ToolCallParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                format!("Invalid params: {}", e),
            );
        }
    };

    let result = dispatch_tool_call(root, &call_params);
    match serde_json::to_value(result) {
        Ok(v) => JsonRpcResponse::success(id, v),
        Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
    }
}

/// Dispatch tool call to appropriate handler
fn dispatch_tool_call(root: &Path, params: &ToolCallParams) -> ToolCallResult {
    let args = &params.arguments;

    match params.name.as_str() {
        // File operations
        "list_directory" => {
            let path = args.get("path").and_then(|v| v.as_str());
            file::list_directory(root, path)
        }
        "get_tree" => {
            let path = args.get("path").and_then(|v| v.as_str());
            let depth = args
                .get("depth")
                .and_then(|v| v.as_u64())
                .map(|d| d as usize);
            file::get_tree(root, path, depth)
        }
        "read_file" => {
            let path = args.get("path").and_then(|v| v.as_str());
            match path {
                Some(p) => file::read_file(root, p),
                None => missing_param("path"),
            }
        }
        "read_files" => {
            let paths = args.get("paths").and_then(|v| v.as_array());
            match paths {
                Some(arr) => {
                    let path_strs: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
                    file::read_files(root, &path_strs)
                }
                None => missing_param("paths"),
            }
        }
        "write_file" => {
            let path = args.get("path").and_then(|v| v.as_str());
            let content = args.get("content").and_then(|v| v.as_str());
            let create_dirs = args
                .get("create_dirs")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            match (path, content) {
                (Some(p), Some(c)) => file::write_file(root, p, c, create_dirs),
                _ => missing_param("path, content"),
            }
        }
        "delete_file" => {
            let path = args.get("path").and_then(|v| v.as_str());
            let recursive = args
                .get("recursive")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let use_trash = args
                .get("use_trash")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            match path {
                Some(p) => file::delete_file(root, p, recursive, use_trash),
                None => missing_param("path"),
            }
        }
        "search_code" => {
            let pattern = args.get("pattern").and_then(|v| v.as_str());
            let path = args.get("path").and_then(|v| v.as_str());
            match pattern {
                Some(p) => file::search_code(root, p, path),
                None => missing_param("pattern"),
            }
        }

        // Git operations
        "get_git_status" => git::get_git_status(root),
        "get_git_diff" => {
            let path = args.get("path").and_then(|v| v.as_str());
            let staged = args
                .get("staged")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            match path {
                Some(p) => git::get_git_diff(root, p, staged),
                None => missing_param("path"),
            }
        }
        "git_log" => {
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|l| l as usize);
            let path = args.get("path").and_then(|v| v.as_str());
            git::git_log(root, limit, path)
        }
        "stage_files" => {
            let paths = args.get("paths").and_then(|v| v.as_array());
            let path_strs: Vec<&str> = paths
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            git::stage_files(root, &path_strs)
        }
        "create_commit" => {
            let message = args.get("message").and_then(|v| v.as_str());
            match message {
                Some(m) => git::create_commit(root, m),
                None => missing_param("message"),
            }
        }

        // Analysis operations
        "get_file_symbols" => {
            let path = args.get("path").and_then(|v| v.as_str());
            match path {
                Some(p) => analysis::get_file_symbols(root, p),
                None => missing_param("path"),
            }
        }
        "get_definitions" => {
            let path = args.get("path").and_then(|v| v.as_str());
            let line = args.get("line").and_then(|v| v.as_u64()).map(|l| l as u32);
            let column = args
                .get("column")
                .and_then(|v| v.as_u64())
                .map(|c| c as u32);
            match path {
                Some(p) => analysis::get_definitions(root, p, line, column),
                None => missing_param("path"),
            }
        }
        "get_references" => {
            let path = args.get("path").and_then(|v| v.as_str());
            let symbol = args.get("symbol").and_then(|v| v.as_str());
            match (path, symbol) {
                (Some(p), Some(s)) => analysis::get_references(root, p, s),
                _ => missing_param("path, symbol"),
            }
        }
        "get_diagnostics" => {
            let path = args.get("path").and_then(|v| v.as_str());
            match path {
                Some(p) => analysis::get_diagnostics(root, p),
                None => missing_param("path"),
            }
        }

        // Dependency operations
        "get_dependency_graph" => {
            let path = args.get("path").and_then(|v| v.as_str());
            let depth = args
                .get("depth")
                .and_then(|v| v.as_u64())
                .map(|d| d as usize);
            match path {
                Some(p) => dependency::get_dependency_graph(root, p, depth),
                None => missing_param("path"),
            }
        }
        "get_import_tree" => {
            let path = args.get("path").and_then(|v| v.as_str());
            match path {
                Some(p) => dependency::get_import_tree(root, p),
                None => missing_param("path"),
            }
        }
        "find_circular_deps" => {
            let path = args.get("path").and_then(|v| v.as_str());
            dependency::find_circular_deps(root, path)
        }

        // Context operations
        "get_smart_context" => {
            let focus_file = args.get("focus_file").and_then(|v| v.as_str());
            let max_tokens = args
                .get("max_tokens")
                .and_then(|v| v.as_u64())
                .map(|t| t as usize);
            let include_deps = args
                .get("include_deps")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let include_tests = args
                .get("include_tests")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            match focus_file {
                Some(f) => {
                    context::get_smart_context(root, f, max_tokens, include_deps, include_tests)
                }
                None => missing_param("focus_file"),
            }
        }
        "estimate_tokens" => {
            let paths = args.get("paths").and_then(|v| v.as_array());
            match paths {
                Some(arr) => {
                    let path_strs: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
                    context::estimate_tokens_handler(root, &path_strs)
                }
                None => missing_param("paths"),
            }
        }
        "compress_context" => {
            let path = args.get("path").and_then(|v| v.as_str());
            let max_tokens = args
                .get("max_tokens")
                .and_then(|v| v.as_u64())
                .map(|t| t as usize);
            match path {
                Some(p) => context::compress_context_handler(root, p, max_tokens),
                None => missing_param("path"),
            }
        }

        // Project operations
        "run_build" => {
            let command = args.get("command").and_then(|v| v.as_str());
            project::run_build(root, command)
        }
        "run_test" => {
            let path = args.get("path").and_then(|v| v.as_str());
            let filter = args.get("filter").and_then(|v| v.as_str());
            project::run_test(root, path, filter)
        }
        "run_lint" => {
            let path = args.get("path").and_then(|v| v.as_str());
            let fix = args.get("fix").and_then(|v| v.as_bool()).unwrap_or(false);
            project::run_lint(root, path, fix)
        }
        "get_project_stats" => {
            let path = args.get("path").and_then(|v| v.as_str());
            project::get_project_stats(root, path)
        }

        // Unknown tool
        _ => ToolCallResult {
            content: vec![ToolContent::text(format!("Unknown tool: {}", params.name))],
            is_error: Some(true),
        },
    }
}

/// Create error result for missing parameter
fn missing_param(param: &str) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent::text(format!(
            "Missing required parameter: {}",
            param
        ))],
        is_error: Some(true),
    }
}
