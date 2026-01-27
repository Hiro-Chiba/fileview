//! iTerm2 inline images protocol implementation
//!
//! The iTerm2 inline images protocol uses OSC escape sequences to display
//! base64-encoded images. It's simpler than Sixel but with good support.
//!
//! Supported terminals: iTerm2, WezTerm, mintty, Konsole

use base64::{engine::general_purpose::STANDARD, Engine};
use image::{DynamicImage, GenericImageView, ImageEncoder};
use std::borrow::Cow;
use std::io::{self, Write};

/// iTerm2 image configuration
#[derive(Debug, Clone)]
pub struct ITerm2Config {
    /// Maximum width in pixels (will scale down if larger)
    pub max_width: u32,
    /// Maximum height in pixels (will scale down if larger)
    pub max_height: u32,
    /// Width to display (in character cells, 0 = auto)
    pub width: u32,
    /// Height to display (in character cells, 0 = auto)
    pub height: u32,
    /// Preserve aspect ratio
    pub preserve_aspect_ratio: bool,
    /// Whether to use PNG (true) or JPEG (false)
    pub use_png: bool,
    /// Image name (optional)
    pub name: Option<String>,
    /// Whether to inline the image or download it
    pub inline: bool,
}

impl Default for ITerm2Config {
    fn default() -> Self {
        Self {
            max_width: 800,
            max_height: 600,
            width: 0,
            height: 0,
            preserve_aspect_ratio: true,
            use_png: true,
            name: None,
            inline: true,
        }
    }
}

/// Encode an image using iTerm2 inline images protocol
pub fn encode_iterm2(image: &DynamicImage, config: &ITerm2Config) -> String {
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

    // Encode image to PNG or JPEG
    let image_data = if config.use_png {
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
    } else {
        let mut jpeg_data = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_data, 90);
        if encoder
            .write_image(
                img.to_rgb8().as_raw(),
                width,
                height,
                image::ExtendedColorType::Rgb8,
            )
            .is_err()
        {
            return String::new();
        }
        jpeg_data
    };

    // Base64 encode
    let encoded = STANDARD.encode(&image_data);

    // Build OSC sequence
    // Format: ESC ] 1337 ; File = [args] : base64data BEL
    let mut output = String::from("\x1b]1337;File=");

    // Add parameters
    let mut params = Vec::new();

    if let Some(ref name) = config.name {
        let name_b64 = STANDARD.encode(name.as_bytes());
        params.push(format!("name={}", name_b64));
    }

    params.push(format!("size={}", image_data.len()));

    if config.inline {
        params.push("inline=1".to_string());
    }

    if config.width > 0 {
        params.push(format!("width={}", config.width));
    }

    if config.height > 0 {
        params.push(format!("height={}", config.height));
    }

    if !config.preserve_aspect_ratio {
        params.push("preserveAspectRatio=0".to_string());
    }

    output.push_str(&params.join(";"));
    output.push(':');
    output.push_str(&encoded);

    // End with BEL or ST
    output.push('\x07');

    output
}

/// Write iTerm2 image directly to a writer
pub fn write_iterm2<W: Write>(
    writer: &mut W,
    image: &DynamicImage,
    config: &ITerm2Config,
) -> io::Result<()> {
    let iterm2_data = encode_iterm2(image, config);
    writer.write_all(iterm2_data.as_bytes())?;
    writer.flush()
}

