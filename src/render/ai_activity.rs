//! Live AI activity reflection UI.
//!
//! Two surfaces:
//! - A single status-bar line (rendered inline by `render/status.rs`).
//! - A popup list view opened via `Alt+L`.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};

use crate::ai_activity::{ActivityEvent, STATUS_FRESH_WINDOW_MS};
use crate::core::{AppState, ViewMode};

/// Build the status-bar indicator text for the most recent AI event.
///
/// Returns `None` when nothing has happened recently (the caller should fall
/// back to its normal status rendering).
pub fn status_indicator(state: &AppState) -> Option<String> {
    let event = state.ai_activity.fresh_event()?;
    let follow = if state.ai_activity.follow_mode {
        "*"
    } else {
        ""
    };
    let path = event.short_path(Some(&state.root));
    Some(format!(
        "[AI{}] {}: {} {}",
        follow,
        event.short_source(),
        event.tool,
        path
    ))
}

/// Render the AI activity log popup (`Alt+L`).
pub fn render_ai_activity_popup(frame: &mut Frame, state: &AppState) {
    if !matches!(state.mode, ViewMode::AiActivityLog) {
        return;
    }

    let area = frame.area();
    let width = area.width.saturating_sub(4).clamp(40, 100);
    let height = area.height.saturating_sub(4).clamp(10, 24);
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup);

    let selected = state.ai_activity.log_selected;
    let max_items = (height.saturating_sub(2) as usize).max(1);
    // Scroll offset: keep the selected row in view when log_selected exceeds
    // the number of rows we can draw. Offset is the index of the topmost row.
    let offset = if selected < max_items {
        0
    } else {
        selected + 1 - max_items
    };
    let items: Vec<ListItem> = state
        .ai_activity
        .recent_events
        .iter()
        .enumerate()
        .skip(offset)
        .take(max_items)
        .map(|(idx, event)| {
            let style = if idx == selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(vec![Span::styled(
                format_event_line(event, &state.root),
                style,
            )]))
        })
        .collect();

    let title = format!(
        " AI Activity (follow: {}) ",
        if state.ai_activity.follow_mode {
            "on"
        } else {
            "off"
        }
    );
    let widget = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(widget, popup);
}

fn format_event_line(event: &ActivityEvent, root: &std::path::Path) -> String {
    let age_sec = if event.is_recent(STATUS_FRESH_WINDOW_MS * 60) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        format!("{}s", (now.saturating_sub(event.ts)) / 1000)
    } else {
        "old".to_string()
    };
    format!(
        "[{}] {}: {} {}",
        age_sec,
        event.short_source(),
        event.tool,
        event.short_path(Some(root))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_activity::{ActivityEvent, AiActivityState};
    use std::path::PathBuf;

    fn sample_state() -> AppState {
        let mut s = AppState::new(PathBuf::from("/root"));
        s.ai_activity = AiActivityState::default();
        s
    }

    #[test]
    fn status_indicator_none_when_no_events() {
        let state = sample_state();
        assert!(status_indicator(&state).is_none());
    }

    #[test]
    fn status_indicator_shows_recent_event() {
        let mut state = sample_state();
        state.ai_activity.record(ActivityEvent::now(
            "claude-pid-1",
            "read_file",
            Some(PathBuf::from("/root/src/a.rs")),
        ));
        let indicator = status_indicator(&state).expect("indicator");
        assert!(indicator.starts_with("[AI"), "got: {}", indicator);
        assert!(indicator.contains("claude"));
        assert!(indicator.contains("read_file"));
        assert!(indicator.contains("src/a.rs"));
    }

    #[test]
    fn status_indicator_marks_follow_mode() {
        let mut state = sample_state();
        state.ai_activity.follow_mode = true;
        state.ai_activity.record(ActivityEvent::now(
            "claude",
            "read_file",
            Some(PathBuf::from("/root/a.rs")),
        ));
        let indicator = status_indicator(&state).unwrap();
        assert!(indicator.starts_with("[AI*]"), "got: {}", indicator);
    }
}
