//! Status bar rendering across density modes plus the context-budget segment.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::format::{get_file_info, get_file_info_narrow, get_file_size_only};
use crate::core::{AppState, PreviewDisplayMode, SortMode, UiDensity};
use crate::integrate::{humanize_tokens, BudgetSeverity};
use crate::render::layout::LayoutEngine;
use crate::render::theme::theme;
use crate::util::utf8_prefix;

/// Build the spans rendered for the context budget bar.
///
/// Returns an empty vector when nothing is selected.
fn budget_segment_spans(state: &AppState, density: UiDensity) -> Vec<Span<'static>> {
    if state.selected_paths.is_empty() {
        return Vec::new();
    }
    let t = theme();
    let used = state.known_token_total();
    let pending = state.pending_token_count();
    let window = state.budget_model.window_tokens();
    let percent = if window > 0 {
        ((used as f64 / window as f64) * 100.0).round() as u32
    } else {
        0
    };
    let severity = BudgetSeverity::from_usage(used, window);
    let color = match severity {
        BudgetSeverity::Ok => t.git_staged,
        BudgetSeverity::Warn => t.warning,
        BudgetSeverity::Hot => t.git_conflict,
        BudgetSeverity::Over => Color::Magenta,
    };
    let used_s = humanize_tokens(used);
    let win_s = humanize_tokens(window);
    let pending_marker = if pending > 0 {
        format!("+{}?", pending)
    } else {
        String::new()
    };
    let text = match density {
        UiDensity::Full => format!(
            "[ctx {}f {}{}/{} {} {}%]",
            state.selected_paths.len(),
            used_s,
            if pending_marker.is_empty() {
                String::new()
            } else {
                format!(" {}", pending_marker)
            },
            win_s,
            state.budget_model.short_label(),
            percent,
        ),
        UiDensity::Compact => format!(
            "[ctx {}{}/{} {}%]",
            used_s,
            if pending_marker.is_empty() {
                String::new()
            } else {
                format!(" {}", pending_marker)
            },
            win_s,
            percent,
        ),
        UiDensity::Narrow => format!("[ctx {}/{}]", used_s, win_s),
        UiDensity::Ultra => format!("[{}]", used_s),
    };
    vec![Span::styled(text, Style::default().fg(color))]
}

/// Render the status bar with adaptive layout based on screen width
/// Compose the status-bar message, giving AI activity precedence when a
/// recent event is available. Returns an owned `String` because we may need
/// to concatenate with the regular `state.message`.
pub fn effective_status_message(state: &AppState, fallback: &str) -> String {
    let indicator = crate::render::ai_activity::status_indicator(state);
    match (indicator, state.message.as_deref()) {
        (Some(ind), Some(msg)) => format!("{} · {}", ind, msg),
        (Some(ind), None) => ind,
        (None, Some(msg)) => msg.to_string(),
        (None, None) => fallback.to_string(),
    }
}

pub fn render_status_bar(
    frame: &mut Frame,
    state: &AppState,
    focused_path: Option<&PathBuf>,
    area: Rect,
) {
    // Check if peek mode is enabled - render peek preview instead of normal status
    if state.preview_display_mode == PreviewDisplayMode::Peek {
        render_peek_status(frame, state, focused_path, area);
        return;
    }

    let density = state.ui_density_for_width(area.width);
    let layout = LayoutEngine::from_rect_with_density(area, density);

    // Adaptive layout based on UI density
    match layout.density {
        UiDensity::Ultra => render_ultra_compact_status(frame, state, area),
        UiDensity::Narrow => render_compact_status(frame, state, focused_path, area),
        UiDensity::Compact => render_narrow_status(frame, state, focused_path, area),
        UiDensity::Full => render_full_status(frame, state, focused_path, area),
    }
}

