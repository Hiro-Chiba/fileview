//! Image preview with ratatui-image protocol support

use image::{DynamicImage, GenericImageView};
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
        let dyn_img = open_image(path)?;
        let (width, height) = dyn_img.dimensions();
        let protocol = picker.new_resize_protocol(dyn_img);

        Ok(Self {
            width,
            height,
            protocol,
        })
    }
}

/// Longest side, in pixels, that an SVG is rasterized to. The terminal
/// protocol scales the result to the pane, so this only bounds sharpness and
/// memory, whatever size the SVG declares.
#[cfg(feature = "svg")]
const SVG_RASTER_SIZE: f32 = 1024.0;

/// SVGs are parsed in memory, so larger files are refused instead of read.
#[cfg(feature = "svg")]
const MAX_SVG_BYTES: u64 = 8 * 1024 * 1024;

/// Open an image file, rasterizing SVG when the `svg` feature is enabled.
pub fn open_image(path: &std::path::Path) -> anyhow::Result<DynamicImage> {
    #[cfg(feature = "svg")]
    if is_svg_file(path) {
        return rasterize_svg(&read_svg(path)?);
    }
    Ok(image::open(path)?)
}

#[cfg(feature = "svg")]
fn is_svg_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
}

#[cfg(feature = "svg")]
fn read_svg(path: &std::path::Path) -> anyhow::Result<Vec<u8>> {
    use std::io::Read;

    // Reading through `take` also bounds a file that grows after the check.
    let mut data = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_SVG_BYTES + 1)
        .read_to_end(&mut data)?;
    anyhow::ensure!(
        data.len() as u64 <= MAX_SVG_BYTES,
        "SVG is larger than {} MiB",
        MAX_SVG_BYTES / 1024 / 1024
    );
    Ok(data)
}

#[cfg(feature = "svg")]
fn rasterize_svg(data: &[u8]) -> anyhow::Result<DynamicImage> {
    use resvg::{tiny_skia, usvg};

    let tree = usvg::Tree::from_data(data, &usvg::Options::default())?;
    let size = tree.size();
    let scale = SVG_RASTER_SIZE / size.width().max(size.height());
    let width = (size.width() * scale).round().max(1.0) as u32;
    let height = (size.height() * scale).round().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| anyhow::anyhow!("invalid SVG size {width}x{height}"))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    // tiny-skia stores premultiplied alpha, while `image` expects straight alpha.
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    let image = image::RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| anyhow::anyhow!("SVG raster buffer size mismatch"))?;
    Ok(DynamicImage::ImageRgba8(image))
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
    let area_pixel_width = area.width as f64 * font_size.width as f64;
    let area_pixel_height = area.height as f64 * font_size.height as f64;

    // Calculate scale factors for both dimensions
    let scale_x = area_pixel_width / img_width as f64;
    let scale_y = area_pixel_height / img_height as f64;

    // Use the smaller scale to maintain aspect ratio
    let scale = scale_x.min(scale_y);

    // Calculate scaled image size in pixels, then convert to cells
    let scaled_pixel_width = img_width as f64 * scale;
    let scaled_pixel_height = img_height as f64 * scale;

    let scaled_cell_width = (scaled_pixel_width / font_size.width as f64).round() as u16;
    let scaled_cell_height = (scaled_pixel_height / font_size.height as f64).round() as u16;

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
    ) || (cfg!(feature = "svg") && ext.as_deref() == Some("svg"))
}

#[cfg(all(test, feature = "svg"))]
mod svg_tests {
    use super::*;

    #[test]
    fn svg_is_rasterized_to_the_fixed_longest_side_keeping_aspect() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32">
            <rect width="64" height="32" fill="#ff0000"/></svg>"##;
        let image = rasterize_svg(svg).unwrap();
        assert_eq!(image.dimensions(), (1024, 512));
        assert_eq!(image.get_pixel(512, 256).0, [255, 0, 0, 255]);
    }

    #[test]
    fn transparent_areas_stay_transparent() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">
            <circle cx="5" cy="5" r="2" fill="#00ff00"/></svg>"##;
        let image = rasterize_svg(svg).unwrap();
        assert_eq!(image.get_pixel(0, 0).0[3], 0);
        assert_eq!(image.get_pixel(512, 512).0, [0, 255, 0, 255]);
    }

    #[test]
    fn oversized_svg_is_refused_without_parsing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("huge.svg");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_SVG_BYTES + 1).unwrap();
        let err = open_image(&path).unwrap_err().to_string();
        assert!(err.contains("larger than"), "{err}");
    }

    #[test]
    fn gzip_svg_is_rejected_because_svgz_support_is_off() {
        assert!(rasterize_svg(&[0x1f, 0x8b, 0x08, 0x00]).is_err());
    }

    #[test]
    fn invalid_svg_is_an_error() {
        assert!(rasterize_svg(b"<svg").is_err());
    }

    #[test]
    fn svg_files_are_images_and_open_through_open_image() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Icon.SVG");
        std::fs::write(
            &path,
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><rect width="8" height="8"/></svg>"#,
        )
        .unwrap();
        assert!(is_image_file(&path));
        assert_eq!(open_image(&path).unwrap().dimensions(), (1024, 1024));
    }
}
