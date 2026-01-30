//! FileView - A minimal file tree UI for terminal emulators

use std::env;
use std::io::{self, stdout, BufRead, IsTerminal};
use std::path::{Path, PathBuf};
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

use fileview::action::file as file_ops;
use fileview::core::{AppState, FocusTarget, ViewMode};
use fileview::handler::{
    action::{
        get_filename_str, get_target_directory, handle_action, ActionContext, ActionResult,
        EntrySnapshot,
    },
    key::{handle_key_event, update_input_buffer, KeyAction},
    mouse::{handle_mouse_event, ClickDetector, MouseAction, PathBuffer},
};
use fileview::integrate::{exit_code, Callback, OutputFormat};
use fileview::render::{
    collect_paths, create_image_picker, fuzzy_match, is_binary_file, is_image_file, is_text_file,
    render_directory_info, render_fuzzy_finder, render_help_popup, render_hex_preview,
    render_image_preview, render_input_popup, render_status_bar, render_text_preview, render_tree,
    visible_height, DirectoryInfo, FontSize, FuzzyMatch, HexPreview, ImagePreview, Picker,
    TextPreview,
};
use fileview::tree::TreeNavigator;

/// Application configuration from CLI args
struct Config {
    root: PathBuf,
    pick_mode: bool,
    output_format: OutputFormat,
    callback: Option<Callback>,
    icons_enabled: Option<bool>,
    /// Shell integration: output directory path on exit (for cd)
    choosedir_mode: bool,
    /// Paths read from stdin (for pipeline integration)
    stdin_paths: Option<Vec<PathBuf>>,
}

impl Config {
    fn from_args() -> anyhow::Result<Self> {
        let mut args = env::args().skip(1).peekable();
        let mut root = env::current_dir()?;
        let mut pick_mode = false;
        let mut output_format = OutputFormat::default();
        let mut callback: Option<Callback> = None;
        let mut icons_enabled: Option<bool> = None;
        let mut choosedir_mode = false;
        let mut stdin_mode = false;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--pick" | "-p" => pick_mode = true,
                "--choosedir" => choosedir_mode = true,
                "--stdin" => stdin_mode = true,
                "--icons" | "-i" => icons_enabled = Some(true),
                "--no-icons" => icons_enabled = Some(false),
                "--format" | "-f" => {
                    if let Some(fmt) = args.next() {
                        output_format = OutputFormat::from_str(&fmt).map_err(|_| {
                            anyhow::anyhow!(
                                "Invalid format '{}'. Valid formats: lines, null, json",
                                fmt
                            )
                        })?;
                    } else {
                        anyhow::bail!("--format requires a value (lines, null, or json)");
                    }
                }
                "--on-select" => {
                    if let Some(cmd) = args.next() {
                        callback = Some(Callback::new(cmd));
                    } else {
                        anyhow::bail!("--on-select requires a command");
                    }
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(exit_code::SUCCESS);
                }
                "--version" | "-V" => {
                    println!("fv {}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(exit_code::SUCCESS);
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
                unknown => {
                    anyhow::bail!(
                        "Unknown option: {}. Use --help for usage information.",
                        unknown
                    );
                }
            }
        }

        // Handle stdin mode
        let stdin_paths = if stdin_mode {
            Some(read_stdin_paths()?)
        } else {
            None
        };

        // If stdin paths are provided, determine root from common ancestor
        let root = if let Some(ref paths) = stdin_paths {
            find_common_ancestor(paths)?
        } else {
            root
        };

        Ok(Self {
            root,
            pick_mode,
            output_format,
            callback,
            icons_enabled,
            choosedir_mode,
            stdin_paths,
        })
    }
}

/// Read paths from stdin (one path per line)
fn read_stdin_paths() -> anyhow::Result<Vec<PathBuf>> {
    let stdin = io::stdin();

    // Check if stdin is a TTY (not piped)
    if stdin.is_terminal() {
        anyhow::bail!("--stdin requires piped input");
    }

    let paths: Vec<PathBuf> = stdin
        .lock()
        .lines()
        .map_while(Result::ok)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let path = PathBuf::from(line.trim());
            // Normalize path (resolve relative paths, handle . and ..)
            if path.is_absolute() {
                path
            } else {
                env::current_dir()
                    .map(|cwd| cwd.join(&path))
                    .unwrap_or(path)
            }
        })
        .collect();

    if paths.is_empty() {
        anyhow::bail!("No paths provided via stdin");
    }

    Ok(paths)
}

