//! Help overlay constants, line builders, and the help popup renderer.

use std::sync::OnceLock;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::core::{AppState, ViewMode};
use crate::render::theme::theme;

// Help overlay constants
const HELP_OVERLAY_BG_COLOR: Color = Color::Rgb(30, 30, 40);
const HELP_OVERLAY_MARGIN: u16 = 1;
const HELP_MIN_WIDTH: u16 = 24;

#[derive(Debug, Clone, Copy)]
enum HelpKeyStyle {
    Solid,
    Outline,
    Plain,
}

fn help_key_style() -> HelpKeyStyle {
    static STYLE: OnceLock<HelpKeyStyle> = OnceLock::new();
    *STYLE.get_or_init(|| {
        match std::env::var("FILEVIEW_HELP_KEY_STYLE")
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("outline") => HelpKeyStyle::Outline,
            Some("plain") => HelpKeyStyle::Plain,
            _ => HelpKeyStyle::Solid,
        }
    })
}

/// Create a styled key span with background highlight
fn help_key(key: &str) -> Span<'_> {
    let t = theme();
    let style = match help_key_style() {
        HelpKeyStyle::Solid => Style::default()
            .fg(t.help_key_fg)
            .bg(t.help_key_bg)
            .add_modifier(Modifier::BOLD),
        HelpKeyStyle::Outline => Style::default()
            .fg(t.help_key_bg)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        HelpKeyStyle::Plain => Style::default().fg(t.info),
    };

    Span::styled(key, style)
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

/// Build help content for narrow width (< 45 chars) - vertical layout
fn build_narrow_help() -> Vec<Line<'static>> {
    vec![
        help_section("Navigation"),
        Line::from(vec![help_key(" j "), help_key(" ↓ "), help_desc(" Down")]),
        Line::from(vec![help_key(" k "), help_key(" ↑ "), help_desc(" Up")]),
        Line::from(vec![
            help_key(" g "),
            help_desc(" Top  "),
            help_key(" G "),
            help_desc(" Bottom"),
        ]),
        Line::from(""),
        help_section("Tree"),
        Line::from(vec![help_key(" l "), help_key(" → "), help_desc(" Expand")]),
        Line::from(vec![
            help_key(" h "),
            help_key(" ← "),
            help_desc(" Collapse"),
        ]),
        Line::from(vec![
            help_key(" L "),
            help_desc(" All  "),
            help_key(" H "),
            help_desc(" All"),
        ]),
        Line::from(vec![
            help_key(" Tab "),
            help_key(" Enter "),
            help_desc(" Toggle"),
        ]),
        Line::from(""),
        help_section("Selection"),
        Line::from(vec![help_key(" Space "), help_desc(" Mark")]),
        Line::from(vec![help_key(" ^G "), help_desc(" Git changed")]),
        Line::from(vec![help_key(" ^T "), help_desc(" Test pair")]),
        Line::from(""),
        help_section("File"),
        Line::from(vec![
            help_key(" a "),
            help_desc(" File "),
            help_key(" A "),
            help_desc(" Dir"),
        ]),
        Line::from(vec![help_key(" r "), help_desc(" Rename")]),
        Line::from(vec![
            help_key(" y "),
            help_desc(" Cp "),
            help_key(" d "),
            help_desc(" Cut "),
            help_key(" p "),
            help_desc(" Paste"),
        ]),
        Line::from(vec![help_key(" D "), help_desc(" Delete")]),
        Line::from(""),
        help_section("Clipboard"),
        Line::from(vec![
            help_key(" c "),
            help_desc(" Path "),
            help_key(" C "),
            help_desc(" Name"),
        ]),
        Line::from(vec![help_key(" Y "), help_desc(" Content")]),
        Line::from(vec![help_key(" ^Y "), help_desc(" Claude fmt")]),
        Line::from(""),
        help_section("Search"),
        Line::from(vec![help_key(" / "), help_desc(" Search")]),
        Line::from(vec![
            help_key(" n "),
            help_desc(" Next "),
            help_key(" N "),
            help_desc(" Prev"),
        ]),
        Line::from(vec![help_key(" ^P "), help_desc(" Fuzzy")]),
        Line::from(vec![
            help_key(" F "),
            help_desc(" Filter "),
            help_key(" S "),
            help_desc(" Sort"),
        ]),
        Line::from(""),
        help_section("Preview"),
        Line::from(vec![
            help_key(" o "),
            help_desc(" Open "),
            help_key(" P "),
            help_desc(" Quick"),
        ]),
        Line::from(vec![help_key(" b "), help_key(" f "), help_desc(" Scroll")]),
        Line::from(vec![
            help_key(" [ "),
            help_key(" ] "),
            help_desc(" PDF page"),
        ]),
        Line::from(""),
        help_section("Git"),
        Line::from(vec![
            help_key(" s "),
            help_desc(" Stage "),
            help_key(" u "),
            help_desc(" Unstage"),
        ]),
        Line::from(""),
        help_section("Bookmarks"),
        Line::from(vec![help_key(" m "), help_desc("+1-9 Set")]),
        Line::from(vec![help_key(" ' "), help_desc("+1-9 Jump")]),
        Line::from(""),
        help_section("Tabs"),
        Line::from(vec![
            help_key(" ^T "),
            help_desc(" New "),
            help_key(" ^W "),
            help_desc(" Close"),
        ]),
        Line::from(vec![
            help_key(" A-t "),
            help_desc(" Next "),
            help_key(" A-T "),
            help_desc(" Prev"),
        ]),
        Line::from(""),
        help_section("Other"),
        Line::from(vec![
            help_key(" . "),
            help_desc(" Hidden "),
            help_key(" F5 "),
            help_desc(" Refresh"),
        ]),
        Line::from(vec![
            help_key(" q "),
            help_desc(" Quit "),
            help_key(" Q "),
            help_desc(" Quit+cd"),
        ]),
        Line::from(vec![
            help_key(" ? "),
            help_desc(" Help "),
            help_key(" Esc "),
            help_desc(" Cancel"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "? or Esc to close",
            Style::default().fg(Color::DarkGray),
        )),
    ]
}

