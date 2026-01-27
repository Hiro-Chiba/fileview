//! FileView - A minimal file tree UI for terminal emulators

use std::env;
use std::io::{self, stdout};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;
use std::time::Duration;

use crossterm::{
    cursor,
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event,
    },
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use fileview::action::{file as file_ops, Clipboard, ClipboardContent};
use fileview::core::{AppState, InputPurpose, PendingAction, ViewMode};
use fileview::handler::{
    key::{create_delete_targets, handle_key_event, update_input_buffer, KeyAction},
    mouse::{handle_mouse_event, ClickDetector, MouseAction, PathBuffer},
};
use fileview::integrate::{exit_code, Callback, OutputFormat, PickResult};
use fileview::render::{
    is_binary_file, is_image_file, is_text_file, render_directory_info, render_hex_preview,
    render_image_preview, render_input_popup, render_status_bar, render_text_preview, render_tree,
    visible_height, DirectoryInfo, HexPreview, ImagePreview, TextPreview,
};
use fileview::tree::TreeNavigator;

/// Application configuration from CLI args
struct Config {
    root: PathBuf,
    pick_mode: bool,
    output_format: OutputFormat,
    callback: Option<Callback>,
}

impl Config {
    fn from_args() -> anyhow::Result<Self> {
        let mut args = env::args().skip(1).peekable();
        let mut root = env::current_dir()?;
        let mut pick_mode = false;
        let mut output_format = OutputFormat::default();
        let mut callback: Option<Callback> = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--pick" | "-p" => pick_mode = true,
                "--format" | "-f" => {
                    if let Some(fmt) = args.next() {
                        output_format = OutputFormat::from_str(&fmt).unwrap_or_default();
                    }
                }
                "--on-select" => {
                    if let Some(cmd) = args.next() {
                        callback = Some(Callback::new(cmd));
                    }
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                "--version" | "-V" => {
                    println!("fv {}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                path if !path.starts_with('-') => {
                    let p = PathBuf::from(path);
                    if p.is_dir() {
                        root = p.canonicalize()?;
                    } else if p.is_file() {
                        if let Some(parent) = p.parent() {
                            root = parent.canonicalize()?;
                        }
                    } else {
                        anyhow::bail!("Path does not exist: {}", path);
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            root,
            pick_mode,
            output_format,
            callback,
        })
    }
}

fn print_help() {
    println!(
        r#"fv - FileView: A minimal file tree UI

USAGE:
    fv [OPTIONS] [PATH]

OPTIONS:
    -p, --pick          Pick mode: output selected path(s) to stdout
    -f, --format FMT    Output format for pick mode: lines, null, json
    --on-select CMD     Run command when file is selected (use {{path}}, {{name}}, etc.)
    -h, --help          Show this help message
    -V, --version       Show version

KEYBINDINGS:
    j/↓         Move down
    k/↑         Move up
    g           Go to top
    G           Go to bottom
    l/→/Tab     Expand / Toggle
    h/←/BS      Collapse
    H           Collapse all
    L           Expand all
    Space       Toggle mark
    Enter       Toggle expand (or select in pick mode)
    y           Copy to clipboard
    d           Cut to clipboard
    D/Del       Delete (with confirmation)
    p           Paste
    a           New file
    A           New directory
    r           Rename
    /           Search
    n           Next search result
    .           Toggle hidden files
    R/F5        Refresh
    o           Open preview
    P           Toggle quick preview panel
    c           Copy path to system clipboard
    C           Copy filename to system clipboard
    q/Esc       Quit (or cancel in pick mode)
    ?           Show help

PLACEHOLDERS for --on-select:
    {{path}}    Full path
    {{dir}}     Parent directory
    {{name}}    Filename with extension
    {{stem}}    Filename without extension
    {{ext}}     Extension only
"#
    );
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(exit_code::ERROR as u8)
        }
    }
}

fn run() -> anyhow::Result<i32> {
    let config = Config::from_args()?;

    // Initialize terminal
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let result = run_app(&mut terminal, config);

    // Restore terminal
    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste,
        cursor::Show
    )?;

    result
}

