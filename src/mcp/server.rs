//! MCP server implementation
//!
//! JSON-RPC server communicating over stdin/stdout.

use std::io::{self, BufRead, Write};
use std::path::Path;

use serde_json::json;

use super::handlers;
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

    JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
}

/// Handle tools/list request
fn handle_tools_list(id: Option<serde_json::Value>) -> JsonRpcResponse {
    let tools = vec![
        Tool {
            name: "list_directory".to_string(),
            description: "List files and directories in a path".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path from root (optional, defaults to root)"
                    }
                }
            }),
        },
        Tool {
            name: "get_tree".to_string(),
            description: "Get directory tree structure".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path from root (optional, defaults to root)"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum depth to traverse (optional, defaults to unlimited)"
                    }
                }
            }),
        },
        Tool {
            name: "read_file".to_string(),
            description: "Read content of a file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "get_git_status".to_string(),
            description: "Get git status showing changed and staged files".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "get_git_diff".to_string(),
            description: "Get git diff for a specific file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "staged": {
                        "type": "boolean",
                        "description": "If true, show staged changes (--cached). Default: false"
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "search_code".to_string(),
            description: "Search for code patterns in the repository using grep/ripgrep"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The search pattern (regex supported)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional relative path to limit search scope"
                    }
                },
                "required": ["pattern"]
            }),
        },
        Tool {
            name: "git_log".to_string(),
            description: "Get git commit history".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of commits to return. Default: 10"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional path to filter commits"
                    }
                }
            }),
        },
        Tool {
            name: "stage_files".to_string(),
            description: "Stage files for git commit".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of paths to stage. Empty or omit to stage all changes."
                    }
                }
            }),
        },
        Tool {
            name: "create_commit".to_string(),
            description: "Create a git commit with staged changes".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Commit message"
                    }
                },
                "required": ["message"]
            }),
        },
        Tool {
            name: "write_file".to_string(),
            description: "Write content to a file (create or overwrite)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    },
                    "create_dirs": {
                        "type": "boolean",
                        "description": "Create parent directories if they don't exist. Default: false"
                    }
                },
                "required": ["path", "content"]
            }),
        },
        Tool {
            name: "delete_file".to_string(),
            description: "Delete a file or directory (moves to trash by default for safety)"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file or directory"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Delete directories recursively. Default: false"
                    },
                    "use_trash": {
                        "type": "boolean",
                        "description": "Move to trash instead of permanent deletion. Default: true"
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "read_files".to_string(),
            description: "Read multiple files at once".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of relative paths to read"
                    }
                },
                "required": ["paths"]
            }),
        },
        Tool {
            name: "get_file_symbols".to_string(),
            description: "Extract code symbols (functions, classes, etc.) from a file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    }
                },
                "required": ["path"]
            }),
        },
    ];

    let result = ToolListResult { tools };
    JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
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

    let result = match call_params.name.as_str() {
        "list_directory" => {
            let path = call_params.arguments.get("path").and_then(|v| v.as_str());
            handlers::list_directory(root, path)
        }
        "get_tree" => {
            let path = call_params.arguments.get("path").and_then(|v| v.as_str());
            let depth = call_params
                .arguments
                .get("depth")
                .and_then(|v| v.as_u64())
                .map(|d| d as usize);
            handlers::get_tree(root, path, depth)
        }
        "read_file" => {
            let path = call_params.arguments.get("path").and_then(|v| v.as_str());
            match path {
                Some(p) => handlers::read_file(root, p),
                None => ToolCallResult {
                    content: vec![ToolContent::text(
                        "Missing required parameter: path".to_string(),
                    )],
                    is_error: Some(true),
                },
            }
        }
        "get_git_status" => handlers::get_git_status(root),
        "get_git_diff" => {
            let path = call_params.arguments.get("path").and_then(|v| v.as_str());
            let staged = call_params
                .arguments
                .get("staged")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            match path {
                Some(p) => handlers::get_git_diff(root, p, staged),
                None => ToolCallResult {
                    content: vec![ToolContent::text(
                        "Missing required parameter: path".to_string(),
                    )],
                    is_error: Some(true),
                },
            }
        }
        "search_code" => {
            let pattern = call_params
                .arguments
                .get("pattern")
                .and_then(|v| v.as_str());
            let path = call_params.arguments.get("path").and_then(|v| v.as_str());
            match pattern {
                Some(p) => handlers::search_code(root, p, path),
                None => ToolCallResult {
                    content: vec![ToolContent::text(
                        "Missing required parameter: pattern".to_string(),
                    )],
                    is_error: Some(true),
                },
            }
        }
        "git_log" => {
            let limit = call_params
                .arguments
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|l| l as usize);
            let path = call_params.arguments.get("path").and_then(|v| v.as_str());
            handlers::git_log(root, limit, path)
        }
        "stage_files" => {
            let paths = call_params
                .arguments
                .get("paths")
                .and_then(|v| v.as_array());
            let path_strs: Vec<&str> = paths
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            handlers::stage_files(root, &path_strs)
        }
        "create_commit" => {
            let message = call_params
                .arguments
                .get("message")
                .and_then(|v| v.as_str());
            match message {
                Some(m) => handlers::create_commit(root, m),
                None => ToolCallResult {
                    content: vec![ToolContent::text(
                        "Missing required parameter: message".to_string(),
                    )],
                    is_error: Some(true),
                },
            }
        }
        "write_file" => {
            let path = call_params.arguments.get("path").and_then(|v| v.as_str());
            let content = call_params
                .arguments
                .get("content")
                .and_then(|v| v.as_str());
            let create_dirs = call_params
                .arguments
                .get("create_dirs")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            match (path, content) {
                (Some(p), Some(c)) => handlers::write_file(root, p, c, create_dirs),
                _ => ToolCallResult {
                    content: vec![ToolContent::text(
                        "Missing required parameters: path, content".to_string(),
                    )],
                    is_error: Some(true),
                },
            }
        }
        "delete_file" => {
            let path = call_params.arguments.get("path").and_then(|v| v.as_str());
            let recursive = call_params
                .arguments
                .get("recursive")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let use_trash = call_params
                .arguments
                .get("use_trash")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            match path {
                Some(p) => handlers::delete_file(root, p, recursive, use_trash),
                None => ToolCallResult {
                    content: vec![ToolContent::text(
                        "Missing required parameter: path".to_string(),
                    )],
                    is_error: Some(true),
                },
            }
        }
        "read_files" => {
            let paths = call_params
                .arguments
                .get("paths")
                .and_then(|v| v.as_array());
            match paths {
                Some(arr) => {
                    let path_strs: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
                    handlers::read_files(root, &path_strs)
                }
                None => ToolCallResult {
                    content: vec![ToolContent::text(
                        "Missing required parameter: paths (array)".to_string(),
                    )],
                    is_error: Some(true),
                },
            }
        }
        "get_file_symbols" => {
            let path = call_params.arguments.get("path").and_then(|v| v.as_str());
            match path {
                Some(p) => handlers::get_file_symbols(root, p),
                None => ToolCallResult {
                    content: vec![ToolContent::text(
                        "Missing required parameter: path".to_string(),
                    )],
                    is_error: Some(true),
                },
            }
        }
        _ => ToolCallResult {
            content: vec![ToolContent::text(format!(
                "Unknown tool: {}",
                call_params.name
            ))],
            is_error: Some(true),
        },
    };

    JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
}
