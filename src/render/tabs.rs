//! Tab bar rendering

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::core::TabManager;

/// Render the tab bar at the top of the screen
pub fn render_tab_bar(frame: &mut Frame, tabs: &TabManager, area: Rect) {
    if tabs.len() <= 1 {
        // Don't render tab bar if only one tab
        return;
    }

    let mut spans = Vec::new();

    for (i, tab) in tabs.tabs.iter().enumerate() {
        let is_active = i == tabs.active_index;

        // Tab number
        let num = format!(" {} ", i + 1);

        // Tab name (truncated if needed)
        let max_name_len = 15;
        let name = tab.short_name(max_name_len);

        if is_active {
            spans.push(Span::styled(
                num,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                name,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(num, Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(name, Style::default().fg(Color::Gray)));
        }

        // Separator
        if i < tabs.len() - 1 {
            spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        }
    }

    // Add help hint
    spans.push(Span::styled(
        "  [Ctrl+T: new, Ctrl+W: close, Alt+t/T: switch]",
        Style::default().fg(Color::DarkGray),
    ));

    let line = Line::from(spans);
    let para = Paragraph::new(line);
    frame.render_widget(para, area);
}
