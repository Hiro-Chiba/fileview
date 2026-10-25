//! Status bar, popups, and help overlay rendering.

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
