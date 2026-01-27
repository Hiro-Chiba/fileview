//! Unified image rendering with automatic protocol selection
//!
//! This module provides a unified interface for rendering images across
//! different terminal emulators using the best available protocol.

use image::{DynamicImage, GenericImageView};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::iterm2::{encode_iterm2, ITerm2Config};
use super::kitty::{encode_kitty, KittyConfig};
use super::sixel::{encode_sixel, SixelConfig};
use super::terminal::{detect_best_protocol, ImageProtocol};

/// Image rendering result
pub enum ImageRenderResult {
    /// Rendered using ratatui widgets (HalfBlock)
    Widget(Vec<Line<'static>>),
    /// Rendered using terminal escape sequences (Sixel, Kitty, iTerm2)
    EscapeSequence(String),
}

/// Configuration for unified image rendering
#[derive(Debug, Clone)]
pub struct ImageRenderConfig {
    /// Maximum width in pixels for protocol-based rendering
    pub max_width: u32,
    /// Maximum height in pixels for protocol-based rendering
    pub max_height: u32,
    /// Force a specific protocol (None = auto-detect)
    pub force_protocol: Option<ImageProtocol>,
}

impl Default for ImageRenderConfig {
    fn default() -> Self {
        Self {
            max_width: 800,
            max_height: 600,
            force_protocol: None,
        }
    }
}

/// Get the current image protocol (auto-detected or forced)
pub fn get_active_protocol(config: &ImageRenderConfig) -> ImageProtocol {
    config.force_protocol.unwrap_or_else(detect_best_protocol)
}

/// Render an image using the best available protocol
pub fn render_image(image: &DynamicImage, config: &ImageRenderConfig) -> ImageRenderResult {
    let protocol = get_active_protocol(config);

    match protocol {
        ImageProtocol::Sixel => {
            let sixel_config = SixelConfig {
                max_width: config.max_width,
                max_height: config.max_height,
                colors: 256,
                transparent: true,
            };
            ImageRenderResult::EscapeSequence(encode_sixel(image, &sixel_config))
        }
        ImageProtocol::Kitty => {
            let kitty_config = KittyConfig {
                max_width: config.max_width,
                max_height: config.max_height,
                ..Default::default()
            };
            ImageRenderResult::EscapeSequence(encode_kitty(image, &kitty_config))
        }
        ImageProtocol::ITerm2 => {
            let iterm2_config = ITerm2Config {
                max_width: config.max_width,
                max_height: config.max_height,
                ..Default::default()
            };
            ImageRenderResult::EscapeSequence(encode_iterm2(image, &iterm2_config))
        }
        ImageProtocol::HalfBlock => {
            let rgb = image.to_rgb8();
            let width = rgb.width();
            let height = rgb.height();
            let pixels: Vec<(u8, u8, u8)> = rgb.pixels().map(|p| (p[0], p[1], p[2])).collect();
            let lines = render_halfblock(&pixels, width, height, config.max_width, config.max_height);
            ImageRenderResult::Widget(lines)
        }
    }
}

/// Render an image using half-block characters
fn render_halfblock(
    pixels: &[(u8, u8, u8)],
    img_width: u32,
    img_height: u32,
    target_width: u32,
    target_height: u32,
) -> Vec<Line<'static>> {
    if target_width == 0 || target_height == 0 || img_width == 0 || img_height == 0 {
        return vec![Line::from("Image too small to display")];
    }

    // Terminal characters are roughly 2:1 (height:width ratio)
    let char_aspect = 2.0;

    let img_aspect = img_width as f32 / img_height as f32;
    let term_pixel_width = target_width as f32;
    let term_pixel_height = target_height as f32;

    let adjusted_term_aspect = (term_pixel_width * char_aspect) / term_pixel_height;

    let (display_width, display_height) = if img_aspect > adjusted_term_aspect {
        // Image is wider - fit to width
        let w = target_width;
        let h = ((target_width as f32 / char_aspect) / img_aspect * 2.0) as u32;
        (w, h.max(2))
    } else {
        // Image is taller - fit to height
        let h = target_height;
        let w = (target_height as f32 / 2.0 * img_aspect * char_aspect) as u32;
        (w.max(1), h)
    };

    let term_rows = display_height / 2;
    let mut lines = Vec::new();

