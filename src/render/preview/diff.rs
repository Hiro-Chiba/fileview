//! Git diff preview

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::common::get_border_style;

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
