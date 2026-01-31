//! Preview rendering (text, images, directory info, archives, and PDFs)

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use image::GenericImageView;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, FontSize, Resize, StatefulImage};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use tempfile::NamedTempFile;

/// Maximum depth for recursive directory size calculation (for performance)
const MAX_DIR_SIZE_DEPTH: u32 = 3;

/// Convert Unix timestamp to date string (YYYY-MM-DD)
fn unix_timestamp_to_date(secs: i64) -> String {
    const SECONDS_PER_DAY: i64 = 86400;
    const DAYS_IN_MONTH: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let mut days = secs / SECONDS_PER_DAY;
    let mut year = 1970i64;

    // Find year
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    // Find month and day
    let leap = is_leap_year(year);
    let mut month = 1;
    for (i, &d) in DAYS_IN_MONTH.iter().enumerate() {
        let days_in_month = if i == 1 && leap { 29 } else { d };
        if days < days_in_month {
            break;
        }
        days -= days_in_month;
        month += 1;
    }
    let day = days + 1;

    format!("{:04}-{:02}-{:02}", year, month, day)
}

/// Check if a year is a leap year
fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Lazy-initialized syntax set (100+ languages)
static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();

/// Lazy-initialized theme (base16-ocean.dark)
static THEME: OnceLock<Theme> = OnceLock::new();

/// Get the shared syntax set (lazy-initialized)
fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Get the shared theme (lazy-initialized)
fn get_theme() -> &'static Theme {
    THEME.get_or_init(|| {
        let ts = ThemeSet::load_defaults();
        ts.themes["base16-ocean.dark"].clone()
    })
}

/// A segment of styled text (text with color)
#[derive(Debug, Clone)]
pub struct StyledSegment {
    pub text: String,
    pub color: Color,
}

/// A line with syntax highlighting
#[derive(Debug, Clone)]
pub struct StyledLine {
    pub segments: Vec<StyledSegment>,
}

/// Maximum bytes to read for hex preview
const HEX_PREVIEW_MAX_BYTES: usize = 4096;

/// Number of bytes per line in hex preview
const HEX_BYTES_PER_LINE: usize = 16;

/// Maximum entries to display in archive preview
const ARCHIVE_MAX_ENTRIES: usize = 500;

/// Maximum length for archive entry names (prevent DoS from malicious archives)
const MAX_ENTRY_NAME_LEN: usize = 4096;

/// Get border style based on focus state
fn get_border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    }
}

/// Truncate archive entry name if too long
fn truncate_entry_name(name: String) -> String {
    if name.len() > MAX_ENTRY_NAME_LEN {
        format!("{}...", &name[..MAX_ENTRY_NAME_LEN - 3])
    } else {
        name
    }
}

/// Text preview content
pub struct TextPreview {
    pub lines: Vec<String>,
    /// Syntax-highlighted lines (None for plain text)
    pub styled_lines: Option<Vec<StyledLine>>,
    pub scroll: usize,
}

impl TextPreview {
    /// Create a new text preview without syntax highlighting
    pub fn new(content: &str) -> Self {
        let lines: Vec<String> = content.lines().map(String::from).collect();
        Self {
            lines,
            styled_lines: None,
            scroll: 0,
        }
    }

    /// Create a new text preview with syntax highlighting based on file extension
    pub fn with_highlighting(content: &str, path: &Path) -> Self {
        let lines: Vec<String> = content.lines().map(String::from).collect();
        let styled_lines = highlight_content(content, path);
        Self {
            lines,
            styled_lines,
            scroll: 0,
        }
    }
}

