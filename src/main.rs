mod app;
mod config;
mod csv_io;
mod db;
mod debt;
mod events;
mod model;
mod recurring;
mod ui;
mod validation;

use crate::app::state::App;
use events::run_app;

use crossterm::{
    ExecutableCommand,
    event::{DisableBracketedPaste, EnableBracketedPaste},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::{CrosstermBackend, Terminal};
use std::io::stdout;
use std::result::Result as StdResult;

const HELP: &str = concat!(
    "A terminal app for tracking your personal budget.\n\n",
    "Usage: ",
    env!("CARGO_BIN_NAME"),
    " [OPTIONS]\n\n",
    "Options:\n",
    "  -h, --help     Print this help\n",
    "  -V, --version  Print version\n\n",
    "Run with no arguments to start the app. Press Ctrl+H inside it for the keybindings."
);

fn main() -> StdResult<(), Box<dyn std::error::Error>> {
    match std::env::args().nth(1).as_deref() {
        None => {}
        Some("-h" | "--help") => {
            println!("{HELP}");
            return Ok(());
        }
        Some("-V" | "--version") => {
            println!("{} {}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some(other) => {
            eprintln!("unrecognized argument: {other}\n\n{HELP}");
            std::process::exit(2);
        }
    }

    app::updater::clean_up();

    // Restore the terminal before printing a panic.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    enable_raw_mode()?;
    stdout()
        .execute(EnterAlternateScreen)?
        .execute(EnableBracketedPaste)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut app = App::new();
    let initial_status = app.status_message.clone();

    let run_result = run_app(&mut terminal, &mut app);

    restore_terminal();

    if let Some(msg) = initial_status {
        eprintln!("Initial Status: {}", msg);
    }

    match run_result {
        Ok(_) => match app.after_exit.take() {
            Some(after) => {
                // Release database and ledger locks before restarting.
                drop(app);
                drop(terminal);
                app::updater::run_after_exit(after)
            }
            None => Ok(()),
        },
        Err(run_err) => {
            eprintln!("Application Error: {}", run_err);
            Err(run_err)
        }
    }
}

fn restore_terminal() {
    let _ = stdout().execute(DisableBracketedPaste);
    let _ = disable_raw_mode();
    let _ = stdout().execute(LeaveAlternateScreen);
    let _ = stdout().execute(crossterm::cursor::Show);
}
