//! Status bar and input popup rendering

use std::path::PathBuf;
use std::time::SystemTime;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::theme::theme;
use crate::core::{AppState, InputPurpose, PendingAction, SortMode, ViewMode};

/// Render the status bar with adaptive layout based on screen width
pub fn render_status_bar(
    frame: &mut Frame,
    state: &AppState,
    focused_path: Option<&PathBuf>,
    area: Rect,
) {
    let width = area.width;

    // Adaptive layout based on screen width
    if width < 60 {
        // Very narrow: single panel with minimal info
        render_compact_status(frame, state, focused_path, area);
    } else if width < 100 {
        // Narrow: abbreviated display
        render_narrow_status(frame, state, focused_path, area);
    } else {
        // Wide: full display (original implementation)
        render_full_status(frame, state, focused_path, area);
    }
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
    let message = state.message.as_deref().unwrap_or("?");
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
    let message = state.message.as_deref().unwrap_or("? help");
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

    let stats = format!(
        "{}{}{}",
        file_info,
        if selected_count > 0 {
            format!(" | Sel:{}", selected_count)
        } else {
            String::new()
        },
        clipboard_info
    );
    let stats_widget = Paragraph::new(stats).block(Block::default().borders(Borders::ALL));
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
    let message = state.message.as_deref().unwrap_or("? for help");
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

    let stats = format!(
        "{}{}{}",
        file_info,
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

/// Get file size and modification time as a formatted string (full display)
fn get_file_info(path: &std::path::Path) -> Option<String> {
    let metadata = path.metadata().ok()?;

    // Format size
    let size_str = if metadata.is_dir() {
        "--".to_string()
    } else {
        format_size(metadata.len())
    };

    // Format modification time
    let mtime_str = metadata
        .modified()
        .ok()
        .map(format_relative_time)
        .unwrap_or_else(|| "--".to_string());

    Some(format!("{} · {}", size_str, mtime_str))
}

/// Get file size and abbreviated modification time (narrow display)
fn get_file_info_narrow(path: &std::path::Path) -> Option<String> {
    let metadata = path.metadata().ok()?;

    // Format size
    let size_str = if metadata.is_dir() {
        "--".to_string()
    } else {
        format_size(metadata.len())
    };

    // Format modification time (abbreviated)
    let mtime_str = metadata
        .modified()
        .ok()
        .map(format_relative_time_short)
        .unwrap_or_else(|| "--".to_string());

    Some(format!("{} · {}", size_str, mtime_str))
}

/// Get file size only (compact display)
fn get_file_size_only(path: &std::path::Path) -> Option<String> {
    let metadata = path.metadata().ok()?;

    if metadata.is_dir() {
        Some("--".to_string())
    } else {
        Some(format_size(metadata.len()))
    }
}

/// Format file size in human-readable format
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format time as relative (e.g., "2h ago", "Yesterday", "Jan 30")
fn format_relative_time(time: SystemTime) -> String {
    let now = SystemTime::now();
    let duration = match now.duration_since(time) {
        Ok(d) => d,
        Err(_) => return "Future".to_string(),
    };

    let secs = duration.as_secs();
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;

    if secs < 60 {
        "Just now".to_string()
    } else if mins < 60 {
        format!("{}m ago", mins)
    } else if hours < 24 {
        format!("{}h ago", hours)
    } else if days == 1 {
        "Yesterday".to_string()
    } else if days < 7 {
        format!("{}d ago", days)
    } else {
        // Use date format for older files
        use std::time::UNIX_EPOCH;
        let timestamp = time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format_date_from_timestamp(timestamp)
    }
}

/// Format time as short relative (e.g., "2m", "5h", "3d") for narrow displays
fn format_relative_time_short(time: SystemTime) -> String {
    let now = SystemTime::now();
    let duration = match now.duration_since(time) {
        Ok(d) => d,
        Err(_) => return "?".to_string(),
    };

    let secs = duration.as_secs();
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;

    if secs < 60 {
        "now".to_string()
    } else if mins < 60 {
        format!("{}m", mins)
    } else if hours < 24 {
        format!("{}h", hours)
    } else if days < 30 {
        format!("{}d", days)
    } else {
        // Use abbreviated date for older files
        use std::time::UNIX_EPOCH;
        let timestamp = time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format_date_short_from_timestamp(timestamp)
    }
}

/// Format timestamp as "M/D" for narrow displays
fn format_date_short_from_timestamp(timestamp: u64) -> String {
    let secs_per_day: u64 = 86400;
    let days_since_epoch = timestamp / secs_per_day;

    let mut year = 1970u32;
    let mut remaining_days = days_since_epoch as u32;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let months = [
        31,
        if is_leap_year(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];

    let mut month = 1u32;
    let mut day = remaining_days + 1;

    for (i, &days) in months.iter().enumerate() {
        if remaining_days < days {
            month = (i + 1) as u32;
            day = remaining_days + 1;
            break;
        }
        remaining_days -= days;
    }

    format!("{}/{}", month, day)
}

/// Format timestamp as "Mon DD" or "Mon DD YYYY" if not current year
fn format_date_from_timestamp(timestamp: u64) -> String {
    // Simple month calculation (approximate, but good enough for display)
    let secs_per_day: u64 = 86400;
    let days_since_epoch = timestamp / secs_per_day;

    // Calculate year, month, day (simplified)
    let mut year = 1970u32;
    let mut remaining_days = days_since_epoch as u32;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let months = [
        ("Jan", 31),
        ("Feb", if is_leap_year(year) { 29 } else { 28 }),
        ("Mar", 31),
        ("Apr", 30),
        ("May", 31),
        ("Jun", 30),
        ("Jul", 31),
        ("Aug", 31),
        ("Sep", 30),
        ("Oct", 31),
        ("Nov", 30),
        ("Dec", 31),
    ];

    let mut month_name = "Jan";
    let mut day = remaining_days + 1;

    for (name, days) in months.iter() {
        if remaining_days < *days {
            month_name = name;
            day = remaining_days + 1;
            break;
        }
        remaining_days -= days;
    }

    // Get current year for comparison
    let now_timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let current_year = 1970 + (now_timestamp / (365 * secs_per_day)) as u32;

    if year == current_year {
        format!("{} {}", month_name, day)
    } else {
        format!("{} {} {}", month_name, day, year)
    }
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
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
        ViewMode::BookmarkSet => {
            draw_mini_popup(frame, "Set bookmark (1-9)");
        }
        ViewMode::BookmarkJump => {
            draw_mini_popup(frame, "Jump to bookmark (1-9)");
        }
        ViewMode::Filter { query } => {
            draw_input_popup(frame, "Filter (e.g., *.rs)", query);
        }
        _ => {}
    }
}

/// Draw a simple input popup
fn draw_input_popup(frame: &mut Frame, title: &str, content: &str) {
    let t = theme();
    let area = centered_rect(60, 3, frame.area());

    let input = Paragraph::new(content)
        .style(Style::default().fg(t.warning))
        .block(Block::default().borders(Borders::ALL).title(title));

    frame.render_widget(Clear, area);
    frame.render_widget(input, area);
}

/// Draw a small notification popup (for bookmark modes, etc.)
fn draw_mini_popup(frame: &mut Frame, message: &str) {
    let t = theme();
    let width = (message.len() + 4).min(50) as u16;
    let area = centered_rect(width, 3, frame.area());

    let popup = Paragraph::new(message)
        .style(Style::default().fg(t.border_active))
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
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
            "!! WARNING: FOLDER MOVE !!",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]));
        content.push(Line::from(vec![Span::styled(
            "Folders and all contents will be moved to trash",
            Style::default().fg(Color::Yellow),
        )]));
        content.push(Line::from(""));
    }

    content.push(Line::from(vec![Span::styled(
        format!("Move {} item(s) to trash:", paths.len()),
        Style::default().add_modifier(Modifier::BOLD),
    )]));

    for path in paths.iter().take(max_items_to_show) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        let style = if path.is_dir() {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        content.push(Line::from(vec![Span::raw("  "), Span::styled(name, style)]));
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
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" to cancel"),
    ]));

    let title = if has_directories {
        " !! MOVE FOLDERS TO TRASH !! "
    } else {
        " Move to Trash "
    };

    let title_style = if has_directories {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
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

// Help overlay constants
const HELP_OVERLAY_BG_COLOR: Color = Color::Rgb(30, 30, 40);
const HELP_OVERLAY_MARGIN: u16 = 2;

/// Create a styled key span with background highlight
fn help_key(key: &str) -> Span<'_> {
    Span::styled(
        key,
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
}

/// Create a description span
fn help_desc(desc: &str) -> Span<'_> {
    Span::styled(desc, Style::default().fg(Color::White))
}