/// Perform syntax highlighting on content based on file extension
fn highlight_content(content: &str, path: &Path) -> Option<Vec<StyledLine>> {
    let ss = get_syntax_set();
    let theme = get_theme();

    // Detect syntax from file extension or first line (shebang)
    let syntax = path
        .extension()
        .and_then(|e| e.to_str())
        .and_then(|ext| ss.find_syntax_by_extension(ext))
        .or_else(|| ss.find_syntax_by_first_line(content.lines().next().unwrap_or("")))?;

    let mut h = HighlightLines::new(syntax, theme);
    let mut styled_lines = Vec::new();

    for line in LinesWithEndings::from(content) {
        let ranges = h.highlight_line(line, ss).ok()?;
        let segments = ranges
            .iter()
            .map(|(style, text)| {
                let color = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                StyledSegment {
                    text: text.to_string(),
                    color,
                }
            })
            .collect();
        styled_lines.push(StyledLine { segments });
    }

    Some(styled_lines)
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
    let start = preview.scroll;
    let end = (start + visible_height).min(preview.lines.len());

    let lines: Vec<Line> = if let Some(ref styled_lines) = preview.styled_lines {
        // Render with syntax highlighting
        styled_lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, styled_line)| {
                let line_num = start + i + 1;
                let mut spans = vec![Span::styled(
                    format!("{:4} ", line_num),
                    Style::default().fg(Color::DarkGray),
                )];
                for segment in &styled_line.segments {
                    spans.push(Span::styled(
                        segment.text.clone(),
                        Style::default().fg(segment.color),
                    ));
                }
                Line::from(spans)
            })
            .collect()
    } else {
        // Render plain text (fallback)
        preview.lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let line_num = start + i + 1;
                Line::from(vec![
                    Span::styled(
                        format!("{:4} ", line_num),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(line.as_str()),
                ])
            })
            .collect()
    };

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", title))
            .border_style(get_border_style(focused)),
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

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Directory Info ")
            .border_style(get_border_style(focused)),
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

/// Archive entry information
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// File/directory name (full path within archive)
    pub name: String,
    /// Size in bytes (0 for directories)
    pub size: u64,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Last modified time (optional)
    pub modified: Option<String>,
}

impl ArchiveEntry {
    /// Sort archive entries: directories first, then alphabetically by name
    pub fn sort_entries(entries: &mut [ArchiveEntry]) {
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });
    }
}

/// Archive preview content
pub struct ArchivePreview {
    /// Archive entries
    pub entries: Vec<ArchiveEntry>,
    /// Total uncompressed size
    pub total_size: u64,
    /// Number of files (not directories)
    pub file_count: usize,
    /// Scroll position
    pub scroll: usize,
}

impl ArchivePreview {
    /// Load archive preview from zip file
    pub fn load_zip(path: &Path) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let mut entries = Vec::new();
        let mut total_size = 0u64;
        let mut file_count = 0usize;

        for i in 0..archive.len().min(ARCHIVE_MAX_ENTRIES) {
            let entry = archive.by_index(i)?;
            let is_dir = entry.is_dir();
            let size = entry.size();
            let name = truncate_entry_name(entry.name().to_string());

            // Format modified time if available
            let modified = entry
                .last_modified()
                .map(|dt| format!("{:04}-{:02}-{:02}", dt.year(), dt.month(), dt.day()));

            if !is_dir {
                total_size += size;
                file_count += 1;
            }

            entries.push(ArchiveEntry {
                name,
                size,
                is_dir,
                modified,
            });
        }

        // Sort entries: directories first, then files, both alphabetically
        ArchiveEntry::sort_entries(&mut entries);

        Ok(Self {
            entries,
            total_size,
            file_count,
            scroll: 0,
        })
    }

    /// Load archive preview from tar.gz file
    pub fn load_tar_gz(path: &Path) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path)?;
        let decompressed = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decompressed);

        let mut entries = Vec::new();
        let mut total_size = 0u64;
        let mut file_count = 0usize;

        for (i, entry_result) in archive.entries()?.enumerate() {
            if i >= ARCHIVE_MAX_ENTRIES {
                break;
            }

            let entry = entry_result?;
            let header = entry.header();
            let is_dir = header.entry_type().is_dir();
            let size = header.size().unwrap_or(0);
            let name = truncate_entry_name(entry.path()?.to_string_lossy().to_string());

            // Format modified time if available
            let modified = header
                .mtime()
                .ok()
                .map(|mtime| unix_timestamp_to_date(mtime as i64));

            if !is_dir {
                total_size += size;
                file_count += 1;
            }

            entries.push(ArchiveEntry {
                name,
                size,
                is_dir,
                modified,
            });
        }

        // Sort entries: directories first, then files, both alphabetically
        ArchiveEntry::sort_entries(&mut entries);

        Ok(Self {
            entries,
            total_size,
            file_count,
            scroll: 0,
        })
    }

    /// Get visible line count
    pub fn line_count(&self) -> usize {
        self.entries.len() + 2 // +2 for header lines
    }
}

