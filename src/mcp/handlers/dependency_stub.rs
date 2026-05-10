//! No-op dependency handlers used when the `ai` feature is off.
//!
//! Mirrors the public surface of `dependency.rs` so the MCP server's
//! dispatch table compiles unchanged. Each handler returns
//! `error_result("ai feature disabled")` so an AI agent calling these
//! tools gets a clear message instead of a panic.

use std::path::Path;

use super::{error_result, ToolCallResult};

const DISABLED: &str = "ai feature disabled (rebuild with --features ai)";

pub fn get_dependency_graph(_root: &Path, _path: &str, _depth: Option<usize>) -> ToolCallResult {
    error_result(DISABLED)
}

pub fn get_import_tree(_root: &Path, _path: &str) -> ToolCallResult {
    error_result(DISABLED)
}

pub fn find_circular_deps(_root: &Path, _path: Option<&str>) -> ToolCallResult {
    error_result(DISABLED)
}
