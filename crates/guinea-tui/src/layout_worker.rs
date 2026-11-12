//! Builds long-line display indexes off the input thread.
//!
//! A first End on an unindexed multi-MiB line used to scan the whole line on
//! the UI thread (tens of ms on 12 cores, >100 ms on one or two). Rendering now
//! asks [`guinea_core::Document::layout_pending`] first; when the exact layout
//! would be expensive it shows a caret-anchored approximation and requests a
//! job here. At most one job runs at a time; results are revision-checked by
//! the document before they are installed, so edits made meanwhile win.
//!
//! A requested job starts only after the current frame is flushed
//! ([`LayoutWorker::start`]): spawning index threads mid-frame measurably
//! delayed the terminal write on a two-core affinity (147 ms in one sample).
use guinea_core::{Document, EditError, LayoutJob, LayoutReady};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use crate::app::Tab;

struct Running {
    document: u64,
    receiver: Receiver<Result<LayoutReady, EditError>>,
}

#[derive(Default)]
pub struct LayoutWorker {
    running: Option<Running>,
    queued: Option<(u64, LayoutJob)>,
}

impl LayoutWorker {
    pub fn busy(&self) -> bool {
        self.running.is_some() || self.queued.is_some()
    }

    /// Start indexing `line` unless a job is already running. A different
    /// pending line is requested again by the next frame after this finishes.
    pub fn request(&mut self, document: &Document, line: usize, tab: usize) {
        if self.busy() {
            return;
        }
        self.queued = document.layout_job(line, tab).map(|job| (document.id, job));
    }

    /// Spawn the queued job; called after the frame that requested it.
    pub fn start(&mut self) {
        if self.running.is_some() {
            return;
        }
        let Some((document, job)) = self.queued.take() else {
            return;
        };
        let (sender, receiver) = mpsc::channel();
        self.running = Some(Running { document, receiver });
        std::thread::spawn(move || {
            let _span = crate::trace::span("layout_job");
            let _ = sender.send(job.run());
        });
    }

    /// Install a finished index into the matching tab. Returns true when the
    /// screen should be redrawn (finished, failed or stale results alike).
    pub fn poll(&mut self, tabs: &[Tab]) -> bool {
        self.start();
        let Some(running) = &self.running else {
            return false;
        };
        let result = match running.receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Err(EditError::OutOfBounds),
        };
        let document = running.document;
        self.running = None;
        if let Ok(ready) = result
            && let Some(tab) = tabs.iter().find(|t| t.document.id == document)
        {
            // A stale result is dropped by the document; rendering asks again.
            let _ = tab.document.accept_layout(ready);
        }
        true
    }
}
