//! Image preview with ratatui-image protocol support

use image::GenericImageView;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders},
    Frame,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, FontSize, Resize, StatefulImage};

use super::common::get_border_style;

/// Image preview with ratatui-image protocol support
pub struct ImagePreview {
    pub width: u32,
    pub height: u32,
    /// Protocol state for ratatui-image rendering (Sixel/Kitty/iTerm2/Halfblock)
    pub protocol: StatefulProtocol,
}

impl ImagePreview {
    /// Load image from file path using ratatui-image picker
    pub fn load(path: &std::path::Path, picker: &mut Picker) -> anyhow::Result<Self> {
        let dyn_img = image::open(path)?;
        let (width, height) = dyn_img.dimensions();
        let protocol = picker.new_resize_protocol(dyn_img);

        Ok(Self {
            width,
            height,
            protocol,
        })
    }
}

/// Calculate centered area for an image within a given area
///
/// This function computes the optimal position and size for displaying an image
/// centered within the available area while maintaining aspect ratio.
///
/// # Arguments
/// * `area` - The available area in terminal cells
/// * `img_width` - The image width in pixels
/// * `img_height` - The image height in pixels
/// * `font_size` - Terminal font size (width, height) in pixels
///
/// # Returns
/// A `Rect` representing the centered area where the image should be rendered
pub fn calculate_centered_image_area(
    area: Rect,
    img_width: u32,
    img_height: u32,
    font_size: FontSize,
) -> Rect {
    // Convert cell dimensions to pixel dimensions using font_size
    let area_pixel_width = area.width as f64 * font_size.0 as f64;
    let area_pixel_height = area.height as f64 * font_size.1 as f64;

    // Calculate scale factors for both dimensions
    let scale_x = area_pixel_width / img_width as f64;
    let scale_y = area_pixel_height / img_height as f64;

    // Use the smaller scale to maintain aspect ratio
    let scale = scale_x.min(scale_y);

    // Calculate scaled image size in pixels, then convert to cells
    let scaled_pixel_width = img_width as f64 * scale;
    let scaled_pixel_height = img_height as f64 * scale;

    let scaled_cell_width = (scaled_pixel_width / font_size.0 as f64).round() as u16;
    let scaled_cell_height = (scaled_pixel_height / font_size.1 as f64).round() as u16;

    // Calculate padding to center the image
    let padding_x = area.width.saturating_sub(scaled_cell_width) / 2;
    let padding_y = area.height.saturating_sub(scaled_cell_height) / 2;

    // Create centered area
    Rect::new(
        area.x + padding_x,
        area.y + padding_y,
        scaled_cell_width.min(area.width),
        scaled_cell_height.min(area.height),
    )
}

/// Render image preview using ratatui-image (Sixel/Kitty/iTerm2/Halfblock)
/// The image is centered within the preview area while maintaining aspect ratio
pub fn render_image_preview(
    frame: &mut Frame,
    img: &mut ImagePreview,
    area: Rect,
    title: &str,
    focused: bool,
    font_size: FontSize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ({}x{}) ", title, img.width, img.height))
        .border_style(get_border_style(focused));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Calculate centered area for the image
    let centered_area = calculate_centered_image_area(inner_area, img.width, img.height, font_size);

    // Render image using ratatui-image's StatefulImage widget
    // Use Resize::Scale to ensure the image fills the centered area
    let image_widget = StatefulImage::default().resize(Resize::Scale(None));
    frame.render_stateful_widget(image_widget, centered_area, &mut img.protocol);
}

/// Check if a file is likely an image
pub fn is_image_file(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(
        ext.as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico")
    )
}