/// Render archive preview
pub fn render_archive_preview(
    frame: &mut Frame,
    preview: &ArchivePreview,
    area: Rect,
    title: &str,
    focused: bool,
) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let separator = "─".repeat(area.width.saturating_sub(4) as usize);

    let mut lines: Vec<Line> = Vec::new();

    // Header: archive info
    lines.push(Line::from(vec![Span::styled(
        format!(
            "  {} files, {}",
            preview.file_count,
            format_size(preview.total_size)
        ),
        Style::default().fg(Color::Cyan),
    )]));

    lines.push(Line::from(vec![Span::styled(
        format!("  {}", separator),
        Style::default().fg(Color::DarkGray),
    )]));

    // Skip header lines in scroll calculation
    let header_lines = 2;
    let content_start = preview.scroll.saturating_sub(header_lines);

    // Entry list
    for entry in preview
        .entries
        .iter()
        .skip(content_start)
        .take(visible_height.saturating_sub(header_lines))
    {
        let (icon, color) = if entry.is_dir {
            ("\u{f07b}", Color::Blue) // Folder icon
        } else {
            ("\u{f016}", Color::White) // File icon
        };

        let size_str = if entry.is_dir {
            String::new()
        } else {
            format_size(entry.size)
        };

        let date_str = entry.modified.as_deref().unwrap_or("");

        // Calculate name display width
        let max_name_width = area.width.saturating_sub(24) as usize;
        let display_name = if entry.name.len() > max_name_width {
            format!("{}...", &entry.name[..max_name_width.saturating_sub(3)])
        } else {
            entry.name.clone()
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", icon), Style::default().fg(color)),
            Span::styled(
                format!("{:<width$}", display_name, width = max_name_width),
                Style::default().fg(color),
            ),
            Span::styled(
                format!("{:>8}  ", size_str),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(date_str.to_string(), Style::default().fg(Color::DarkGray)),
        ]));
    }

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", title))
            .border_style(get_border_style(focused)),
    );

    frame.render_widget(widget, area);
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

/// Check if a file is a tar.gz archive (handles double extension)
pub fn is_tar_gz_file(path: &std::path::Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    path_str.ends_with(".tar.gz") || path_str.ends_with(".tgz")
}

/// Check if a file is an archive (zip or tar.gz)
pub fn is_archive_file(path: &std::path::Path) -> bool {
    // Check tar.gz first (has double extension)
    if is_tar_gz_file(path) {
        return true;
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(
        ext.as_deref(),
        Some("zip" | "jar" | "apk" | "ipa" | "xpi" | "epub")
    )
}

/// Check if a file is a PDF
pub fn is_pdf_file(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(ext.as_deref(), Some("pdf"))
}

/// Cached pdftoppm path detection
static PDFTOPPM_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Find pdftoppm executable path (lazy detection with caching)
pub fn find_pdftoppm() -> Option<&'static PathBuf> {
    PDFTOPPM_PATH
        .get_or_init(|| {
            let candidates = [
                "/usr/bin/pdftoppm",
                "/usr/local/bin/pdftoppm",
                "/opt/homebrew/bin/pdftoppm",
            ];
            for path in candidates {
                let p = PathBuf::from(path);
                if p.exists() {
                    return Some(p);
                }
            }
            // fallback: which pdftoppm
            std::process::Command::new("which")
                .arg("pdftoppm")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| PathBuf::from(s.trim()))
                .filter(|p| p.exists())
        })
        .as_ref()
}

/// Cached pdfinfo path detection
static PDFINFO_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Find pdfinfo executable path (lazy detection with caching)
fn find_pdfinfo() -> Option<&'static PathBuf> {
    PDFINFO_PATH
        .get_or_init(|| {
            let candidates = [
                "/usr/bin/pdfinfo",
                "/usr/local/bin/pdfinfo",
                "/opt/homebrew/bin/pdfinfo",
            ];
            for path in candidates {
                let p = PathBuf::from(path);
                if p.exists() {
                    return Some(p);
                }
            }
            // fallback: which pdfinfo
            std::process::Command::new("which")
                .arg("pdfinfo")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| PathBuf::from(s.trim()))
                .filter(|p| p.exists())
        })
        .as_ref()
}

