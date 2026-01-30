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

use fileview::app::{run_app, Config};
use fileview::integrate::exit_code;
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
