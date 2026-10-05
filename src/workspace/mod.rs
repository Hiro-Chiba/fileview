//! Shared workspace metadata and incremental file search.

mod cache;
pub mod cli;
pub mod content;
pub mod content_service;
mod index;
pub mod query;
mod service;

pub use index::{WorkspaceIndex, WorkspaceMatch};
pub use service::{cache_directory, SearchResponse, WorkspaceEngine, WorkspaceStatus};
