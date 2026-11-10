//! A single bounded recovery worker. The UI captures shared roots, never bytes.
use crate::app::Tab;
use guinea_io::recovery::{Checkpoint, Recovered, Store};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

enum Request {
    Checkpoint {
        drafts: Vec<Checkpoint>,
        open: Vec<u64>,
    },
    Stop {
        discard_recovered: bool,
    },
}
pub enum Outcome {
    Loaded(Vec<Recovered>, Vec<String>),
    Saved,
    Error(String),
}
enum Reply {
    Outcome(Outcome),
    Stopped(Result<(), String>),
}
type Key = (u64, u64, bool, bool, Option<PathBuf>);
pub struct Recovery {
    sender: Option<SyncSender<Request>>,
    receiver: Option<Receiver<Reply>>,
    worker: Option<JoinHandle<()>>,
    loaded: bool,
    pending: bool,
    last: Instant,
    key: Vec<Key>,
    pub enabled: bool,
    pub failed: bool,
}
impl Default for Recovery {
    fn default() -> Self {
        Self {
            sender: None,
            receiver: None,
            worker: None,
            loaded: false,
            pending: false,
            last: Instant::now(),
            key: Vec::new(),
            enabled: false,
            failed: false,
        }
    }
}
fn directory(preferences: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("GUINEANOTE_RECOVERY_DIR") {
        return Some(path.into());
    }
    // Explicit preferences roots (including PTY fixtures) stay self-contained.
    if std::env::var_os("GUINEANOTE_PREFERENCES").is_some() {
        return preferences.and_then(|p| p.parent().map(|p| p.join("recovery")));
    }
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/state")));
    base.map(|p| p.join("GuineaNote/recovery"))
}
impl Recovery {
    pub fn start(preferences: Option<PathBuf>, enabled: bool) -> Self {
        if !enabled {
            return Self::default();
        }
        let (tx, rx) = mpsc::sync_channel::<Request>(1);
        let (reply, receiver) = mpsc::sync_channel(4);
        let path = directory(preferences);
        let worker = thread::spawn(move || {
            let load = crate::trace::span("recovery_load");
            let opened = path
                .ok_or_else(|| std::io::Error::other("No private recovery directory available"))
                .and_then(|path| Store::open(&path));
            let (mut store, recovered, warnings) = match opened {
                Ok(result) => result,
                Err(error) => {
                    let _ = reply.send(Reply::Outcome(Outcome::Error(error.to_string())));
                    return;
                }
            };
            drop(load);
            if reply
                .send(Reply::Outcome(Outcome::Loaded(recovered, warnings)))
                .is_err()
            {
                return;
            }
            while let Ok(request) = rx.recv() {
                match request {
                    Request::Checkpoint { drafts, open } => {
                        let _span = crate::trace::span("recovery_checkpoint");
                        let outcome = match store.checkpoint(drafts, &open) {
                            Ok(()) => Outcome::Saved,
                            Err(error) => Outcome::Error(error.to_string()),
                        };
                        if reply.send(Reply::Outcome(outcome)).is_err() {
                            break;
                        }
                    }
                    Request::Stop { discard_recovered } => {
                        let _span = crate::trace::span("recovery_discard");
                        let _ = reply.send(Reply::Stopped(
                            store.discard(discard_recovered).map_err(|e| e.to_string()),
                        ));
                        break;
                    }
                }
            }
            // Disconnect/panic preserves checkpoints. Only explicit Stop discards.
        });
        Self {
            sender: Some(tx),
            receiver: Some(receiver),
            worker: Some(worker),
            pending: true,
            last: Instant::now() - Duration::from_secs(2),
            enabled: true,
            ..Self::default()
        }
    }
    pub fn busy(&self) -> bool {
        self.pending
    }
    pub fn poll(&mut self) -> Option<Outcome> {
        let result = match self.receiver.as_ref()?.try_recv() {
            Ok(Reply::Outcome(outcome)) => Some(outcome),
            Ok(Reply::Stopped(_)) => None,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.sender.take()?;
                Some(Outcome::Error(
                    "Recovery worker stopped; save your changes".into(),
                ))
            }
        };
        self.pending = false;
        if let Some(outcome) = &result {
            if matches!(outcome, Outcome::Loaded(..)) {
                self.loaded = true;
            }
            self.failed = matches!(outcome, Outcome::Error(_));
            if self.failed {
                self.key.clear();
            }
        }
        result
    }
    pub fn checkpoint(&mut self, tabs: &[Tab]) {
        if !self.enabled || self.pending || self.last.elapsed() < Duration::from_secs(1) {
            return;
        }
        let sources = tabs
            .iter()
            .filter(|t| t.review_source.is_none())
            .collect::<Vec<_>>();
        let key = sources
            .iter()
            .map(|t| {
                (
                    t.document.id,
                    t.document.revision(),
                    t.document.bom,
                    t.document.is_dirty(),
                    t.document.path.clone(),
                )
            })
            .collect::<Vec<_>>();
        if key == self.key {
            return;
        }
        let Some(sender) = &self.sender else {
            return;
        };
        let open = sources.iter().map(|t| t.document.id).collect();
        let drafts = sources
            .iter()
            .filter(|t| t.document.is_dirty())
            .map(|t| Checkpoint::capture(&t.document))
            .collect();
        if sender
            .try_send(Request::Checkpoint { drafts, open })
            .is_ok()
        {
            self.key = key;
            self.pending = true;
            self.last = Instant::now();
        }
    }
    pub fn finish(&mut self) -> Result<(), String> {
        if let Some(sender) = self.sender.take()
            && sender
                .send(Request::Stop {
                    discard_recovered: self.loaded,
                })
                .is_ok()
            && let Some(receiver) = &self.receiver
        {
            while let Ok(reply) = receiver.recv() {
                if let Reply::Stopped(result) = reply {
                    result?;
                    break;
                }
            }
        }
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| "Recovery worker panicked")?;
        }
        Ok(())
    }
}
