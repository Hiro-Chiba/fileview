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
