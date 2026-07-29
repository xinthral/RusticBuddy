//! @file main.rs
//! @brief Entry point for the RusticBuddy TUI: sets up the terminal, wires
//!        together the database, ::app::App state, and ::ui::render, and
//!        runs the main event loop.

use std::io;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};

mod app;
mod database;
mod models;
mod ui;

/// @brief Opens the database, builds the ::app::App, and drives the draw/handle-event loop until quit.
/// @param terminal The already-initialized ratatui terminal to draw into.
/// @return `Ok(())` once the app requests quit, or an I/O error from drawing/event handling.
fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let db = database::open().expect("Failed to open database");
    let mut app = app::App::new(db);

    loop {
        terminal.draw(|frame| ui::render(frame, &mut app))?;
        app.handle_event()?;
        if app.should_quit {
            break;
        }
    }
    Ok(())
}

/// @brief Program entry point.
///
/// Puts the terminal into raw/alternate-screen mode, runs the application
/// via ::run, and always restores the terminal afterward (even if ::run
/// returned an error) before propagating the result.
///
/// @return `Ok(())` on a clean exit, or the first I/O error encountered.
fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}