//! Replay UI for past `fv --mcp-server` activity.
//!
//! Two-pane TUI flow:
//!
//! 1. **Session picker** — lists every session directory under
//!    `<cache>/fileview/sessions/` with a readable `session.json` and an
//!    `activity.jsonl`. The list is sorted by activity-log mtime so recent
//!    sessions appear on top.
//! 2. **Event scrub** — once a session is chosen, lists every event in its
//!    `activity.jsonl`. `j` / `k` (or arrows) move; `Enter` exits the replay
//!    UI and prints the selected event's path to stdout so the parent shell
//!    can hand it back to a fresh `fv`.
//!
//! Implementation notes:
//!
//! - This runs on its own terminal session (alternate screen, raw mode); it
//!   does not reuse `run_app`'s loop because there is no tree, no preview,
//!   no clipboard work, etc.
//! - All I/O is best effort. A missing session directory or a malformed
//!   activity log degrades gracefully to "empty list".

use std::io::{stdout, Stdout, Write};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};

use crate::ai_activity::{read_session_events, ActivityEvent, SessionInfo, SessionRegistry};

/// Outcome of running the replay TUI.
#[derive(Debug, Clone, Default)]
pub struct ReplayOutcome {
    /// When the user selected an event with `Enter`, this is the absolute
    /// path the parent shell should jump to. May be `None` when the user
    /// quit without picking anything.
    pub selected_path: Option<PathBuf>,
}

/// Run the replay UI. `preselect` is the session directory name (the original
/// pid as a string) when the user ran `fv --replay <id>`; otherwise the
/// session picker is shown first.
pub fn run_replay_app(preselect: Option<&str>) -> anyhow::Result<ReplayOutcome> {
    let registry = SessionRegistry::new()?;
    let sessions = registry.list_history();

    if sessions.is_empty() {
        eprintln!(
            "No fileview sessions found under {}",
            registry.base_dir().display()
        );
        return Ok(ReplayOutcome::default());
    }

    let preselected_index =
        preselect.and_then(|id| sessions.iter().position(|s| matches_session_id(s, id)));

    let mut terminal = enter_terminal()?;
    let result = match preselected_index {
        Some(idx) => run_event_scrub(&mut terminal, &sessions[idx]),
        None => run_two_stage(&mut terminal, &sessions),
    };
    leave_terminal(&mut terminal)?;
    result
}

fn matches_session_id(s: &SessionInfo, id: &str) -> bool {
    s.dir
        .file_name()
        .map(|n| n.to_string_lossy())
        .map(|name| name == id)
        .unwrap_or(false)
}

fn enter_terminal() -> anyhow::Result<Terminal<CrosstermBackend<Stdout>>> {
    terminal::enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    Ok(Terminal::new(backend)?)
}

fn leave_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, cursor::Show)?;
    Ok(())
}

fn run_two_stage(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    sessions: &[SessionInfo],
) -> anyhow::Result<ReplayOutcome> {
    let mut selected = 0usize;
    loop {
        terminal.draw(|frame| {
            draw_session_picker(frame, sessions, selected);
        })?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let ev = event::read()?;
        let key = match ev {
            Event::Key(k) if k.kind == KeyEventKind::Press => k,
            _ => continue,
        };
        match navigate_session_picker(key, sessions, &mut selected) {
            PickerStep::Continue => {}
            PickerStep::Quit => return Ok(ReplayOutcome::default()),
            PickerStep::Choose => {
                let outcome = run_event_scrub(terminal, &sessions[selected])?;
                if outcome.selected_path.is_some() {
                    return Ok(outcome);
                }
                // Returned with no selection: fall back to the picker.
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PickerStep {
    Continue,
    Quit,
    Choose,
}

fn navigate_session_picker(
    key: KeyEvent,
    sessions: &[SessionInfo],
    selected: &mut usize,
) -> PickerStep {
    let max = sessions.len().saturating_sub(1);
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => PickerStep::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => PickerStep::Quit,
        KeyCode::Up | KeyCode::Char('k') => {
            *selected = selected.saturating_sub(1);
            PickerStep::Continue
        }
        KeyCode::Down | KeyCode::Char('j') => {
            *selected = (*selected + 1).min(max);
            PickerStep::Continue
        }
        KeyCode::Char('g') => {
            *selected = 0;
            PickerStep::Continue
        }
        KeyCode::Char('G') => {
            *selected = max;
            PickerStep::Continue
        }
        KeyCode::Enter => PickerStep::Choose,
        _ => PickerStep::Continue,
    }
}

fn run_event_scrub(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    session: &SessionInfo,
) -> anyhow::Result<ReplayOutcome> {
    let events = read_session_events(&session.activity_log).unwrap_or_default();
    let mut selected = 0usize;
    loop {
        terminal.draw(|frame| {
            draw_event_scrub(frame, session, &events, selected);
        })?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let ev = event::read()?;
        let key = match ev {
            Event::Key(k) if k.kind == KeyEventKind::Press => k,
            _ => continue,
        };
        let max = events.len().saturating_sub(1);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return Ok(ReplayOutcome::default()),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(ReplayOutcome::default());
            }
            KeyCode::Up | KeyCode::Char('k') => {
                selected = selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                selected = (selected + 1).min(max);
            }
            KeyCode::Char('K') => {
                selected = selected.saturating_sub(10);
            }
            KeyCode::Char('J') => {
                selected = (selected + 10).min(max);
            }
            KeyCode::Char('g') => selected = 0,
            KeyCode::Char('G') => selected = max,
            KeyCode::Enter => {
                if let Some(ev) = events.get(selected) {
                    if let Some(path) = ev.path.clone() {
                        return Ok(ReplayOutcome {
                            selected_path: Some(path),
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

fn draw_session_picker(frame: &mut Frame, sessions: &[SessionInfo], selected: usize) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "fv --replay",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  past AI sessions  "),
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::raw(" navigate  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" open  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" quit"),
    ]))
    .block(Block::default().borders(Borders::ALL).title(" Replay "));
    frame.render_widget(header, chunks[0]);

    let inner_height = chunks[1].height.saturating_sub(2) as usize;
    let start = list_window_start(selected, sessions.len(), inner_height);
    let lines: Vec<Line<'static>> = sessions
        .iter()
        .enumerate()
        .skip(start)
        .take(inner_height)
        .map(|(i, s)| render_session_row(i, s, i == selected))
        .collect();

    let title = format!(" {} session(s) ", sessions.len());
    let body = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(body, chunks[1]);
}

fn render_session_row(_index: usize, info: &SessionInfo, focused: bool) -> Line<'static> {
    let pid = info
        .dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let root = info.meta.root.display().to_string();
    let prefix = if focused { "▶ " } else { "  " };
    let style = if focused {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::styled(prefix, style.fg(Color::Cyan)),
        Span::styled(format!("pid {:>6}  ", pid), style.fg(Color::Yellow)),
        Span::styled(root, style),
    ])
}

fn draw_event_scrub(
    frame: &mut Frame,
    session: &SessionInfo,
    events: &[ActivityEvent],
    selected: usize,
) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(area);

    let pid = session
        .dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("session {}", pid),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::raw(session.meta.root.display().to_string()),
        Span::raw("  "),
        Span::styled(
            format!("({} events)", events.len()),
            Style::default().fg(Color::Gray),
        ),
        Span::raw("  "),
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::raw(" step  "),
        Span::styled("J/K", Style::default().fg(Color::Yellow)),
        Span::raw(" ±10  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" pick path  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" back"),
    ]))
    .block(Block::default().borders(Borders::ALL).title(" Replay "));
    frame.render_widget(header, chunks[0]);

    let inner_height = chunks[1].height.saturating_sub(2) as usize;
    let start = list_window_start(selected, events.len(), inner_height);
    let lines: Vec<Line<'static>> = if events.is_empty() {
        vec![Line::from(Span::styled(
            "No events recorded for this session.",
            Style::default().fg(Color::Gray),
        ))]
    } else {
        events
            .iter()
            .enumerate()
            .skip(start)
            .take(inner_height)
            .map(|(i, ev)| render_event_row(i, ev, i == selected, &session.meta.root))
            .collect()
    };

    let title = format!(
        " event {}/{} ",
        selected.saturating_add(1).min(events.len().max(1)),
        events.len()
    );
    let body = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(body, chunks[1]);
}

