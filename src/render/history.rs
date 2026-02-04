//! AI history popup rendering.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};

use crate::core::{AppState, ViewMode};

/// Render AI history popup (Ctrl+Shift+P)
pub fn render_ai_history_popup(frame: &mut Frame, state: &AppState) {
    let ViewMode::AiHistory { selected } = &state.mode else {
        return;
    };

    let area = frame.area();
    let width = area.width.saturating_sub(6).clamp(30, 80);
    let height = area.height.saturating_sub(6).clamp(8, 20);
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup);

    let max_items = height.saturating_sub(2) as usize;
    let items: Vec<ListItem> = state
        .ai_history
        .iter()
        .take(max_items.max(1))
        .enumerate()
        .map(|(idx, item)| {
            let style = if idx == *selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(vec![Span::styled(item.title.clone(), style)]))
        })
        .collect();

    let widget = List::new(items).block(
        Block::default()
            .title(" AI History ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(widget, popup);
}