/// Snapshot of entry data for use after dropping borrow
#[derive(Clone)]
struct EntrySnapshot {
    path: PathBuf,
    name: String,
    is_dir: bool,
    depth: usize,
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: Config,
) -> anyhow::Result<i32> {
    let mut state = AppState::new(config.root.clone());
    state.pick_mode = config.pick_mode;

    let mut navigator = TreeNavigator::new(&config.root, state.show_hidden)?;
    let mut click_detector = ClickDetector::new();
    let mut path_buffer = PathBuffer::new();

    // Preview cache
    let mut text_preview: Option<TextPreview> = None;
    let mut image_preview: Option<ImagePreview> = None;
    let mut dir_info: Option<DirectoryInfo> = None;
    let mut hex_preview: Option<HexPreview> = None;

    loop {
        // Get visible entries and create snapshots
        let entries = navigator.visible_entries();
        let total_entries = entries.len();
        let snapshots: Vec<EntrySnapshot> = entries
            .iter()
            .map(|e| EntrySnapshot {
                path: e.path.clone(),
                name: e.name.clone(),
                is_dir: e.is_dir,
                depth: e.depth,
            })
            .collect();

        // Ensure focus is within bounds
        if state.focus_index >= total_entries && total_entries > 0 {
            state.focus_index = total_entries - 1;
        }

        // Get focused entry path
        let focused_path = snapshots.get(state.focus_index).map(|e| e.path.clone());

        // Update preview if needed (side panel or fullscreen mode)
        let needs_preview = state.preview_visible || matches!(state.mode, ViewMode::Preview { .. });
        if needs_preview {
            if let Some(path) = &focused_path {
                if path.is_dir() {
                    // Load directory info
                    if let Ok(info) = DirectoryInfo::from_path(path) {
                        dir_info = Some(info);
                        text_preview = None;
                        image_preview = None;
                        hex_preview = None;
                    }
                } else if is_text_file(path) {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        text_preview = Some(TextPreview::new(&content));
                        image_preview = None;
                        dir_info = None;
                        hex_preview = None;
                    }
                } else if is_image_file(path) {
                    if let Ok(img) = ImagePreview::load(path) {
                        image_preview = Some(img);
                        text_preview = None;
                        dir_info = None;
                        hex_preview = None;
                    }
                } else if is_binary_file(path) || path.is_file() {
                    // Binary file or unknown type - show hex preview
                    if let Ok(hex) = HexPreview::load(path) {
                        hex_preview = Some(hex);
                        text_preview = None;
                        image_preview = None;
                        dir_info = None;
                    }
                } else {
                    // Clear all previews
                    text_preview = None;
                    image_preview = None;
                    dir_info = None;
                    hex_preview = None;
                }
            }
        }

        // Render
        terminal.draw(|frame| {
            let size = frame.area();

            // Check if fullscreen preview mode is active
            let is_fullscreen_preview = matches!(state.mode, ViewMode::Preview { .. });

            if is_fullscreen_preview {
                // Fullscreen preview mode - render preview only
                let title = focused_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| format!(" {} (press o or q to close) ", n.to_string_lossy()))
                    .unwrap_or_else(|| " Preview (press o or q to close) ".to_string());

                if let Some(ref di) = dir_info {
                    render_directory_info(frame, di, size);
                } else if let Some(ref tp) = text_preview {
                    render_text_preview(frame, tp, size, &title);
                } else if let Some(ref ip) = image_preview {
                    render_image_preview(frame, ip, size, &title);
                } else if let Some(ref hp) = hex_preview {
                    render_hex_preview(frame, hp, size, &title);
                } else {
                    let block = Block::default().borders(Borders::ALL).title(title);
                    let para = Paragraph::new("No preview available").block(block);
                    frame.render_widget(para, size);
                }
            } else {
                // Normal mode - tree with optional side preview
                let main_chunks = if state.preview_visible {
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

                // Adjust viewport
                let vis_height = visible_height(tree_chunks[0]);
                state.adjust_viewport(vis_height);

                // Render tree
                render_tree(frame, &state, &entries, tree_chunks[0]);

                // Render status bar
                render_status_bar(frame, &state, total_entries, tree_chunks[1]);

                // Render preview if visible
                if state.preview_visible && main_chunks.len() > 1 {
                    let preview_area = main_chunks[1];
                    if let Some(ref di) = dir_info {
                        render_directory_info(frame, di, preview_area);
                    } else if let Some(ref tp) = text_preview {
                        let title = focused_path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        render_text_preview(frame, tp, preview_area, &title);
                    } else if let Some(ref ip) = image_preview {
                        let title = focused_path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        render_image_preview(frame, ip, preview_area, &title);
                    } else if let Some(ref hp) = hex_preview {
                        let title = focused_path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        render_hex_preview(frame, hp, preview_area, &title);
                    } else {
                        let block = Block::default().borders(Borders::ALL).title(" Preview ");
                        let para = Paragraph::new("No preview available").block(block);
                        frame.render_widget(para, preview_area);
                    }
                }

                // Render input popup if needed
                render_input_popup(frame, &state);
            }
        })?;

        // Drop the entries borrow before event handling
        drop(entries);

        // Check drop buffer timeout (for file drop detection via rapid key input)
        if path_buffer.is_ready() {
            let paths = path_buffer.take_paths();
            if !paths.is_empty() {
                // Valid paths detected - copy files
                if let Some(focused) = &focused_path {
                    let dest = if focused.is_dir() {
                        focused.clone()
                    } else {
                        focused.parent().unwrap_or(&state.root).to_path_buf()
                    };
                    for src in &paths {
                        let _ = file_ops::copy_to(src, &dest);
                    }
                    navigator.reload()?;
                    state.refresh_git_status();
                    state.set_message(format!("Dropped {} file(s)", paths.len()));
                }
            } else {
                // Not valid paths - check if it starts with '/' for search
                let buffer = path_buffer.take_raw();
                if let Some(rest) = buffer.strip_prefix('/') {
                    // Treat as search command
                    state.mode = ViewMode::Search {
                        query: rest.to_string(),
                    };
                }
            }
        }

        // Handle events
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    // Handle input buffer updates first
                    if let ViewMode::Input {
                        purpose,
                        buffer,
                        cursor,
                    } = &state.mode
                    {
                        if let Some((new_buf, new_cur)) = update_input_buffer(key, buffer, *cursor)
                        {
                            state.mode = ViewMode::Input {
                                purpose: purpose.clone(),
                                buffer: new_buf,
                                cursor: new_cur,
                            };
                            continue;
                        }
                    }

                    if let ViewMode::Search { query } = &state.mode {
                        if let Some((new_buf, _)) = update_input_buffer(key, query, query.len()) {
                            state.mode = ViewMode::Search { query: new_buf };
                            continue;
                        }
                    }

                    // Buffer characters for potential file drop detection (Ghostty, etc.)
                    // Only in Browse mode to avoid interfering with text input
                    if matches!(state.mode, ViewMode::Browse) {
                        if let crossterm::event::KeyCode::Char(c) = key.code {
                            // Start buffering on path-like characters
                            if matches!(c, '/' | '\'' | '"' | '\\') {
                                path_buffer.push(c);
                                continue;
                            }

                            // Continue buffering if we already have content
                            if !path_buffer.is_empty() {
                                path_buffer.push(c);
                                continue;
                            }
                        }
                    }

                    let action = handle_key_event(&state, key);
                    match handle_action(
                        action,
                        &mut state,
                        &mut navigator,
                        &focused_path,
                        &snapshots,
                        &config,
                        &mut text_preview,
                    )? {
                        ActionResult::Continue => {}
                        ActionResult::Quit(code) => return Ok(code),
                    }
                }
                Event::Mouse(mouse) => {
                    let tree_top = 0; // Assuming tree starts at row 0
                    let action = handle_mouse_event(mouse, &mut click_detector, tree_top);

                    match action {
                        MouseAction::Click { row } => {
                            let idx = state.viewport_top + row as usize;
                            if idx < snapshots.len() {
                                state.focus_index = idx;
                            }
                        }
                        MouseAction::DoubleClick { row } => {
                            let idx = state.viewport_top + row as usize;
                            if idx < snapshots.len() {
                                state.focus_index = idx;
                                if let Some(entry) = snapshots.get(idx) {
                                    if entry.is_dir {
                                        let _ = navigator.toggle_expand(&entry.path);
                                    }
                                }
                            }
                        }
                        MouseAction::ScrollUp(n) => {
                            state.focus_index = state.focus_index.saturating_sub(n);
                        }
                        MouseAction::ScrollDown(n) => {
                            state.focus_index =
                                (state.focus_index + n).min(snapshots.len().saturating_sub(1));
                        }
                        MouseAction::FileDrop { paths } => {
                            // Handle dropped files - copy them to current directory
                            if let Some(focused) = &focused_path {
                                let dest = if focused.is_dir() {
                                    focused.clone()
                                } else {
                                    focused.parent().unwrap_or(&state.root).to_path_buf()
                                };
                                for src in &paths {
                                    let _ = file_ops::copy_to(src, &dest);
                                }
                                navigator.reload()?;
                                state.refresh_git_status();
                                state.set_message(format!("Dropped {} file(s)", paths.len()));
                            }
                        }
                        MouseAction::None => {}
                    }
                }
                Event::Paste(text) => {
                    // Handle terminal paste - might be file drop
                    for c in text.chars() {
                        path_buffer.push(c);
                    }
                    let paths = path_buffer.take_paths();
                    if !paths.is_empty() {
                        if let Some(focused) = &focused_path {
                            let dest = if focused.is_dir() {
                                focused.clone()
                            } else {
                                focused.parent().unwrap_or(&state.root).to_path_buf()
                            };
                            for src in &paths {
                                let _ = file_ops::copy_to(src, &dest);
                            }
                            navigator.reload()?;
                            state.refresh_git_status();
                            state.set_message(format!("Dropped {} file(s)", paths.len()));
                        }
                    }
                    path_buffer.clear();
                }
                _ => {}
            }
        }

        // Check quit flag
        if state.should_quit {
            return Ok(exit_code::SUCCESS);
        }
    }
}

