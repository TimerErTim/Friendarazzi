use std::sync::Mutex;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use friendarazzi_core::ports::Clock;
use futures::channel::oneshot;

pub struct FixedClock {
    current_time: Mutex<DateTime<Utc>>,
    waiters: Mutex<Vec<(DateTime<Utc>, oneshot::Sender<()>)>>,
}

impl FixedClock {
    pub fn new(current_time: DateTime<Utc>) -> Self {
        Self {
            current_time: Mutex::new(current_time),
            waiters: Mutex::new(Vec::new()),
        }
    }

    pub fn advance_by(&self, duration: Duration) {
        let ready = {
            let mut current_time = self.current_time.lock().unwrap();
            *current_time += duration;
            Self::drain_ready_waiters(&self.waiters, *current_time)
        };
        for tx in ready {
            let _ = tx.send(());
        }
    }

    pub fn set_time(&mut self, time: DateTime<Utc>) {
        let ready = {
            let mut current_time = self.current_time.lock().unwrap();
            *current_time = time;
            Self::drain_ready_waiters(&self.waiters, *current_time)
        };
        for tx in ready {
            let _ = tx.send(());
        }
    }

    fn drain_ready_waiters(
        waiters: &Mutex<Vec<(DateTime<Utc>, oneshot::Sender<()>)>>,
        now: DateTime<Utc>,
    ) -> Vec<oneshot::Sender<()>> {
        let mut guard = waiters.lock().unwrap();
        let mut ready = Vec::new();
        let mut remaining = Vec::with_capacity(guard.len());
        for (deadline, tx) in guard.drain(..) {
            if deadline <= now {
                ready.push(tx);
            } else {
                remaining.push((deadline, tx));
            }
        }
        *guard = remaining;
        ready
    }
}

#[async_trait::async_trait]
impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        *self.current_time.lock().unwrap()
    }

    fn today(&self) -> NaiveDate {
        self.current_time.lock().unwrap().date_naive()
    }

    async fn sleep_until(&self, deadline: DateTime<Utc>) {
        let rx = {
            let current_time = self.current_time.lock().unwrap();
            if deadline <= *current_time {
                return;
            }
            let (tx, rx) = oneshot::channel();
            self.waiters.lock().unwrap().push((deadline, tx));
            rx
        };
        let _ = rx.await;
    }
}
