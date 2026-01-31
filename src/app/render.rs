//! Rendering helpers for the event loop

use std::path::PathBuf;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::PreviewState;
use crate::core::{AppState, FocusTarget, ViewMode};
use crate::handler::action::get_filename_str;
use crate::render::{
    render_archive_preview, render_bulk_rename_dialog, render_diff_preview, render_directory_info,
    render_fuzzy_finder, render_help_popup, render_hex_preview, render_image_preview,
    render_input_popup, render_pdf_preview, render_status_bar, render_text_preview, render_tree,
    FontSize, FuzzyMatch, Picker,
};
use crate::tree::TreeEntry;

/// Context for rendering a frame
pub struct RenderContext<'a> {
    pub state: &'a AppState,
    pub entries: Vec<&'a TreeEntry>,
    pub focused_path: Option<&'a PathBuf>,
    pub preview: &'a mut PreviewState,
    pub fuzzy_results: &'a [FuzzyMatch],
    pub image_picker: &'a mut Option<Picker>,
}

/// Render a complete frame
pub fn render_frame(frame: &mut Frame, mut ctx: RenderContext) {
    let size = frame.area();

    // Get font size for image centering (default to typical terminal cell size)
    let font_size: FontSize = ctx
        .image_picker
        .as_ref()
        .map(|p| p.font_size())
        .unwrap_or((10, 20));

    // Check if fullscreen preview mode is active
    let is_fullscreen_preview = matches!(ctx.state.mode, ViewMode::Preview { .. });

    if is_fullscreen_preview {
        render_fullscreen_preview(frame, &mut ctx, size, font_size);
    } else {
        render_normal_mode(frame, &mut ctx, size, font_size);
    }
}

/// Render fullscreen preview mode
fn render_fullscreen_preview(
    frame: &mut Frame,
    ctx: &mut RenderContext,
    size: Rect,
    font_size: FontSize,
) {
    let filename = get_filename_str(ctx.focused_path);
    let title = if filename.is_empty() {
        " Preview (press o or q to close) ".to_string()
    } else {
        format!(" {} (press o or q to close) ", filename)
    };

    if let Some(ref di) = ctx.preview.dir_info {
        render_directory_info(frame, di, size, false);
    } else if let Some(ref dp) = ctx.preview.diff {
        render_diff_preview(frame, dp, size, &title, false);
    } else if let Some(ref tp) = ctx.preview.text {
        render_text_preview(frame, tp, size, &title, false);
    } else if let Some(ref mut ip) = ctx.preview.image {
        render_image_preview(frame, ip, size, &title, false, font_size);
    } else if let Some(ref mut pdf) = ctx.preview.pdf {
        render_pdf_preview(frame, pdf, size, &filename, false, font_size);
    } else if let Some(ref hp) = ctx.preview.hex {
        render_hex_preview(frame, hp, size, &title, false);
    } else if let Some(ref ap) = ctx.preview.archive {
        render_archive_preview(frame, ap, size, &title, false);
    } else {
        let block = Block::default().borders(Borders::ALL).title(title);
        let para = Paragraph::new("No preview available").block(block);
        frame.render_widget(para, size);
    }
}

/// Render normal mode (tree with optional side preview)
fn render_normal_mode(frame: &mut Frame, ctx: &mut RenderContext, size: Rect, font_size: FontSize) {
    let main_chunks = if ctx.state.preview_visible {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(size)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(size)
    };

    // Tree area with status bar
    let tree_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(main_chunks[0]);

    // Render tree (viewport adjustment is done in event loop)
    render_tree(frame, ctx.state, &ctx.entries, tree_chunks[0]);

    // Render status bar
    render_status_bar(frame, ctx.state, ctx.focused_path, tree_chunks[1]);

    // Render preview if visible
    if ctx.state.preview_visible && main_chunks.len() > 1 {
        render_side_preview(frame, ctx, main_chunks[1], font_size);
    }

    // Render input popup if needed
    render_input_popup(frame, ctx.state);

    // Render fuzzy finder if in FuzzyFinder mode
    if let ViewMode::FuzzyFinder { query, selected } = &ctx.state.mode {
        // Bound selected index to results length
        let bounded_selected = if ctx.fuzzy_results.is_empty() {
            0
        } else {
            (*selected).min(ctx.fuzzy_results.len() - 1)
        };
        render_fuzzy_finder(frame, query, ctx.fuzzy_results, bounded_selected, size);
    }

    // Render help popup if in Help mode
    render_help_popup(frame, ctx.state);

    // Render bulk rename dialog if in BulkRename mode
    if matches!(ctx.state.mode, ViewMode::BulkRename { .. }) {
        render_bulk_rename_dialog(frame, ctx.state);
    }
}

/// Render side preview panel
fn render_side_preview(
    frame: &mut Frame,
    ctx: &mut RenderContext,
    area: Rect,
    font_size: FontSize,
) {
    let title = get_filename_str(ctx.focused_path);
    let preview_focused = ctx.state.focus_target == FocusTarget::Preview;

    if let Some(ref di) = ctx.preview.dir_info {
        render_directory_info(frame, di, area, preview_focused);
    } else if let Some(ref dp) = ctx.preview.diff {
        render_diff_preview(frame, dp, area, &title, preview_focused);
    } else if let Some(ref tp) = ctx.preview.text {
        render_text_preview(frame, tp, area, &title, preview_focused);
    } else if let Some(ref mut ip) = ctx.preview.image {
        render_image_preview(frame, ip, area, &title, preview_focused, font_size);
    } else if let Some(ref mut pdf) = ctx.preview.pdf {
        render_pdf_preview(frame, pdf, area, &title, preview_focused, font_size);
    } else if let Some(ref hp) = ctx.preview.hex {
        render_hex_preview(frame, hp, area, &title, preview_focused);
    } else if let Some(ref ap) = ctx.preview.archive {
        render_archive_preview(frame, ap, area, &title, preview_focused);
    } else {
        let border_style = if preview_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Preview ")
            .border_style(border_style);
        let para = Paragraph::new("No preview available").block(block);
        frame.render_widget(para, area);
    }
}