fn render_event_row(
    _index: usize,
    ev: &ActivityEvent,
    focused: bool,
    root: &std::path::Path,
) -> Line<'static> {
    let prefix = if focused { "▶ " } else { "  " };
    let style = if focused {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let path_disp = ev.short_path(Some(root));
    let summary = ev.summary.clone().unwrap_or_default();
    let suffix = if summary.is_empty() {
        String::new()
    } else {
        format!("  · {}", summary)
    };
    Line::from(vec![
        Span::styled(prefix, style.fg(Color::Cyan)),
        Span::styled(format!("{:>13} ms  ", ev.ts), style.fg(Color::Gray)),
        Span::styled(
            format!("{:<10} ", ev.short_source()),
            style.fg(Color::Yellow),
        ),
        Span::styled(format!("{:<14} ", ev.tool), style.fg(Color::Magenta)),
        Span::styled(path_disp, style),
        Span::styled(suffix, style.fg(Color::Gray)),
    ])
}

fn list_window_start(selected: usize, total: usize, window: usize) -> usize {
    if total <= window {
        return 0;
    }
    let half = window / 2;
    if selected < half {
        0
    } else if selected + (window - half) >= total {
        total.saturating_sub(window)
    } else {
        selected.saturating_sub(half)
    }
}

/// Print the selected outcome on stdout for shell integration.
///
/// The replay TUI runs in alternate-screen mode, so any earlier `println!`
/// would be erased on exit. This helper is meant to be called *after* the
/// terminal has been restored.
pub fn print_outcome(outcome: &ReplayOutcome) {
    if let Some(path) = outcome.selected_path.as_ref() {
        let mut stdout = stdout();
        let _ = writeln!(stdout, "{}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_window_centers_selected_when_far_into_list() {
        // 100 items, 10-row window, cursor at 50: should center.
        assert_eq!(list_window_start(50, 100, 10), 45);
    }

    #[test]
    fn list_window_clamps_to_top() {
        assert_eq!(list_window_start(2, 100, 10), 0);
    }

    #[test]
    fn list_window_clamps_to_bottom() {
        assert_eq!(list_window_start(98, 100, 10), 90);
    }

    #[test]
    fn list_window_zero_when_total_fits() {
        assert_eq!(list_window_start(3, 5, 10), 0);
    }

    #[test]
    fn matches_session_id_compares_dir_name() {
        let info = SessionInfo {
            meta: crate::ai_activity::SessionMeta {
                pid: 42,
                root: std::path::PathBuf::from("/x"),
                started_at: 0,
            },
            dir: std::path::PathBuf::from("/cache/fileview/sessions/42"),
            meta_file: std::path::PathBuf::from("/cache/fileview/sessions/42/session.json"),
            activity_log: std::path::PathBuf::from("/cache/fileview/sessions/42/activity.jsonl"),
        };
        assert!(matches_session_id(&info, "42"));
        assert!(!matches_session_id(&info, "99"));
    }
}
