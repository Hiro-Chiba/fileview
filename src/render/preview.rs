//! Preview rendering (text, images, and directory info)

use std::path::Path;

use image::GenericImageView;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, FontSize, Resize, StatefulImage};

/// Maximum depth for recursive directory size calculation (for performance)
const MAX_DIR_SIZE_DEPTH: u32 = 3;

/// Maximum bytes to read for hex preview
const HEX_PREVIEW_MAX_BYTES: usize = 4096;

/// Number of bytes per line in hex preview
const HEX_BYTES_PER_LINE: usize = 16;

/// Text preview content
pub struct TextPreview {
    pub lines: Vec<String>,
    pub scroll: usize,
}

impl TextPreview {
    pub fn new(content: &str) -> Self {
        let lines: Vec<String> = content.lines().map(String::from).collect();
        Self { lines, scroll: 0 }
    }
}

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

/// Directory information for preview
#[derive(Debug, Clone)]
pub struct DirectoryInfo {
    /// Directory name
    pub name: String,
    /// Number of files
    pub file_count: usize,
    /// Number of subdirectories
    pub dir_count: usize,
    /// Number of hidden items
    pub hidden_count: usize,
    /// Total size in bytes
    pub total_size: u64,
}

impl DirectoryInfo {
    /// Compute directory info from path
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        let mut file_count = 0;
        let mut dir_count = 0;
        let mut hidden_count = 0;
        let mut total_size = 0u64;

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_name = entry.file_name().to_string_lossy().to_string();
                let is_hidden = entry_name.starts_with('.');

                if is_hidden {
                    hidden_count += 1;
                }

                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        file_count += 1;
                        total_size += metadata.len();
                    } else if metadata.is_dir() {
                        dir_count += 1;
                        // Optionally calculate subdirectory size (can be slow for large dirs)
                        if let Ok(dir_size) = calculate_dir_size(&entry.path()) {
                            total_size += dir_size;
                        }
                    }
                }
            }
        }

        Ok(Self {
            name,
            file_count,
            dir_count,
            hidden_count,
            total_size,
        })
    }
}

/// Calculate total size of a directory (recursive, with depth limit)
fn calculate_dir_size(path: &Path) -> anyhow::Result<u64> {
    calculate_dir_size_recursive(path, 0, MAX_DIR_SIZE_DEPTH as usize)
}

fn calculate_dir_size_recursive(
    path: &Path,
    depth: usize,
    max_depth: usize,
) -> anyhow::Result<u64> {
    if depth > max_depth {
        return Ok(0);
    }

    let mut total = 0u64;

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total += metadata.len();
                } else if metadata.is_dir() {
                    if let Ok(sub_size) =
                        calculate_dir_size_recursive(&entry.path(), depth + 1, max_depth)
                    {
                        total += sub_size;
                    }
                }
            }
        }
    }

    Ok(total)
}

/// Format bytes as human-readable string
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Render text preview
pub fn render_text_preview(
    frame: &mut Frame,
    preview: &TextPreview,
    area: Rect,
    title: &str,
    focused: bool,
) {
    let visible_height = area.height.saturating_sub(2) as usize;

    let lines: Vec<Line> = preview
        .lines
        .iter()
        .skip(preview.scroll)
        .take(visible_height)
        .enumerate()
        .map(|(i, line)| {
            let line_num = preview.scroll + i + 1;
            Line::from(vec![
                Span::styled(
                    format!("{:4} ", line_num),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(line.as_str()),
            ])
        })
        .collect();

    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", title))
            .border_style(border_style),
    );

    frame.render_widget(widget, area);
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
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ({}x{}) ", title, img.width, img.height))
        .border_style(border_style);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Calculate centered area for the image
    let centered_area = calculate_centered_image_area(inner_area, img.width, img.height, font_size);

    // Render image using ratatui-image's StatefulImage widget
    // Use Resize::Scale to ensure the image fills the centered area
    let image_widget = StatefulImage::default().resize(Resize::Scale(None));
    frame.render_stateful_widget(image_widget, centered_area, &mut img.protocol);
}

