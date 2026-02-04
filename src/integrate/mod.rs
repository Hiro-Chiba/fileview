//! Integrate module - External integration features
//!
//! Provides integration with external tools:
//! - Pick mode: Use fileview as a file picker (--pick)
//! - Callback: Run commands on file selection (--on-select)
//! - Tree mode: Output directory tree to stdout (--tree)
//! - Content output: Include file contents in pick output (--with-content)
//! - Context mode: Output project context for AI tools (--context)
//! - Session: Save/restore selection state

pub mod benchmark;
pub mod callback;
pub mod claude_init;
pub mod context;
pub mod context_pack;
pub mod pick;
pub mod plugin_cmd;
pub mod related;
pub mod session;
pub mod tree;

pub use benchmark::run_ai_benchmark;
pub use callback::{Callback, CallbackResult};
pub use claude_init::claude_init;
pub use context::{build_project_context, output_context};
pub use context_pack::{
    build_context_pack, build_context_pack_with_options, output_context_pack,
    output_context_pack_with_options, ContextAgent, ContextPackFormat, ContextPackOptions,
    ContextPackPreset,
};
pub use pick::{
    exit_code, output_paths, output_paths_claude_format, output_paths_with_content, OutputFormat,
    PickResult,
};
pub use plugin_cmd::{plugin_init, plugin_test};
pub use related::{collect_related_candidates, collect_related_paths, RelatedCandidate};
pub use session::{load_session, load_session_named, save_session, save_session_named, Session};
pub use tree::{output_tree, print_tree_recursive_pub};
