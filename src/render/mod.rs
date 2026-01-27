//! Render module - UI rendering

pub mod icons;
pub mod preview;
pub mod status;
pub mod terminal;
pub mod tree;

pub use icons::get_icon;
pub use preview::{
    is_binary_file, is_image_file, is_text_file, render_directory_info, render_hex_preview,
    render_image_preview, render_text_preview, DirectoryInfo, HexPreview, ImagePreview,
    TextPreview,
};
pub use ratatui_image::picker::Picker;
pub use status::{render_input_popup, render_status_bar};
pub use terminal::{RecommendedProtocol, TerminalBrand};
pub use tree::{render_tree, visible_height};

/// Create an image picker for protocol detection
///
/// This should be called BEFORE entering alternate screen mode.
///
/// Detection priority:
/// 1. `FILEVIEW_IMAGE_PROTOCOL` environment variable (explicit override)
/// 2. Terminal brand detection → recommended protocol
/// 3. Query terminal capabilities via escape sequences
/// 4. Chafa → Halfblocks fallback (Chafa requires `chafa` feature)
///
/// Set environment variable `FILEVIEW_IMAGE_PROTOCOL` to control behavior:
/// - `auto` (default): Auto-detect terminal protocol
/// - `halfblocks`: Force halfblock rendering (most compatible)
/// - `chafa`: Force Chafa rendering (requires `chafa` feature and libchafa)
/// - `sixel`, `kitty`, `iterm2`: Force specific protocol
pub fn create_image_picker() -> Option<Picker> {
    use ratatui_image::picker::ProtocolType;

    // 1. Check for environment variable override (highest priority)
    if let Ok(protocol) = std::env::var("FILEVIEW_IMAGE_PROTOCOL") {
        match protocol.to_lowercase().as_str() {
            "halfblocks" | "half" => return Some(Picker::halfblocks()),
            "chafa" => {
                return try_chafa_picker().or_else(|| Some(Picker::halfblocks()));
            }
            "sixel" => {
                return try_protocol(ProtocolType::Sixel);
            }
            "kitty" => {
                return try_protocol(ProtocolType::Kitty);
            }
            "iterm2" | "iterm" => {
                return try_protocol(ProtocolType::Iterm2);
            }
            _ => {} // "auto" or unknown, continue with auto-detection
        }
    }

    // 2. Terminal brand detection
    let terminal = TerminalBrand::detect();
    match terminal.recommended_protocol() {
        RecommendedProtocol::Kitty => {
            if let Some(picker) = try_protocol(ProtocolType::Kitty) {
                return Some(picker);
            }
        }
        RecommendedProtocol::Iterm2 => {
            if let Some(picker) = try_protocol(ProtocolType::Iterm2) {
                return Some(picker);
            }
        }
        RecommendedProtocol::Sixel => {
            if let Some(picker) = try_protocol(ProtocolType::Sixel) {
                return Some(picker);
            }
        }
        RecommendedProtocol::Chafa => {
            // Terminals like VSCode/Alacritty prefer Chafa
            if let Some(picker) = try_chafa_picker() {
                return Some(picker);
            }
            // Fallback to halfblocks if Chafa not available
            return Some(Picker::halfblocks());
        }
        RecommendedProtocol::Query => {
            // Unknown terminal or tmux - try query first
        }
    }

    // 3. Query terminal capabilities (for unknown terminals or if brand-specific failed)
    if let Ok(picker) = Picker::from_query_stdio() {
        return Some(picker);
    }

    // 4. Fallback chain: Chafa → Halfblocks
    try_chafa_picker().or_else(|| Some(Picker::halfblocks()))
}

/// Try to create a picker with a specific protocol type
fn try_protocol(protocol_type: ratatui_image::picker::ProtocolType) -> Option<Picker> {
    let mut picker = Picker::halfblocks();
    picker.set_protocol_type(protocol_type);
    Some(picker)
}

/// Try to create a Chafa picker
///
/// Returns None if Chafa feature is not enabled or libchafa is not available
#[cfg(feature = "chafa")]
fn try_chafa_picker() -> Option<Picker> {
    Picker::from_chafa().ok()
}

/// Fallback when Chafa feature is not enabled
#[cfg(not(feature = "chafa"))]
fn try_chafa_picker() -> Option<Picker> {
    None
}
