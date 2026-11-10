//! Shared admission for CPU-heavy preview/search work, across all tabs.
use std::sync::{
    OnceLock,
    atomic::{AtomicUsize, Ordering},
    mpsc::{self, SyncSender},
};
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
pub struct Permit;
impl Permit {
    pub fn acquire() -> Option<Self> {
        ACTIVE
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 2).then_some(n + 1)
            })
            .ok()
            .map(|_| Self)
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::Release);
    }
}

type Garbage = Box<dyn std::any::Any + Send>;
static RECLAIM: OnceLock<SyncSender<Garbage>> = OnceLock::new();
/// Evict large cached rows on a bounded consumer. On overload the caller keeps
/// ownership and retries, rather than creating threads or blocking the input.
pub fn reclaim<T: Send + 'static>(item: T) -> Result<(), T> {
    let tx = RECLAIM.get_or_init(|| {
        let (tx, rx) = mpsc::sync_channel::<Garbage>(8);
        std::thread::spawn(move || {
            for item in rx {
                drop(item);
            }
        });
        tx
    });
    tx.try_send(Box::new(item)).map_err(|e| {
        let item = match e {
            mpsc::TrySendError::Full(item) | mpsc::TrySendError::Disconnected(item) => item,
        };
        match item.downcast::<T>() {
            Ok(item) => *item,
            Err(_) => unreachable!("original cache type"),
        }
    })
}
