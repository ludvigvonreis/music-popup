//! The only way a thread can influence the UI.
//!
//! * `remote_signal(init)`  -> (signal, sender): latest value wins, bounded.
//! * `remote_events(handler)` -> sender: every message is delivered, in order.
//!
//! The signal/handler lives on the main thread. The sender is `Send + Clone`
//! and is all a worker thread ever holds. `send` pushes into a queue and then
//! pokes the event loop; the loop calls `drain_remote()` which moves queued
//! values into signals inside a single `batch` (one effect pass, one frame).

use std::{
    cell::RefCell,
    collections::VecDeque,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use crate::reactive::{RwSignal, batch, create_signal};

type WakeFn = Arc<dyn Fn() + Send + Sync>;

/// Returned by `send` once the receiving side is gone (its scope was
/// disposed). Worker loops should treat it as "stop".
#[derive(Debug)]
pub struct Disconnected;

thread_local! {
    static WAKE: RefCell<Option<WakeFn>> = RefCell::new(None);
    static DRAINERS: RefCell<Vec<Box<dyn FnMut() -> bool>>> = RefCell::new(Vec::new());
}

/// Called by `App` before the root component is built.
pub(crate) fn set_wake_hook(f: WakeFn) {
    WAKE.with_borrow_mut(|w| *w = Some(f));
}

fn wake_hook() -> WakeFn {
    WAKE.with_borrow(|w| w.clone())
        .expect("remote_signal/remote_events must be called inside App::run")
}

fn add_drainer(f: impl FnMut() -> bool + 'static) {
    DRAINERS.with_borrow_mut(|d| d.push(Box::new(f)));
}

/// Called by the event loop on every wakeup. A drainer returns false when its
/// owner was disposed; it is then dropped, which closes the sender side.
pub(crate) fn drain_remote() {
    // Take the list so drainers (or effects they trigger) can register new
    // drainers without a RefCell double borrow.
    let mut list = DRAINERS.with_borrow_mut(std::mem::take);
    batch(|| list.retain_mut(|d| d()));
    DRAINERS.with_borrow_mut(|cur| {
        list.extend(std::mem::take(cur));
        *cur = list;
    });
}

// ---------------------------------------------------------------------------
// Shared queue
// ---------------------------------------------------------------------------

struct Shared<T> {
    queue: Mutex<VecDeque<T>>,
    latest_only: bool,
    open: AtomicBool,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Lives inside the drainer. When the drainer is dropped, senders see `Disconnected`.
struct Closer<T>(Arc<Shared<T>>);

impl<T> Drop for Closer<T> {
    fn drop(&mut self) {
        self.0.open.store(false, Ordering::Release);
    }
}

pub struct RemoteSender<T> {
    shared: Arc<Shared<T>>,
    wake: WakeFn,
}

impl<T> Clone for RemoteSender<T> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
            wake: self.wake.clone(),
        }
    }
}

impl<T: Send> RemoteSender<T> {
    /// Queue a value (data first, wake second) and poke the event loop.
    pub fn send(&self, value: T) -> Result<(), Disconnected> {
        if !self.shared.open.load(Ordering::Acquire) {
            return Err(Disconnected);
        }
        {
            let mut q = lock(&self.shared.queue);
            if self.shared.latest_only {
                q.clear();
            }
            q.push_back(value);
        }
        (self.wake)();
        Ok(())
    }
}

fn new_link<T>(latest_only: bool) -> (Arc<Shared<T>>, WakeFn) {
    let shared = Arc::new(Shared {
        queue: Mutex::new(VecDeque::new()),
        latest_only,
        open: AtomicBool::new(true),
    });
    (shared, wake_hook())
}

// ---------------------------------------------------------------------------
// Public primitives
// ---------------------------------------------------------------------------

/// A signal that threads can feed. Only the newest queued value is kept, so a
/// slow UI never builds a backlog.
pub fn remote_signal<T: Send + 'static>(init: T) -> (RwSignal<T>, RemoteSender<T>) {
    let sig = create_signal(init);
    let (shared, wake) = new_link::<T>(true);
    let closer = Closer(shared.clone());

    add_drainer(move || {
        if !sig.is_alive() {
            return false;
        }
        let items = std::mem::take(&mut *lock(&closer.0.queue));
        if let Some(last) = items.into_iter().last() {
            sig.set(last);
        }
        true
    });

    (sig, RemoteSender { shared, wake })
}

/// Delivers every message, in order, to `handler` on the main thread. Use for
/// discrete events: "track changed", "play pressed". The handler typically
/// sets signals. Unbounded, so don't use it for high-rate data.
pub fn remote_events<T: Send + 'static>(mut handler: impl FnMut(T) + 'static) -> RemoteSender<T> {
    // Dies with the enclosing scope; that is how we notice disposal.
    let alive = create_signal(());
    let (shared, wake) = new_link::<T>(false);
    let closer = Closer(shared.clone());

    add_drainer(move || {
        if !alive.is_alive() {
            return false;
        }
        let items = std::mem::take(&mut *lock(&closer.0.queue));
        for item in items {
            handler(item);
        }
        true
    });

    RemoteSender { shared, wake }
}

/// Run a worker thread. Give it `RemoteSender`s, never signals (signals are
/// `!Send`, so the compiler enforces this).
pub fn spawn(f: impl FnOnce() + Send + 'static) {
    thread::Builder::new()
        .name("worker".into())
        .spawn(f)
        .expect("failed to spawn worker thread");
}
