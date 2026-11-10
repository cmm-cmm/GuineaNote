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

struct Sample {
    stage: &'static str,
    micros: u128,
    since_start: u128,
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
    let _ = writeln!(writer, "stage,microseconds,since_start_microseconds");
    for sample in rx {
        if sample.stage == "trace_stop" {
            break;
        }
        if writeln!(
            writer,
            "{},{},{}",
            sample.stage, sample.micros, sample.since_start
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
        "dropped_samples,{},0",
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
                });
            }
            let _ = writer.join();
        }
    }
}
