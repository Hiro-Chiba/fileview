//! Tree rendering

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::core::{AppState, FocusTarget};
use crate::git::FileStatus;
use crate::render::icons;
use crate::tree::TreeEntry;

/// Render the file tree widget
pub fn render_tree(frame: &mut Frame, state: &AppState, entries: &[&TreeEntry], area: Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;

    let items: Vec<ListItem> = entries
        .iter()
        .skip(state.viewport_top)
        .take(visible_height)
        .enumerate()
        .map(|(i, entry)| {
            let absolute_index = state.viewport_top + i;
            render_entry(state, entry, absolute_index)
        })
        .collect();

    let title = format!(
        " {} ",
        abbreviate_path(&state.root, area.width as usize - 4)
    );

    // Highlight border when tree has focus (and preview is visible)
    let border_style = if state.preview_visible && state.focus_target == FocusTarget::Tree {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style),
    );

    frame.render_widget(list, area);
}

/// Render a single tree entry as a ListItem
fn render_entry(state: &AppState, entry: &TreeEntry, index: usize) -> ListItem<'static> {
    let indent = "  ".repeat(entry.depth);

    let icon = if state.icons_enabled {
        icons::get_icon(&entry.path, entry.is_dir, entry.is_expanded())
    } else if entry.is_dir {
        if entry.is_expanded() {
            "▼"
        } else {
            "▶"
        }
    } else {
        " "
    };

    let is_focused = index == state.focus_index;
    let is_selected = state.selected_paths.contains(&entry.path);
    let is_cut = state
        .clipboard
        .as_ref()
        .is_some_and(|c| c.is_cut() && c.paths().contains(&entry.path));

    let mark_indicator = if is_selected { "*" } else { " " };

    // Get git status color and staging info
    let git_status = state
        .git_status
        .as_ref()
        .map(|g| g.get_status(&entry.path))
        .unwrap_or(FileStatus::Clean);

    let is_staged = state
        .git_status
        .as_ref()
        .is_some_and(|g| g.is_staged(&entry.path));

    let mut style = Style::default();

    // Apply git status color first
    style = match git_status {
        FileStatus::Modified => style.fg(Color::Yellow),
        FileStatus::Added | FileStatus::Untracked => style.fg(Color::Green),
        FileStatus::Deleted => style.fg(Color::Red),
        FileStatus::Renamed => style.fg(Color::Cyan),
        FileStatus::Ignored => style.fg(Color::DarkGray),
        FileStatus::Conflict => style.fg(Color::Magenta),
        FileStatus::Clean => {
            if entry.is_dir {
                style.fg(Color::Blue)
            } else {
                style
            }
        }
    };

    // Override with cut style if applicable
    if is_cut {
        style = style.fg(Color::DarkGray);
    }

    // Apply focus style
    if is_focused {
        style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
    }

    // Stage indicator: + for staged, ~ for modified (unstaged)
    let stage_indicator = if is_staged {
        Span::styled("+", Style::default().fg(Color::Green))
    } else if git_status == FileStatus::Modified {
        Span::styled("~", Style::default().fg(Color::Yellow))
    } else {
        Span::raw(" ")
    };

    let line = Line::from(vec![
        Span::styled(mark_indicator, Style::default().fg(Color::Yellow)),
        stage_indicator,
        Span::styled(format!("{}{} {}", indent, icon, entry.name), style),
    ]);

    ListItem::new(line)
}

/// Abbreviate a path to fit within max_width
fn abbreviate_path(path: &std::path::Path, max_width: usize) -> String {
    let full_path = path.display().to_string();

    if full_path.len() <= max_width {
        return full_path;
    }

    let components: Vec<&str> = full_path.split('/').collect();
    if components.is_empty() {
        return full_path;
    }

    let last = components.last().unwrap_or(&"");

    let mut abbreviated: Vec<String> = components[..components.len() - 1]
        .iter()
        .map(|c| {
            if c.is_empty() {
                String::new()
            } else {
                c.chars().next().unwrap_or_default().to_string()
            }
        })
        .collect();
    abbreviated.push((*last).to_string());

    let result = abbreviated.join("/");

    if result.len() > max_width {
        if last.len() > max_width {
            format!("...{}", &last[last.len().saturating_sub(max_width - 3)..])
        } else {
            (*last).to_string()
        }
    } else {
        result
    }
}

/// Calculate visible height for the tree area
pub fn visible_height(area: Rect) -> usize {
    area.height.saturating_sub(2) as usize
}
