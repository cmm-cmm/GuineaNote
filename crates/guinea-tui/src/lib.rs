mod app;
mod clipboard;
mod commands;
mod config;
mod dialogs;
mod file_info;
mod file_picker;
mod i18n;
mod input;
mod keyboard;
mod layout_worker;
mod preferences;
mod recovery;
mod render;
mod review;
mod search;
mod session;
mod syntax_state;
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
    // Guinea Note edits files; folder/workspace modes were removed.
    if let Some(folder) = files.iter().find(|path| path.is_dir()) {
        return Err(format!(
            "{} is a folder; Guinea Note opens files (use Ctrl+O to browse for one)",
            folder.display()
        )
        .into());
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
    let preferences_path = preferences::path();
    // Notepad++-style session: reopened only when no files were named.
    let session_path = session::path(preferences_path.as_deref()).filter(|_| config.session);
    let mut files = files;
    let mut restore = None;
    if files.is_empty()
        && let Some(path) = &session_path
    {
        let (queued, loaded, _) = session::Restore::load(path);
        files = queued;
        restore = Some(loaded);
    }
    let mut app = app::App::new(files, readonly, config);
    if let Some(restore) = restore {
        if restore.is_empty() {
            if restore.missing > 0 {
                app.message = app.config.language.render(i18n::Msg::SessionRestored {
                    files: 0,
                    missing: restore.missing,
                });
            }
        } else {
            app.session = Some(restore);
        }
    }
    app.preferences.path = preferences_path;
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
        {
            let _span = trace::span("input_replay");
            app.replay_ready_input();
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
            app.layout.start();
            if let Some(sequence) = app.clipboard.osc52.take() {
                // OSC 52 copy: written between frames, never inside one.
                use std::io::Write;
                let mut out = stdout();
                out.write_all(sequence.as_bytes())?;
                out.flush()?;
            }
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
            app.begin_input();
            let mut drained = true;
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
                        drained = false;
                        break;
                    }
                    let Some(event) = input.read(Duration::ZERO)? else {
                        drained = true;
                        break;
                    };
                    drained = false;
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
            app.end_input(drained);
        }
    }
    if let Some(path) = &session_path {
        let _span = trace::span("session_save");
        // Best effort: a failed session write must not lose the exit status
        // of drafts/recovery below, and there is no screen left to report on.
        let _ = session::save(path, &app.tabs, app.active);
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
