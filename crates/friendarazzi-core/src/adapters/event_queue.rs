use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use async_trait::async_trait;
use futures::StreamExt;
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::stream::FuturesUnordered;
use tracing::{debug, error};

use crate::ResultReport;
use crate::application::WeakApp;
use crate::ports::{EventQueue, EventQueueError};
use crate::service::Event;

const DEFAULT_WORKER_COUNT: usize = 2;

async fn execute_event(app: WeakApp, event: Event) {
    let Some(inner) = app.upgrade_inner() else {
        return;
    };
    debug!("executing queued event");
    if let Err(err) = inner.dispatch_event(event).await {
        error!(err, "queued event execution failed");
    } else {
        debug!("queued event completed");
    }
}

/// In-memory blocking [`EventQueue`] adapter that executes submitted events inline on [`App`](crate::App).
///
/// Events are dispatched sequentially after the submitting use case finishes and closes its
/// unit of work.
pub struct BlockingEventQueue {
    app: WeakApp,
}

impl BlockingEventQueue {
    pub fn new(app: WeakApp) -> Self {
        Self { app }
    }

    pub fn from_app(app: WeakApp) -> Arc<dyn EventQueue> {
        Arc::new(Self::new(app))
    }
}

#[async_trait]
impl EventQueue for BlockingEventQueue {
    async fn enqueue_submitted(&self, events: Vec<Event>) -> ResultReport<(), EventQueueError> {
        for event in events {
            if self.app.upgrade_inner().is_none() {
                return Err(error_stack::Report::new(EventQueueError::Unavailable));
            }
            execute_event(self.app.clone(), event).await;
        }
        Ok(())
    }
}

/// Default in-memory non-blocking [`EventQueue`] adapter backed by dedicated worker threads.
///
/// Submitted events are held in memory (not persisted across process restarts) and executed
/// concurrently against [`App`](crate::App) on background worker threads.
pub struct MultithreadedEventQueue {
    app: WeakApp,
    worker_count: usize,
    next_worker: AtomicUsize,
    pool: Mutex<Option<WorkerPool>>,
}

struct WorkerPool {
    senders: Vec<UnboundedSender<Event>>,
    handles: Vec<JoinHandle<()>>,
}

impl MultithreadedEventQueue {
    pub fn new(app: WeakApp) -> Self {
        Self::with_workers(app, DEFAULT_WORKER_COUNT)
    }

    pub fn with_workers(app: WeakApp, worker_count: usize) -> Self {
        Self {
            app,
            worker_count: worker_count.max(1),
            next_worker: AtomicUsize::new(0),
            pool: Mutex::new(None),
        }
    }

    pub fn from_app(app: WeakApp) -> Arc<dyn EventQueue> {
        Arc::new(Self::new(app))
    }

    fn ensure_started(&self) -> Result<Vec<UnboundedSender<Event>>, EventQueueError> {
        let mut guard = self.pool.lock().expect("event queue pool mutex poisoned");
        if let Some(pool) = guard.as_ref() {
            return Ok(pool.senders.clone());
        }
        if self.app.shutdown_signal().is_cancelled() {
            return Err(EventQueueError::Unavailable);
        }

        let mut senders = Vec::with_capacity(self.worker_count);
        let mut handles = Vec::with_capacity(self.worker_count);
        for idx in 0..self.worker_count {
            let (tx, rx) = mpsc::unbounded();
            let app = self.app.clone();
            let handle = thread::Builder::new()
                .name(format!("friendarazzi-event-worker-{idx}"))
                .spawn(move || worker_loop(app, rx, idx))
                .expect("failed to spawn event queue worker thread");
            senders.push(tx);
            handles.push(handle);
        }

        let cloned = senders.clone();
        *guard = Some(WorkerPool { senders, handles });
        Ok(cloned)
    }

    pub fn shutdown(&self) {
        self.app.shutdown_signal().cancel();
        let pool = self
            .pool
            .lock()
            .expect("event queue pool mutex poisoned")
            .take();
        if let Some(pool) = pool {
            drop(pool.senders);
            let current_thread = thread::current().id();
            for handle in pool.handles {
                if handle.thread().id() != current_thread {
                    let _ = handle.join();
                }
            }
        }
    }
}

impl Drop for MultithreadedEventQueue {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn worker_loop(app: WeakApp, mut rx: UnboundedReceiver<Event>, worker_index: usize) {
    debug!(worker_index, "multithreaded event queue worker started");
    let shutdown = app.shutdown_signal().clone();
    futures::executor::block_on(async move {
        let mut in_flight = FuturesUnordered::new();
        let mut shutdown_fut = Box::pin(futures::FutureExt::fuse(shutdown.notified()));

        loop {
            futures::select_biased! {
                _ = shutdown_fut => {
                    debug!(worker_index, "multithreaded event queue worker shutting down");
                    break;
                }
                maybe_event = rx.next() => match maybe_event {
                    Some(event) => in_flight.push(execute_event(app.clone(), event)),
                    None => break,
                },
                _ = in_flight.next() => {}
            }
        }
    });
    debug!(worker_index, "multithreaded event queue worker stopped");
}

#[async_trait]
impl EventQueue for MultithreadedEventQueue {
    async fn enqueue_submitted(&self, events: Vec<Event>) -> ResultReport<(), EventQueueError> {
        if events.is_empty() {
            return Ok(());
        }
        let senders = self.ensure_started().map_err(error_stack::Report::new)?;
        for event in events {
            let idx = self.next_worker.fetch_add(1, Ordering::Relaxed) % senders.len();
            if senders[idx].unbounded_send(event).is_err() {
                return Err(error_stack::Report::new(EventQueueError::Unavailable));
            }
        }
        Ok(())
    }
}
