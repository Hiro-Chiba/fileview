//! MCP (Model Context Protocol) server module
//!
//! Provides a JSON-RPC server that allows Claude Code to interact with fileview.
//!
//! ## Module Structure
//!
//! - `server`: JSON-RPC server implementation
//! - `types`: Protocol type definitions
//! - `registry`: Tool registration and schema definitions
//! - `security`: Path validation and security utilities
//! - `token`: Token estimation for AI context optimization
//! - `handlers/`: Tool implementations organized by category
//!   - `file`: File operations (read, write, delete, search)
//!   - `git`: Git operations (status, diff, commit)
//!   - `analysis`: Code analysis (symbols, definitions, references)
//!   - `dependency`: Dependency graph analysis
//!   - `context`: AI context optimization
//!   - `project`: Project management (build, test, lint)

pub mod handlers;
pub mod registry;
pub mod security;
pub mod server;
pub mod token;
pub mod types;

pub use server::run_server;
