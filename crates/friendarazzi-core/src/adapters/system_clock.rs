use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::Instant;

use chrono::{DateTime, NaiveDate, Utc};
use futures::channel::oneshot;
use tracing::trace;

use crate::ports::Clock;

struct TimerEntry {
    deadline: Instant,
    seq: u64,
    tx: oneshot::Sender<()>,
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline && self.seq == other.seq
    }
}

impl Eq for TimerEntry {}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .deadline
            .cmp(&self.deadline)
            .then_with(|| other.seq.cmp(&self.seq))
    }
}

struct TimerState {
    heap: BinaryHeap<TimerEntry>,
    next_seq: u64,
}

struct SystemTimer {
    state: Mutex<TimerState>,
    condvar: Condvar,
}

impl SystemTimer {
    fn global() -> &'static Self {
        static TIMER: OnceLock<SystemTimer> = OnceLock::new();
        TIMER.get_or_init(|| {
            let timer = Self {
                state: Mutex::new(TimerState {
                    heap: BinaryHeap::new(),
                    next_seq: 0,
                }),
                condvar: Condvar::new(),
            };
            std::thread::Builder::new()
                .name("friendarazzi-system-clock-timer".to_owned())
                .spawn(|| Self::run_loop())
                .expect("failed to spawn system clock timer thread");
            timer
        })
    }

    fn run_loop() -> ! {
        let timer = Self::global();
        let mut state = timer.state.lock().expect("timer mutex poisoned");
        loop {
            let now = Instant::now();
            while let Some(top) = state.heap.peek() {
                if top.deadline <= now {
                    let entry = state.heap.pop().expect("peeked entry exists");
                    let _ = entry.tx.send(());
                } else {
                    break;
                }
            }

            state = match state.heap.peek() {
                Some(top) => {
                    let wait = top.deadline.saturating_duration_since(now);
                    let (guard, _) = timer
                        .condvar
                        .wait_timeout(state, wait)
                        .expect("timer condvar wait failed");
                    guard
                }
                None => timer
                    .condvar
                    .wait(state)
                    .expect("timer condvar wait failed"),
            };
        }
    }

    fn sleep_for(&self, duration: std::time::Duration) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        let deadline = Instant::now() + duration;
        let mut state = self.state.lock().expect("timer mutex poisoned");
        let seq = state.next_seq;
        state.next_seq += 1;
        state.heap.push(TimerEntry { deadline, seq, tx });
        self.condvar.notify_one();
        rx
    }
}

pub struct SystemClock;

#[async_trait::async_trait]
impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        let now = Utc::now();
        trace!(%now, "system clock now");
        now
    }

    fn today(&self) -> NaiveDate {
        let today = Utc::now().date_naive();
        trace!(%today, "system clock today");
        today
    }

    async fn sleep_until(&self, deadline: DateTime<Utc>) {
        let Ok(duration) = (deadline - Utc::now()).to_std() else {
            return;
        };
        if duration.is_zero() {
            return;
        }
        trace!(%deadline, ?duration, "system clock sleeping");
        let _ = SystemTimer::global().sleep_for(duration).await;
    }
}
