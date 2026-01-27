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
/// Tries to query terminal capabilities first, falls back to halfblock rendering.
///
/// Set environment variable `FILEVIEW_IMAGE_PROTOCOL` to control behavior:
/// - `auto` (default): Auto-detect terminal protocol
/// - `halfblocks`: Force halfblock rendering (most compatible)
/// - `sixel`, `kitty`, `iterm2`: Force specific protocol
pub fn create_image_picker() -> Option<Picker> {
    use ratatui_image::picker::ProtocolType;

    // Check for environment variable override
    if let Ok(protocol) = std::env::var("FILEVIEW_IMAGE_PROTOCOL") {
        match protocol.to_lowercase().as_str() {
            "halfblocks" | "half" => return Some(Picker::halfblocks()),
            "sixel" => {
                let mut picker = Picker::halfblocks();
                picker.set_protocol_type(ProtocolType::Sixel);
                return Some(picker);
            }
            "kitty" => {
                let mut picker = Picker::halfblocks();
                picker.set_protocol_type(ProtocolType::Kitty);
                return Some(picker);
            }
            "iterm2" | "iterm" => {
                let mut picker = Picker::halfblocks();
                picker.set_protocol_type(ProtocolType::Iterm2);
                return Some(picker);
            }
            _ => {} // "auto" or unknown, continue with auto-detection
        }
    }

    // VS Code terminal doesn't properly support advanced image protocols
    // (ratatui-image detects it as iTerm2 compatible, but it doesn't work)
    if is_vscode_terminal() {
        return Some(Picker::halfblocks());
    }

    // Try to query terminal for image protocol support (Sixel/Kitty/iTerm2)
    Picker::from_query_stdio().ok().or_else(|| {
        // Fallback: use halfblock rendering
        Some(Picker::halfblocks())
    })
}

/// Check if running in VS Code integrated terminal
fn is_vscode_terminal() -> bool {
    std::env::var("TERM_PROGRAM")
        .map(|v| v.to_lowercase().contains("vscode"))
        .unwrap_or(false)
}
