//! MCP (Model Context Protocol) server that lets Claude Code interact with fileview over JSON-RPC.

pub mod handlers;
pub mod registry;
pub mod security;
pub mod server;
pub mod token;
pub mod types;

pub use server::run_server;