/// Get total page count from PDF using pdfinfo
fn get_pdf_page_count(path: &Path) -> anyhow::Result<usize> {
    let pdfinfo = find_pdfinfo().ok_or_else(|| anyhow::anyhow!("pdfinfo not found"))?;

    let output = std::process::Command::new(pdfinfo).arg(path).output()?;

    if !output.status.success() {
        anyhow::bail!("pdfinfo failed");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.starts_with("Pages:") {
            let count_str = line.trim_start_matches("Pages:").trim();
            return count_str
                .parse::<usize>()
                .map_err(|_| anyhow::anyhow!("Failed to parse page count"));
        }
    }

    anyhow::bail!("Pages not found in pdfinfo output")
}

/// PDF preview content
pub struct PdfPreview {
    /// Original PDF file path
    pub path: PathBuf,
    /// Current page number (1-indexed)
    pub current_page: usize,
    /// Total number of pages
    pub total_pages: usize,
    /// Rendered image preview
    pub image: ImagePreview,
    /// Temporary file holding the rendered page image (auto-cleanup on drop)
    _temp_file: NamedTempFile,
}

impl PdfPreview {
    /// Load PDF preview for a specific page
    pub fn load(path: &Path, page: usize, picker: &mut Picker) -> anyhow::Result<Self> {
        let pdftoppm = find_pdftoppm()
            .ok_or_else(|| anyhow::anyhow!("PDF preview requires pdftoppm (poppler-utils)"))?;

        // Get total page count
        let total_pages = get_pdf_page_count(path)?;

        // Clamp page to valid range
        let page = page.clamp(1, total_pages);

        // Create temporary file for the rendered image
        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path();

        // Get the base path without extension for pdftoppm output
        let temp_base = temp_path.with_extension("");

        // Run pdftoppm to render the page
        // pdftoppm -png -f <page> -l <page> -singlefile -r 150 input.pdf output_prefix
        let status = std::process::Command::new(pdftoppm)
            .arg("-png")
            .arg("-f")
            .arg(page.to_string())
            .arg("-l")
            .arg(page.to_string())
            .arg("-singlefile")
            .arg("-r")
            .arg("150")
            .arg(path)
            .arg(&temp_base)
            .status()?;

        if !status.success() {
            anyhow::bail!("pdftoppm failed to render page");
        }

        // pdftoppm creates output_prefix.png
        let output_path = temp_base.with_extension("png");

        if !output_path.exists() {
            anyhow::bail!("pdftoppm did not create output image");
        }

        // Load the rendered image
        let image = ImagePreview::load(&output_path, picker)?;

        // Clean up the output file (we'll store it in the temp_file for auto-cleanup)
        // Actually, we need to keep the output file, so let's rename it to temp_path
        std::fs::rename(&output_path, temp_path)?;

        Ok(Self {
            path: path.to_path_buf(),
            current_page: page,
            total_pages,
            image,
            _temp_file: temp_file,
        })
    }

    /// Navigate to a different page
    pub fn go_to_page(&mut self, page: usize, picker: &mut Picker) -> anyhow::Result<()> {
        let page = page.clamp(1, self.total_pages);

        if page == self.current_page {
            return Ok(());
        }

        // Create a new PdfPreview for the target page and update self
        let new_preview = PdfPreview::load(&self.path, page, picker)?;
        *self = new_preview;

        Ok(())
    }

    /// Go to the previous page
    pub fn prev_page(&mut self, picker: &mut Picker) -> anyhow::Result<()> {
        if self.current_page > 1 {
            self.go_to_page(self.current_page - 1, picker)
        } else {
            Ok(())
        }
    }

    /// Go to the next page
    pub fn next_page(&mut self, picker: &mut Picker) -> anyhow::Result<()> {
        if self.current_page < self.total_pages {
            self.go_to_page(self.current_page + 1, picker)
        } else {
            Ok(())
        }
    }
}