    for row in 0..term_rows {
        let mut spans = Vec::new();

        for col in 0..display_width {
            // Calculate source coordinates with area averaging
            let src_x_start = (col as f32 / display_width as f32 * img_width as f32) as u32;
            let src_x_end = ((col + 1) as f32 / display_width as f32 * img_width as f32) as u32;
            let src_y_top_start = (row as f32 * 2.0 / display_height as f32 * img_height as f32) as u32;
            let src_y_top_end = ((row as f32 * 2.0 + 1.0) / display_height as f32 * img_height as f32) as u32;
            let src_y_bottom_start = ((row as f32 * 2.0 + 1.0) / display_height as f32 * img_height as f32) as u32;
            let src_y_bottom_end = ((row as f32 * 2.0 + 2.0) / display_height as f32 * img_height as f32) as u32;

            // Area average for top pixel
            let (r1, g1, b1) = area_average(
                pixels,
                img_width,
                img_height,
                src_x_start,
                src_x_end,
                src_y_top_start,
                src_y_top_end,
            );

            // Area average for bottom pixel
            let (r2, g2, b2) = area_average(
                pixels,
                img_width,
                img_height,
                src_x_start,
                src_x_end,
                src_y_bottom_start,
                src_y_bottom_end,
            );

            // Upper half block: foreground = top pixel, background = bottom pixel
            spans.push(Span::styled(
                "\u{2580}", // ▀
                Style::default()
                    .fg(Color::Rgb(r1, g1, b1))
                    .bg(Color::Rgb(r2, g2, b2)),
            ));
        }

        lines.push(Line::from(spans));
    }

    lines
}

/// Calculate area average for a rectangular region
fn area_average(
    pixels: &[(u8, u8, u8)],
    img_width: u32,
    img_height: u32,
    x_start: u32,
    x_end: u32,
    y_start: u32,
    y_end: u32,
) -> (u8, u8, u8) {
    if img_width == 0 || img_height == 0 || pixels.is_empty() {
        return (0, 0, 0);
    }

    let x_start = x_start.min(img_width - 1);
    let x_end = x_end.min(img_width).max(x_start + 1);
    let y_start = y_start.min(img_height - 1);
    let y_end = y_end.min(img_height).max(y_start + 1);

    let mut r_sum: u32 = 0;
    let mut g_sum: u32 = 0;
    let mut b_sum: u32 = 0;
    let mut count: u32 = 0;

    for y in y_start..y_end {
        for x in x_start..x_end {
            let idx = (y * img_width + x) as usize;
            if let Some(&(r, g, b)) = pixels.get(idx) {
                r_sum += r as u32;
                g_sum += g as u32;
                b_sum += b as u32;
                count += 1;
            }
        }
    }

    if count == 0 {
        (0, 0, 0)
    } else {
        (
            (r_sum / count) as u8,
            (g_sum / count) as u8,
            (b_sum / count) as u8,
        )
    }
}

