use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::{debug, error};

use crate::ResultReport;
use crate::application::{App, AuthContext, EventQueueInit, WeakApp};
use crate::ports::{EventQueue, EventQueueError};
use crate::service::Event;

const DEFAULT_WORKER_COUNT: usize = 2;

struct ScheduledEntry {
    seq: u64,
    run_at: DateTime<Utc>,
    key: Option<String>,
    event: Event,
}

async fn execute_event(app: &App, event: Event) {
    let kind = event.kind();
    debug!(kind, "executing queued event");
    match app.execute(AuthContext::Anonymous, event).await {
        Ok(()) => debug!(kind, "queued event completed"),
        Err(err) => error!(kind, ?err, "queued event execution failed"),
    }
}

fn remove_by_key(entries: &mut Vec<ScheduledEntry>, key: &str) -> usize {
    let before = entries.len();
    entries.retain(|entry| entry.key.as_deref() != Some(key));
    before - entries.len()
}

fn pop_next_due(entries: &mut Vec<ScheduledEntry>, now: DateTime<Utc>) -> Option<ScheduledEntry> {
    let idx = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.run_at <= now)
        .min_by_key(|(_, entry)| (entry.run_at, entry.seq))
        .map(|(idx, _)| idx)?;
    Some(entries.remove(idx))
}

/// In-memory blocking [`EventQueue`] adapter that executes due events inline on [`App`].
///
/// Immediate events (and scheduled events whose `run_at <= clock.now()`) run synchronously
/// before `enqueue` / `schedule` returns. Future-scheduled events are retained in memory and
/// can be driven deterministically via [`BlockingEventQueue::run_due`].
pub struct BlockingEventQueue {
    app: WeakApp,
    state: Mutex<BlockingState>,
}

struct BlockingState {
    entries: Vec<ScheduledEntry>,
    next_seq: u64,
}

impl BlockingEventQueue {
    pub fn new(app: WeakApp) -> Self {
        Self {
            app,
            state: Mutex::new(BlockingState {
                entries: Vec::new(),
                next_seq: 0,
            }),
        }
    }

    pub fn adapter() -> EventQueueInit {
        EventQueueInit::from(|app: WeakApp| Arc::new(Self::new(app)) as Arc<dyn EventQueue>)
    }

    pub fn pending_len(&self) -> usize {
        self.state
            .lock()
            .expect("blocking queue mutex poisoned")
            .entries
            .len()
    }

    /// Execute all pending scheduled events whose `run_at <= app.clock().now()`.
    pub async fn run_due(&self) -> usize {
        let Some(app) = self.app.upgrade() else {
            return 0;
        };
        let mut executed = 0;
        loop {
            let now = app.clock.now();
            let next = {
                let mut state = self.state.lock().expect("blocking queue mutex poisoned");
                pop_next_due(&mut state.entries, now)
            };
            let Some(entry) = next else {
                break;
            };
            execute_event(&app, entry.event).await;
            executed += 1;
        }
        executed
    }
}

#[async_trait]
impl EventQueue for BlockingEventQueue {
    async fn enqueue(&self, event: Event) -> ResultReport<(), EventQueueError> {
        let Some(app) = self.app.upgrade() else {
            return Err(error_stack::Report::new(EventQueueError::Unavailable));
        };
        execute_event(&app, event).await;
        Ok(())
    }

    async fn schedule(
        &self,
        event: Event,
        run_at: DateTime<Utc>,
        key: Option<&str>,
    ) -> ResultReport<(), EventQueueError> {
        let Some(app) = self.app.upgrade() else {
            return Err(error_stack::Report::new(EventQueueError::Unavailable));
        };
        let now = app.clock.now();
        {
            let mut state = self.state.lock().expect("blocking queue mutex poisoned");
            if let Some(k) = key {
                remove_by_key(&mut state.entries, k);
            }
            if run_at > now {
                let seq = state.next_seq;
                state.next_seq += 1;
                state.entries.push(ScheduledEntry {
                    seq,
                    run_at,
                    key: key.map(str::to_owned),
                    event,
                });
                debug!(%run_at, key, "scheduled future event in blocking queue");
                return Ok(());
            }
        }
        execute_event(&app, event).await;
        Ok(())
    }

