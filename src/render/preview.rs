//! Preview rendering (text and images)

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

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

/// Image preview data (RGB pixels)
pub struct ImagePreview {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<(u8, u8, u8)>,
}

impl ImagePreview {
    /// Load image from file path
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let img = image::open(path)?;
        let rgb = img.to_rgb8();
        let width = rgb.width();
        let height = rgb.height();

        let pixels: Vec<(u8, u8, u8)> = rgb.pixels().map(|p| (p[0], p[1], p[2])).collect();

        Ok(Self {
            width,
            height,
            pixels,
        })
    }
}

/// Render text preview
pub fn render_text_preview(frame: &mut Frame, preview: &TextPreview, area: Rect, title: &str) {
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

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", title)),
    );

    frame.render_widget(widget, area);
}

/// Render image preview using half-block characters
pub fn render_image_preview(frame: &mut Frame, img: &ImagePreview, area: Rect, title: &str) {
    let img_width = area.width.saturating_sub(2) as u32;
    let img_height = (area.height.saturating_sub(2) * 2) as u32;

    let lines = render_image_to_lines(img, img_width, img_height);

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ({}x{}) ", title, img.width, img.height)),
    );

    frame.render_widget(widget, area);
}

/// Convert image to terminal lines using half-block rendering
fn render_image_to_lines(
    img: &ImagePreview,
    target_width: u32,
    target_height: u32,
) -> Vec<Line<'static>> {
    if target_width == 0 || target_height == 0 || img.width == 0 || img.height == 0 {
        return vec![Line::from("Image too small to display")];
    }

    // Terminal characters are roughly 2:1 (height:width ratio)
    let char_aspect = 2.0;

    let img_aspect = img.width as f32 / img.height as f32;
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
            let src_x = ((col as f32 / display_width as f32) * img.width as f32) as u32;
            let src_y_top = ((row as f32 * 2.0 / display_height as f32) * img.height as f32) as u32;
            let src_y_bottom =
                (((row as f32 * 2.0 + 1.0) / display_height as f32) * img.height as f32) as u32;

            let src_x = src_x.min(img.width - 1);
            let src_y_top = src_y_top.min(img.height - 1);
            let src_y_bottom = src_y_bottom.min(img.height - 1);

            let idx_top = (src_y_top * img.width + src_x) as usize;
            let idx_bottom = (src_y_bottom * img.width + src_x) as usize;

            let (r1, g1, b1) = img.pixels.get(idx_top).copied().unwrap_or((0, 0, 0));
            let (r2, g2, b2) = img.pixels.get(idx_bottom).copied().unwrap_or((0, 0, 0));

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

/// Check if a file is likely a text file
pub fn is_text_file(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(
        ext.as_deref(),
        Some(
            "txt" | "md" | "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "html" | "css" | "json"
                | "toml" | "yaml" | "yml" | "xml" | "sh" | "bash" | "zsh" | "c" | "h" | "cpp"
                | "hpp" | "java" | "go" | "rb" | "php" | "sql" | "vim" | "lua" | "el" | "lisp"
                | "scm" | "hs" | "ml" | "ex" | "exs" | "erl" | "clj" | "swift" | "kt" | "scala"
                | "r" | "jl" | "pl" | "pm" | "awk" | "sed" | "conf" | "cfg" | "ini" | "env"
                | "gitignore" | "dockerignore" | "makefile" | "cmake"
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
