//! Hex preview for binary files

use std::path::Path;

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::common::{format_size, get_border_style, HEX_BYTES_PER_LINE, HEX_PREVIEW_MAX_BYTES};

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
        let n = file.read(&mut bytes)?;
        bytes.truncate(n);

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

    let size_str = format_size(preview.size);
    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ({}) ", title, size_str))
            .border_style(get_border_style(focused)),
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

/// Check if a file is likely a binary file (not text, not image, not archive)
pub fn is_binary_file(path: &Path) -> bool {
    use super::archive::is_archive_file;
    use super::image::is_image_file;
    use super::text::is_text_file;

    // If it's a known text, image, or archive file, it's not binary
    if is_text_file(path) || is_image_file(path) || is_archive_file(path) {
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
