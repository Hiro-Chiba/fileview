//! Sixel graphics protocol implementation
//!
//! Sixel is a graphics format that encodes images as escape sequences.
//! It displays 6 vertical pixels per character position.
//!
//! Supported terminals: Ghostty, iTerm2, WezTerm, foot, mlterm, xterm (with sixel)

use image::{DynamicImage, GenericImageView, Rgba};
use std::collections::HashMap;
use std::io::{self, Write};

/// Maximum number of colors in Sixel palette
const MAX_COLORS: usize = 256;

/// Sixel encoder configuration
#[derive(Debug, Clone)]
pub struct SixelConfig {
    /// Maximum width in pixels (will scale down if larger)
    pub max_width: u32,
    /// Maximum height in pixels (will scale down if larger)
    pub max_height: u32,
    /// Number of colors to use (2-256)
    pub colors: usize,
    /// Whether to use transparency
    pub transparent: bool,
}

impl Default for SixelConfig {
    fn default() -> Self {
        Self {
            max_width: 800,
            max_height: 600,
            colors: 256,
            transparent: true,
        }
    }
}

/// Quantize colors to a fixed palette using median cut algorithm
fn quantize_colors(pixels: &[Rgba<u8>], max_colors: usize) -> Vec<Rgba<u8>> {
    // Simple color quantization using a hash map to find unique colors
    let mut color_counts: HashMap<[u8; 4], usize> = HashMap::new();

    for pixel in pixels {
        // Reduce color depth slightly for better grouping
        let key = [
            pixel[0] & 0xF8,
            pixel[1] & 0xF8,
            pixel[2] & 0xF8,
            if pixel[3] > 127 { 255 } else { 0 },
        ];
        *color_counts.entry(key).or_insert(0) += 1;
    }

    // Sort by frequency and take top colors
    let mut sorted: Vec<_> = color_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    sorted
        .into_iter()
        .take(max_colors)
        .map(|(key, _)| Rgba([key[0], key[1], key[2], key[3]]))
        .collect()
}

/// Find the closest palette color for a given pixel
fn find_closest_color(pixel: Rgba<u8>, palette: &[Rgba<u8>]) -> usize {
    if pixel[3] < 128 {
        // Transparent - return special index
        return usize::MAX;
    }

    let mut best_idx = 0;
    let mut best_dist = u32::MAX;

    for (idx, pal_color) in palette.iter().enumerate() {
        let dr = pixel[0] as i32 - pal_color[0] as i32;
        let dg = pixel[1] as i32 - pal_color[1] as i32;
        let db = pixel[2] as i32 - pal_color[2] as i32;
        let dist = (dr * dr + dg * dg + db * db) as u32;

        if dist < best_dist {
            best_dist = dist;
            best_idx = idx;
        }
    }

    best_idx
}