/// Find the common ancestor directory of all paths
fn find_common_ancestor(paths: &[PathBuf]) -> anyhow::Result<PathBuf> {
    if paths.is_empty() {
        return env::current_dir().map_err(Into::into);
    }

    // Start with the first path's parent (or itself if it's a directory)
    let first = &paths[0];
    let mut ancestor = if first.is_dir() {
        first.clone()
    } else {
        first.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| {
            env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
        })
    };

    // Find common prefix with all other paths
    for path in paths.iter().skip(1) {
        let other = if path.is_dir() {
            path.clone()
        } else {
            path.parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| ancestor.clone())
        };

        // Walk up until we find a common ancestor
        while !other.starts_with(&ancestor) {
            if let Some(parent) = ancestor.parent() {
                ancestor = parent.to_path_buf();
            } else {
                // Reached root
                break;
            }
        }
    }

    Ok(ancestor)
}

fn print_help() {
    println!(
        r#"fv - FileView: A minimal file tree UI

USAGE:
    fv [OPTIONS] [PATH]
    command | fv --stdin [OPTIONS]

OPTIONS:
    -p, --pick          Pick mode: output selected path(s) to stdout
    -f, --format FMT    Output format for pick mode: lines, null, json
    --stdin             Read paths from stdin (one per line)
    --on-select CMD     Run command when file is selected (use {{path}}, {{name}}, etc.)
    --choosedir         Output directory path on exit (press Q to cd there)
    -i, --icons         Enable Nerd Fonts icons (default)
    --no-icons          Disable icons
    -h, --help          Show this help message
    -V, --version       Show version

ENVIRONMENT:
    FILEVIEW_ICONS=0            Disable icons
    FILEVIEW_IMAGE_PROTOCOL     Force image protocol: auto, halfblocks, chafa, sixel, kitty, iterm2

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
    Ctrl+P      Fuzzy finder
    .           Toggle hidden files
    R/F5        Refresh
    o           Open preview
    P           Toggle quick preview panel
    c           Copy path to system clipboard
    C           Copy filename to system clipboard
    q/Esc       Quit (or cancel in pick mode)
    Q           Quit and cd to current directory (with --choosedir)
    ?           Show help

PLACEHOLDERS for --on-select:
    {{path}}    Full path
    {{dir}}     Parent directory
    {{name}}    Filename with extension
    {{stem}}    Filename without extension
    {{ext}}     Extension only

EXIT CODES:
    0           Success (normal exit or file selected)
    1           Cancelled (user cancelled selection in pick mode)
    2           Error (runtime error)
    3           Invalid arguments (unknown option or invalid value)
"#
    );
}

fn main() -> ExitCode {
    // Parse config first to return INVALID exit code for argument errors
    let config = match Config::from_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            return ExitCode::from(exit_code::INVALID as u8);
        }
    };

    match run_with_config(config) {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(exit_code::ERROR as u8)
        }
    }
}

fn run_with_config(config: Config) -> anyhow::Result<i32> {
    // Initialize image picker BEFORE entering alternate screen
    // (terminal capability detection requires normal screen mode)
    let mut image_picker = create_image_picker();

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
    let choosedir_mode = config.choosedir_mode;
    let result = run_app(&mut terminal, config, &mut image_picker);

    // Restore terminal
    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste,
        cursor::Show
    )?;

    // Handle result and output choosedir path if requested
    match result {
        Ok(app_result) => {
            if choosedir_mode {
                if let Some(path) = app_result.choosedir_path {
                    println!("{}", path.display());
                }
            }
            Ok(app_result.exit_code)
        }
        Err(e) => Err(e),
    }
}