    async fn cancel(&self, key: &str) -> ResultReport<(), EventQueueError> {
        let mut state = self.state.lock().expect("blocking queue mutex poisoned");
        let removed = remove_by_key(&mut state.entries, key);
        debug!(key, removed, "cancelled event in blocking queue");
        Ok(())
    }
}

struct QueueState {
    entries: Vec<ScheduledEntry>,
    next_seq: u64,
    shutdown: bool,
    active_workers: usize,
}

struct MultithreadedInner {
    app: WeakApp,
    state: Mutex<QueueState>,
    condvar: Condvar,
    idle_condvar: Condvar,
}

/// Default in-memory non-blocking [`EventQueue`] adapter backed by dedicated worker threads.
///
/// Enqueued and scheduled events are held in memory (not persisted across process restarts)
/// and executed asynchronously against [`App`] on background worker threads.
pub struct MultithreadedEventQueue {
    inner: Arc<MultithreadedInner>,
    workers: Mutex<Vec<JoinHandle<()>>>,
    worker_count: usize,
}

impl MultithreadedEventQueue {
    pub fn new(app: WeakApp) -> Self {
        Self::with_workers(app, DEFAULT_WORKER_COUNT)
    }

    pub fn with_workers(app: WeakApp, worker_count: usize) -> Self {
        let worker_count = worker_count.max(1);
        Self {
            inner: Arc::new(MultithreadedInner {
                app,
                state: Mutex::new(QueueState {
                    entries: Vec::new(),
                    next_seq: 0,
                    shutdown: false,
                    active_workers: 0,
                }),
                condvar: Condvar::new(),
                idle_condvar: Condvar::new(),
            }),
            workers: Mutex::new(Vec::new()),
            worker_count,
        }
    }

    pub fn adapter(worker_count: usize) -> EventQueueInit {
        EventQueueInit::from(move |app: WeakApp| {
            Arc::new(Self::with_workers(app, worker_count)) as Arc<dyn EventQueue>
        })
    }

    fn ensure_started(&self) {
        let mut workers = self.workers.lock().expect("workers mutex poisoned");
        if !workers.is_empty() {
            return;
        }
        let state = self
            .inner
            .state
            .lock()
            .expect("multithreaded queue mutex poisoned");
        if state.shutdown {
            return;
        }
        drop(state);

        for idx in 0..self.worker_count {
            let inner = Arc::clone(&self.inner);
            let handle = thread::Builder::new()
                .name(format!("friendarazzi-event-worker-{idx}"))
                .spawn(move || worker_loop(inner, idx))
                .expect("failed to spawn event queue worker thread");
            workers.push(handle);
        }
    }

    /// Wake sleeping workers so they re-evaluate due times (useful with a fake clock in tests).
    pub fn notify(&self) {
        self.inner.condvar.notify_all();
    }

    pub fn pending_len(&self) -> usize {
        self.inner
            .state
            .lock()
            .expect("multithreaded queue mutex poisoned")
            .entries
            .len()
    }

    /// Block the current thread until no due events remain and all workers are idle.
    pub fn wait_due_idle(&self) {
        let Some(app) = self.inner.app.upgrade() else {
            return;
        };
        let mut state = self
            .inner
            .state
            .lock()
            .expect("multithreaded queue mutex poisoned");
        loop {
            let now = app.clock.now();
            let has_due = state.entries.iter().any(|e| e.run_at <= now);
            if (!has_due && state.active_workers == 0) || state.shutdown {
                break;
            }
            state = self
                .inner
                .idle_condvar
                .wait(state)
                .expect("idle condvar wait failed");
        }
    }