/// Create a section header
fn help_section(title: &str) -> Line<'_> {
    Line::from(Span::styled(
        title,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))
}

/// Render help popup overlay with all keybindings
pub fn render_help_popup(frame: &mut Frame, state: &AppState) {
    if !matches!(state.mode, ViewMode::Help) {
        return;
    }

    let area = frame.area();

    // Calculate overlay size - use most of the screen
    let overlay_width = area.width.saturating_sub(HELP_OVERLAY_MARGIN * 2).max(60);
    let overlay_height = area.height.saturating_sub(HELP_OVERLAY_MARGIN * 2).max(20);
    let overlay_x = (area.width.saturating_sub(overlay_width)) / 2;
    let overlay_y = (area.height.saturating_sub(overlay_height)) / 2;

    let overlay_area = Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);

    // Build comprehensive help content with all keybindings
    let content = vec![
        // Navigation
        help_section("Navigation"),
        Line::from(vec![
            help_key(" j "),
            help_desc("/"),
            help_key(" ↓ "),
            help_desc(" Down   "),
            help_key(" k "),
            help_desc("/"),
            help_key(" ↑ "),
            help_desc(" Up   "),
            help_key(" g "),
            help_desc(" Top   "),
            help_key(" G "),
            help_desc(" Bottom"),
        ]),
        Line::from(""),
        // Tree
        help_section("Tree"),
        Line::from(vec![
            help_key(" l "),
            help_desc("/"),
            help_key(" → "),
            help_desc(" Expand   "),
            help_key(" h "),
            help_desc("/"),
            help_key(" ← "),
            help_desc(" Collapse   "),
            help_key(" Tab "),
            help_desc(" Toggle"),
        ]),
        Line::from(vec![
            help_key(" L "),
            help_desc(" Expand all   "),
            help_key(" H "),
            help_desc(" Collapse all   "),
            help_key(" Enter "),
            help_desc(" Toggle/Pick"),
        ]),
        Line::from(""),
        // Selection
        help_section("Selection"),
        Line::from(vec![
            help_key(" Space "),
            help_desc(" Mark   "),
            help_key(" Ctrl+G "),
            help_desc(" Git changed   "),
            help_key(" Ctrl+T "),
            help_desc(" Test pair"),
        ]),
        Line::from(""),
        // File Operations
        help_section("File Operations"),
        Line::from(vec![
            help_key(" a "),
            help_desc(" New file   "),
            help_key(" A "),
            help_desc(" New dir   "),
            help_key(" r "),
            help_desc(" Rename   "),
            help_key(" R "),
            help_desc(" Bulk rename"),
        ]),
        Line::from(vec![
            help_key(" y "),
            help_desc(" Copy   "),
            help_key(" d "),
            help_desc(" Cut   "),
            help_key(" p "),
            help_desc(" Paste   "),
            help_key(" D "),
            help_desc("/"),
            help_key(" Del "),
            help_desc(" Delete"),
        ]),
        Line::from(""),
        // Clipboard
        help_section("Clipboard"),
        Line::from(vec![
            help_key(" c "),
            help_desc(" Path   "),
            help_key(" C "),
            help_desc(" Filename   "),
            help_key(" Y "),
            help_desc(" Content   "),
            help_key(" Ctrl+Y "),
            help_desc(" Claude format"),
        ]),
        Line::from(""),
        // Search & Filter
        help_section("Search & Filter"),
        Line::from(vec![
            help_key(" / "),
            help_desc(" Search   "),
            help_key(" n "),
            help_desc(" Next   "),
            help_key(" N "),
            help_desc(" Prev   "),
            help_key(" Ctrl+P "),
            help_desc(" Fuzzy finder"),
        ]),
        Line::from(vec![
            help_key(" F "),
            help_desc(" Filter   "),
            help_key(" S "),
            help_desc(" Sort mode"),
        ]),
        Line::from(""),
        // Preview
        help_section("Preview"),
        Line::from(vec![
            help_key(" o "),
            help_desc(" Full preview   "),
            help_key(" P "),
            help_desc(" Quick preview"),
        ]),
        Line::from(vec![
            help_key(" b "),
            help_desc("/"),
            help_key(" PgUp "),
            help_desc(" Page up   "),
            help_key(" f "),
            help_desc("/"),
            help_key(" PgDn "),
            help_desc(" Page down   "),
            help_key(" [ "),
            help_key(" ] "),
            help_desc(" PDF pages"),
        ]),
        Line::from(""),
        // Git
        help_section("Git"),
        Line::from(vec![
            help_key(" s "),
            help_desc(" Stage   "),
            help_key(" u "),
            help_desc(" Unstage"),
        ]),
        Line::from(""),
        // Bookmarks
        help_section("Bookmarks"),
        Line::from(vec![
            help_key(" m "),
            help_desc("+"),
            help_key(" 1-9 "),
            help_desc(" Set   "),
            help_key(" ' "),
            help_desc("+"),
            help_key(" 1-9 "),
            help_desc(" Jump"),
        ]),
        Line::from(""),
        // Tabs
        help_section("Tabs"),
        Line::from(vec![
            help_key(" Ctrl+T "),
            help_desc(" New   "),
            help_key(" Ctrl+W "),
            help_desc(" Close   "),
            help_key(" Alt+t "),
            help_desc(" Next   "),
            help_key(" Alt+T "),
            help_desc(" Prev"),
        ]),
        Line::from(""),
        // Other
        help_section("Other"),
        Line::from(vec![
            help_key(" . "),
            help_desc(" Hidden   "),
            help_key(" F5 "),
            help_desc(" Refresh   "),
            help_key(" ? "),
            help_desc(" Help   "),
            help_key(" q "),
            help_desc(" Quit   "),
            help_key(" Q "),
            help_desc(" Quit+cd"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press ? or Esc to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    // Clear background and render
    frame.render_widget(Clear, overlay_area);

    let paragraph = Paragraph::new(content).block(
        Block::default()
            .title(" Keybindings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(HELP_OVERLAY_BG_COLOR)),
    );

    frame.render_widget(paragraph, overlay_area);
}
