//! Custom preview from external command

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::common::get_border_style;

/// Custom preview content from external command
pub struct CustomPreview {
    /// Output lines from the command
    pub lines: Vec<String>,
    /// The command that was executed
    pub command: String,
    /// Scroll position
    pub scroll: usize,
}

impl CustomPreview {
    /// Execute a custom preview command and capture output
    ///
    /// The command template can use $f as a placeholder for the file path.
    pub fn execute(command_template: &str, file_path: &std::path::Path) -> anyhow::Result<Self> {
        use std::process::Command;

        // Expand $f placeholder
        let cmd = command_template.replace("$f", &file_path.display().to_string());

        // Execute command via shell
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", &cmd]).output()
        } else {
            Command::new("sh").args(["-c", &cmd]).output()
        }?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<String> = stdout.lines().map(String::from).collect();

        Ok(Self {
            lines,
            command: cmd,
            scroll: 0,
        })
    }

    /// Get the total number of lines
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

/// Render custom preview output
pub fn render_custom_preview(
    frame: &mut Frame,
    preview: &CustomPreview,
    area: Rect,
    title: &str,
    focused: bool,
) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let start = preview.scroll;
    let end = (start + visible_height).min(preview.lines.len());

    let lines: Vec<Line> = preview.lines[start..end]
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
        .collect();

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", title))
            .border_style(get_border_style(focused)),
    );

    frame.render_widget(widget, area);
}