/// Render image preview with automatic protocol selection
///
/// For HalfBlock mode, renders using ratatui widgets.
/// For Sixel/Kitty/iTerm2, renders the escape sequence above the widget area.
pub fn render_image_preview_unified(
    frame: &mut Frame,
    image: &DynamicImage,
    area: Rect,
    title: &str,
    focused: bool,
    protocol: Option<ImageProtocol>,
) {
    let img_width = area.width.saturating_sub(2) as u32;
    let img_height = (area.height.saturating_sub(2) * 2) as u32;

    let config = ImageRenderConfig {
        max_width: img_width * 10, // Scale up for better quality
        max_height: img_height * 10,
        force_protocol: protocol,
    };

    let active_protocol = get_active_protocol(&config);

    // For escape sequence protocols, we need special handling
    // Currently, we fall back to HalfBlock for widget rendering
    // TODO: Implement direct escape sequence output
    let lines = match active_protocol {
        ImageProtocol::HalfBlock => {
            let rgb = image.to_rgb8();
            let width = rgb.width();
            let height = rgb.height();
            let pixels: Vec<(u8, u8, u8)> = rgb.pixels().map(|p| (p[0], p[1], p[2])).collect();
            render_halfblock(&pixels, width, height, img_width, img_height)
        }
        _ => {
            // For now, use HalfBlock for ratatui widget rendering
            // The escape sequence protocols will be used when outputting directly
            let rgb = image.to_rgb8();
            let width = rgb.width();
            let height = rgb.height();
            let pixels: Vec<(u8, u8, u8)> = rgb.pixels().map(|p| (p[0], p[1], p[2])).collect();
            render_halfblock(&pixels, width, height, img_width, img_height)
        }
    };

    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let (width, height) = image.dimensions();
    let protocol_str = format!("{}", active_protocol);

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ({}x{}) [{}] ", title, width, height, protocol_str))
            .border_style(border_style),
    );

    frame.render_widget(widget, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn create_test_image(width: u32, height: u32, color: Rgb<u8>) -> DynamicImage {
        let img = RgbImage::from_fn(width, height, |_, _| color);
        DynamicImage::ImageRgb8(img)
    }

    #[test]
    fn test_image_render_config_default() {
        let config = ImageRenderConfig::default();
        assert_eq!(config.max_width, 800);
        assert_eq!(config.max_height, 600);
        assert!(config.force_protocol.is_none());
    }

    #[test]
    fn test_get_active_protocol_auto() {
        let config = ImageRenderConfig::default();
        let protocol = get_active_protocol(&config);
        // Should return some protocol (depends on terminal)
        assert!(matches!(
            protocol,
            ImageProtocol::Sixel
                | ImageProtocol::Kitty
                | ImageProtocol::ITerm2
                | ImageProtocol::HalfBlock
        ));
    }

    #[test]
    fn test_get_active_protocol_forced() {
        let config = ImageRenderConfig {
            force_protocol: Some(ImageProtocol::Kitty),
            ..Default::default()
        };
        assert_eq!(get_active_protocol(&config), ImageProtocol::Kitty);
    }

    #[test]
    fn test_render_image_halfblock() {
        let img = create_test_image(100, 100, Rgb([255, 0, 0]));
        let config = ImageRenderConfig {
            force_protocol: Some(ImageProtocol::HalfBlock),
            max_width: 50,
            max_height: 50,
        };
        let result = render_image(&img, &config);
        match result {
            ImageRenderResult::Widget(lines) => {
                assert!(!lines.is_empty());
            }
            _ => panic!("Expected Widget result for HalfBlock"),
        }
    }

    #[test]
    fn test_render_image_sixel() {
        let img = create_test_image(100, 100, Rgb([0, 255, 0]));
        let config = ImageRenderConfig {
            force_protocol: Some(ImageProtocol::Sixel),
            max_width: 200,
            max_height: 200,
        };
        let result = render_image(&img, &config);
        match result {
            ImageRenderResult::EscapeSequence(seq) => {
                assert!(seq.starts_with("\x1bPq"));
            }
            _ => panic!("Expected EscapeSequence result for Sixel"),
        }
    }

    #[test]
    fn test_render_image_kitty() {
        let img = create_test_image(100, 100, Rgb([0, 0, 255]));
        let config = ImageRenderConfig {
            force_protocol: Some(ImageProtocol::Kitty),
            max_width: 200,
            max_height: 200,
        };
        let result = render_image(&img, &config);
        match result {
            ImageRenderResult::EscapeSequence(seq) => {
                assert!(seq.starts_with("\x1b_G"));
            }
            _ => panic!("Expected EscapeSequence result for Kitty"),
        }
    }

    #[test]
    fn test_render_image_iterm2() {
        let img = create_test_image(100, 100, Rgb([128, 128, 128]));
        let config = ImageRenderConfig {
            force_protocol: Some(ImageProtocol::ITerm2),
            max_width: 200,
            max_height: 200,
        };
        let result = render_image(&img, &config);
        match result {
            ImageRenderResult::EscapeSequence(seq) => {
                assert!(seq.starts_with("\x1b]1337;File="));
            }
            _ => panic!("Expected EscapeSequence result for iTerm2"),
        }
    }

    #[test]
    fn test_area_average_single_pixel() {
        let pixels = vec![(255, 128, 64)];
        let result = area_average(&pixels, 1, 1, 0, 1, 0, 1);
        assert_eq!(result, (255, 128, 64));
    }

    #[test]
    fn test_area_average_multiple_pixels() {
        let pixels = vec![
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (255, 255, 255),
        ];
        let result = area_average(&pixels, 2, 2, 0, 2, 0, 2);
        // Average of all 4 pixels
        assert_eq!(result, (127, 127, 127)); // (255+0+0+255)/4, etc.
    }

    #[test]
    fn test_area_average_empty() {
        let pixels: Vec<(u8, u8, u8)> = vec![];
        let result = area_average(&pixels, 0, 0, 0, 1, 0, 1);
        assert_eq!(result, (0, 0, 0));
    }

    #[test]
    fn test_render_halfblock_creates_lines() {
        let pixels = vec![(255, 0, 0); 100];
        let lines = render_halfblock(&pixels, 10, 10, 5, 10);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_halfblock_empty_dimensions() {
        let pixels = vec![(255, 0, 0); 100];
        let lines = render_halfblock(&pixels, 10, 10, 0, 0);
        assert_eq!(lines.len(), 1);
        // Should show error message
    }

    #[test]
    fn test_image_render_result_types() {
        // Ensure both variants exist and can be created
        let widget_result = ImageRenderResult::Widget(vec![Line::from("test")]);
        let escape_result = ImageRenderResult::EscapeSequence("test".to_string());

        match widget_result {
            ImageRenderResult::Widget(_) => {}
            _ => panic!("Expected Widget"),
        }
        match escape_result {
            ImageRenderResult::EscapeSequence(_) => {}
            _ => panic!("Expected EscapeSequence"),
        }
    }
}
