//! Opt-in latency tracing. No source text, paths, clipboard or key contents.
//! UI samples use try_send; file writes run on a bounded background consumer.
use std::{
    fs::{File, OpenOptions},
    io::{self, BufWriter, Write},
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
    thread::{self, JoinHandle},
    time::Instant,
};

#[derive(Default)]
struct Sample {
    stage: &'static str,
    micros: u128,
    since_start: u128,
    operation: &'static str,
    phase: &'static str,
    bytes: usize,
    width: u16,
    height: u16,
    batch: usize,
    agent_active: bool,
}
static SENDER: OnceLock<SyncSender<Sample>> = OnceLock::new();
static START: OnceLock<Instant> = OnceLock::new();
static DROPPED: AtomicU64 = AtomicU64::new(0);

pub struct Trace {
    writer: Option<JoinHandle<()>>,
}
pub fn start() -> io::Result<Trace> {
    let Some(path) = std::env::var_os("GUINEANOTE_TRACE") else {
        return Ok(Trace { writer: None });
    };
    // Never overwrite an existing trace or user file.
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let (tx, rx) = mpsc::sync_channel::<Sample>(4096);
    let _ = START.set(Instant::now());
    let _ = SENDER.set(tx);
    let writer = thread::spawn(move || write(file, rx));
    Ok(Trace {
        writer: Some(writer),
    })
}
fn write(file: File, rx: mpsc::Receiver<Sample>) {
    let mut writer = BufWriter::new(file);
    let _ = writeln!(
        writer,
        "stage,microseconds,since_start_microseconds,operation,phase,file_bytes,viewport_width,viewport_height,batch_events,agent_active"
    );
    for sample in rx {
        if sample.stage == "trace_stop" {
            break;
        }
        if writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{},{}",
            sample.stage,
            sample.micros,
            sample.since_start,
            sample.operation,
            sample.phase,
            sample.bytes,
            sample.width,
            sample.height,
            sample.batch,
            sample.agent_active
        )
        .is_err()
        {
            break;
        }
        // Persist diagnostic output during a crash without flushing on the UI.
        let _ = writer.flush();
    }
    let _ = writeln!(
        writer,
        "dropped_samples,{},0,,,0,0,0,0,false",
        DROPPED.load(Ordering::Relaxed)
    );
    let _ = writer.flush();
}
pub struct Span {
    stage: &'static str,
    started: Option<Instant>,
}
pub fn span(stage: &'static str) -> Span {
    Span {
        stage,
        started: SENDER.get().map(|_| Instant::now()),
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        if let (Some(started), Some(tx)) = (self.started, SENDER.get())
            && tx
                .try_send(Sample {
                    stage: self.stage,
                    micros: started.elapsed().as_micros(),
                    since_start: START.get().map_or(0, |s| s.elapsed().as_micros()),
                    ..Sample::default()
                })
                .is_err()
        {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    }
}
impl Drop for Trace {
    fn drop(&mut self) {
        if let Some(writer) = self.writer.take() {
            if let Some(tx) = SENDER.get() {
                // Only shutdown may wait for the diagnostic consumer.
                let _ = tx.send(Sample {
                    stage: "trace_stop",
                    micros: 0,
                    since_start: 0,
                    ..Sample::default()
                });
            }
            let _ = writer.join();
        }
    }
}

fn send(mut sample: Sample) {
    sample.since_start = START.get().map_or(0, |s| s.elapsed().as_micros());
    if let Some(tx) = SENDER.get()
        && tx.try_send(sample).is_err()
    {
        DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}
pub struct Interaction {
    start: Instant,
    operation: &'static str,
    document: u64,
    bytes: usize,
    width: u16,
    height: u16,
    agent_active: bool,
}
pub fn received(app: &crate::app::App, event: &crossterm::event::Event) -> Option<Interaction> {
    use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
    use guinea_core::TextBuffer;
    SENDER.get()?;
    let start = Instant::now();
    let operation = match event {
        Event::Resize(..) => "resize",
        Event::Mouse(_) => "mouse",
        Event::Paste(_) => "paste",
        Event::Key(key) if key.kind != KeyEventKind::Release => {
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            match key.code {
                KeyCode::F(6) | KeyCode::PageUp | KeyCode::PageDown
                    if ctrl || key.code == KeyCode::F(6) =>
                {
                    "tabs"
                }
                KeyCode::F(2 | 7 | 8 | 9) => "dialogs",
                KeyCode::F(11) => "focus_mode",
                KeyCode::Char('p' | 't' | 'n') if ctrl => "tabs",
                KeyCode::Char('e' | 'g') if ctrl => "project_open",
                KeyCode::Char('f' | 'h') if ctrl => "search_ui",
                KeyCode::Char('z' | 'y') if ctrl => "history",
                _ if matches!(&app.modal, Some(crate::app::Modal::Project(_))) => "project_query",
                _ if matches!(&app.modal, Some(crate::app::Modal::Search(_))) => "search_query",
                _ if app.modal.is_some() => "dialog_input",
                KeyCode::End
                    if app.document().buffer().len_bytes() >= 1024 * 1024
                        && app.document().buffer().len_lines() == 1 =>
                {
                    "long_line_end"
                }
                KeyCode::Home
                    if app.document().buffer().len_bytes() >= 1024 * 1024
                        && app.document().buffer().len_lines() == 1 =>
                {
                    "long_line_home"
                }
                KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Home
                | KeyCode::End => "caret",
                KeyCode::Delete | KeyCode::Backspace => "delete",
                KeyCode::Char(_) if !ctrl => "typing",
                _ => "commands",
            }
        }
        _ => return None,
    };
    Some(Interaction {
        start,
        operation,
        document: app.document().id,
        bytes: app.document().buffer().len_bytes(),
        width: app.hits.editor_text.width,
        height: app.hits.editor_text.height,
        agent_active: app.tabs.iter().any(|tab| tab.ai_action.is_some()),
    })
}
#[derive(Default)]
pub struct Frames {
    pending: Vec<Interaction>,
    seen: std::collections::HashSet<(u64, &'static str)>,
}
impl Frames {
    pub fn record(&mut self, interaction: Option<Interaction>, redraw: bool) {
        if let Some(interaction) = interaction
            && redraw
        {
            if self.pending.len() < 128 {
                self.pending.push(interaction);
            } else {
                DROPPED.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    /// Called only after Terminal::draw has successfully flushed the frame.
    pub fn flushed(&mut self) {
        let end = Instant::now();
        let batch = self.pending.len();
        for input in self.pending.drain(..) {
            let cold = self.seen.insert((input.document, input.operation));
            send(Sample {
                stage: "input_to_flush",
                micros: end.duration_since(input.start).as_micros(),
                operation: input.operation,
                phase: if cold { "cold" } else { "warm" },
                bytes: input.bytes,
                width: input.width,
                height: input.height,
                batch,
                agent_active: input.agent_active,
                ..Sample::default()
            });
        }
        // Diagnostic classification is bounded; eviction starts a new cold epoch.
        if self.seen.len() > 1024 {
            self.seen.clear();
        }
    }
}
/// Aggregate writer costs per frame, avoiding a trace row for every escape code.
#[derive(Clone, Default)]
pub struct WriteTimes(std::rc::Rc<std::cell::Cell<(u128, u128)>>);
impl WriteTimes {
    pub fn frame(&self) {
        let (write, flush) = self.0.replace((0, 0));
        if SENDER.get().is_some() {
            send(Sample {
                stage: "terminal_write",
                micros: write / 1000,
                ..Sample::default()
            });
            send(Sample {
                stage: "terminal_flush",
                micros: flush / 1000,
                ..Sample::default()
            });
        }
    }
}
pub struct Writer<W> {
    inner: W,
    times: WriteTimes,
}
impl<W> Writer<W> {
    pub fn new(inner: W, times: WriteTimes) -> Self {
        Self { inner, times }
    }
}
impl<W: Write> Write for Writer<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let start = SENDER.get().map(|_| Instant::now());
        let result = self.inner.write(bytes);
        if let Some(start) = start {
            let (old, flush) = self.times.0.get();
            self.times.0.set((old + start.elapsed().as_nanos(), flush));
        }
        result
    }
    fn flush(&mut self) -> io::Result<()> {
        let start = SENDER.get().map(|_| Instant::now());
        let result = self.inner.flush();
        if let Some(start) = start {
            let (write, old) = self.times.0.get();
            self.times.0.set((write, old + start.elapsed().as_nanos()));
        }
        result
    }
}
