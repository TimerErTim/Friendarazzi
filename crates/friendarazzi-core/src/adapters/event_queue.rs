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
use crate::service::{Event, handle_event};

const DEFAULT_WORKER_COUNT: usize = 2;

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
    async fn enqueue(&self, events: Vec<Event>) -> ResultReport<(), EventQueueError> {
        for event in events {
            let Some(app) = self.app.upgrade() else {
                return Err(error_stack::Report::new(EventQueueError::Unavailable));
            };
            if let Err(err) = handle_event(&app, event).await {
                error!(?err, "queued event execution failed");
            } else {
                debug!("queued event completed");
            }
        }
        Ok(())
    }
}

/// Default in-memory non-blocking [`EventQueue`] adapter backed by dedicated worker threads.
///
/// Submitted events are held in memory (not persisted across process restarts) and executed
/// concurrently against [`App`](crate::App) on background worker threads until the [`App`](crate::App)
/// is dropped.
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

    fn ensure_started(&self) -> Vec<UnboundedSender<Event>> {
        let mut guard = self.pool.lock().expect("event queue pool mutex poisoned");
        if let Some(pool) = guard.as_ref() {
            return pool.senders.clone();
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
        cloned
    }
}

impl Drop for MultithreadedEventQueue {
    fn drop(&mut self) {
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

fn worker_loop(app: WeakApp, mut rx: UnboundedReceiver<Event>, worker_index: usize) {
    debug!(worker_index, "multithreaded event queue worker started");
    futures::executor::block_on(async move {
        let mut in_flight = FuturesUnordered::new();

        loop {
            futures::select! {
                maybe_event = rx.next() => match maybe_event {
                    Some(event) => {
                        let Some(app_ref) = app.upgrade() else {
                            debug!(worker_index, "event queue worker stopping: app dropped");
                            break;
                        };
                        in_flight.push(Box::pin(async move {
                            if let Err(err) = handle_event(&app_ref, event).await {
                                error!(?err, "failed to dispatch event");
                            }
                        }));
                    }
                    None => break,
                },
                _ = in_flight.next() => {}
            }
        }

        while in_flight.next().await.is_some() {}
    });
    debug!(worker_index, "multithreaded event queue worker stopped");
}

#[async_trait]
impl EventQueue for MultithreadedEventQueue {
    async fn enqueue(&self, events: Vec<Event>) -> ResultReport<(), EventQueueError> {
        if events.is_empty() {
            return Ok(());
        }
        let senders = self.ensure_started();
        for event in events {
            let idx = self.next_worker.fetch_add(1, Ordering::Relaxed) % senders.len();
            if senders[idx].unbounded_send(event).is_err() {
                return Err(error_stack::Report::new(EventQueueError::Unavailable));
            }
        }
        Ok(())
    }
}
