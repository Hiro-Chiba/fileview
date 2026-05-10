//! Main event loop for the application

use std::io::Stdout;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event};
use ratatui::prelude::*;

use crate::action::file as file_ops;
use crate::ai_activity::{ActivityWatcher, SessionInfo, SessionRegistry};
use crate::app::{Config, PreviewState};
use crate::core::{AppState, FocusTarget, TabManager, ViewMode};
use crate::handler::{
    action::{
        display::reveal_path_in_tree, get_target_directory, handle_action, reload_tree,
        update_bulk_rename_buffer, ActionContext, ActionResult, EntrySnapshot,
    },
    key::{handle_key_event, update_input_buffer, KeyAction},
    mouse::{handle_mouse_event, ClickDetector, MouseAction, PathBuffer},
};
use crate::plugin::{PluginAction, PluginEvent, PluginManager};
use crate::render::{
    collect_paths, fuzzy_match_incremental, visible_height, FuzzyMatch, FuzzyState, Picker,
};
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
    // Warm the heavy lazy caches in a background thread so the first
    // preview, syntax highlight, or token estimate doesn't pay the
    // 30 to 80 ms first-call cost on the UI thread. The thread is
    // detached: results land in `OnceLock`s shared with the rest of
    // the app, so we don't need to hold onto a JoinHandle.
    thread::Builder::new()
        .name("fv-warmup".into())
        .spawn(|| {
            crate::render::warmup_syntax();
            let _ = crate::mcp::token::estimate_tokens("");
        })
        .ok();

    let mut state = AppState::new(config.root.clone());
    state.pick_mode = config.pick_mode;
    state.select_mode = config.select_mode;
    state.multi_select = config.multi_select;
    if let Some(model) = config.budget_default_model {
        state.budget_model = model;
    }

    // Resolve --diff scope. Errors are surfaced to the status bar but never
    // fatal: the user can keep using the tree as a normal browser.
    if let Some(scope) = config.diff_scope.clone() {
        match crate::git::compute_diff_range(&config.root, scope.as_deref()) {
            Ok(range) => {
                let count = range.file_count();
                let label = match &range.revspec {
                    Some(r) => format!("range {}", r),
                    None => "working tree".to_string(),
                };
                state.set_message(format!("Diff scope: {} ({} files)", label, count));
                state.diff_range = Some(range);
            }
            Err(e) => {
                state.set_message(format!("Diff scope unavailable: {}", e));
            }
        }
    }

    // Kick off a one-shot repo fingerprint detection on a background thread.
    // The result is shown in the status bar once it lands; we never block
    // the UI on it. Skip in stdin mode since the "repo" concept doesn't
    // apply when paths come from a pipe.
    let fingerprint_rx: Option<mpsc::Receiver<crate::integrate::Fingerprint>> =
        if config.stdin_paths.is_some() {
            None
        } else {
            let (tx, rx) = mpsc::channel();
            let root_for_fp = config.root.clone();
            thread::Builder::new()
                .name("fv-fingerprint".into())
                .spawn(move || {
                    let fp =
                        crate::integrate::detect_fingerprint(&root_for_fp, Duration::from_secs(2));
                    let _ = tx.send(fp);
                })
                .ok();
            Some(rx)
        };
    let mut fingerprint_shown = false;

    // Apply config file settings
    state.show_hidden = config.show_hidden;
    if let Some(icons) = config.icons_enabled {
        state.icons_enabled = icons;
    } else {
        // Check environment variable, then fall back to config file setting
        let env_icons = std::env::var("FILEVIEW_ICONS")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .ok();
        if let Some(icons) = env_icons {
            state.icons_enabled = icons;
        }
        // Note: config file icons setting is already applied in AppState::new via env var check
    }

    // Create tab manager with initial tab
    let mut tab_manager = TabManager::new(config.root.clone(), state.show_hidden)?;

    // Create navigator based on stdin mode
    let mut navigator = if let Some(paths) = config.stdin_paths.clone() {
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
        commands: config.commands.clone(),
    };

    // Preview state
    let mut preview = PreviewState::new();

    // Fuzzy finder state
    let mut fuzzy_paths: Vec<PathBuf> = Vec::new();
    let mut fuzzy_results: Vec<FuzzyMatch> = Vec::new();
    let mut fuzzy_state = FuzzyState::new();

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

    // Register this process with the AI activity session registry so the MCP
    // server can deliver tool-call events here. Failures are non-fatal but are
    // surfaced to the status bar so the user isn't left wondering why
    // `--follow-ai` does nothing.
    let ai_session: Option<SessionInfo> = SessionRegistry::new()
        .and_then(|r| r.register_current(&config.root))
        .ok();
    let ai_watcher = ai_session
        .as_ref()
        .and_then(|s| ActivityWatcher::start(s.activity_log.clone()).ok());
    state.ai_activity.follow_mode = config.follow_ai && ai_watcher.is_some();
    if config.follow_ai {
        if ai_watcher.is_some() {
            state.set_message("AI follow mode on (Alt+A to toggle)");
        } else {
            state.set_message("AI follow requested but activity registry unavailable");
        }
    }

    // Git status polling timer (configurable, default 5 seconds)
    let mut last_git_poll = Instant::now();
    let git_poll_interval = config.git_poll_interval;

    // Initialize plugin manager
    let mut plugin_manager = PluginManager::new().ok();
    if let Some(ref mut pm) = plugin_manager {
        // Load plugins from ~/.config/fileview/plugins/init.lua
        if let Err(e) = pm.load_plugins() {
            state.set_message(format!("Plugin error: {}", e));
        } else {
            // Update context with initial state
            let selected: Vec<PathBuf> = state.selected_paths.iter().cloned().collect();
            pm.update_context(None, config.root.clone(), selected);

            // Fire Start event
            let _ = pm.fire_event(PluginEvent::Start, None);

            // Process any startup notifications
            for msg in pm.take_notifications() {
                state.set_message(msg);
            }
        }
    }

    // Track previous state for plugin events
    let mut prev_focused_path: Option<PathBuf> = None;
    let mut prev_root = config.root.clone();
    let mut prev_selection_count = state.selected_paths.len();

    // Dirty-frame rendering: only redraw when state changes
    let mut needs_redraw = true; // first frame always renders
    let mut snapshots: Vec<EntrySnapshot> = Vec::new();
    let mut focused_path: Option<PathBuf> = None;

    loop {
        // Initialize git status after the first frame is rendered.
        // On the first iteration, we skip to render the UI immediately.
        // On the second iteration, we detect Git status.
        if skip_git_init_once {
            skip_git_init_once = false;
        } else if state.git_status.is_none() {
            state.init_git_status();
            needs_redraw = true;
        }

        let frame_needs_redraw = needs_redraw;
        needs_redraw = false;

        // Get visible entries and apply filter if set
        if frame_needs_redraw {
            navigator.ensure_cache();
            let all_entries = navigator.visible_entries();
            let entries: Vec<_> = if let Some(ref pattern) = state.filter_pattern {
                all_entries
                    .iter()
                    .filter(|e| {
                        // Always show directories for navigation
                        e.is_dir || crate::handler::action::matches_filter(&e.name, pattern)
                    })
                    .collect()
            } else {
                all_entries.iter().collect()
            };
            let total_entries = entries.len();
            snapshots = entries
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
            focused_path = snapshots.get(state.focus_index).map(|e| e.path.clone());

            // Update preview if needed (side panel or fullscreen mode)
            let needs_preview =
                state.preview_visible || matches!(state.mode, ViewMode::Preview { .. });
            if needs_preview {
                preview.update_with_custom(
                    focused_path.as_ref(),
                    image_picker,
                    &mut state,
                    &config.preview_custom.custom,
                );
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
                tab_manager: Some(&tab_manager),
            };
            terminal.draw(|frame| render_frame(frame, render_context))?;
        } // end if frame_needs_redraw

        // Sync watcher with expanded directories (only when dirty)
        if let Some(ref mut watcher) = file_watcher {
            if navigator.is_watcher_dirty() {
                let expanded = navigator.expanded_paths();
                watcher.sync_with_expanded(&expanded);
                navigator.clear_watcher_dirty();
            }
        }

        // Check file watcher events (auto-refresh on file changes)
        if let Some(ref watcher) = file_watcher {
            if watcher.poll() {
                reload_tree(&mut navigator, &mut state)?;
                last_git_poll = Instant::now(); // Reset git poll timer
                needs_redraw = true;
            }
        }

        // Drain AI activity events and apply them to state.
        if let Some(watcher) = ai_watcher.as_ref() {
            let events = watcher.drain();
            if !events.is_empty() {
                for event in events {
                    state.ai_activity.record(event);
                }
                // Apply follow-mode auto-navigation only when it is safe
                // (browse or preview; never during text input / fuzzy / etc.)
                let safe_to_follow = state.ai_activity.follow_mode
                    && matches!(state.mode, ViewMode::Browse | ViewMode::Preview { .. });
                if safe_to_follow {
                    if let Some(target) = state
                        .ai_activity
                        .last_event
                        .as_ref()
                        .and_then(|e| e.path.clone())
                    {
                        if target.starts_with(&state.root) && target.exists() {
                            let _ = reveal_path_in_tree(&mut navigator, &mut state, &target);
                        }
                    }
                }
                needs_redraw = true;
            }
        }

        // Surface the repo fingerprint once the background thread is done.
        // We only fire it once per session, and only when there is no other
        // user-facing message that would be clobbered by it.
        if !fingerprint_shown {
            if let Some(rx) = fingerprint_rx.as_ref() {
                if let Ok(fp) = rx.try_recv() {
                    fingerprint_shown = true;
                    if state.message.is_none() {
                        state.set_message(fp.describe());
                        needs_redraw = true;
                    }
                }
            } else {
                fingerprint_shown = true;
            }
        }

        // Sync the budget bar's marked-token cache with the current
        // selection set. Bulk operations (Select All, Invert, etc.) mutate
        // selected_paths directly, so we reconcile here rather than wiring
        // every action handler. Then drain any worker results that arrived
        // since the last frame.
        let sync_added = {
            let before = state.marked_token_cache.len();
            state.sync_budget_cache();
            state.marked_token_cache.len() != before
        };
        let drained = state.drain_budget_results();
        if sync_added || drained > 0 {
            needs_redraw = true;
        }

        // Git status polling (configurable interval)
        if last_git_poll.elapsed() >= git_poll_interval {
            state.refresh_git_status();
            last_git_poll = Instant::now();
            needs_redraw = true;
        }

        // Poll for completed async preview loads (worker + image loader)
        if preview.poll_preview_result(image_picker, &mut state) {
            needs_redraw = true;
        }

        // Check drop buffer timeout (for file drop detection via rapid key input)
        if path_buffer.is_ready() {
            needs_redraw = true;
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
            needs_redraw = true;
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
                            // Refresh results when query changes (incremental narrowing)
                            fuzzy_results = fuzzy_match_incremental(
                                &new_buf,
                                &fuzzy_paths,
                                &state.root,
                                &mut fuzzy_state,
                            );
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

                    // Handle bulk rename text input
                    if matches!(state.mode, ViewMode::BulkRename { .. })
                        && update_bulk_rename_buffer(key, &mut state)
                    {
                        continue;
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

                    // Handle tab operations
                    match &action {
                        KeyAction::NewTab => {
                            // Create new tab with current directory
                            let current_dir = focused_path
                                .as_ref()
                                .and_then(|p| {
                                    if p.is_dir() {
                                        Some(p.clone())
                                    } else {
                                        p.parent().map(|p| p.to_path_buf())
                                    }
                                })
                                .unwrap_or_else(|| state.root.clone());

                            match tab_manager.new_tab(current_dir, state.show_hidden) {
                                Ok(()) => {
                                    // Sync state from new tab
                                    let tab = tab_manager.active();
                                    navigator = tab.navigator.clone();
                                    state.root = tab.root.clone();
                                    state.focus_index = 0;
                                    state.viewport_top = 0;
                                    state.selected_paths.clear();
                                    state.mode = ViewMode::Browse;
                                    state.set_message(format!(
                                        "Tab {}: {}",
                                        tab_manager.len(),
                                        tab.name
                                    ));
                                }
                                Err(e) => {
                                    state.set_message(format!("Failed to create tab: {}", e));
                                }
                            }
                            continue;
                        }
                        KeyAction::CloseTab => {
                            if tab_manager.len() > 1 {
                                // Save current tab state before closing
                                tab_manager.active_mut().navigator = navigator.clone();
                                tab_manager.active_mut().focus_index = state.focus_index;
                                tab_manager.active_mut().viewport_top = state.viewport_top;
                                tab_manager.active_mut().selected_paths =
                                    state.selected_paths.clone();
                                tab_manager.active_mut().mode = state.mode.clone();

                                if tab_manager.close_tab() {
                                    // Restore state from new active tab
                                    let tab = tab_manager.active();
                                    navigator = tab.navigator.clone();
                                    state.root = tab.root.clone();
                                    state.focus_index = tab.focus_index;
                                    state.viewport_top = tab.viewport_top;
                                    state.selected_paths = tab.selected_paths.clone();
                                    state.mode = tab.mode.clone();
                                    state.set_message(format!(
                                        "Closed tab, {} remaining",
                                        tab_manager.len()
                                    ));
                                }
                            } else {
                                state.set_message("Cannot close last tab");
                            }
                            continue;
                        }
                        KeyAction::NextTab => {
                            if tab_manager.len() > 1 {
                                // Save current tab state
                                tab_manager.active_mut().navigator = navigator.clone();
                                tab_manager.active_mut().focus_index = state.focus_index;
                                tab_manager.active_mut().viewport_top = state.viewport_top;
                                tab_manager.active_mut().selected_paths =
                                    state.selected_paths.clone();
                                tab_manager.active_mut().mode = state.mode.clone();

                                tab_manager.next_tab();

                                // Restore state from new active tab
                                let tab = tab_manager.active();
                                navigator = tab.navigator.clone();
                                state.root = tab.root.clone();
                                state.focus_index = tab.focus_index;
                                state.viewport_top = tab.viewport_top;
                                state.selected_paths = tab.selected_paths.clone();
                                state.mode = tab.mode.clone();
                            }
                            continue;
                        }
                        KeyAction::PrevTab => {
                            if tab_manager.len() > 1 {
                                // Save current tab state
                                tab_manager.active_mut().navigator = navigator.clone();
                                tab_manager.active_mut().focus_index = state.focus_index;
                                tab_manager.active_mut().viewport_top = state.viewport_top;
                                tab_manager.active_mut().selected_paths =
                                    state.selected_paths.clone();
                                tab_manager.active_mut().mode = state.mode.clone();

                                tab_manager.prev_tab();

                                // Restore state from new active tab
                                let tab = tab_manager.active();
                                navigator = tab.navigator.clone();
                                state.root = tab.root.clone();
                                state.focus_index = tab.focus_index;
                                state.viewport_top = tab.viewport_top;
                                state.selected_paths = tab.selected_paths.clone();
                                state.mode = tab.mode.clone();
                            }
                            continue;
                        }
                        _ => {}
                    }

                    // Handle fuzzy finder special actions
                    if matches!(action, KeyAction::OpenFuzzyFinder) {
                        // Collect paths when fuzzy finder opens
                        fuzzy_paths = if state.stdin_mode {
                            navigator.collect_all_paths()
                        } else {
                            collect_paths(&state.root, state.show_hidden)
                        };
                        fuzzy_state.reset();
                        fuzzy_results = fuzzy_match_incremental(
                            "",
                            &fuzzy_paths,
                            &state.root,
                            &mut fuzzy_state,
                        );
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
                        &mut preview.pdf,
                        &mut preview.diff,
                        &mut preview.custom,
                        image_picker,
                    )? {
                        ActionResult::Continue => {}
                        ActionResult::Quit(code) => {
                            // Fire BeforeQuit event
                            if let Some(ref mut pm) = plugin_manager {
                                let _ = pm.fire_event(PluginEvent::BeforeQuit, None);
                            }
                            return Ok(AppResult {
                                exit_code: code,
                                choosedir_path: state.choosedir_path.clone(),
                            });
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
                            navigator.ensure_cache();
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
                Event::Resize(..) => {
                    // Terminal resized - redraw is already flagged
                }
                _ => {}
            }
        }

        // === Plugin event handling ===
        // Only process plugin events when we have fresh snapshots to avoid
        // spurious FileSelected events from stale focused_path values.
        if frame_needs_redraw {
            if let Some(ref mut pm) = plugin_manager {
                // Update plugin context with current state
                let selected: Vec<PathBuf> = state.selected_paths.iter().cloned().collect();
                pm.update_context(focused_path.clone(), state.root.clone(), selected);

                // Fire FileSelected event when focus changes
                if focused_path != prev_focused_path {
                    if let Some(ref path) = focused_path {
                        let _ =
                            pm.fire_event(PluginEvent::FileSelected, Some(&path.to_string_lossy()));
                    }
                    prev_focused_path = focused_path.clone();
                }

                // Fire DirectoryChanged event when root changes
                if state.root != prev_root {
                    let _ = pm.fire_event(
                        PluginEvent::DirectoryChanged,
                        Some(&state.root.to_string_lossy()),
                    );
                    prev_root = state.root.clone();
                }

                // Fire SelectionChanged event when selection count changes
                if state.selected_paths.len() != prev_selection_count {
                    let _ = pm.fire_event(PluginEvent::SelectionChanged, None);
                    prev_selection_count = state.selected_paths.len();
                }

                // Process plugin notifications
                for msg in pm.take_notifications() {
                    state.set_message(msg);
                }

                // Process plugin actions
                for action in pm.take_actions() {
                    match action {
                        PluginAction::Navigate(path) => {
                            if path.is_dir() {
                                match TreeNavigator::new(&path, state.show_hidden) {
                                    Ok(new_nav) => {
                                        navigator = new_nav;
                                        state.root = path;
                                        state.focus_index = 0;
                                        state.viewport_top = 0;
                                    }
                                    Err(e) => {
                                        state.set_message(format!("Navigate failed: {}", e));
                                    }
                                }
                            }
                        }
                        PluginAction::Select(path) => {
                            state.selected_paths.insert(path);
                        }
                        PluginAction::Deselect(path) => {
                            state.selected_paths.remove(&path);
                        }
                        PluginAction::ClearSelection => {
                            state.selected_paths.clear();
                        }
                        PluginAction::Refresh => {
                            let _ = reload_tree(&mut navigator, &mut state);
                        }
                        PluginAction::SetClipboard(text) => {
                            #[cfg(feature = "clipboard")]
                            {
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    let _ = clipboard.set_text(&text);
                                }
                            }
                            #[cfg(not(feature = "clipboard"))]
                            {
                                let _ = text;
                            }
                        }
                        PluginAction::Focus(path) => {
                            // Expand parent directories to make the target visible
                            if let Err(e) = navigator.reveal_path(&path) {
                                state.set_message(format!("Focus failed: {}", e));
                            } else {
                                // Find the target in visible entries and set focus
                                navigator.ensure_cache();
                                let entries = navigator.visible_entries();
                                if let Some(idx) = entries.iter().position(|e| e.path == path) {
                                    state.focus_index = idx;
                                }
                            }
                        }
                    }
                }
            }
        } // end if frame_needs_redraw (plugin events)

        // Check quit flag
        if state.should_quit {
            // Fire BeforeQuit event
            if let Some(ref mut pm) = plugin_manager {
                let _ = pm.fire_event(PluginEvent::BeforeQuit, None);
            }
            // Unregister this AI activity session so stale dirs don't linger.
            if let (Ok(registry), Some(session)) = (SessionRegistry::new(), ai_session.as_ref()) {
                registry.unregister(session);
            }
            drop(ai_watcher);
            return Ok(AppResult {
                exit_code: crate::integrate::exit_code::SUCCESS,
                choosedir_path: state.choosedir_path.clone(),
            });
        }
    }
}
