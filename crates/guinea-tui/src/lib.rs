mod app;
mod config;
mod explorer;
mod input;
mod render;
mod theme;
mod workspace_render;

use crossterm::{
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    io::{self, IsTerminal, stdout},
    path::PathBuf,
    time::Duration,
};

pub struct TerminalGuard;

impl TerminalGuard {
    pub fn enter() -> io::Result<Self> {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            Self::restore();
            previous(info);
        }));
        let guard = Self;
        terminal::enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, EnableBracketedPaste)?;
        Ok(guard)
    }
    fn restore() {
        // Mouse restoration can fail if capture was never enabled. It must not
        // prevent alternate-screen/cursor/raw-mode cleanup (including panic).
        let _ = execute!(stdout(), DisableMouseCapture);
        let _ = execute!(
            stdout(),
            DisableBracketedPaste,
            LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        let _ = terminal::disable_raw_mode();
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        Self::restore();
    }
}

pub fn run(
    files: Vec<PathBuf>,
    readonly: bool,
    config: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::load(config.as_deref())?;
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("GuineaNote requires an interactive terminal (try --help)".into());
    }
    let _guard = TerminalGuard::enter()?;
    if config.mouse {
        execute!(stdout(), EnableMouseCapture)?;
    }
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut app = app::App::new(files, readonly, config);
    let mut input = input::Input::new();
    while !app.exit {
        app.poll_jobs();
        if app.redraw {
            terminal.draw(|frame| render::draw(frame, &mut app))?;
            app.redraw = false;
        }
        // At most four idle wakeups/sec, no redraw without state changes.
        if let Some(event) = input.read(Duration::from_millis(250))? {
            app.handle(event);
        }
    }
    Ok(())
}
