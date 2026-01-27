//! Render module - UI rendering

pub mod preview;
pub mod status;
pub mod tree;

pub use preview::{
    is_binary_file, is_image_file, is_text_file, render_directory_info, render_hex_preview,
    render_image_preview, render_text_preview, DirectoryInfo, HexPreview, ImagePreview,
    TextPreview,
};
pub use status::{render_input_popup, render_status_bar};
pub use tree::{render_tree, visible_height};
