//! Integrate module - External integration features
//!
//! Provides integration with external tools:
//! - Pick mode: Use fileview as a file picker (--pick)
//! - Callback: Run commands on file selection (--on-select)
//! - Tree mode: Output directory tree to stdout (--tree)
//! - Content output: Include file contents in pick output (--with-content)

pub mod callback;
pub mod pick;
pub mod tree;

pub use callback::{Callback, CallbackResult};
pub use pick::{
    exit_code, output_paths, output_paths_claude_format, output_paths_with_content, OutputFormat,
    PickResult,
};
pub use tree::{output_tree, print_tree_recursive_pub};
