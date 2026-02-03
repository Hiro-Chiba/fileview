//! Tree rendering

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use super::layout::LayoutEngine;
use super::theme::theme;
use crate::core::{AppState, FocusTarget, UiDensity};
use crate::git::FileStatus;
use crate::render::icons;
use crate::tree::TreeEntry;

/// Render the file tree widget
pub fn render_tree(frame: &mut Frame, state: &AppState, entries: &[&TreeEntry], area: Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let layout = LayoutEngine::from_rect(area);
    let tree_cols = layout.tree_columns(area);

    let items: Vec<ListItem> = entries
        .iter()
        .skip(state.viewport_top)
        .take(visible_height)
        .enumerate()
        .map(|(i, entry)| {
            let absolute_index = state.viewport_top + i;
            render_entry(state, entry, absolute_index, &layout, &tree_cols)
        })
        .collect();

    let title = format!(
        " {} ",
        abbreviate_path(&state.root, area.width as usize - 4)
    );

    // Highlight border when tree has focus (and preview is visible)
    let t = theme();
    let border_style = if state.preview_visible && state.focus_target == FocusTarget::Tree {
        Style::default().fg(t.border_active)
    } else {
        Style::default().fg(t.border)
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
fn render_entry(
    state: &AppState,
    entry: &TreeEntry,
    index: usize,
    layout: &LayoutEngine,
    tree_cols: &super::layout::TreeColumns,
) -> ListItem<'static> {
    let t = theme();
    let density = layout.density;

    // Adjust indent based on density
    let indent_str = match density {
        UiDensity::Ultra | UiDensity::Narrow => " ".repeat(entry.depth),
        UiDensity::Compact | UiDensity::Full => "  ".repeat(entry.depth),
    };

    // Icon selection based on density and settings
    let icon = if tree_cols.show_icons && state.icons_enabled {
        icons::get_icon(&entry.path, entry.is_dir, entry.is_expanded())
    } else if entry.is_dir {
        // Use compact indicators in narrow modes
        if entry.is_expanded() {
            "▾"
        } else {
            "▸"
        }
    } else {
        ""
    };

    let is_focused = index == state.focus_index;
    let is_selected = state.selected_paths.contains(&entry.path);
    let is_cut = state
        .clipboard
        .as_ref()
        .is_some_and(|c| c.is_cut() && c.paths().contains(&entry.path));

    // Compact mark indicator for ultra mode
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

    // Apply git status color first (using theme colors)
    style = match git_status {
        FileStatus::Modified => style.fg(t.git_modified),
        FileStatus::Added | FileStatus::Untracked => style.fg(t.git_untracked),
        FileStatus::Deleted => style.fg(t.git_deleted),
        FileStatus::Renamed => style.fg(t.git_renamed),
        FileStatus::Ignored => style.fg(t.git_ignored),
        FileStatus::Conflict => style.fg(t.git_conflict),
        FileStatus::Clean => {
            if entry.is_dir {
                style.fg(t.directory)
            } else {
                style
            }
        }
    };

    // Override with cut style if applicable
    if is_cut {
        style = style.fg(t.git_ignored);
    }

    // Apply focus style
    if is_focused {
        style = style.bg(t.selection).add_modifier(Modifier::BOLD);
    }

    // Stage indicator: compact in ultra mode
    let stage_indicator = match density {
        UiDensity::Ultra => {
            // In ultra mode, combine mark and stage into one char
            if is_staged {
                Span::styled("✓", Style::default().fg(t.git_staged))
            } else if git_status == FileStatus::Modified {
                Span::styled("M", Style::default().fg(t.git_modified))
            } else {
                Span::raw("")
            }
        }
        _ => {
            if is_staged {
                Span::styled("+", Style::default().fg(t.git_staged))
            } else if git_status == FileStatus::Modified {
                Span::styled("~", Style::default().fg(t.git_modified))
            } else {
                Span::raw(" ")
            }
        }
    };

    // Truncate filename if needed for narrow modes
    let max_name_width = tree_cols.filename_width_at_depth(entry.depth) as usize;
    let display_name = if entry.name.len() > max_name_width && max_name_width > 3 {
        format!("{}…", &entry.name[..max_name_width - 1])
    } else {
        entry.name.clone()
    };

    // Build the line based on density
    let line = match density {
        UiDensity::Ultra => {
            // Ultra compact: mark + indent + icon + name + stage (at end)
            let entry_text = if icon.is_empty() {
                format!("{}{}", indent_str, display_name)
            } else {
                format!("{}{} {}", indent_str, icon, display_name)
            };
            Line::from(vec![
                Span::styled(mark_indicator, Style::default().fg(t.mark)),
                Span::styled(entry_text, style),
                stage_indicator,
            ])
        }
        UiDensity::Narrow => {
            // Narrow: mark + stage + indent + icon + name
            let entry_text = if icon.is_empty() {
                format!("{}{}", indent_str, display_name)
            } else {
                format!("{}{} {}", indent_str, icon, display_name)
            };
            Line::from(vec![
                Span::styled(mark_indicator, Style::default().fg(t.mark)),
                stage_indicator,
                Span::styled(entry_text, style),
            ])
        }
        _ => {
            // Full/Compact: standard layout with space after icon
            let icon_with_space = if icon.is_empty() {
                String::new()
            } else {
                format!("{} ", icon)
            };
            Line::from(vec![
                Span::styled(mark_indicator, Style::default().fg(t.mark)),
                stage_indicator,
                Span::styled(
                    format!("{}{}{}", indent_str, icon_with_space, display_name),
                    style,
                ),
            ])
        }
    };

    ListItem::new(line)
}

/// Abbreviate a path to fit within max_width
/// Adaptive abbreviation based on available width:
/// - max_width < 20: filename only, truncated if needed
/// - max_width 20-30: last directory + filename
/// - max_width > 30: single-char abbreviation for parent dirs
fn abbreviate_path(path: &std::path::Path, max_width: usize) -> String {
    let full_path = path.display().to_string();

    // If it fits, return as-is
    if full_path.len() <= max_width {
        return full_path;
    }

    let components: Vec<&str> = full_path.split('/').collect();
    if components.is_empty() {
        return full_path;
    }

    let last = components.last().unwrap_or(&"");

    // Ultra-narrow: filename only (< 20 chars)
    if max_width < 20 {
        if last.len() > max_width {
            // Truncate filename with ellipsis
            return format!("…{}", &last[last.len().saturating_sub(max_width - 1)..]);
        }
        return (*last).to_string();
    }

    // Very narrow (20-30): show last dir + filename
    if max_width < 30 {
        if components.len() >= 2 {
            let parent = components[components.len() - 2];
            let short_parent = if parent.len() > 8 {
                format!("{}…", &parent[..7])
            } else {
                parent.to_string()
            };
            let result = format!("{}/{}", short_parent, last);
            if result.len() <= max_width {
                return result;
            }
        }
        // Fall back to just filename
        if last.len() > max_width {
            return format!("…{}", &last[last.len().saturating_sub(max_width - 1)..]);
        }
        return (*last).to_string();
    }

    // Standard narrow: single-char abbreviation for all parent dirs
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
            format!("…{}", &last[last.len().saturating_sub(max_width - 1)..])
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
