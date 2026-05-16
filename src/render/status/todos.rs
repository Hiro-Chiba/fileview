//! TODO/FIXME aggregator popup renderer.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::core::{AppState, ViewMode};
use crate::render::theme::theme;

/// Render the TODO/FIXME aggregator popup.
pub fn render_todos_popup(frame: &mut Frame, state: &AppState) {
    use crate::integrate::TodoTag;

    let selected = match state.mode {
        ViewMode::Todos { selected } => selected,
        _ => return,
    };

    let area = frame.area();
    let overlay_width = area.width.saturating_sub(8).max(60);
    let overlay_height = area.height.saturating_sub(4).max(10);
    let overlay_x = (area.width.saturating_sub(overlay_width)) / 2;
    let overlay_y = (area.height.saturating_sub(overlay_height)) / 2;
    let overlay_area = Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);

    frame.render_widget(Clear, overlay_area);

    let t = theme();
    let inner_width = overlay_width.saturating_sub(2) as usize;
    let visible_rows = overlay_height.saturating_sub(2) as usize;

    // Center the selected row in the visible window.
    let total = state.todo_items.len();
    let start = if total <= visible_rows {
        0
    } else {
        let half = visible_rows / 2;
        if selected < half {
            0
        } else if selected + (visible_rows - half) >= total {
            total.saturating_sub(visible_rows)
        } else {
            selected.saturating_sub(half)
        }
    };

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(visible_rows);
    if total == 0 {
        let msg = if state.todo_partial {
            "No TODOs in scanned region (scan was partial)"
        } else {
            "No TODOs found"
        };
        lines.push(Line::from(Span::styled(
            msg.to_string(),
            Style::default().fg(t.git_ignored),
        )));
    } else {
        for (i, item) in state
            .todo_items
            .iter()
            .enumerate()
            .skip(start)
            .take(visible_rows)
        {
            let tag_color = match item.tag {
                TodoTag::Todo => t.info,
                TodoTag::Fixme => t.warning,
                TodoTag::Xxx => t.warning,
                TodoTag::Hack => t.git_conflict,
                TodoTag::Bug => t.git_conflict,
                TodoTag::Note => t.git_staged,
            };
            let rel = item
                .path
                .strip_prefix(&state.root)
                .unwrap_or(&item.path)
                .display()
                .to_string();
            let prefix = if i == selected { "▶ " } else { "  " };
            let location = format!("{}:{}", rel, item.line);
            let msg = if item.message.is_empty() {
                String::new()
            } else {
                format!("  {}", item.message)
            };

            let line_style = if i == selected {
                Style::default()
                    .bg(t.selection)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Truncate the rendered text so it never overflows the popup width.
            let rendered = format!("{}{:6} {}{}", prefix, item.tag.as_str(), location, msg);
            let truncated = if rendered.chars().count() > inner_width {
                let mut out: String = rendered
                    .chars()
                    .take(inner_width.saturating_sub(1))
                    .collect();
                out.push('…');
                out
            } else {
                rendered
            };

            // Build line with the tag colored even when the row is selected.
            let tag_part = format!("{}{:6} ", prefix, item.tag.as_str());
            let rest_start = tag_part.chars().count().min(truncated.chars().count());
            let rest: String = truncated.chars().skip(rest_start).collect();

            lines.push(Line::from(vec![
                Span::styled(tag_part, line_style.fg(tag_color)),
                Span::styled(rest, line_style),
            ]));
        }
    }

    let title = if state.todo_partial {
        format!(
            " TODOs · {} (partial) · Esc to close ",
            state.todo_items.len()
        )
    } else {
        format!(" TODOs · {} · Esc to close ", state.todo_items.len())
    };

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border_active)),
    );

    frame.render_widget(paragraph, overlay_area);
}