/// Render an image using iTerm2 protocol for terminal display
pub fn render_iterm2_image(image: &DynamicImage, max_width: u32, max_height: u32) -> String {
    let config = ITerm2Config {
        max_width,
        max_height,
        ..Default::default()
    };
    encode_iterm2(image, &config)
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
    fn test_iterm2_config_default() {
        let config = ITerm2Config::default();
        assert_eq!(config.max_width, 800);
        assert_eq!(config.max_height, 600);
        assert_eq!(config.width, 0);
        assert_eq!(config.height, 0);
        assert!(config.preserve_aspect_ratio);
        assert!(config.use_png);
        assert!(config.inline);
        assert!(config.name.is_none());
    }

    #[test]
    fn test_encode_iterm2_basic() {
        let img = create_test_image(10, 10, Rgb([255, 0, 0]));
        let config = ITerm2Config::default();
        let iterm2 = encode_iterm2(&img, &config);

        // Should start with OSC 1337 and end with BEL
        assert!(iterm2.starts_with("\x1b]1337;File="));
        assert!(iterm2.ends_with("\x07"));

        // Should contain size and inline parameters
        assert!(iterm2.contains("size="));
        assert!(iterm2.contains("inline=1"));
    }

    #[test]
    fn test_encode_iterm2_empty_image() {
        let img = create_test_image(0, 0, Rgb([0, 0, 0]));
        let config = ITerm2Config::default();
        let iterm2 = encode_iterm2(&img, &config);
        assert!(iterm2.is_empty());
    }

    #[test]
    fn test_encode_iterm2_with_name() {
        let img = create_test_image(10, 10, Rgb([0, 255, 0]));
        let config = ITerm2Config {
            name: Some("test.png".to_string()),
            ..Default::default()
        };
        let iterm2 = encode_iterm2(&img, &config);
        assert!(iterm2.contains("name="));
    }

    #[test]
    fn test_encode_iterm2_with_dimensions() {
        let img = create_test_image(10, 10, Rgb([0, 0, 255]));
        let config = ITerm2Config {
            width: 40,
            height: 20,
            ..Default::default()
        };
        let iterm2 = encode_iterm2(&img, &config);
        assert!(iterm2.contains("width=40"));
        assert!(iterm2.contains("height=20"));
    }

    #[test]
    fn test_encode_iterm2_no_preserve_aspect() {
        let img = create_test_image(10, 10, Rgb([128, 128, 128]));
        let config = ITerm2Config {
            preserve_aspect_ratio: false,
            ..Default::default()
        };
        let iterm2 = encode_iterm2(&img, &config);
        assert!(iterm2.contains("preserveAspectRatio=0"));
    }

    #[test]
    fn test_encode_iterm2_jpeg() {
        let img = create_test_image(10, 10, Rgb([64, 64, 64]));
        let config = ITerm2Config {
            use_png: false,
            ..Default::default()
        };
        let iterm2 = encode_iterm2(&img, &config);

        // Should still produce valid output
        assert!(!iterm2.is_empty());
        assert!(iterm2.starts_with("\x1b]1337;File="));
    }

    #[test]
    fn test_encode_iterm2_scaling() {
        let img = create_test_image(2000, 1500, Rgb([255, 255, 0]));
        let config = ITerm2Config {
            max_width: 100,
            max_height: 100,
            ..Default::default()
        };
        let iterm2 = encode_iterm2(&img, &config);

        // Should produce output (image was scaled)
        assert!(!iterm2.is_empty());
        assert!(iterm2.starts_with("\x1b]1337;File="));
    }

    #[test]
    fn test_render_iterm2_image() {
        let img = create_test_image(50, 50, Rgb([255, 128, 64]));
        let iterm2 = render_iterm2_image(&img, 100, 100);
        assert!(!iterm2.is_empty());
        assert!(iterm2.starts_with("\x1b]1337;File="));
    }

    #[test]
    fn test_write_iterm2() {
        let img = create_test_image(10, 10, Rgb([0, 128, 255]));
        let config = ITerm2Config::default();
        let mut buffer = Vec::new();

        write_iterm2(&mut buffer, &img, &config).unwrap();

        assert!(!buffer.is_empty());
        assert!(buffer.starts_with(b"\x1b]1337;File="));
    }

    #[test]
    fn test_encode_iterm2_base64_data() {
        let img = create_test_image(5, 5, Rgb([100, 150, 200]));
        let config = ITerm2Config::default();
        let iterm2 = encode_iterm2(&img, &config);

        // Should have colon before base64 data
        assert!(iterm2.contains(':'));

        // Data after colon should be valid base64
        if let Some(idx) = iterm2.find(':') {
            let data_part = &iterm2[idx + 1..iterm2.len() - 1]; // exclude BEL
            assert!(STANDARD.decode(data_part).is_ok());
        }
    }
}
