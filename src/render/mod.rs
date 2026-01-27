//! Render module - UI rendering

pub mod icons;
pub mod preview;
pub mod status;
pub mod tree;

pub use icons::get_icon;
pub use preview::{
    is_binary_file, is_image_file, is_text_file, render_directory_info, render_hex_preview,
    render_image_preview, render_text_preview, DirectoryInfo, HexPreview, ImagePreview,
    TextPreview,
};
pub use ratatui_image::picker::Picker;
pub use status::{render_input_popup, render_status_bar};
pub use tree::{render_tree, visible_height};

/// Create an image picker for protocol detection
///
/// This should be called BEFORE entering alternate screen mode.
/// Returns None if terminal capability detection fails.
pub fn create_image_picker() -> Option<Picker> {
    Picker::from_query_stdio().ok()
}
