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

use fileview::app::{run_app, Config, SessionAction};
use fileview::integrate::{exit_code, load_session, output_context, output_tree, Session};
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

    if config.mcp_server {
        return run_mcp_server(&config);
    }

    // Handle session actions (non-interactive)
    if let Some(action) = config.session_action {
        return run_session_action(&config, action);
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
