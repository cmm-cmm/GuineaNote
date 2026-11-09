//! Typed messages to the sole TUI state owner. No filesystem work in UI calls.
use crate::{Error, Result};
use guinea_core::Document;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Sender, SyncSender},
};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub tab: String,
    pub document: String,
    pub title: String,
    pub path: Option<String>,
    pub revision: u64,
    pub dirty: bool,
    pub readonly: bool,
    pub review: bool,
    pub ai: bool,
    pub active: bool,
}
pub enum UiCommand {
    ListTabs,
    Acquire {
        document: u64,
        lease: String,
        action: String,
    },
    Release {
        lease: String,
    },
    Activate {
        tab: u64,
    },
    Open {
        document: Document,
    },
    New,
    Apply {
        document: u64,
        lease: String,
        expected_revision: u64,
        prepared: Document,
    },
    Saved {
        document: u64,
        lease: String,
        expected_revision: u64,
        saved: Document,
    },
    Review {
        document: u64,
        split: bool,
    },
    Preview {
        title: String,
        text: String,
    },
    Close {
        tab: u64,
    },
}
pub enum UiReply {
    Document(Document),
    Tabs(Vec<TabInfo>),
    Value(Value),
}
pub struct UiCall {
    pub command: UiCommand,
    pub reply: Sender<Result<UiReply>>,
    pub cancelled: Arc<AtomicBool>,
}
#[derive(Clone)]
pub struct Live {
    pub session: String,
    pub tx: SyncSender<UiCall>,
    pub wake: Option<Arc<dyn Fn() + Send + Sync>>,
}
impl Live {
    pub fn call(&self, command: UiCommand) -> Result<UiReply> {
        let (tx, rx) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        self.tx
            .try_send(UiCall {
                command,
                reply: tx,
                cancelled: cancelled.clone(),
            })
            .map_err(|_| Error::new("BUSY", "Editor request queue is full or closed"))?;
        if let Some(wake) = &self.wake {
            wake();
        }
        match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(result) => result,
            Err(_) => {
                cancelled.store(true, Ordering::Release);
                Err(Error::new(
                    "SESSION_GONE",
                    "Editor did not complete the request; inspect transaction status before retrying writes",
                ))
            }
        }
    }
    pub fn document_id(&self, id: u64) -> String {
        format!("{}:d{id}", self.session)
    }
    pub fn tab_id(&self, id: u64) -> String {
        format!("{}:t{id}", self.session)
    }
    pub fn parse_id(&self, value: &str, kind: char) -> Result<u64> {
        value
            .strip_prefix(&format!("{}:{kind}", self.session))
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| Error::new("NOT_FOUND", "Handle is not from this editor session"))
    }
    pub fn acquire(&self, document: &str, action: &str) -> Result<(Document, Lease)> {
        let document = self.parse_id(document, 'd')?;
        let lease = Lease {
            live: self.clone(),
            id: crate::id(),
        };
        match self.call(UiCommand::Acquire {
            document,
            lease: lease.id.clone(),
            action: action.into(),
        })? {
            UiReply::Document(snapshot) => Ok((snapshot, lease)),
            _ => Err(Error::new("INTERNAL", "Unexpected snapshot reply")),
        }
    }
}
pub struct Lease {
    live: Live,
    pub id: String,
}
impl Drop for Lease {
    fn drop(&mut self) {
        // UI drains this bounded queue even while I/O is active. Waiting is on
        // the worker only; release is never silently lost when the queue fills.
        let (reply, _) = mpsc::channel();
        let _ = self.live.tx.send(UiCall {
            command: UiCommand::Release {
                lease: self.id.clone(),
            },
            reply,
            cancelled: Arc::new(AtomicBool::new(false)),
        });
        if let Some(wake) = &self.live.wake {
            wake();
        }
    }
}