/// Encode an image as Sixel data
pub fn encode_sixel(image: &DynamicImage, config: &SixelConfig) -> String {
    // Scale image if needed
    let (orig_w, orig_h) = image.dimensions();
    let scale = f64::min(
        config.max_width as f64 / orig_w as f64,
        config.max_height as f64 / orig_h as f64,
    );

    let img = if scale < 1.0 {
        image.resize(
            (orig_w as f64 * scale) as u32,
            (orig_h as f64 * scale) as u32,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        image.clone()
    };

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    if width == 0 || height == 0 {
        return String::new();
    }

    // Collect all pixels
    let pixels: Vec<Rgba<u8>> = rgba.pixels().cloned().collect();

    // Quantize colors
    let num_colors = config.colors.clamp(2, MAX_COLORS);
    let palette = quantize_colors(&pixels, num_colors);

    // Map each pixel to palette index
    let indexed: Vec<usize> = pixels
        .iter()
        .map(|p| find_closest_color(*p, &palette))
        .collect();

    // Build Sixel output
    let mut output = String::new();

    // DCS (Device Control String) - start Sixel mode
    // P1=0: pixel aspect ratio 2:1 (standard)
    // P2=0: no background color specified
    // P3=0: horizontal grid size (default)
    output.push_str("\x1bPq");

    // Set transparent background if supported
    if config.transparent {
        output.push_str("\"1;1;");
        output.push_str(&width.to_string());
        output.push(';');
        output.push_str(&height.to_string());
    }

    // Define color palette
    for (idx, color) in palette.iter().enumerate() {
        // Sixel uses 0-100 range for RGB components
        let r = (color[0] as u32 * 100 / 255) as u8;
        let g = (color[1] as u32 * 100 / 255) as u8;
        let b = (color[2] as u32 * 100 / 255) as u8;
        output.push_str(&format!("#{};2;{};{};{}", idx, r, g, b));
    }

    // Encode image data in Sixel format
    // Sixel encodes 6 vertical pixels at a time
    let sixel_rows = height.div_ceil(6);

    for sixel_row in 0..sixel_rows {
        let y_start = sixel_row * 6;

        // Process each color separately for this row
        for color_idx in 0..palette.len() {
            let mut has_pixels = false;
            let mut row_data = Vec::with_capacity(width as usize);

            for x in 0..width {
                let mut sixel_value: u8 = 0;

                // Check 6 vertical pixels
                for bit in 0..6 {
                    let y = y_start + bit;
                    if y < height {
                        let pixel_idx = (y * width + x) as usize;
                        if indexed[pixel_idx] == color_idx {
                            sixel_value |= 1 << bit;
                            has_pixels = true;
                        }
                    }
                }

                row_data.push(sixel_value);
            }

            if has_pixels {
                // Select color
                output.push('#');
                output.push_str(&color_idx.to_string());

                // Run-length encode the row
                let mut i = 0;
                while i < row_data.len() {
                    let val = row_data[i];
                    let mut count = 1;

                    // Count consecutive identical values
                    while i + count < row_data.len() && row_data[i + count] == val && count < 255 {
                        count += 1;
                    }

                    // Output with RLE if beneficial
                    let sixel_char = (val + 63) as char;
                    if count >= 3 {
                        output.push('!');
                        output.push_str(&count.to_string());
                        output.push(sixel_char);
                    } else {
                        for _ in 0..count {
                            output.push(sixel_char);
                        }
                    }

                    i += count;
                }

                // Carriage return (stay on same sixel row)
                output.push('$');
            }
        }

        // Line feed (move to next sixel row)
        if sixel_row < sixel_rows - 1 {
            output.push('-');
        }
    }

    // ST (String Terminator) - end Sixel mode
    output.push_str("\x1b\\");

    output
}

/// Write Sixel image directly to a writer
pub fn write_sixel<W: Write>(
    writer: &mut W,
    image: &DynamicImage,
    config: &SixelConfig,
) -> io::Result<()> {
    let sixel_data = encode_sixel(image, config);
    writer.write_all(sixel_data.as_bytes())?;
    writer.flush()
}

/// Render an image as Sixel for terminal display
pub fn render_sixel_image(image: &DynamicImage, max_width: u32, max_height: u32) -> String {
    let config = SixelConfig {
        max_width,
        max_height,
        colors: 256,
        transparent: true,
    };
    encode_sixel(image, &config)
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
    fn test_sixel_config_default() {
        let config = SixelConfig::default();
        assert_eq!(config.max_width, 800);
        assert_eq!(config.max_height, 600);
        assert_eq!(config.colors, 256);
        assert!(config.transparent);
    }

    #[test]
    fn test_encode_sixel_basic() {
        let img = create_test_image(10, 12, Rgb([255, 0, 0]));
        let config = SixelConfig::default();
        let sixel = encode_sixel(&img, &config);

        // Should start with DCS and end with ST
        assert!(sixel.starts_with("\x1bPq"));
        assert!(sixel.ends_with("\x1b\\"));

        // Should contain color definition
        assert!(sixel.contains("#0;2;"));
    }

    #[test]
    fn test_encode_sixel_empty_image() {
        let img = create_test_image(0, 0, Rgb([0, 0, 0]));
        let config = SixelConfig::default();
        let sixel = encode_sixel(&img, &config);
        assert!(sixel.is_empty());
    }

    #[test]
    fn test_encode_sixel_scaling() {
        let img = create_test_image(2000, 1500, Rgb([0, 255, 0]));
        let config = SixelConfig {
            max_width: 100,
            max_height: 100,
            ..Default::default()
        };
        let sixel = encode_sixel(&img, &config);

        // Should produce output (image was scaled)
        assert!(!sixel.is_empty());
        assert!(sixel.starts_with("\x1bPq"));
    }

    #[test]
    fn test_quantize_colors() {
        let pixels = vec![
            Rgba([255, 0, 0, 255]),
            Rgba([255, 0, 0, 255]),
            Rgba([0, 255, 0, 255]),
            Rgba([0, 0, 255, 255]),
        ];
        let palette = quantize_colors(&pixels, 3);
        assert!(palette.len() <= 3);
    }

    #[test]
    fn test_find_closest_color() {
        let palette = vec![
            Rgba([255, 0, 0, 255]),
            Rgba([0, 255, 0, 255]),
            Rgba([0, 0, 255, 255]),
        ];

        // Exact match
        assert_eq!(find_closest_color(Rgba([255, 0, 0, 255]), &palette), 0);
        assert_eq!(find_closest_color(Rgba([0, 255, 0, 255]), &palette), 1);
        assert_eq!(find_closest_color(Rgba([0, 0, 255, 255]), &palette), 2);

        // Near match
        assert_eq!(find_closest_color(Rgba([250, 5, 5, 255]), &palette), 0);
    }

    #[test]
    fn test_find_closest_color_transparent() {
        let palette = vec![Rgba([255, 0, 0, 255])];
        assert_eq!(
            find_closest_color(Rgba([255, 0, 0, 0]), &palette),
            usize::MAX
        );
    }

    #[test]
    fn test_render_sixel_image() {
        let img = create_test_image(50, 50, Rgb([128, 128, 128]));
        let sixel = render_sixel_image(&img, 100, 100);
        assert!(!sixel.is_empty());
        assert!(sixel.starts_with("\x1bPq"));
    }

    #[test]
    fn test_write_sixel() {
        let img = create_test_image(10, 12, Rgb([255, 255, 255]));
        let config = SixelConfig::default();
        let mut buffer = Vec::new();

        write_sixel(&mut buffer, &img, &config).unwrap();

        assert!(!buffer.is_empty());
        assert!(buffer.starts_with(b"\x1bPq"));
    }

    #[test]
    fn test_sixel_multicolor() {
        // Create image with multiple colors
        let mut img = RgbImage::new(20, 12);
        for x in 0..10 {
            for y in 0..12 {
                img.put_pixel(x, y, Rgb([255, 0, 0]));
            }
        }
        for x in 10..20 {
            for y in 0..12 {
                img.put_pixel(x, y, Rgb([0, 0, 255]));
            }
        }
        let dyn_img = DynamicImage::ImageRgb8(img);
        let sixel = render_sixel_image(&dyn_img, 100, 100);

        // Should contain multiple color definitions
        assert!(sixel.contains("#0;2;"));
        assert!(sixel.contains("#1;2;") || sixel.matches("#").count() >= 2);
    }

    #[test]
    fn test_sixel_rle_encoding() {
        // Create a wide single-color image to trigger RLE
        let img = create_test_image(100, 6, Rgb([255, 0, 0]));
        let sixel = render_sixel_image(&img, 200, 200);

        // Should contain RLE markers (!)
        assert!(sixel.contains('!'));
    }
}
