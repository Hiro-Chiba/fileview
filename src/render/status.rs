//! Status bar and input popup rendering

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::core::{AppState, InputPurpose, PendingAction, ViewMode};

/// Render the status bar
pub fn render_status_bar(frame: &mut Frame, state: &AppState, total_entries: usize, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: message or help hint
    let message = state.message.as_deref().unwrap_or("? for help");
    let msg_widget = Paragraph::new(message).block(Block::default().borders(Borders::ALL));
    frame.render_widget(msg_widget, chunks[0]);

    // Right: stats
    let selected_count = state.selected_paths.len();
    let clipboard_info = state
        .clipboard
        .as_ref()
        .map(|c| {
            if c.is_cut() {
                format!(" | Cut: {}", c.paths().len())
            } else {
                format!(" | Copied: {}", c.paths().len())
            }
        })
        .unwrap_or_default();

    let stats = format!(
        "{}/{}{}{}",
        state.focus_index + 1,
        total_entries,
        if selected_count > 0 {
            format!(" | Selected: {}", selected_count)
        } else {
            String::new()
        },
        clipboard_info
    );
    let stats_widget = Paragraph::new(stats).block(Block::default().borders(Borders::ALL));
    frame.render_widget(stats_widget, chunks[1]);
}

/// Render input popup based on current mode
pub fn render_input_popup(frame: &mut Frame, state: &AppState) {
    match &state.mode {
        ViewMode::Input {
            purpose,
            buffer,
            cursor: _,
        } => {
            let title = match purpose {
                InputPurpose::CreateFile => "New File",
                InputPurpose::CreateDir => "New Directory",
                InputPurpose::Rename { .. } => "Rename",
            };
            draw_input_popup(frame, title, buffer);
        }
        ViewMode::Search { query } => {
            draw_input_popup(frame, "Search", query);
        }
        ViewMode::Confirm { action } => {
            draw_confirm_popup(frame, action);
        }
        _ => {}
    }
}

/// Draw a simple input popup
fn draw_input_popup(frame: &mut Frame, title: &str, content: &str) {
    let area = centered_rect(60, 3, frame.area());

    let input = Paragraph::new(content)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title(title));

    frame.render_widget(Clear, area);
    frame.render_widget(input, area);
}

/// Draw confirmation popup
fn draw_confirm_popup(frame: &mut Frame, action: &PendingAction) {
    match action {
        PendingAction::Delete { targets } => {
            draw_delete_confirm_popup(frame, targets);
        }
    }
}

/// Draw delete confirmation popup
fn draw_delete_confirm_popup(frame: &mut Frame, paths: &[std::path::PathBuf]) {
    let max_items_to_show = 8;
    let items_count = paths.len().min(max_items_to_show);
    let has_more = paths.len() > max_items_to_show;
    let has_directories = paths.iter().any(|p| p.is_dir());

    let warning_lines = if has_directories { 2 } else { 0 };
    let more_line = if has_more { 1 } else { 0 };
    let height = (3 + warning_lines + items_count + more_line + 2) as u16;

    let area = centered_rect(60, height, frame.area());

    let mut content = Vec::new();

    if has_directories {
        content.push(Line::from(vec![Span::styled(
            "!! WARNING: FOLDER DELETION !!",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )]));
        content.push(Line::from(vec![Span::styled(
            "Folders and all contents will be permanently deleted",
            Style::default().fg(Color::Yellow),
        )]));
        content.push(Line::from(""));
    }

    content.push(Line::from(vec![Span::styled(
        format!("Delete {} item(s):", paths.len()),
        Style::default().add_modifier(Modifier::BOLD),
    )]));

    for path in paths.iter().take(max_items_to_show) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        let style = if path.is_dir() {
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        content.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(name, style),
        ]));
    }

    if has_more {
        content.push(Line::from(vec![Span::styled(
            format!("  ... and {} more", paths.len() - max_items_to_show),
            Style::default().fg(Color::DarkGray),
        )]));
    }

    content.push(Line::from(""));
    content.push(Line::from(vec![
        Span::styled(
            "y",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" to confirm, "),
        Span::styled(
            "n",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" to cancel"),
    ]));

    let title = if has_directories {
        " !! DELETE FOLDERS !! "
    } else {
        " Confirm Delete "
    };

    let title_style = if has_directories {
        Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let popup = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(if has_directories {
                Style::default().fg(Color::Red)
            } else {
                Style::default()
            })
            .title(Span::styled(title, title_style)),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

/// Create a centered rectangle
fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
