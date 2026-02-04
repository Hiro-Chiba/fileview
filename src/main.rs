//! FileView - A minimal file tree UI for terminal emulators

use std::io::stdout;
use std::process::ExitCode;

use crossterm::{
    cursor,
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use fileview::app::{run_app, Config, InitAction, PluginAction, SessionAction};
use fileview::integrate::{
    claude_init, collect_related_candidates, collect_related_paths, exit_code, load_session,
    load_session_named, output_context, output_context_pack_with_options, output_paths,
    output_tree, plugin_init, plugin_test, run_ai_benchmark, Session,
};
use fileview::render::create_image_picker;

fn main() -> ExitCode {
    // Parse config first to return INVALID exit code for argument errors
    let config = match Config::from_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            return ExitCode::from(exit_code::INVALID as u8);
        }
    };

    // Handle non-interactive modes first
    if config.tree_mode {
        return run_tree_mode(&config);
    }

    if config.context_mode {
        return run_context_mode(&config);
    }

    if config.benchmark_ai {
        return run_benchmark_ai_mode(&config);
    }

    if let Some(preset) = config.context_pack {
        return run_context_pack_mode(&config, preset);
    }

    if let Some(ref path) = config.select_related_path {
        return run_select_related_mode(path, config.explain_selection);
    }

    if config.mcp_server {
        return run_mcp_server(&config);
    }

    if let Some(ref name) = config.resume_ai_session {
        return run_resume_ai_session(&config, name);
    }

    // Handle session actions (non-interactive)
    if let Some(action) = config.session_action {
        return run_session_action(&config, action);
    }

    if let Some(action) = config.init_action {
        return run_init_action(&config, action);
    }

    if let Some(action) = config.plugin_action {
        return run_plugin_action(&config, action);
    }

    match run_with_config(config) {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(exit_code::ERROR as u8)
        }
    }
}

/// Run in tree output mode (non-interactive)
fn run_tree_mode(config: &Config) -> ExitCode {
    match output_tree(&config.root, config.tree_depth, config.show_hidden) {
        Ok(_) => ExitCode::from(exit_code::SUCCESS as u8),
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(exit_code::ERROR as u8)
        }
    }
}

/// Run in context output mode (non-interactive)
fn run_context_mode(config: &Config) -> ExitCode {
    match output_context(&config.root) {
        Ok(_) => ExitCode::from(exit_code::SUCCESS as u8),
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(exit_code::ERROR as u8)
        }
    }
}

/// Run AI benchmark mode (non-interactive)
fn run_benchmark_ai_mode(config: &Config) -> ExitCode {
    match run_ai_benchmark(
        &config.root,
        &config.benchmark_scenario,
        config.benchmark_iterations,
    ) {
        Ok(_) => ExitCode::from(exit_code::SUCCESS as u8),
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(exit_code::ERROR as u8)
        }
    }
}

/// Run in context-pack output mode (non-interactive)
fn run_context_pack_mode(
    config: &Config,
    preset: fileview::integrate::ContextPackPreset,
) -> ExitCode {
    match output_context_pack_with_options(&config.root, preset, &config.context_pack_options) {
        Ok(_) => ExitCode::from(exit_code::SUCCESS as u8),
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(exit_code::ERROR as u8)
        }
    }
}

/// Run in related-file output mode (non-interactive)
fn run_select_related_mode(path: &std::path::Path, explain: bool) -> ExitCode {
    if explain {
        let related = collect_related_candidates(path);
        for c in related {
            let json = serde_json::json!({
                "path": c.path.display().to_string(),
                "score": c.score,
                "reasons": c.reasons,
            });
            println!("{}", json);
        }
        return ExitCode::from(exit_code::SUCCESS as u8);
    }

    let related = collect_related_paths(path);
    match output_paths(&related, fileview::integrate::OutputFormat::Lines) {
        Ok(_) => ExitCode::from(exit_code::SUCCESS as u8),
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(exit_code::ERROR as u8)
        }
    }
}

