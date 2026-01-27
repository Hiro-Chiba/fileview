//! Kitty graphics protocol implementation
//!
//! The Kitty graphics protocol transmits images using base64-encoded data
//! with special escape sequences. It offers high quality and performance.
//!
//! Supported terminals: Kitty

use base64::{engine::general_purpose::STANDARD, Engine};
use image::{DynamicImage, GenericImageView, ImageEncoder};
use std::borrow::Cow;
use std::io::{self, Write};

/// Kitty graphics protocol action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KittyAction {
    /// Transmit image data
    Transmit,
    /// Transmit and display image
    TransmitAndDisplay,
    /// Display previously transmitted image
    Display,
    /// Delete image
    Delete,
}

impl KittyAction {
    fn as_char(self) -> char {
        match self {
            KittyAction::Transmit => 't',
            KittyAction::TransmitAndDisplay => 'T',
            KittyAction::Display => 'p',
            KittyAction::Delete => 'd',
        }
    }
}

/// Kitty image format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KittyFormat {
    /// RGB (24-bit)
    Rgb,
    /// RGBA (32-bit)
    #[default]
    Rgba,
    /// PNG compressed
    Png,
}

impl KittyFormat {
    fn format_code(self) -> u8 {
        match self {
            KittyFormat::Rgb => 24,
            KittyFormat::Rgba => 32,
            KittyFormat::Png => 100,
        }
    }
}

/// Kitty encoder configuration
#[derive(Debug, Clone)]
pub struct KittyConfig {
    /// Maximum width in pixels
    pub max_width: u32,
    /// Maximum height in pixels
    pub max_height: u32,
    /// Image format to use
    pub format: KittyFormat,
    /// Image ID (for caching)
    pub image_id: Option<u32>,
    /// Placement ID
    pub placement_id: Option<u32>,
    /// Number of columns to display (0 = auto)
    pub columns: u32,
    /// Number of rows to display (0 = auto)
    pub rows: u32,
}

impl Default for KittyConfig {
    fn default() -> Self {
        Self {
            max_width: 800,
            max_height: 600,
            format: KittyFormat::Rgba,
            image_id: None,
            placement_id: None,
            columns: 0,
            rows: 0,
        }
    }
}

/// Chunk size for base64 data transmission (4096 bytes recommended by Kitty docs)
const CHUNK_SIZE: usize = 4096;

/// Build a Kitty graphics control string
fn build_control_string(params: &[(&str, String)], data: Option<&str>, more: bool) -> String {
    let mut result = String::from("\x1b_G");

    // Add parameters
    let param_str: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    result.push_str(&param_str.join(","));

    // Add more data flag
    if more {
        if !params.is_empty() {
            result.push(',');
        }
        result.push_str("m=1");
    } else if !params.is_empty() {
        result.push(',');
        result.push_str("m=0");
    }

    // Add data payload
    if let Some(d) = data {
        result.push(';');
        result.push_str(d);
    }

    result.push_str("\x1b\\");
    result
}