/// Render PDF preview
pub fn render_pdf_preview(
    frame: &mut Frame,
    pdf: &mut PdfPreview,
    area: Rect,
    title: &str,
    focused: bool,
    font_size: FontSize,
) {
    // Title with page info and navigation hint
    let full_title = format!(
        " {} ({}/{}) [/] prev/next ",
        title, pdf.current_page, pdf.total_pages
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .title(full_title)
        .border_style(get_border_style(focused));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Calculate centered area for the image
    let centered_area =
        calculate_centered_image_area(inner_area, pdf.image.width, pdf.image.height, font_size);

    // Render image using ratatui-image's StatefulImage widget
    let image_widget = StatefulImage::default().resize(Resize::Scale(None));
    frame.render_stateful_widget(image_widget, centered_area, &mut pdf.image.protocol);
}

/// Git diff preview content
pub struct DiffPreview {
    /// The diff data
    pub diff: crate::git::FileDiff,
    /// Scroll position
    pub scroll: usize,
}

impl DiffPreview {
    /// Create a new diff preview
    pub fn new(diff: crate::git::FileDiff) -> Self {
        Self { diff, scroll: 0 }
    }

    /// Get the total number of lines
    pub fn line_count(&self) -> usize {
        self.diff.lines.len()
    }
}

/// Render git diff preview
pub fn render_diff_preview(
    frame: &mut Frame,
    preview: &DiffPreview,
    area: Rect,
    title: &str,
    focused: bool,
) {
    use crate::git::DiffLine;

    let visible_height = area.height.saturating_sub(2) as usize;
    let start = preview.scroll;
    let end = (start + visible_height).min(preview.diff.lines.len());

    let lines: Vec<Line> = preview.diff.lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, diff_line)| {
            let line_num = start + i + 1;
            let (content, style) = match diff_line {
                DiffLine::Added(text) => (format!("+{}", text), Style::default().fg(Color::Green)),
                DiffLine::Removed(text) => (format!("-{}", text), Style::default().fg(Color::Red)),
                DiffLine::Context(text) => {
                    (format!(" {}", text), Style::default().fg(Color::DarkGray))
                }
                DiffLine::HunkHeader(text) => (
                    text.clone(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                DiffLine::Other(text) => (text.clone(), Style::default().fg(Color::DarkGray)),
            };

            Line::from(vec![
                Span::styled(
                    format!("{:4} ", line_num),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(content, style),
            ])
        })
        .collect();

    // Title with additions/deletions info
    let full_title = format!(
        " {} (+{} -{}) ",
        title, preview.diff.additions, preview.diff.deletions
    );

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(full_title)
            .border_style(get_border_style(focused)),
    );

    frame.render_widget(widget, area);
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
    // is_archive_file tests
    // =========================================================================

    #[test]
    fn test_is_archive_file_zip_variants() {
        // Standard zip
        assert!(is_archive_file(Path::new("file.zip")));

        // Java archive (jar)
        assert!(is_archive_file(Path::new("library.jar")));

        // Android package (apk)
        assert!(is_archive_file(Path::new("app.apk")));

        // iOS package (ipa)
        assert!(is_archive_file(Path::new("app.ipa")));

        // Firefox extension (xpi)
        assert!(is_archive_file(Path::new("addon.xpi")));

        // E-book (epub)
        assert!(is_archive_file(Path::new("book.epub")));
    }

    #[test]
    fn test_is_archive_file_case_insensitive() {
        // Uppercase
        assert!(is_archive_file(Path::new("FILE.ZIP")));
        assert!(is_archive_file(Path::new("LIBRARY.JAR")));
        assert!(is_archive_file(Path::new("APP.APK")));

        // Mixed case
        assert!(is_archive_file(Path::new("File.Zip")));
        assert!(is_archive_file(Path::new("Library.Jar")));
    }

    #[test]
    fn test_is_archive_file_non_archive() {
        // Text files
        assert!(!is_archive_file(Path::new("file.txt")));
        assert!(!is_archive_file(Path::new("file.md")));
        assert!(!is_archive_file(Path::new("file.rs")));

        // Image files
        assert!(!is_archive_file(Path::new("image.png")));
        assert!(!is_archive_file(Path::new("image.jpg")));

        // Unsupported archive formats (7z, rar, plain tar)
        assert!(!is_archive_file(Path::new("file.7z")));
        assert!(!is_archive_file(Path::new("file.rar")));
        assert!(!is_archive_file(Path::new("file.tar")));
    }

    // =========================================================================
    // is_tar_gz_file tests
    // =========================================================================

    #[test]
    fn test_is_tar_gz_file() {
        // Standard tar.gz
        assert!(is_tar_gz_file(Path::new("file.tar.gz")));

        // tgz extension
        assert!(is_tar_gz_file(Path::new("file.tgz")));

        // Case insensitive
        assert!(is_tar_gz_file(Path::new("FILE.TAR.GZ")));
        assert!(is_tar_gz_file(Path::new("FILE.TGZ")));
        assert!(is_tar_gz_file(Path::new("File.Tar.Gz")));
    }

    #[test]
    fn test_is_tar_gz_file_is_archive() {
        // tar.gz files should be recognized as archives
        assert!(is_archive_file(Path::new("file.tar.gz")));
        assert!(is_archive_file(Path::new("file.tgz")));
    }

    #[test]
    fn test_is_tar_gz_file_non_tar_gz() {
        // Plain tar is not tar.gz
        assert!(!is_tar_gz_file(Path::new("file.tar")));

        // gz without tar is not tar.gz
        assert!(!is_tar_gz_file(Path::new("file.gz")));

        // Other archives
        assert!(!is_tar_gz_file(Path::new("file.zip")));
    }

    #[test]
    fn test_is_archive_file_no_extension() {
        // Files without extension
        assert!(!is_archive_file(Path::new("Makefile")));
        assert!(!is_archive_file(Path::new("README")));
    }

    // =========================================================================
    // ArchiveEntry tests
    // =========================================================================

    #[test]
    fn test_archive_entry_struct_file() {
        let entry = ArchiveEntry {
            name: "src/main.rs".to_string(),
            size: 1024,
            is_dir: false,
            modified: Some("2024-01-15".to_string()),
        };

        assert_eq!(entry.name, "src/main.rs");
        assert_eq!(entry.size, 1024);
        assert!(!entry.is_dir);
        assert_eq!(entry.modified, Some("2024-01-15".to_string()));
    }

    #[test]
    fn test_archive_entry_struct_directory() {
        let entry = ArchiveEntry {
            name: "src/".to_string(),
            size: 0,
            is_dir: true,
            modified: None,
        };

        assert_eq!(entry.name, "src/");
        assert_eq!(entry.size, 0);
        assert!(entry.is_dir);
        assert!(entry.modified.is_none());
    }

    // =========================================================================
    // ArchivePreview tests
    // =========================================================================

    #[test]
    fn test_archive_preview_line_count() {
        let preview = ArchivePreview {
            entries: vec![
                ArchiveEntry {
                    name: "file1.txt".to_string(),
                    size: 100,
                    is_dir: false,
                    modified: None,
                },
                ArchiveEntry {
                    name: "file2.txt".to_string(),
                    size: 200,
                    is_dir: false,
                    modified: None,
                },
            ],
            total_size: 300,
            file_count: 2,
            scroll: 0,
        };

        // line_count = entries.len() + 2 (for header lines)
        assert_eq!(preview.line_count(), 4);
    }

    #[test]
    fn test_archive_preview_empty() {
        let preview = ArchivePreview {
            entries: vec![],
            total_size: 0,
            file_count: 0,
            scroll: 0,
        };

        // Even empty archive has 2 header lines
        assert_eq!(preview.line_count(), 2);
    }

    // =========================================================================
    // is_pdf_file tests
    // =========================================================================

    #[test]
    fn test_is_pdf_file() {
        assert!(is_pdf_file(Path::new("document.pdf")));
        assert!(is_pdf_file(Path::new("DOCUMENT.PDF")));
        assert!(is_pdf_file(Path::new("Document.Pdf")));
        assert!(is_pdf_file(Path::new("/path/to/file.pdf")));
    }

    #[test]
    fn test_is_pdf_file_non_pdf() {
        assert!(!is_pdf_file(Path::new("document.txt")));
        assert!(!is_pdf_file(Path::new("document.doc")));
        assert!(!is_pdf_file(Path::new("document.docx")));
        assert!(!is_pdf_file(Path::new("image.png")));
        assert!(!is_pdf_file(Path::new("no_extension")));
    }

    // =========================================================================
    // find_pdftoppm tests
    // =========================================================================

    #[test]
    fn test_find_pdftoppm_returns_consistent() {
        // Call twice to verify OnceLock caching works
        let result1 = find_pdftoppm();
        let result2 = find_pdftoppm();

        // Both calls should return the same result
        assert_eq!(result1.is_some(), result2.is_some());
        if let (Some(p1), Some(p2)) = (result1, result2) {
            assert_eq!(p1, p2);
        }
    }

    #[test]
    fn test_find_pdftoppm_path_exists_if_found() {
        if let Some(path) = find_pdftoppm() {
            assert!(path.exists(), "pdftoppm path should exist");
            assert!(
                path.to_string_lossy().contains("pdftoppm"),
                "Path should contain pdftoppm"
            );
        }
    }
}