/// Render peek mode status bar (shows file preview in status area)
fn render_peek_status(
    frame: &mut Frame,
    state: &AppState,
    focused_path: Option<&PathBuf>,
    area: Rect,
) {
    let t = theme();
    let density = state.ui_density_for_width(area.width);
    let layout = LayoutEngine::from_rect_with_density(area, density);
    let max_preview_lines = layout.peek_preview_lines();

    // Build header line with minimal info based on density
    let mut header_spans = Vec::new();

    // Peek indicator
    header_spans.push(Span::styled(
        "P",
        Style::default().fg(t.info).add_modifier(Modifier::BOLD),
    ));

    // Selection count
    if !state.selected_paths.is_empty() {
        header_spans.push(Span::raw(" "));
        header_spans.push(Span::styled(
            format!("{}*", state.selected_paths.len()),
            Style::default().fg(t.mark),
        ));
    }

    // Git branch (abbreviated based on density)
    if let Some(branch) = state.git_status.as_ref().and_then(|g| g.branch()) {
        let max_branch_len = match layout.density {
            UiDensity::Ultra => 4,
            UiDensity::Narrow => 6,
            _ => 8,
        };
        header_spans.push(Span::raw(" "));
        let branch_abbrev = if branch.len() > max_branch_len {
            format!(
                "\u{e0a0}{}…",
                utf8_prefix(branch, max_branch_len.saturating_sub(1))
            )
        } else {
            format!("\u{e0a0}{}", branch)
        };
        header_spans.push(Span::styled(
            branch_abbrev,
            Style::default().fg(t.git_staged),
        ));
    }

    // File name (only if enough space)
    if let Some(path) = focused_path {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let separator = match layout.density {
                UiDensity::Ultra => " ",
                _ => " │ ",
            };
            let max_name_len = match layout.density {
                UiDensity::Ultra => (area.width as usize).saturating_sub(12),
                UiDensity::Narrow => (area.width as usize).saturating_sub(15),
                _ => (area.width as usize).saturating_sub(20),
            };
            if max_name_len > 3 {
                header_spans.push(Span::styled(separator, Style::default().fg(t.git_ignored)));
                let display_name = if name.len() > max_name_len {
                    format!("{}…", utf8_prefix(name, max_name_len.saturating_sub(1)))
                } else {
                    name.to_string()
                };
                header_spans.push(Span::styled(
                    display_name,
                    Style::default().fg(t.border_active),
                ));
            }
        }
    }

    // Read first few lines of the file for peek preview
    let preview_lines = if let Some(path) = focused_path {
        read_peek_lines(
            path,
            max_preview_lines.min(area.height.saturating_sub(2) as usize),
        )
    } else {
        vec!["(No file)".to_string()]
    };

    let mut content = vec![Line::from(header_spans)];
    for line in preview_lines {
        // Truncate long lines
        let max_width = area.width.saturating_sub(2) as usize;
        let display = if line.len() > max_width {
            format!("{}…", utf8_prefix(&line, max_width.saturating_sub(1)))
        } else {
            line
        };
        content.push(Line::from(Span::styled(
            display,
            Style::default().fg(t.git_ignored),
        )));
    }

    // Compact title for ultra mode
    let title = match layout.density {
        UiDensity::Ultra => " P ",
        _ => " Peek ",
    };

    let widget = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border_active))
            .title(title),
    );
    frame.render_widget(widget, area);
}

/// Read first few lines of a file for peek preview
fn read_peek_lines(path: &PathBuf, max_lines: usize) -> Vec<String> {
    if path.is_dir() {
        return vec!["(directory)".to_string()];
    }

    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return vec!["(cannot read)".to_string()],
    };

    let reader = BufReader::new(file);
    let mut lines = Vec::new();

    for line in reader.lines().take(max_lines) {
        match line {
            Ok(l) => {
                // Skip empty lines at the start
                if lines.is_empty() && l.trim().is_empty() {
                    continue;
                }
                // Replace tabs with spaces
                lines.push(l.replace('\t', "  "));
            }
            Err(_) => {
                if lines.is_empty() {
                    return vec!["(binary file)".to_string()];
                }
                break;
            }
        }
    }

    if lines.is_empty() {
        vec!["(empty file)".to_string()]
    } else {
        lines
    }
}

/// Render ultra-compact status bar for extremely narrow screens (< 25 chars)
/// Shows minimal info: `? 3* ⎇m` (help, selection count, git branch)
/// Optimized for 20-24 character width terminals
fn render_ultra_compact_status(frame: &mut Frame, state: &AppState, area: Rect) {
    let t = theme();
    let mut spans = Vec::new();
    let inner_width = area.width.saturating_sub(2) as usize; // Account for borders

    // For ultra-narrow, prioritize: selection > branch > sort > filter > message
    let selected_count = state.selected_paths.len();

    // Selection count first (most important in ultra mode)
    if selected_count > 0 {
        spans.push(Span::styled(
            format!("{}*", selected_count),
            Style::default().fg(t.mark),
        ));
    }

    // Git branch (very abbreviated)
    if let Some(branch) = state.git_status.as_ref().and_then(|g| g.branch()) {
        if !spans.is_empty() {
            spans.push(Span::raw(" "));
        }
        // Max 4 chars for branch in ultra mode
        let branch_abbrev = if branch.len() > 4 {
            format!("\u{e0a0}{}…", utf8_prefix(branch, 3))
        } else {
            format!("\u{e0a0}{}", branch)
        };
        spans.push(Span::styled(
            branch_abbrev,
            Style::default().fg(t.git_staged),
        ));
    }

    // Sort mode indicator (single char, only if not default)
    if state.sort_mode != SortMode::Name {
        if !spans.is_empty() {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(
            state.sort_mode.short_name(),
            Style::default().fg(t.git_conflict),
        ));
    }

    // Filter indicator (just an icon)
    if state.filter_pattern.is_some() {
        if !spans.is_empty() {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled("\u{f0b0}", Style::default().fg(t.warning)));
    }

    // Budget segment (single bracketed token count)
    let budget_spans = budget_segment_spans(state, UiDensity::Ultra);
    if !budget_spans.is_empty() {
        if !spans.is_empty() {
            spans.push(Span::raw(" "));
        }
        spans.extend(budget_spans);
    }

    // Help hint only if there's space
    let current_width: usize = spans.iter().map(|s| s.width()).sum();
    if current_width < inner_width.saturating_sub(2) && spans.is_empty() {
        spans.push(Span::styled("?", Style::default().fg(t.info)));
    }

    // Message (only if there's significant space left). AI activity indicator
    // takes precedence when a recent event is available.
    let message = effective_status_message(state, "");
    if !message.is_empty() {
        let used_width: usize = spans.iter().map(|s| s.width()).sum();
        let available = inner_width.saturating_sub(used_width + 1);
        if available > 3 {
            spans.push(Span::raw(" "));
            let truncated = if message.len() > available {
                format!("{}…", utf8_prefix(&message, available.saturating_sub(1)))
            } else {
                message.clone()
            };
            spans.push(Span::raw(truncated));
        }
    }

    let content = Line::from(spans);
    let widget = Paragraph::new(content).block(Block::default().borders(Borders::ALL));
    frame.render_widget(widget, area);
}

