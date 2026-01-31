//! Bulk rename dialog rendering

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::core::{AppState, ViewMode};

/// Render the bulk rename dialog
pub fn render_bulk_rename_dialog(frame: &mut Frame, state: &AppState) {
    let ViewMode::BulkRename {
        from_pattern,
        to_pattern,
        selected_field,
        cursor,
    } = &state.mode
    else {
        return;
    };

    let area = centered_rect(60, 12, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    // Create the dialog block
    let block = Block::default()
        .title(" Bulk Rename ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout for the dialog content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Info
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // From label
            Constraint::Length(1), // From input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // To label
            Constraint::Length(1), // To input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Help
        ])
        .margin(1)
        .split(inner);

    // Info line
    let info = format!("{} file(s) selected", state.selected_paths.len());
    let info_widget = Paragraph::new(info)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);
    frame.render_widget(info_widget, chunks[0]);

    // From pattern field
    let from_label = Paragraph::new("Match pattern:").style(if *selected_field == 0 {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    });
    frame.render_widget(from_label, chunks[2]);

    let from_value = render_input_field(from_pattern, *selected_field == 0, *cursor);
    frame.render_widget(from_value, chunks[3]);

    // To pattern field
    let to_label = Paragraph::new("Replace with:").style(if *selected_field == 1 {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    });
    frame.render_widget(to_label, chunks[5]);

    let to_value = render_input_field(to_pattern, *selected_field == 1, *cursor);
    frame.render_widget(to_value, chunks[6]);

    // Help line
    let help_spans = vec![
        Span::styled("Tab", Style::default().fg(Color::Cyan)),
        Span::raw(": switch field  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(": execute  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(": cancel"),
    ];
    let help = Paragraph::new(Line::from(help_spans)).alignment(Alignment::Center);
    frame.render_widget(help, chunks[8]);
}

/// Render an input field with cursor
fn render_input_field(value: &str, is_active: bool, cursor: usize) -> Paragraph<'static> {
    let style = if is_active {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let display = if is_active {
        // Show cursor
        let before: String = value.chars().take(cursor).collect();
        let cursor_char = value.chars().nth(cursor).unwrap_or(' ');
        let after: String = value.chars().skip(cursor + 1).collect();

        let spans = vec![
            Span::styled(before, style),
            Span::styled(
                cursor_char.to_string(),
                style.add_modifier(Modifier::REVERSED),
            ),
            Span::styled(after, style),
        ];
        Line::from(spans)
    } else {
        Line::from(Span::styled(value.to_string(), style))
    };

    Paragraph::new(display).style(style)
}

/// Create a centered rectangle
fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_width = (area.width as u32 * percent_x as u32 / 100) as u16;
    let popup_width = popup_width.max(40).min(area.width);

    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;

    Rect::new(area.x + x, area.y + y, popup_width, height.min(area.height))
}