/// Build help content for wide width (>= 45 chars) - horizontal layout
fn build_wide_help() -> Vec<Line<'static>> {
    vec![
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
            help_desc(" Up   "),
            help_key(" f "),
            help_desc("/"),
            help_key(" PgDn "),
            help_desc(" Down   "),
            help_key(" [ "),
            help_key(" ] "),
            help_desc(" PDF pages"),
        ]),
        Line::from(""),
        help_section("Git"),
        Line::from(vec![
            help_key(" s "),
            help_desc(" Stage   "),
            help_key(" u "),
            help_desc(" Unstage"),
        ]),
        Line::from(""),
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
    ]
}

/// Render help popup overlay with all keybindings (responsive)
pub fn render_help_popup(frame: &mut Frame, state: &AppState) {
    if !matches!(state.mode, ViewMode::Help) {
        return;
    }

    let area = frame.area();

    // Calculate overlay size - fit to screen with margins
    let overlay_width = area
        .width
        .saturating_sub(HELP_OVERLAY_MARGIN * 2)
        .max(HELP_MIN_WIDTH);
    let overlay_height = area.height.saturating_sub(HELP_OVERLAY_MARGIN * 2).max(10);
    let overlay_x = (area.width.saturating_sub(overlay_width)) / 2;
    let overlay_y = (area.height.saturating_sub(overlay_height)) / 2;

    let overlay_area = Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);

    // Choose layout based on available width
    let inner_width = overlay_width.saturating_sub(2);
    let content = if inner_width < 45 {
        build_narrow_help()
    } else {
        build_wide_help()
    };

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