/// Run as MCP server (JSON-RPC over stdin/stdout)
fn run_mcp_server(config: &Config) -> ExitCode {
    match fileview::mcp::run_server(&config.root) {
        Ok(_) => ExitCode::from(exit_code::SUCCESS as u8),
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(exit_code::ERROR as u8)
        }
    }
}

/// Run session action (save/restore/clear)
fn run_session_action(config: &Config, action: SessionAction) -> ExitCode {
    match action {
        SessionAction::Save => {
            // Note: Save requires interactive mode to capture selection
            // This is a placeholder - actual save happens on exit in interactive mode
            eprintln!("Session save: Use in interactive mode and press 's' to save session");
            ExitCode::from(exit_code::SUCCESS as u8)
        }
        SessionAction::Restore => match load_session(&config.root) {
            Ok((selected, focus)) => {
                println!("Session restored:");
                println!("  Selected: {} file(s)", selected.len());
                for path in &selected {
                    println!("    {}", path.display());
                }
                if let Some(f) = focus {
                    println!("  Focus: {}", f.display());
                }
                ExitCode::from(exit_code::SUCCESS as u8)
            }
            Err(e) => {
                eprintln!("Failed to restore session: {}", e);
                ExitCode::from(exit_code::ERROR as u8)
            }
        },
        SessionAction::Clear => match Session::delete(&config.root) {
            Ok(_) => {
                println!("Session cleared");
                ExitCode::from(exit_code::SUCCESS as u8)
            }
            Err(e) => {
                eprintln!("Failed to clear session: {}", e);
                ExitCode::from(exit_code::ERROR as u8)
            }
        },
    }
}

/// Resume named AI session and print selected files.
fn run_resume_ai_session(config: &Config, name: &str) -> ExitCode {
    match load_session_named(&config.root, Some(name)) {
        Ok((selected, focus)) => {
            println!("AI session restored: {}", name);
            println!("  Selected: {} file(s)", selected.len());
            for path in &selected {
                println!("    {}", path.display());
            }
            if let Some(f) = focus {
                println!("  Focus: {}", f.display());
            }
            ExitCode::from(exit_code::SUCCESS as u8)
        }
        Err(e) => {
            eprintln!("Failed to restore AI session '{}': {}", name, e);
            ExitCode::from(exit_code::ERROR as u8)
        }
    }
}

/// Run init action.
fn run_init_action(config: &Config, action: InitAction) -> ExitCode {
    match action {
        InitAction::Claude => {
            match claude_init(&config.root, config.init_path.as_deref(), config.init_force) {
                Ok((path, changed)) => {
                    if changed {
                        println!("Claude config updated: {}", path.display());
                    } else {
                        println!("Claude config already up-to-date: {}", path.display());
                    }
                    ExitCode::from(exit_code::SUCCESS as u8)
                }
                Err(e) => {
                    eprintln!("Failed to init Claude config: {}", e);
                    ExitCode::from(exit_code::ERROR as u8)
                }
            }
        }
    }
}

/// Run plugin command action (non-interactive)
fn run_plugin_action(config: &Config, action: PluginAction) -> ExitCode {
    match action {
        PluginAction::Init => match plugin_init(config.plugin_path.as_deref()) {
            Ok(path) => {
                println!("Plugin initialized: {}", path.display());
                ExitCode::from(exit_code::SUCCESS as u8)
            }
            Err(e) => {
                eprintln!("Failed to initialize plugin: {}", e);
                ExitCode::from(exit_code::ERROR as u8)
            }
        },
        PluginAction::Test => {
            let Some(path) = config.plugin_path.as_deref() else {
                eprintln!("plugin test requires a .lua path");
                return ExitCode::from(exit_code::INVALID as u8);
            };
            match plugin_test(path) {
                Ok(notifications) => {
                    println!("Plugin test passed: {}", path.display());
                    if !notifications.is_empty() {
                        println!("Notifications:");
                        for n in notifications {
                            println!("  - {}", n);
                        }
                    }
                    ExitCode::from(exit_code::SUCCESS as u8)
                }
                Err(e) => {
                    eprintln!("Plugin test failed: {}", e);
                    ExitCode::from(exit_code::ERROR as u8)
                }
            }
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
