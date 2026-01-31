//! Main event loop for the application

use std::io::Stdout;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event};
use ratatui::prelude::*;

use crate::action::file as file_ops;
use crate::app::{Config, PreviewState};
use crate::core::{AppState, FocusTarget, ViewMode};
use crate::handler::{
    action::{
        get_target_directory, handle_action, reload_tree, ActionContext, ActionResult,
        EntrySnapshot,
    },
    key::{handle_key_event, update_input_buffer, KeyAction},
    mouse::{handle_mouse_event, ClickDetector, MouseAction, PathBuffer},
};
use crate::render::{collect_paths, fuzzy_match, visible_height, FuzzyMatch, Picker};
use crate::tree::TreeNavigator;
use crate::watcher::FileWatcher;

use super::render::{render_frame, RenderContext};

/// Result of running the app
pub struct AppResult {
    pub exit_code: i32,
    pub choosedir_path: Option<PathBuf>,
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
    reload_tree(navigator, state)?;

    let message = if fail_count == 0 {
        format!("Dropped {} file(s)", success_count)
    } else {
        format!("Dropped {} file(s), {} failed", success_count, fail_count)
    };
    state.set_message(message);
    Ok(success_count)
}

/// Main event loop
pub fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
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

    // Preview state
    let mut preview = PreviewState::new();

    // Fuzzy finder state
    let mut fuzzy_paths: Vec<PathBuf> = Vec::new();
    let mut fuzzy_results: Vec<FuzzyMatch> = Vec::new();

    // Lazy initialization: defer Git detection until after the first frame
    // to improve perceived startup time (first frame renders faster)
    let mut skip_git_init_once = true;

    // Initialize file watcher (disabled in stdin mode)
    let mut file_watcher = if !state.stdin_mode {
        match FileWatcher::new(&config.root) {
            Ok(watcher) => {
                state.watch_enabled = true;
                Some(watcher)
            }
            Err(_) => {
                // Watcher initialization failed, continue without watching
                None
            }
        }
    } else {
        None
    };

    // Git status polling timer (5 seconds balances responsiveness and CPU usage)
    let mut last_git_poll = Instant::now();
    const GIT_POLL_INTERVAL: Duration = Duration::from_secs(5);

    // Track previous expanded paths for watcher sync
    let mut prev_expanded: Vec<PathBuf> = Vec::new();

    loop {
        // Initialize git status after the first frame is rendered.
        // On the first iteration, we skip to render the UI immediately.
        // On the second iteration, we detect Git status.
        if skip_git_init_once {
            skip_git_init_once = false;
        } else if state.git_status.is_none() {
            state.init_git_status();
        }
        // Get visible entries and apply filter if set
        let all_entries = navigator.visible_entries();
        let entries: Vec<_> = if let Some(ref pattern) = state.filter_pattern {
            all_entries
                .into_iter()
                .filter(|e| {
                    // Always show directories for navigation
                    e.is_dir || crate::handler::action::matches_filter(&e.name, pattern)
                })
                .collect()
        } else {
            all_entries
        };
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
            preview.update(focused_path.as_ref(), image_picker, &mut state);
        }

        // Adjust viewport before rendering
        // Get terminal size to calculate visible height
        let term_size = terminal.size()?;
        let tree_height = if state.preview_visible {
            term_size.width / 2
        } else {
            term_size.width
        };
        // Account for status bar (3 lines)
        let vis_height = visible_height(ratatui::layout::Rect {
            x: 0,
            y: 0,
            width: tree_height,
            height: term_size.height.saturating_sub(3),
        });
        state.adjust_viewport(vis_height);

        // Render
        let render_context = RenderContext {
            state: &state,
            entries,
            focused_path: focused_path.as_ref(),
            preview: &mut preview,
            fuzzy_results: &fuzzy_results,
            image_picker,
        };
        terminal.draw(|frame| render_frame(frame, render_context))?;

        // Sync watcher with expanded directories (only when changed)
        if let Some(ref mut watcher) = file_watcher {
            let current_expanded = navigator.expanded_paths();
            if current_expanded != prev_expanded {
                watcher.sync_with_expanded(&current_expanded);
                prev_expanded = current_expanded;
            }
        }

        // Check file watcher events (auto-refresh on file changes)
        if let Some(ref watcher) = file_watcher {
            if watcher.poll() {
                reload_tree(&mut navigator, &mut state)?;
                last_git_poll = Instant::now(); // Reset git poll timer
            }
        }

        // Git status polling (every 3 seconds)
        if last_git_poll.elapsed() >= GIT_POLL_INTERVAL {
            state.refresh_git_status();
            last_git_poll = Instant::now();
        }

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

        // Handle events (60ms timeout balances responsiveness and CPU usage)
        if event::poll(Duration::from_millis(60))? {
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

                    // Handle filter text input
                    if let ViewMode::Filter { query } = &state.mode {
                        if let Some((new_buf, _)) = update_input_buffer(key, query, query.len()) {
                            state.mode = ViewMode::Filter { query: new_buf };
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
                        &mut preview.text,
                        &mut preview.hex,
                        &mut preview.archive,
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
                            state.set_message(format!("Failed: reveal path - {}", e));
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
                                // Scroll preview (text, hex, or archive)
                                if let Some(ref mut tp) = preview.text {
                                    tp.scroll = tp.scroll.saturating_sub(amount);
                                }
                                if let Some(ref mut hp) = preview.hex {
                                    hp.scroll = hp.scroll.saturating_sub(amount);
                                }
                                if let Some(ref mut ap) = preview.archive {
                                    ap.scroll = ap.scroll.saturating_sub(amount);
                                }
                            } else {
                                // Scroll file list
                                state.focus_index = state.focus_index.saturating_sub(amount);
                            }
                        }
                        MouseAction::ScrollDown { amount, col } => {
                            if state.preview_visible && col >= preview_boundary {
                                // Scroll preview (text, hex, or archive)
                                if let Some(ref mut tp) = preview.text {
                                    tp.scroll += amount;
                                }
                                if let Some(ref mut hp) = preview.hex {
                                    hp.scroll += amount;
                                }
                                if let Some(ref mut ap) = preview.archive {
                                    ap.scroll += amount;
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
                exit_code: crate::integrate::exit_code::SUCCESS,
                choosedir_path: state.choosedir_path.clone(),
            });
        }
    }
}
