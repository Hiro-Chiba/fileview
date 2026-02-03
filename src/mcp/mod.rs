//! MCP (Model Context Protocol) server module
//!
//! Provides a JSON-RPC server that allows Claude Code to interact with fileview.

mod handlers;
mod server;
mod types;

pub use server::run_server;
