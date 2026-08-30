//! Status bar, popups, and help overlay rendering.
//!
//! Split into sibling modules for navigability:
//! - `bar`: status bar density variants and the budget segment
//! - `format`: cached file metadata and size/time formatters
//! - `popup`: input, confirm, and delete-confirm popups
//! - `help`: help overlay constants and builders
//! - `todos`: TODO/FIXME popup

mod bar;
mod format;
mod help;
mod popup;
mod todos;

pub use bar::{effective_status_message, render_status_bar};
pub(crate) use format::invalidate_file_info_cache;
pub use help::render_help_popup;
pub use popup::render_input_popup;
pub use todos::render_todos_popup;