/// Render compact status bar for very narrow screens (< 60 chars)
/// Shows only the most essential information in a single panel
fn render_compact_status(
    frame: &mut Frame,
    state: &AppState,
    focused_path: Option<&PathBuf>,
    area: Rect,
) {
    let t = theme();

    // Build compact content: "? | 1.2KB | main | Sel:3"
    let mut spans = Vec::new();

    // Help or message (highest priority)
    let message_owned = effective_status_message(state, "?");
    let message = message_owned.as_str();
    spans.push(Span::raw(format!(" {}", message)));

    // File size only (no modification time)
    if let Some(size) = focused_path.and_then(|p| get_file_size_only(p.as_path())) {
        spans.push(Span::styled(" | ", Style::default().fg(t.git_ignored)));
        spans.push(Span::raw(size));
    }

    // Git branch (abbreviated, medium priority)
    if let Some(branch) = state.git_status.as_ref().and_then(|g| g.branch()) {
        spans.push(Span::styled(" | ", Style::default().fg(t.git_ignored)));
        spans.push(Span::styled(
            format!("\u{e0a0}{}", branch),
            Style::default().fg(t.git_staged),
        ));
    }

    // Selection count (abbreviated)
    let selected_count = state.selected_paths.len();
    if selected_count > 0 {
        spans.push(Span::styled(" | ", Style::default().fg(t.git_ignored)));
        spans.push(Span::raw(format!("Sel:{}", selected_count)));
    }

    let budget_spans = budget_segment_spans(state, UiDensity::Compact);
    if !budget_spans.is_empty() {
        spans.push(Span::styled(" | ", Style::default().fg(t.git_ignored)));
        spans.extend(budget_spans);
    }

    let content = Line::from(spans);
    let widget = Paragraph::new(content).block(Block::default().borders(Borders::ALL));
    frame.render_widget(widget, area);
}

