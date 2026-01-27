//! Debug tool to see what events are sent when dragging files
//!
//! Run with: cargo run --example debug_events
//! Then drag a file from Finder to the terminal window.
//! Press 'q' to quit.

use std::io::stdout;
use std::time::Duration;

use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event,
    },
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};

fn main() -> anyhow::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;

    println!("Drag a file from Finder to this window.");
    println!("Press 'q' to quit.\n");

    loop {
        if event::poll(Duration::from_millis(100))? {
            let evt = event::read()?;

            match &evt {
                Event::Key(key) => {
                    println!("Key: {:?}", key);
                    if key.code == crossterm::event::KeyCode::Char('q') {
                        break;
                    }
                }
                Event::Mouse(mouse) => {
                    println!("Mouse: {:?}", mouse);
                }
                Event::Paste(text) => {
                    println!("Paste: {:?}", text);
                    println!("  Length: {} chars", text.len());
                    println!("  Lines: {:?}", text.lines().collect::<Vec<_>>());
                }
                Event::Resize(w, h) => {
                    println!("Resize: {}x{}", w, h);
                }
                Event::FocusGained => {
                    println!("FocusGained");
                }
                Event::FocusLost => {
                    println!("FocusLost");
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    execute!(
        stdout,
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste
    )?;

    Ok(())
}
