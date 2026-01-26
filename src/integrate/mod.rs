//! Integrate module - External integration features
//!
//! Provides integration with external tools:
//! - Pick mode: Use fileview as a file picker (--pick)
//! - Callback: Run commands on file selection (--on-select)

pub mod callback;
pub mod pick;

pub use callback::{Callback, CallbackResult};
pub use pick::{exit_code, output_paths, OutputFormat, PickResult};
