//! MCP tool handlers

pub mod analysis;
pub mod context;
#[cfg(feature = "ai")]
pub mod dependency;
#[cfg(not(feature = "ai"))]
#[path = "dependency_stub.rs"]
pub mod dependency;
pub mod file;
pub mod git;
pub mod project;
pub mod workspace;

pub use super::types::{ToolCallResult, ToolContent};

pub fn error_result(message: &str) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent::text(message.to_string())],
        is_error: Some(true),
    }
}

pub fn success_result(message: String) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent::text(message)],
        is_error: None,
    }
}
