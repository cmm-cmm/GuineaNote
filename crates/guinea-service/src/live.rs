//! Typed messages to the sole TUI state owner. No filesystem work in UI calls.
use crate::{Error, Result};
use guinea_core::Document;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, Ordering},
    mpsc::{self, Sender, SyncSender},
};
use std::time::{Duration, Instant};

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
    pub state: Arc<AtomicU8>,
}
#[derive(Clone)]
pub struct Live {
    pub session: String,
    pub tx: SyncSender<UiCall>,
    pub wake: Option<Arc<dyn Fn() + Send + Sync>>,
}
impl Live {
    pub fn call(&self, command: UiCommand) -> Result<UiReply> {
        self.call_timeout(command, Duration::from_secs(30))
    }
    fn call_timeout(&self, command: UiCommand, timeout: Duration) -> Result<UiReply> {
        self.call_inner(command, timeout, None)
    }
    pub fn call_cancelable(&self, command: UiCommand, cancelled: &AtomicBool) -> Result<UiReply> {
        self.call_inner(command, Duration::from_secs(30), Some(cancelled))
    }
    fn call_inner(
        &self,
        command: UiCommand,
        timeout: Duration,
        cancelled: Option<&AtomicBool>,
    ) -> Result<UiReply> {
        let (tx, rx) = mpsc::channel();
        let state = Arc::new(AtomicU8::new(0));
        let must_complete = matches!(command, UiCommand::Saved { .. } | UiCommand::Release { .. });
        let call = UiCall {
            command,
            reply: tx,
            state: state.clone(),
        };
        if must_complete {
            self.tx.send(call).map_err(|_| {
                Error::new(
                    "SESSION_GONE",
                    "Editor closed after filesystem commit; inspect disk",
                )
            })?;
        } else {
            self.tx
                .try_send(call)
                .map_err(|_| Error::new("BUSY", "Editor request queue is full or closed"))?;
        }
        if let Some(wake) = &self.wake {
            wake();
        }
        let deadline = Instant::now() + timeout;
        loop {
            if !must_complete
                && (Instant::now() >= deadline
                    || cancelled.is_some_and(|c| c.load(Ordering::Acquire)))
            {
                if state
                    .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    return Err(Error::new(
                        "CANCELLED",
                        "UI request cancelled before execution; nothing was committed",
                    ));
                }
                // The UI already owns the commit; obtain its actual receipt.
                return rx.recv().map_err(|_| {
                    Error::new(
                        "SESSION_GONE",
                        "Editor closed; inspect disk before retrying writes",
                    )
                })?;
            }
            match rx.recv_timeout(Duration::from_millis(5)) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(Error::new(
                        "SESSION_GONE",
                        "Editor closed; inspect disk before retrying writes",
                    ));
                }
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
        let id = crate::id();
        let reply = self.call(UiCommand::Acquire {
            document,
            lease: id.clone(),
            action: action.into(),
        })?;
        let lease = Lease {
            live: self.clone(),
            id,
        };
        match reply {
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
            state: Arc::new(AtomicU8::new(0)),
        });
        if let Some(wake) = &self.live.wake {
            wake();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queue_overload_rejects_acquire_without_waiting_for_an_unowned_release() {
        let (tx, _rx) = mpsc::sync_channel(1);
        let (reply, _) = mpsc::channel();
        tx.send(UiCall {
            command: UiCommand::ListTabs,
            reply,
            state: Arc::new(AtomicU8::new(0)),
        })
        .ok()
        .unwrap();
        let live = Live {
            session: "session".into(),
            tx,
            wake: None,
        };
        let (reply, result) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let error = live.acquire("session:d1", "read").err().unwrap();
            let _ = reply.send(error.code);
        });
        assert_eq!(result.recv_timeout(Duration::from_secs(1)).unwrap(), "BUSY");
        worker.join().unwrap();
    }
    #[test]
    fn queued_timeout_cannot_later_commit_and_started_commit_returns_its_result() {
        let (tx, rx) = mpsc::sync_channel(4);
        let live = Live {
            session: "session".into(),
            tx,
            wake: None,
        };
        let error = match live.call_timeout(UiCommand::New, Duration::from_millis(2)) {
            Err(e) => e,
            _ => panic!("queued call must time out"),
        };
        assert_eq!(error.code, "CANCELLED");
        let call = rx.recv().unwrap();
        assert_eq!(call.state.load(Ordering::Acquire), 2);
        drop(call);
        let other = live.clone();
        let worker = std::thread::spawn(move || {
            other.call_timeout(UiCommand::New, Duration::from_millis(5))
        });
        let call = rx.recv().unwrap();
        assert!(
            call.state
                .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        );
        std::thread::sleep(Duration::from_millis(20));
        call.reply
            .send(Ok(UiReply::Value(serde_json::json!({"committed":true}))))
            .ok()
            .unwrap();
        let UiReply::Value(value) = worker.join().unwrap().unwrap() else {
            panic!("missing result")
        };
        assert_eq!(value["committed"], true);
        let error = live
            .call_cancelable(UiCommand::New, &AtomicBool::new(true))
            .err()
            .unwrap();
        assert_eq!(error.code, "CANCELLED");
        assert_eq!(rx.recv().unwrap().state.load(Ordering::Acquire), 2);
    }
}