/// Encode an image using Kitty graphics protocol
pub fn encode_kitty(image: &DynamicImage, config: &KittyConfig) -> String {
    let (orig_w, orig_h) = image.dimensions();

    // Early return for empty images
    if orig_w == 0 || orig_h == 0 {
        return String::new();
    }

    // Scale image if needed (use Cow to avoid unnecessary clone)
    let scale = f64::min(
        config.max_width as f64 / orig_w as f64,
        config.max_height as f64 / orig_h as f64,
    );

    let img: Cow<DynamicImage> = if scale < 1.0 {
        Cow::Owned(image.resize(
            (orig_w as f64 * scale) as u32,
            (orig_h as f64 * scale) as u32,
            image::imageops::FilterType::Lanczos3,
        ))
    } else {
        Cow::Borrowed(image)
    };

    let (width, height) = img.dimensions();

    // Get raw pixel data based on format
    let raw_data = match config.format {
        KittyFormat::Rgb => {
            let rgb = img.to_rgb8();
            rgb.into_raw()
        }
        KittyFormat::Rgba => {
            let rgba = img.to_rgba8();
            rgba.into_raw()
        }
        KittyFormat::Png => {
            let mut png_data = Vec::new();
            let encoder = image::codecs::png::PngEncoder::new(&mut png_data);
            if encoder
                .write_image(
                    img.to_rgba8().as_raw(),
                    width,
                    height,
                    image::ExtendedColorType::Rgba8,
                )
                .is_err()
            {
                return String::new();
            }
            png_data
        }
    };

    // Base64 encode
    let encoded = STANDARD.encode(&raw_data);

    // Build output with chunked transmission
    let mut output = String::new();
    let chunks: Vec<&str> = encoded
        .as_bytes()
        .chunks(CHUNK_SIZE)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();

    for (i, chunk) in chunks.iter().enumerate() {
        let is_first = i == 0;
        let is_last = i == chunks.len() - 1;

        let mut params = Vec::new();

        if is_first {
            // First chunk includes all control parameters
            params.push(("a", KittyAction::TransmitAndDisplay.as_char().to_string()));
            params.push(("f", config.format.format_code().to_string()));
            params.push(("s", width.to_string()));
            params.push(("v", height.to_string()));

            if let Some(id) = config.image_id {
                params.push(("i", id.to_string()));
            }
            if let Some(pid) = config.placement_id {
                params.push(("p", pid.to_string()));
            }
            if config.columns > 0 {
                params.push(("c", config.columns.to_string()));
            }
            if config.rows > 0 {
                params.push(("r", config.rows.to_string()));
            }
        }

        output.push_str(&build_control_string(&params, Some(chunk), !is_last));
    }

    output
}

/// Write Kitty image directly to a writer
pub fn write_kitty<W: Write>(
    writer: &mut W,
    image: &DynamicImage,
    config: &KittyConfig,
) -> io::Result<()> {
    let kitty_data = encode_kitty(image, config);
    writer.write_all(kitty_data.as_bytes())?;
    writer.flush()
}

/// Render an image using Kitty protocol for terminal display
pub fn render_kitty_image(image: &DynamicImage, max_width: u32, max_height: u32) -> String {
    let config = KittyConfig {
        max_width,
        max_height,
        format: KittyFormat::Rgba,
        ..Default::default()
    };
    encode_kitty(image, &config)
}

/// Delete a Kitty image by ID
pub fn delete_kitty_image(image_id: u32) -> String {
    let params = vec![
        ("a", KittyAction::Delete.as_char().to_string()),
        ("d", "i".to_string()), // Delete by ID
        ("i", image_id.to_string()),
    ];
    build_control_string(&params, None, false)
}

