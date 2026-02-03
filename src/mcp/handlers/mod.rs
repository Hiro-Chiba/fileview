//! MCP tool handlers
//!
//! Organized handlers for all MCP tools.

pub mod analysis;
pub mod context;
pub mod dependency;
pub mod file;
pub mod git;
pub mod project;

// Re-export common types
pub use super::types::{ToolCallResult, ToolContent};

/// Helper function to create an error result
pub fn error_result(message: &str) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent::text(message.to_string())],
        is_error: Some(true),
    }
}

/// Helper function to create a success result
pub fn success_result(message: String) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent::text(message)],
        is_error: None,
    }
}