/// Render directory info preview
pub fn render_directory_info(frame: &mut Frame, info: &DirectoryInfo, area: Rect, focused: bool) {
    let separator = "─".repeat(area.width.saturating_sub(4) as usize);

    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("  \u{f07b} {}", info.name), // Folder icon
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            format!("  {}", separator),
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Files:        ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", info.file_count),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Directories:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", info.dir_count),
                Style::default().fg(Color::Blue),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Hidden:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", info.hidden_count),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Total Size:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format_size(info.total_size),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Directory Info ")
            .border_style(border_style),
    );

    frame.render_widget(widget, area);
}

/// Hex preview content for binary files
pub struct HexPreview {
    /// Raw bytes
    pub bytes: Vec<u8>,
    /// File size
    pub size: u64,
    /// Scroll position (in lines)
    pub scroll: usize,
}

impl HexPreview {
    /// Load hex preview from file (limited to first 4KB)
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        use std::io::Read;

        let metadata = std::fs::metadata(path)?;
        let size = metadata.len();

        let mut file = std::fs::File::open(path)?;
        let mut bytes = vec![0u8; HEX_PREVIEW_MAX_BYTES.min(size as usize)];
        file.read_exact(&mut bytes)?;

        Ok(Self {
            bytes,
            size,
            scroll: 0,
        })
    }

    /// Get the number of lines in the hex dump
    pub fn line_count(&self) -> usize {
        self.bytes.len().div_ceil(HEX_BYTES_PER_LINE)
    }
}

/// Render hex preview (xxd-style)
pub fn render_hex_preview(
    frame: &mut Frame,
    preview: &HexPreview,
    area: Rect,
    title: &str,
    focused: bool,
) {
    let visible_height = area.height.saturating_sub(2) as usize;

    let lines: Vec<Line> = preview
        .bytes
        .chunks(HEX_BYTES_PER_LINE)
        .enumerate()
        .skip(preview.scroll)
        .take(visible_height)
        .map(|(i, chunk)| {
            let offset = (preview.scroll + i) * HEX_BYTES_PER_LINE;
            render_hex_line(offset, chunk)
        })
        .collect();

    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let size_str = format_size(preview.size);
    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ({}) ", title, size_str))
            .border_style(border_style),
    );

    frame.render_widget(widget, area);
}

/// Render a single hex dump line
fn render_hex_line(offset: usize, bytes: &[u8]) -> Line<'static> {
    let mut spans = Vec::new();

    // Offset (8 hex digits)
    spans.push(Span::styled(
        format!("{:08x}: ", offset),
        Style::default().fg(Color::DarkGray),
    ));

    // Hex bytes (groups of 2, 8 groups total)
    for (i, byte) in bytes.iter().enumerate() {
        let color = if byte.is_ascii_graphic() || *byte == b' ' {
            Color::Green
        } else if *byte == 0 {
            Color::DarkGray
        } else {
            Color::Yellow
        };

        spans.push(Span::styled(
            format!("{:02x}", byte),
            Style::default().fg(color),
        ));

        // Add space after each byte, extra space after 8 bytes
        if i == 7 {
            spans.push(Span::raw("  "));
        } else if i < bytes.len() - 1 {
            spans.push(Span::raw(" "));
        }
    }

    // Pad if less than full line
    if bytes.len() < HEX_BYTES_PER_LINE {
        let missing = HEX_BYTES_PER_LINE - bytes.len();
        let padding = if bytes.len() <= 8 {
            missing * 3 + 1 // +1 for the extra space at position 8
        } else {
            missing * 3
        };
        spans.push(Span::raw(" ".repeat(padding)));
    }

    spans.push(Span::raw("  "));

    // ASCII representation
    let ascii: String = bytes
        .iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '.'
            }
        })
        .collect();

    spans.push(Span::styled(ascii, Style::default().fg(Color::Cyan)));

    Line::from(spans)
}

/// Check if a file is likely a binary file (not text, not image)
pub fn is_binary_file(path: &Path) -> bool {
    // If it's a known text or image file, it's not binary
    if is_text_file(path) || is_image_file(path) {
        return false;
    }

    // Check if file has no extension or unknown extension
    let ext = path.extension().and_then(|e| e.to_str());

    // Files without extension might be binary
    if ext.is_none() {
        // Try to detect by reading first few bytes
        if let Ok(mut file) = std::fs::File::open(path) {
            use std::io::Read;
            let mut buffer = [0u8; 512];
            if let Ok(n) = file.read(&mut buffer) {
                // Check for null bytes or high concentration of non-printable chars
                let non_printable = buffer[..n]
                    .iter()
                    .filter(|&&b| b == 0 || (b < 32 && b != b'\n' && b != b'\r' && b != b'\t'))
                    .count();
                return non_printable > n / 10; // More than 10% non-printable
            }
        }
        return false;
    }

    // Known binary extensions
    matches!(
        ext.map(|e| e.to_lowercase()).as_deref(),
        Some(
            "exe"
                | "dll"
                | "so"
                | "dylib"
                | "a"
                | "o"
                | "obj"
                | "bin"
                | "dat"
                | "db"
                | "sqlite"
                | "class"
                | "pyc"
                | "pyo"
                | "wasm"
        )
    )
}

