//! Tree rendering

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::core::AppState;
use crate::git::FileStatus;
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
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));

    frame.render_widget(list, area);
}

/// Render a single tree entry as a ListItem
fn render_entry(state: &AppState, entry: &TreeEntry, index: usize) -> ListItem<'static> {
    let indent = "  ".repeat(entry.depth);

    let icon = if entry.is_dir {
        if entry.is_expanded() {
            "\u{f07c}" // Open folder icon
        } else {
            "\u{f07b}" // Closed folder icon
        }
    } else {
        get_file_icon(&entry.name)
    };

    let is_focused = index == state.focus_index;
    let is_selected = state.selected_paths.contains(&entry.path);
    let is_cut = state
        .clipboard
        .as_ref()
        .is_some_and(|c| c.is_cut() && c.paths().contains(&entry.path));

    let mark_indicator = if is_selected { "*" } else { " " };

    // Get git status color
    let git_status = state
        .git_status
        .as_ref()
        .map(|g| g.get_status(&entry.path))
        .unwrap_or(FileStatus::Clean);

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

    let line = Line::from(vec![
        Span::styled(mark_indicator, Style::default().fg(Color::Yellow)),
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

/// Get file icon based on extension
fn get_file_icon(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext.to_lowercase().as_str() {
        "rs" => "",
        "py" => "",
        "js" | "jsx" => "",
        "ts" | "tsx" => "",
        "html" => "",
        "css" | "scss" | "sass" => "",
        "json" => "",
        "toml" | "yaml" | "yml" => "",
        "md" => "",
        "txt" => "",
        "gitignore" => "",
        "lock" => "",
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "webp" => "",
        "mp3" | "wav" | "flac" => "",
        "mp4" | "mkv" | "avi" => "",
        "zip" | "tar" | "gz" | "rar" => "",
        "pdf" => "",
        "sh" | "bash" | "zsh" => "",
        _ => "",
    }
}

/// Calculate visible height for the tree area
pub fn visible_height(area: Rect) -> usize {
    area.height.saturating_sub(2) as usize
}