/// Handle file drop operation - copy files to target directory.
/// Returns the number of files successfully processed.
fn handle_file_drop(
    paths: &[PathBuf],
    focused_path: Option<&PathBuf>,
    root: &Path,
    navigator: &mut TreeNavigator,
    state: &mut AppState,
) -> anyhow::Result<usize> {
    if paths.is_empty() {
        return Ok(0);
    }

    // Disable file drop in stdin mode
    if state.stdin_mode {
        state.set_message("File operations disabled in stdin mode");
        return Ok(0);
    }

    let dest = get_target_directory(focused_path, root);
    let mut success_count = 0;
    let mut fail_count = 0;
    for src in paths {
        match file_ops::copy_to(src, &dest) {
            Ok(_) => success_count += 1,
            Err(_) => fail_count += 1,
        }
    }
    navigator.reload()?;
    state.refresh_git_status();

    let message = if fail_count == 0 {
        format!("Dropped {} file(s)", success_count)
    } else {
        format!("Dropped {} file(s), {} failed", success_count, fail_count)
    };
    state.set_message(message);
    Ok(success_count)
}

/// Result of running the app
struct AppResult {
    exit_code: i32,
    choosedir_path: Option<PathBuf>,
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: Config,
    image_picker: &mut Option<Picker>,
) -> anyhow::Result<AppResult> {
    let mut state = AppState::new(config.root.clone());
    state.pick_mode = config.pick_mode;
    if let Some(icons) = config.icons_enabled {
        state.icons_enabled = icons;
    }

    // Create navigator based on stdin mode
    let mut navigator = if let Some(paths) = config.stdin_paths {
        state.stdin_mode = true;
        TreeNavigator::from_paths(&config.root, paths, state.show_hidden)?
    } else {
        TreeNavigator::new(&config.root, state.show_hidden)?
    };
    let mut click_detector = ClickDetector::new();
    let mut path_buffer = PathBuffer::new();

    // Create action context from config
    let action_context = ActionContext {
        callback: config.callback.clone(),
        output_format: config.output_format,
    };

    // Preview cache
    let mut text_preview: Option<TextPreview> = None;
    let mut image_preview: Option<ImagePreview> = None;
    let mut dir_info: Option<DirectoryInfo> = None;
    let mut hex_preview: Option<HexPreview> = None;
    let mut last_preview_path: Option<PathBuf> = None;

    // Fuzzy finder state
    let mut fuzzy_paths: Vec<PathBuf> = Vec::new();
    let mut fuzzy_results: Vec<FuzzyMatch> = Vec::new();

    // Lazy initialization: defer Git detection until after the first frame
    // to improve perceived startup time (first frame renders faster)
    let mut skip_git_init_once = true;

    loop {
        // Initialize git status after the first frame is rendered.
        // On the first iteration, we skip to render the UI immediately.
        // On the second iteration, we detect Git status.
        if skip_git_init_once {
            skip_git_init_once = false;
        } else if state.git_status.is_none() {
            state.init_git_status();
        }
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
            // Only reload preview if the path changed
            let path_changed = focused_path != last_preview_path;
            if path_changed {
                last_preview_path = focused_path.clone();
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
                        match std::fs::read_to_string(path) {
                            Ok(content) => {
                                text_preview = Some(TextPreview::new(&content));
                                image_preview = None;
                                dir_info = None;
                                hex_preview = None;
                            }
                            Err(e) => {
                                state.set_message(format!("Cannot preview: {}", e));
                                text_preview = None;
                                image_preview = None;
                                dir_info = None;
                                hex_preview = None;
                            }
                        }
                    } else if is_image_file(path) {
                        if let Some(ref mut picker) = image_picker {
                            match ImagePreview::load(path, picker) {
                                Ok(img) => {
                                    image_preview = Some(img);
                                    text_preview = None;
                                    dir_info = None;
                                    hex_preview = None;
                                }
                                Err(e) => {
                                    state.set_message(format!("Cannot preview image: {}", e));
                                    text_preview = None;
                                    image_preview = None;
                                    dir_info = None;
                                    hex_preview = None;
                                }
                            }
                        }
                    } else if is_binary_file(path) || path.is_file() {
                        // Binary file or unknown type - show hex preview
                        match HexPreview::load(path) {
                            Ok(hex) => {
                                hex_preview = Some(hex);
                                text_preview = None;
                                image_preview = None;
                                dir_info = None;
                            }
                            Err(e) => {
                                state.set_message(format!("Cannot preview: {}", e));
                                text_preview = None;
                                image_preview = None;
                                dir_info = None;
                                hex_preview = None;
                            }
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
        }

        // Get font size for image centering (default to typical terminal cell size)
        let font_size: FontSize = image_picker
            .as_ref()
            .map(|p| p.font_size())
            .unwrap_or((10, 20));

        // Render
        terminal.draw(|frame| {
            let size = frame.area();

            // Check if fullscreen preview mode is active
            let is_fullscreen_preview = matches!(state.mode, ViewMode::Preview { .. });

            if is_fullscreen_preview {
                // Fullscreen preview mode - render preview only (always focused)
                let filename = get_filename_str(focused_path.as_ref());
                let title = if filename.is_empty() {
                    " Preview (press o or q to close) ".to_string()
                } else {
                    format!(" {} (press o or q to close) ", filename)
                };

                if let Some(ref di) = dir_info {
                    render_directory_info(frame, di, size, false);
                } else if let Some(ref tp) = text_preview {
                    render_text_preview(frame, tp, size, &title, false);
                } else if let Some(ref mut ip) = image_preview {
                    render_image_preview(frame, ip, size, &title, false, font_size);
                } else if let Some(ref hp) = hex_preview {
                    render_hex_preview(frame, hp, size, &title, false);
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
                    let title = get_filename_str(focused_path.as_ref());
                    let preview_focused = state.focus_target == FocusTarget::Preview;
                    if let Some(ref di) = dir_info {
                        render_directory_info(frame, di, preview_area, preview_focused);
                    } else if let Some(ref tp) = text_preview {
                        render_text_preview(frame, tp, preview_area, &title, preview_focused);
                    } else if let Some(ref mut ip) = image_preview {
                        render_image_preview(
                            frame,
                            ip,
                            preview_area,
                            &title,
                            preview_focused,
                            font_size,
                        );
                    } else if let Some(ref hp) = hex_preview {
                        render_hex_preview(frame, hp, preview_area, &title, preview_focused);
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
                        frame.render_widget(para, preview_area);
                    }
                }

                // Render input popup if needed
                render_input_popup(frame, &state);

                // Render fuzzy finder if in FuzzyFinder mode
                if let ViewMode::FuzzyFinder { query, selected } = &state.mode {
                    // Bound selected index to results length
                    let bounded_selected = if fuzzy_results.is_empty() {
                        0
                    } else {
                        (*selected).min(fuzzy_results.len() - 1)
                    };
                    render_fuzzy_finder(frame, query, &fuzzy_results, bounded_selected, size);
                }

                // Render help popup if in Help mode
                render_help_popup(frame, &state);
            }
        })?;

        // Drop the entries borrow before event handling
        drop(entries);

        // Check drop buffer timeout (for file drop detection via rapid key input)
        if path_buffer.is_ready() {
            let paths = path_buffer.take_paths();
            if !paths.is_empty() {
                let root = state.root.clone();
                handle_file_drop(
                    &paths,
                    focused_path.as_ref(),
                    &root,
                    &mut navigator,
                    &mut state,
                )?;
            } else {
                // Not valid paths - check if it starts with '/' for search
                let buffer = path_buffer.take_raw();
                if let Some(rest) = buffer.strip_prefix('/') {
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

                    // Handle fuzzy finder text input
                    if let ViewMode::FuzzyFinder { query, .. } = &state.mode {
                        if let Some((new_buf, _)) = update_input_buffer(key, query, query.len()) {
                            // Refresh results when query changes
                            fuzzy_results = fuzzy_match(&new_buf, &fuzzy_paths, &state.root);
                            state.mode = ViewMode::FuzzyFinder {
                                query: new_buf,
                                selected: 0, // Reset selection on query change
                            };
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

                    let mut action = handle_key_event(&state, key);

                    // Handle fuzzy finder special actions
                    if matches!(action, KeyAction::OpenFuzzyFinder) {
                        // Collect paths when fuzzy finder opens
                        fuzzy_paths = if state.stdin_mode {
                            navigator.collect_all_paths()
                        } else {
                            collect_paths(&state.root, state.show_hidden)
                        };
                        fuzzy_results = fuzzy_match("", &fuzzy_paths, &state.root);
                    }

                    // Fill in actual path for FuzzyConfirm
                    if matches!(action, KeyAction::FuzzyConfirm { .. }) {
                        if let ViewMode::FuzzyFinder { selected, .. } = &state.mode {
                            let actual_selected =
                                (*selected).min(fuzzy_results.len().saturating_sub(1));
                            if let Some(result) = fuzzy_results.get(actual_selected) {
                                action = KeyAction::FuzzyConfirm {
                                    path: result.path.clone(),
                                };
                            }
                        }
                    }

                    match handle_action(
                        action,
                        &mut state,
                        &mut navigator,
                        &focused_path,
                        &snapshots,
                        &action_context,
                        &mut text_preview,
                    )? {
                        ActionResult::Continue => {}
                        ActionResult::Quit(code) => {
                            return Ok(AppResult {
                                exit_code: code,
                                choosedir_path: state.choosedir_path.clone(),
                            })
                        }
                    }

                    // Clamp fuzzy finder selected index to valid range
                    if let ViewMode::FuzzyFinder { selected, .. } = &mut state.mode {
                        if fuzzy_results.is_empty() {
                            *selected = 0;
                        } else {
                            *selected = (*selected).min(fuzzy_results.len() - 1);
                        }
                    }

                    // Handle fuzzy finder jump target
                    if let Some(target) = state.fuzzy_jump_target.take() {
                        // Expand parent directories to make the target visible
                        if let Err(e) = navigator.reveal_path(&target) {
                            state.set_message(format!("Failed to reveal path: {}", e));
                        } else {
                            // Find the target in visible entries and set focus
                            let entries = navigator.visible_entries();
                            if let Some(idx) = entries.iter().position(|e| e.path == target) {
                                state.focus_index = idx;
                            }
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    let tree_top = 0; // Assuming tree starts at row 0
                    let action = handle_mouse_event(mouse, &mut click_detector, tree_top);

                    // Calculate preview boundary for focus switching
                    let preview_boundary = if state.preview_visible {
                        crossterm::terminal::size()
                            .map(|(w, _)| w / 2)
                            .unwrap_or(u16::MAX)
                    } else {
                        u16::MAX // No preview, all clicks go to tree
                    };

                    match action {
                        MouseAction::Click { row, col } => {
                            // Set focus based on click position
                            if state.preview_visible {
                                if col >= preview_boundary {
                                    state.set_focus(FocusTarget::Preview);
                                } else {
                                    state.set_focus(FocusTarget::Tree);
                                    // Only update file selection when clicking on tree
                                    let idx = state.viewport_top + row as usize;
                                    if idx < snapshots.len() {
                                        state.focus_index = idx;
                                    }
                                }
                            } else {
                                let idx = state.viewport_top + row as usize;
                                if idx < snapshots.len() {
                                    state.focus_index = idx;
                                }
                            }
                        }
                        MouseAction::DoubleClick { row, col } => {
                            // Double-click on tree area
                            if col < preview_boundary {
                                state.set_focus(FocusTarget::Tree);
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
                        }
                        MouseAction::ScrollUp { amount, col } => {
                            if state.preview_visible && col >= preview_boundary {
                                // Scroll preview
                                if let Some(ref mut tp) = text_preview {
                                    tp.scroll = tp.scroll.saturating_sub(amount);
                                }
                            } else {
                                // Scroll file list
                                state.focus_index = state.focus_index.saturating_sub(amount);
                            }
                        }
                        MouseAction::ScrollDown { amount, col } => {
                            if state.preview_visible && col >= preview_boundary {
                                // Scroll preview
                                if let Some(ref mut tp) = text_preview {
                                    tp.scroll += amount;
                                }
                            } else {
                                // Scroll file list
                                state.focus_index = (state.focus_index + amount)
                                    .min(snapshots.len().saturating_sub(1));
                            }
                        }
                        MouseAction::FileDrop { paths } => {
                            let root = state.root.clone();
                            handle_file_drop(
                                &paths,
                                focused_path.as_ref(),
                                &root,
                                &mut navigator,
                                &mut state,
                            )?;
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
                        let root = state.root.clone();
                        handle_file_drop(
                            &paths,
                            focused_path.as_ref(),
                            &root,
                            &mut navigator,
                            &mut state,
                        )?;
                    }
                    path_buffer.clear();
                }
                _ => {}
            }
        }

        // Check quit flag
        if state.should_quit {
            return Ok(AppResult {
                exit_code: exit_code::SUCCESS,
                choosedir_path: state.choosedir_path.clone(),
            });
        }
    }
}