enum ActionResult {
    Continue,
    Quit(i32),
}

fn handle_action(
    action: KeyAction,
    state: &mut AppState,
    navigator: &mut TreeNavigator,
    focused_path: &Option<PathBuf>,
    entries: &[EntrySnapshot],
    config: &Config,
    text_preview: &mut Option<TextPreview>,
) -> anyhow::Result<ActionResult> {
    match action {
        KeyAction::None => {}
        KeyAction::Quit => {
            state.should_quit = true;
        }
        KeyAction::Cancel => {
            match &state.mode {
                ViewMode::Browse => {
                    if state.pick_mode {
                        // Cancel in pick mode = exit with cancelled code
                        return Ok(ActionResult::Quit(exit_code::CANCELLED));
                    }
                    state.should_quit = true;
                }
                _ => {
                    state.mode = ViewMode::Browse;
                    state.clear_message();
                }
            }
        }
        KeyAction::MoveUp => {
            state.focus_index = state.focus_index.saturating_sub(1);
        }
        KeyAction::MoveDown => {
            if state.focus_index < entries.len().saturating_sub(1) {
                state.focus_index += 1;
            }
        }
        KeyAction::MoveToTop => {
            state.focus_index = 0;
        }
        KeyAction::MoveToBottom => {
            state.focus_index = entries.len().saturating_sub(1);
        }
        KeyAction::Expand => {
            if let Some(path) = focused_path {
                navigator.expand(path)?;
            }
        }
        KeyAction::Collapse => {
            if let Some(path) = focused_path {
                navigator.collapse(path);
            }
        }
        KeyAction::ToggleExpand => {
            if let Some(path) = focused_path {
                navigator.toggle_expand(path)?;
            }
        }
        KeyAction::CollapseAll => {
            // Collapse all except root
            let entries_to_collapse: Vec<_> = entries
                .iter()
                .filter(|e| e.is_dir && e.depth > 0)
                .map(|e| e.path.clone())
                .collect();
            for path in entries_to_collapse {
                navigator.collapse(&path);
            }
        }
        KeyAction::ExpandAll => {
            // Expand all directories (limited depth to avoid huge trees)
            let entries_to_expand: Vec<_> = entries
                .iter()
                .filter(|e| e.is_dir && e.depth < 5)
                .map(|e| e.path.clone())
                .collect();
            for path in entries_to_expand {
                navigator.expand(&path)?;
            }
        }
        KeyAction::ToggleMark => {
            if let Some(path) = focused_path {
                if state.selected_paths.contains(path) {
                    state.selected_paths.remove(path);
                } else {
                    state.selected_paths.insert(path.clone());
                }
            }
        }
        KeyAction::ClearMarks => {
            state.selected_paths.clear();
        }
        KeyAction::Copy => {
            let paths: Vec<PathBuf> = if state.selected_paths.is_empty() {
                focused_path.clone().into_iter().collect()
            } else {
                state.selected_paths.iter().cloned().collect()
            };
            if !paths.is_empty() {
                let mut clipboard = Clipboard::new();
                let count = paths.len();
                clipboard.copy(paths);
                state.clipboard = Some(clipboard);
                state.set_message(format!("Copied {} item(s)", count));
            }
        }
        KeyAction::Cut => {
            let paths: Vec<PathBuf> = if state.selected_paths.is_empty() {
                focused_path.clone().into_iter().collect()
            } else {
                state.selected_paths.iter().cloned().collect()
            };
            if !paths.is_empty() {
                let mut clipboard = Clipboard::new();
                let count = paths.len();
                clipboard.cut(paths);
                state.clipboard = Some(clipboard);
                state.set_message(format!("Cut {} item(s)", count));
            }
        }
        KeyAction::Paste => {
            if let Some(ref mut clipboard) = state.clipboard {
                if let Some(content) = clipboard.take() {
                    let dest = focused_path
                        .as_ref()
                        .and_then(|p| {
                            if p.is_dir() {
                                Some(p.clone())
                            } else {
                                p.parent().map(|pp| pp.to_path_buf())
                            }
                        })
                        .unwrap_or_else(|| state.root.clone());

                    match content {
                        ClipboardContent::Copy(paths) => {
                            for src in &paths {
                                file_ops::copy_to(src, &dest)?;
                            }
                            state.set_message(format!("Pasted {} item(s)", paths.len()));
                        }
                        ClipboardContent::Cut(paths) => {
                            for src in &paths {
                                if let Some(name) = src.file_name() {
                                    let new_path = dest.join(name);
                                    std::fs::rename(src, new_path)?;
                                }
                            }
                            state.set_message(format!("Moved {} item(s)", paths.len()));
                        }
                    }
                    navigator.reload()?;
                    state.refresh_git_status();
                }
            }
        }
        KeyAction::ConfirmDelete => {
            let targets = create_delete_targets(state, focused_path.as_ref());
            if !targets.is_empty() {
                state.mode = ViewMode::Confirm {
                    action: PendingAction::Delete { targets },
                };
            }
        }
        KeyAction::ExecuteDelete => {
            if let ViewMode::Confirm {
                action: PendingAction::Delete { targets },
            } = &state.mode
            {
                for path in targets {
                    file_ops::delete(path)?;
                }
                state.set_message(format!("Deleted {} item(s)", targets.len()));
                state.selected_paths.clear();
                state.mode = ViewMode::Browse;
                navigator.reload()?;
                state.refresh_git_status();
            }
        }
        KeyAction::StartRename => {
            if let Some(path) = focused_path {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                state.mode = ViewMode::Input {
                    purpose: InputPurpose::Rename {
                        original: path.clone(),
                    },
                    buffer: name.clone(),
                    cursor: name.len(),
                };
            }
        }
        KeyAction::StartNewFile => {
            state.mode = ViewMode::Input {
                purpose: InputPurpose::CreateFile,
                buffer: String::new(),
                cursor: 0,
            };
        }
        KeyAction::StartNewDir => {
            state.mode = ViewMode::Input {
                purpose: InputPurpose::CreateDir,
                buffer: String::new(),
                cursor: 0,
            };
        }
        KeyAction::StartSearch => {
            state.mode = ViewMode::Search {
                query: String::new(),
            };
        }
        KeyAction::SearchNext => {
            if let ViewMode::Search { query } = &state.mode {
                if !query.is_empty() {
                    let query_lower = query.to_lowercase();
                    // Find next match starting from current position
                    let start = (state.focus_index + 1) % entries.len();
                    for i in 0..entries.len() {
                        let idx = (start + i) % entries.len();
                        if entries[idx].name.to_lowercase().contains(&query_lower) {
                            state.focus_index = idx;
                            break;
                        }
                    }
                }
            }
        }
        KeyAction::Refresh => {
            navigator.reload()?;
            state.refresh_git_status();
            state.set_message("Refreshed");
        }
        KeyAction::ToggleHidden => {
            state.show_hidden = !state.show_hidden;
            navigator.set_show_hidden(state.show_hidden)?;
            state.set_message(if state.show_hidden {
                "Showing hidden files"
            } else {
                "Hiding hidden files"
            });
        }
        KeyAction::CopyPath => {
            if let Some(path) = focused_path {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let path_str = path.display().to_string();
                    let _ = clipboard.set_text(&path_str);
                    state.set_message("Path copied to clipboard");
                }
            }
        }
        KeyAction::CopyFilename => {
            if let Some(path) = focused_path {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let _ = clipboard.set_text(&name);
                    state.set_message("Filename copied to clipboard");
                }
            }
        }
        KeyAction::OpenPreview => {
            if matches!(state.mode, ViewMode::Preview { .. }) {
                state.mode = ViewMode::Browse;
            } else {
                state.mode = ViewMode::Preview { scroll: 0 };
            }
        }
        KeyAction::ToggleQuickPreview => {
            state.preview_visible = !state.preview_visible;
        }
        KeyAction::ConfirmInput { value } => {
            match &state.mode {
                ViewMode::Input { purpose, .. } => {
                    match purpose {
                        InputPurpose::CreateFile => {
                            let parent = focused_path
                                .as_ref()
                                .and_then(|p| {
                                    if p.is_dir() {
                                        Some(p.clone())
                                    } else {
                                        p.parent().map(|pp| pp.to_path_buf())
                                    }
                                })
                                .unwrap_or_else(|| state.root.clone());
                            file_ops::create_file(&parent, &value)?;
                            navigator.reload()?;
                            state.refresh_git_status();
                            state.set_message(format!("Created file: {}", value));
                        }
                        InputPurpose::CreateDir => {
                            let parent = focused_path
                                .as_ref()
                                .and_then(|p| {
                                    if p.is_dir() {
                                        Some(p.clone())
                                    } else {
                                        p.parent().map(|pp| pp.to_path_buf())
                                    }
                                })
                                .unwrap_or_else(|| state.root.clone());
                            file_ops::create_dir(&parent, &value)?;
                            navigator.reload()?;
                            state.refresh_git_status();
                            state.set_message(format!("Created directory: {}", value));
                        }
                        InputPurpose::Rename { original } => {
                            file_ops::rename(original, &value)?;
                            navigator.reload()?;
                            state.refresh_git_status();
                            state.set_message(format!("Renamed to: {}", value));
                        }
                    }
                    state.mode = ViewMode::Browse;
                }
                ViewMode::Search { .. } => {
                    // Keep search mode active, just update
                    state.mode = ViewMode::Search { query: value };
                }
                _ => {}
            }
        }
        KeyAction::PreviewScrollUp => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll = tp.scroll.saturating_sub(1);
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll = scroll.saturating_sub(1);
            }
        }
        KeyAction::PreviewScrollDown => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll += 1;
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll += 1;
            }
        }
        KeyAction::PreviewPageUp => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll = tp.scroll.saturating_sub(20);
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll = scroll.saturating_sub(20);
            }
        }
        KeyAction::PreviewPageDown => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll += 20;
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll += 20;
            }
        }
        KeyAction::PreviewToTop => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll = 0;
            }
            if let ViewMode::Preview { scroll } = &mut state.mode {
                *scroll = 0;
            }
        }
        KeyAction::PreviewToBottom => {
            if let Some(ref mut tp) = text_preview {
                tp.scroll = tp.lines.len().saturating_sub(20);
            }
        }
        KeyAction::PickSelect => {
            if state.pick_mode {
                let paths: Vec<PathBuf> = if state.selected_paths.is_empty() {
                    focused_path.clone().into_iter().collect()
                } else {
                    state.selected_paths.iter().cloned().collect()
                };

                if !paths.is_empty() {
                    // Execute callback if configured
                    if let Some(ref callback) = config.callback {
                        for path in &paths {
                            let _ = callback.execute(path);
                        }
                    }

                    // Output paths
                    let result = PickResult::Selected(paths);
                    return Ok(ActionResult::Quit(result.output(config.output_format)?));
                }
            }
        }
        KeyAction::ShowHelp => {
            state.set_message("j/k:move l/h:expand/collapse Space:mark y/d/p:copy/cut/paste D:delete a/A:new r:rename /:search ?:help");
        }
    }

    Ok(ActionResult::Continue)
}
