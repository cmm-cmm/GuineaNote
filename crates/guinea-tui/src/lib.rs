mod app;
mod commands;
mod config;
mod dialogs;
mod explorer;
mod file_info;
mod file_picker;
mod i18n;
mod input;
mod keyboard;
mod preferences;
mod project;
mod recovery;
mod render;
mod review;
mod search;
mod tab_manager;
mod theme;
mod trace;
mod work;
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
    time::{Duration, Instant},
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
        let _span = trace::span("terminal_restore");
        // Mouse restoration can fail if capture was never enabled. It must not
        // prevent alternate-screen/cursor/raw-mode cleanup (including panic).
        {
            let _span = trace::span("mouse_restore");
            let _ = execute!(stdout(), DisableMouseCapture);
        }
        {
            let _span = trace::span("screen_restore");
            let _ = execute!(
                stdout(),
                DisableBracketedPaste,
                LeaveAlternateScreen,
                crossterm::cursor::Show
            );
        }
        {
            let _span = trace::span("raw_restore");
            let _ = terminal::disable_raw_mode();
        }
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
    language: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_with_agent(files, readonly, config, language, AgentOptions::default())
}
#[derive(Default)]
pub struct AgentOptions {
    pub enabled: bool,
    pub write: bool,
    pub secrets: bool,
}
pub fn run_with_agent(
    files: Vec<PathBuf>,
    readonly: bool,
    config: Option<PathBuf>,
    language: Option<&str>,
    agent: AgentOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = config::Config::load(config.as_deref())?;
    if let Some(language) = language {
        config.language = match language {
            "vi" => i18n::Locale::Vi,
            "en" => i18n::Locale::En,
            _ => return Err("language must be vi or en".into()),
        };
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("Guinea Note requires an interactive terminal (try --help)".into());
    }
    let _trace = trace::start()?;
    let _guard = TerminalGuard::enter()?;
    if config.mouse {
        execute!(stdout(), EnableMouseCapture)?;
    }
    let write_times = trace::WriteTimes::default();
    let mut terminal = Terminal::new(CrosstermBackend::new(trace::Writer::new(
        stdout(),
        write_times.clone(),
    )))?;
    let mut interactions = trace::Frames::default();
    let mut app = app::App::new(files, readonly, config);
    app.preferences.path = preferences::path();
    app.recovery = recovery::Recovery::start(app.preferences.path.clone(), app.config.recovery);
    let mut input = input::Input::new();
    if agent.enabled {
        app.enable_agent(agent.write, agent.secrets, input.waker())?;
    }
    while !app.exit {
        {
            let _span = trace::span("poll_jobs");
            app.poll_jobs();
        }
        if app.redraw {
            let _draw = trace::span("draw_total");
            terminal.draw(|frame| {
                let _build = trace::span("frame_build");
                render::draw(frame, &mut app)
            })?;
            interactions.flushed();
            write_times.frame();
            app.redraw = false;
        }
        // At most four idle wakeups/sec, no redraw without state changes.
        if let Some(event) = input.read(Duration::from_millis(if app.background_work() {
            8
        } else {
            250
        }))? {
            let geometry_change = matches!(
                event,
                crossterm::event::Event::Resize(..) | crossterm::event::Event::Mouse(_)
            );
            {
                let receipt = trace::received(&app, &event);
                let _span = trace::span("input_handle");
                app.handle(event);
                interactions.record(receipt, app.redraw);
            }
            // Coalesce bursts (typed paths, key repeats, paste transport) into
            // one frame. Bound the batch so rendering/jobs cannot starve.
            let started = Instant::now();
            if !geometry_change {
                for _ in 0..127 {
                    if app.exit || started.elapsed() >= Duration::from_millis(4) {
                        break;
                    }
                    let Some(event) = input.read(Duration::ZERO)? else {
                        break;
                    };
                    let geometry_change = matches!(
                        event,
                        crossterm::event::Event::Resize(..) | crossterm::event::Event::Mouse(_)
                    );
                    {
                        let receipt = trace::received(&app, &event);
                        let _span = trace::span("input_handle");
                        app.handle(event);
                        interactions.record(receipt, app.redraw);
                    }
                    if geometry_change {
                        break;
                    }
                }
            }
        }
    }
    {
        let _span = trace::span("recovery_finish");
        app.recovery.finish().map_err(io::Error::other)?;
    }
    {
        let _span = trace::span("app_drop");
        drop(app);
    }
    Ok(())
}