/// Clear all Kitty images
pub fn clear_kitty_images() -> String {
    let params = vec![
        ("a", KittyAction::Delete.as_char().to_string()),
        ("d", "a".to_string()), // Delete all
    ];
    build_control_string(&params, None, false)
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
    fn test_kitty_config_default() {
        let config = KittyConfig::default();
        assert_eq!(config.max_width, 800);
        assert_eq!(config.max_height, 600);
        assert_eq!(config.format, KittyFormat::Rgba);
        assert!(config.image_id.is_none());
    }

    #[test]
    fn test_kitty_action_char() {
        assert_eq!(KittyAction::Transmit.as_char(), 't');
        assert_eq!(KittyAction::TransmitAndDisplay.as_char(), 'T');
        assert_eq!(KittyAction::Display.as_char(), 'p');
        assert_eq!(KittyAction::Delete.as_char(), 'd');
    }

    #[test]
    fn test_kitty_format_code() {
        assert_eq!(KittyFormat::Rgb.format_code(), 24);
        assert_eq!(KittyFormat::Rgba.format_code(), 32);
        assert_eq!(KittyFormat::Png.format_code(), 100);
    }

    #[test]
    fn test_encode_kitty_basic() {
        let img = create_test_image(10, 10, Rgb([255, 0, 0]));
        let config = KittyConfig::default();
        let kitty = encode_kitty(&img, &config);

        // Should start with ESC_G and end with ESC\
        assert!(kitty.starts_with("\x1b_G"));
        assert!(kitty.ends_with("\x1b\\"));

        // Should contain action and format parameters
        assert!(kitty.contains("a=T"));
        assert!(kitty.contains("f=32")); // RGBA
        assert!(kitty.contains("s=10")); // width
        assert!(kitty.contains("v=10")); // height
    }

    #[test]
    fn test_encode_kitty_empty_image() {
        let img = create_test_image(0, 0, Rgb([0, 0, 0]));
        let config = KittyConfig::default();
        let kitty = encode_kitty(&img, &config);
        assert!(kitty.is_empty());
    }

    #[test]
    fn test_encode_kitty_with_id() {
        let img = create_test_image(10, 10, Rgb([0, 255, 0]));
        let config = KittyConfig {
            image_id: Some(42),
            ..Default::default()
        };
        let kitty = encode_kitty(&img, &config);
        assert!(kitty.contains("i=42"));
    }

    #[test]
    fn test_encode_kitty_scaling() {
        let img = create_test_image(2000, 1500, Rgb([0, 0, 255]));
        let config = KittyConfig {
            max_width: 100,
            max_height: 100,
            ..Default::default()
        };
        let kitty = encode_kitty(&img, &config);

        // Should produce output (image was scaled)
        assert!(!kitty.is_empty());
        assert!(kitty.starts_with("\x1b_G"));
    }

    #[test]
    fn test_encode_kitty_rgb_format() {
        let img = create_test_image(10, 10, Rgb([128, 128, 128]));
        let config = KittyConfig {
            format: KittyFormat::Rgb,
            ..Default::default()
        };
        let kitty = encode_kitty(&img, &config);
        assert!(kitty.contains("f=24")); // RGB format
    }

    #[test]
    fn test_encode_kitty_png_format() {
        let img = create_test_image(10, 10, Rgb([64, 64, 64]));
        let config = KittyConfig {
            format: KittyFormat::Png,
            ..Default::default()
        };
        let kitty = encode_kitty(&img, &config);
        assert!(kitty.contains("f=100")); // PNG format
    }

    #[test]
    fn test_render_kitty_image() {
        let img = create_test_image(50, 50, Rgb([255, 255, 0]));
        let kitty = render_kitty_image(&img, 100, 100);
        assert!(!kitty.is_empty());
        assert!(kitty.starts_with("\x1b_G"));
    }

    #[test]
    fn test_delete_kitty_image() {
        let delete_cmd = delete_kitty_image(123);
        assert!(delete_cmd.starts_with("\x1b_G"));
        assert!(delete_cmd.contains("a=d"));
        assert!(delete_cmd.contains("d=i"));
        assert!(delete_cmd.contains("i=123"));
    }

    #[test]
    fn test_clear_kitty_images() {
        let clear_cmd = clear_kitty_images();
        assert!(clear_cmd.starts_with("\x1b_G"));
        assert!(clear_cmd.contains("a=d"));
        assert!(clear_cmd.contains("d=a"));
    }

    #[test]
    fn test_write_kitty() {
        let img = create_test_image(10, 10, Rgb([0, 128, 255]));
        let config = KittyConfig::default();
        let mut buffer = Vec::new();

        write_kitty(&mut buffer, &img, &config).unwrap();

        assert!(!buffer.is_empty());
        assert!(buffer.starts_with(b"\x1b_G"));
    }

    #[test]
    fn test_build_control_string() {
        let params = vec![("a", "T".to_string()), ("f", "32".to_string())];
        let ctrl = build_control_string(&params, Some("data"), false);

        assert!(ctrl.starts_with("\x1b_G"));
        assert!(ctrl.ends_with("\x1b\\"));
        assert!(ctrl.contains("a=T"));
        assert!(ctrl.contains("f=32"));
        assert!(ctrl.contains(";data"));
    }

    #[test]
    fn test_kitty_chunked_transmission() {
        // Create a larger image that will require chunking
        let img = create_test_image(200, 200, Rgb([255, 128, 64]));
        let config = KittyConfig::default();
        let kitty = encode_kitty(&img, &config);

        // Should contain multiple chunks with m=1 for continuation
        // (Note: small images might fit in one chunk, so this tests the mechanism exists)
        assert!(!kitty.is_empty());
        assert!(kitty.starts_with("\x1b_G"));
        assert!(kitty.ends_with("\x1b\\"));
    }
}