/// Check if a file is likely a text file
pub fn is_text_file(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(
        ext.as_deref(),
        Some(
            "txt"
                | "md"
                | "rs"
                | "py"
                | "js"
                | "ts"
                | "jsx"
                | "tsx"
                | "html"
                | "css"
                | "json"
                | "toml"
                | "yaml"
                | "yml"
                | "xml"
                | "sh"
                | "bash"
                | "zsh"
                | "c"
                | "h"
                | "cpp"
                | "hpp"
                | "java"
                | "go"
                | "rb"
                | "php"
                | "sql"
                | "vim"
                | "lua"
                | "el"
                | "lisp"
                | "scm"
                | "hs"
                | "ml"
                | "ex"
                | "exs"
                | "erl"
                | "clj"
                | "swift"
                | "kt"
                | "scala"
                | "r"
                | "jl"
                | "pl"
                | "pm"
                | "awk"
                | "sed"
                | "conf"
                | "cfg"
                | "ini"
                | "env"
                | "gitignore"
                | "dockerignore"
                | "makefile"
                | "cmake"
        )
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // format_size tests
    // =========================================================================

    #[test]
    fn test_format_size_bytes() {
        // Edge cases: 0 bytes
        assert_eq!(format_size(0), "0 B");

        // Small values
        assert_eq!(format_size(100), "100 B");

        // Just below 1 KB
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kb_mb_gb() {
        // Exactly 1 KB
        assert_eq!(format_size(1024), "1.0 KB");

        // Exactly 1 MB
        assert_eq!(format_size(1024 * 1024), "1.0 MB");

        // Exactly 1 GB
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");

        // Larger than 1 TB
        assert_eq!(format_size(1024 * 1024 * 1024 * 1024), "1.0 TB");

        // Mixed sizes
        assert_eq!(format_size(1536), "1.5 KB"); // 1.5 KB
        assert_eq!(format_size(2 * 1024 * 1024 + 512 * 1024), "2.5 MB"); // 2.5 MB
    }

    // =========================================================================
    // calculate_centered_image_area tests
    // =========================================================================

    #[test]
    fn test_centered_image_area_square() {
        // Square image in square area - should fill area
        let area = Rect::new(0, 0, 100, 50);
        let font_size: FontSize = (10, 20); // 10px wide, 20px tall per cell

        // Area in pixels: 1000x1000
        // Image: 500x500 (square)
        // Scale: min(1000/500, 1000/500) = 2.0
        // Scaled: 1000x1000 pixels -> 100x50 cells
        let result = calculate_centered_image_area(area, 500, 500, font_size);

        // Image should be centered
        assert_eq!(result.x, 0);
        assert_eq!(result.y, 0);
        assert_eq!(result.width, 100);
        assert_eq!(result.height, 50);
    }

    #[test]
    fn test_centered_image_area_wide() {
        // Wide image - should have padding on top/bottom
        let area = Rect::new(0, 0, 100, 50);
        let font_size: FontSize = (10, 20); // 10px wide, 20px tall per cell

        // Area in pixels: 1000x1000
        // Image: 1000x500 (wide)
        // Scale: min(1000/1000, 1000/500) = 1.0
        // Scaled: 1000x500 pixels -> 100x25 cells
        // Padding: (50-25)/2 = 12 cells top/bottom
        let result = calculate_centered_image_area(area, 1000, 500, font_size);

        // Should be horizontally centered with vertical padding
        assert_eq!(result.x, 0);
        assert!(result.y > 0); // Should have top padding
        assert_eq!(result.width, 100);
        assert!(result.height <= 50);

        // Verify vertical centering
        let expected_height = 25u16;
        let expected_y = (50 - expected_height) / 2;
        assert_eq!(result.y, expected_y);
        assert_eq!(result.height, expected_height);
    }
}