    pub fn shutdown(&self) {
        {
            let mut state = self
                .inner
                .state
                .lock()
                .expect("multithreaded queue mutex poisoned");
            if !state.shutdown {
                debug!("shutting down multithreaded event queue");
                state.shutdown = true;
            }
            self.inner.condvar.notify_all();
            self.inner.idle_condvar.notify_all();
        }
        let handles: Vec<JoinHandle<()>> = {
            let mut workers = self.workers.lock().expect("workers mutex poisoned");
            std::mem::take(&mut *workers)
        };
        let current_thread = thread::current().id();
        for handle in handles {
            // Avoid self-join deadlock if the last `App` clone is dropped inside a worker thread.
            if handle.thread().id() != current_thread {
                let _ = handle.join();
            }
        }
    }
}

impl Drop for MultithreadedEventQueue {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn worker_loop(inner: Arc<MultithreadedInner>, worker_index: usize) {
    debug!(worker_index, "multithreaded event queue worker started");
    let mut state = inner
        .state
        .lock()
        .expect("multithreaded queue mutex poisoned");

    loop {
        if state.shutdown {
            break;
        }
        let Some(app) = inner.app.upgrade() else {
            break;
        };

        let now = app.clock.now();
        if let Some(entry) = pop_next_due(&mut state.entries, now) {
            state.active_workers += 1;
            drop(state);

            futures::executor::block_on(execute_event(&app, entry.event));
            drop(app);

            state = inner
                .state
                .lock()
                .expect("multithreaded queue mutex poisoned");
            state.active_workers -= 1;
            inner.idle_condvar.notify_all();
            continue;
        }
        drop(app);

        let next_run_at = state.entries.iter().map(|e| e.run_at).min();
        state = match next_run_at {
            Some(run_at) => {
                let wait = (run_at - now).to_std().unwrap_or(Duration::ZERO);
                if wait.is_zero() {
                    continue;
                }
                let (guard, _) = inner
                    .condvar
                    .wait_timeout(state, wait)
                    .expect("condvar wait_timeout failed");
                guard
            }
            None => inner.condvar.wait(state).expect("condvar wait failed"),
        };
    }

    debug!(worker_index, "multithreaded event queue worker stopped");
}

#[async_trait]
impl EventQueue for MultithreadedEventQueue {
    async fn enqueue(&self, event: Event) -> ResultReport<(), EventQueueError> {
        let Some(app) = self.inner.app.upgrade() else {
            return Err(error_stack::Report::new(EventQueueError::Unavailable));
        };
        let now = app.clock.now();
        {
            let mut state = self
                .inner
                .state
                .lock()
                .expect("multithreaded queue mutex poisoned");
            if state.shutdown {
                return Err(error_stack::Report::new(EventQueueError::Unavailable));
            }
            let seq = state.next_seq;
            state.next_seq += 1;
            state.entries.push(ScheduledEntry {
                seq,
                run_at: now,
                key: None,
                event,
            });
        }
        self.ensure_started();
        self.inner.condvar.notify_one();
        Ok(())
    }

    async fn schedule(
        &self,
        event: Event,
        run_at: DateTime<Utc>,
        key: Option<&str>,
    ) -> ResultReport<(), EventQueueError> {
        {
            let mut state = self
                .inner
                .state
                .lock()
                .expect("multithreaded queue mutex poisoned");
            if state.shutdown {
                return Err(error_stack::Report::new(EventQueueError::Unavailable));
            }
            if let Some(k) = key {
                remove_by_key(&mut state.entries, k);
            }
            let seq = state.next_seq;
            state.next_seq += 1;
            state.entries.push(ScheduledEntry {
                seq,
                run_at,
                key: key.map(str::to_owned),
                event,
            });
        }
        self.ensure_started();
        self.inner.condvar.notify_all();
        Ok(())
    }

    async fn cancel(&self, key: &str) -> ResultReport<(), EventQueueError> {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("multithreaded queue mutex poisoned");
        let removed = remove_by_key(&mut state.entries, key);
        debug!(key, removed, "cancelled event in multithreaded queue");
        self.inner.idle_condvar.notify_all();
        Ok(())
    }
}
