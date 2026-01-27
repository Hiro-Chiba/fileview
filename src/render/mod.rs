//! Render module - UI rendering

pub mod icons;
pub mod iterm2;
pub mod kitty;
pub mod preview;
pub mod sixel;
pub mod status;
pub mod terminal;
pub mod tree;

pub use icons::get_icon;
pub use iterm2::{encode_iterm2, render_iterm2_image, write_iterm2, ITerm2Config};
pub use kitty::{
    clear_kitty_images, delete_kitty_image, encode_kitty, render_kitty_image, write_kitty,
    KittyAction, KittyConfig, KittyFormat,
};
pub use preview::{
    is_binary_file, is_image_file, is_text_file, render_directory_info, render_hex_preview,
    render_image_preview, render_text_preview, DirectoryInfo, HexPreview, ImagePreview,
    TextPreview,
};
pub use sixel::{encode_sixel, render_sixel_image, write_sixel, SixelConfig};
pub use status::{render_input_popup, render_status_bar};
pub use terminal::{detect_best_protocol, detect_terminal, ImageProtocol, TerminalKind};
pub use tree::{render_tree, visible_height};