/// Render narrow status bar for medium screens (60-99 chars)
/// Shows abbreviated information in two panels
fn render_narrow_status(
    frame: &mut Frame,
    state: &AppState,
    focused_path: Option<&PathBuf>,
    area: Rect,
) {
    // Dynamic split: adjust based on content
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let t = theme();

    // Left panel: message/help + git branch
    let mut left_spans = Vec::new();

    // Watch indicator (if enabled, keep it compact)
    if state.watch_enabled {
        left_spans.push(Span::styled("\u{f06e} ", Style::default().fg(t.info)));
    }

    // Git branch (abbreviated)
    if let Some(branch) = state.git_status.as_ref().and_then(|g| g.branch()) {
        left_spans.push(Span::styled(
            format!("\u{e0a0}{} |", branch),
            Style::default().fg(t.git_staged),
        ));
    }

    // Sort mode (abbreviated, only if non-default)
    if state.sort_mode != SortMode::Name {
        left_spans.push(Span::styled(
            format!("\u{f0dc}{}|", state.sort_mode.short_name()),
            Style::default().fg(t.git_conflict),
        ));
    }

    // Search matches (abbreviated)
    if let Some((current, total)) = state.search_matches {
        left_spans.push(Span::styled(
            format!("{}/{}|", current, total),
            Style::default().fg(t.border_active),
        ));
    }

    // Help or message
    let message_owned = effective_status_message(state, "? help");
    let message = message_owned.as_str();
    left_spans.push(Span::raw(format!(" {}", message)));

    let left_content = Line::from(left_spans);
    let left_widget = Paragraph::new(left_content).block(Block::default().borders(Borders::ALL));
    frame.render_widget(left_widget, chunks[0]);

    // Right panel: file info + selection (abbreviated)
    let file_info = focused_path
        .map(|p| p.as_path())
        .and_then(get_file_info_narrow)
        .unwrap_or_else(|| "--".to_string());

    let selected_count = state.selected_paths.len();
    let clipboard_info = state
        .clipboard
        .as_ref()
        .map(|c| {
            if c.is_cut() {
                format!(" | Cut:{}", c.paths().len())
            } else {
                format!(" | Cp:{}", c.paths().len())
            }
        })
        .unwrap_or_default();

    let stats_text = format!(
        "{}{}{}",
        file_info,
        if selected_count > 0 {
            format!(" | Sel:{}", selected_count)
        } else {
            String::new()
        },
        clipboard_info
    );
    let mut stats_spans: Vec<Span<'static>> = vec![Span::raw(stats_text)];
    let budget_spans = budget_segment_spans(state, UiDensity::Narrow);
    if !budget_spans.is_empty() {
        stats_spans.push(Span::raw(" "));
        stats_spans.extend(budget_spans);
    }
    let stats_widget =
        Paragraph::new(Line::from(stats_spans)).block(Block::default().borders(Borders::ALL));
    frame.render_widget(stats_widget, chunks[1]);
}

/// Render full status bar for wide screens (>= 100 chars)
/// Original implementation with full information display
fn render_full_status(
    frame: &mut Frame,
    state: &AppState,
    focused_path: Option<&PathBuf>,
    area: Rect,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: message or help hint, with mode, git branch, watch, filter, sort, and search indicators
    let mode_indicator = if state.select_mode {
        if state.multi_select {
            "\u{f05c}\u{f05c} " // Multi-select icon
        } else {
            "\u{f05c} " // Select icon (nf-fa-circle_o)
        }
    } else if state.pick_mode {
        "\u{f046} " // Pick icon (nf-fa-check_square_o)
    } else {
        ""
    };

    let watch_indicator = if state.watch_enabled {
        "\u{f06e} " // Eye icon (nf-fa-eye) for file watching
    } else {
        ""
    };

    let filter_indicator = state
        .filter_pattern
        .as_ref()
        .map(|p| format!("\u{f0b0} {} |", p)) // Filter icon
        .unwrap_or_default();

    let branch_info = state
        .git_status
        .as_ref()
        .and_then(|g| g.branch())
        .map(|b| format!("\u{e0a0} {} |", b)) // Git branch icon
        .unwrap_or_default();

    // Sort mode indicator (only show if not default)
    let sort_indicator = if state.sort_mode != SortMode::Name {
        format!("\u{f0dc} {} |", state.sort_mode.display_name()) // Sort icon
    } else {
        String::new()
    };

    // Search match info
    let search_indicator = state
        .search_matches
        .map(|(current, total)| format!("{}/{} matches |", current, total))
        .unwrap_or_default();

    let t = theme();
    let message_owned = effective_status_message(state, "? for help");
    let message = message_owned.as_str();
    let left_content = Line::from(vec![
        Span::styled(mode_indicator, Style::default().fg(t.selection)),
        Span::styled(watch_indicator, Style::default().fg(t.info)),
        Span::styled(filter_indicator, Style::default().fg(t.warning)),
        Span::styled(branch_info, Style::default().fg(t.git_staged)),
        Span::styled(sort_indicator, Style::default().fg(t.git_conflict)),
        Span::styled(search_indicator, Style::default().fg(t.border_active)),
        Span::raw(format!(" {}", message)),
    ]);
    let msg_widget = Paragraph::new(left_content).block(Block::default().borders(Borders::ALL));
    frame.render_widget(msg_widget, chunks[0]);

    // Right: file info + selection stats
    let file_info = focused_path
        .map(|p| p.as_path())
        .and_then(get_file_info)
        .unwrap_or_else(|| "--".to_string());

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

    let stats_text = format!(
        "{}{}{}",
        file_info,
        if selected_count > 0 {
            format!(" | Selected: {}", selected_count)
        } else {
            String::new()
        },
        clipboard_info
    );
    let mut stats_spans: Vec<Span<'static>> = vec![Span::raw(stats_text)];
    let budget_spans = budget_segment_spans(state, UiDensity::Full);
    if !budget_spans.is_empty() {
        stats_spans.push(Span::raw(" "));
        stats_spans.extend(budget_spans);
    }
    let stats_widget =
        Paragraph::new(Line::from(stats_spans)).block(Block::default().borders(Borders::ALL));
    frame.render_widget(stats_widget, chunks[1]);
}
