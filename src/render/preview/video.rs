//! Video preview with thumbnail and metadata

use std::path::PathBuf;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use ratatui_image::{FontSize, Resize, StatefulImage};

use super::common::get_border_style;
use super::image::{calculate_centered_image_area, ImagePreview};

/// Video preview content
pub struct VideoPreview {
    /// Video metadata
    pub metadata: crate::app::VideoMetadata,
    /// Thumbnail image preview (if available)
    pub thumbnail: Option<ImagePreview>,
    /// Error message if thumbnail generation failed
    pub thumbnail_error: Option<String>,
    /// Path to the video file
    pub path: PathBuf,
}

impl VideoPreview {
    /// Create a new video preview without thumbnail (metadata only)
    pub fn new(path: &std::path::Path, metadata: crate::app::VideoMetadata) -> Self {
        Self {
            metadata,
            thumbnail: None,
            thumbnail_error: None,
            path: path.to_path_buf(),
        }
    }

    /// Check if thumbnail is ready
    pub fn has_thumbnail(&self) -> bool {
        self.thumbnail.is_some()
    }
}

/// Render video preview with thumbnail and metadata
pub fn render_video_preview(
    frame: &mut Frame,
    preview: &mut VideoPreview,
    area: Rect,
    title: &str,
    focused: bool,
    font_size: FontSize,
) {
    let meta = &preview.metadata;

    // Build title with duration
    let full_title = format!(" {} [{}] ", title, meta.format_duration());

    let block = Block::default()
        .borders(Borders::ALL)
        .title(full_title)
        .border_style(get_border_style(focused));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Split area: top for thumbnail, bottom for metadata
    let has_thumbnail = preview.thumbnail.is_some();
    let metadata_height = 8u16; // Lines needed for metadata

    if has_thumbnail && inner_area.height > metadata_height + 4 {
        // Layout with thumbnail and metadata
        let thumbnail_height = inner_area.height.saturating_sub(metadata_height);
        let thumbnail_area = Rect::new(
            inner_area.x,
            inner_area.y,
            inner_area.width,
            thumbnail_height,
        );
        let metadata_area = Rect::new(
            inner_area.x,
            inner_area.y + thumbnail_height,
            inner_area.width,
            metadata_height,
        );

        // Render thumbnail
        if let Some(ref mut thumb) = preview.thumbnail {
            let centered_area =
                calculate_centered_image_area(thumbnail_area, thumb.width, thumb.height, font_size);
            let image_widget = StatefulImage::default().resize(Resize::Scale(None));
            frame.render_stateful_widget(image_widget, centered_area, &mut thumb.protocol);
        }

        // Render metadata below
        render_video_metadata(
            frame,
            meta,
            metadata_area,
            preview.thumbnail_error.as_deref(),
        );
    } else {
        // Metadata only (no thumbnail or not enough space)
        render_video_metadata(frame, meta, inner_area, preview.thumbnail_error.as_deref());
    }
}

/// Render video metadata information
fn render_video_metadata(
    frame: &mut Frame,
    meta: &crate::app::VideoMetadata,
    area: Rect,
    thumbnail_error: Option<&str>,
) {
    let separator = "─".repeat(area.width.saturating_sub(4) as usize);

    let mut lines = vec![
        Line::from(vec![Span::styled(
            format!("  {}", separator),
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(""),
        // Duration
        Line::from(vec![
            Span::styled(
                "  \u{f144} Duration:   ",
                Style::default().fg(Color::DarkGray),
            ), // Play icon
            Span::styled(
                meta.format_duration(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        // Resolution
        Line::from(vec![
            Span::styled(
                "  \u{f03e} Resolution: ",
                Style::default().fg(Color::DarkGray),
            ), // Screen icon
            Span::styled(meta.format_resolution(), Style::default().fg(Color::White)),
            if let Some(fps) = meta.frame_rate {
                Span::styled(
                    format!(" @ {:.2} fps", fps),
                    Style::default().fg(Color::DarkGray),
                )
            } else {
                Span::raw("")
            },
        ]),
        // Codec
        Line::from(vec![
            Span::styled(
                "  \u{f1c8} Codec:      ",
                Style::default().fg(Color::DarkGray),
            ), // File video icon
            Span::styled(&meta.codec, Style::default().fg(Color::Yellow)),
            if let Some(ref audio) = meta.audio_codec {
                Span::styled(format!(" / {}", audio), Style::default().fg(Color::Green))
            } else {
                Span::raw("")
            },
        ]),
        // File size
        Line::from(vec![
            Span::styled(
                "  \u{f0c7} Size:       ",
                Style::default().fg(Color::DarkGray),
            ), // Disk icon
            Span::styled(meta.format_size(), Style::default().fg(Color::White)),
            if let Some(bitrate) = meta.format_bitrate() {
                Span::styled(
                    format!(" ({})", bitrate),
                    Style::default().fg(Color::DarkGray),
                )
            } else {
                Span::raw("")
            },
        ]),
    ];

    // Show thumbnail error if present
    if let Some(err) = thumbnail_error {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  \u{f071} ", Style::default().fg(Color::Yellow)), // Warning icon
            Span::styled(
                format!("Thumbnail: {}", err),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, area);
}
