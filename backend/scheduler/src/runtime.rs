use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use ::scheduler::{
    InMemoryStateStore, Scheduler, SchedulerConfig, SchedulerError, SchedulerHandle,
    SchedulerReport, StateStore,
};
use chrono_tz::Tz;
use friendarazzi_core::App;
use friendarazzi_core::domain::READY_GRACE_PERIOD_SECONDS;
use tracing::info;

use crate::jobs::run;
use crate::observer::TracingObserver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FriendarazziSchedulerConfig {
    pub timezone: Tz,
    pub grace_period: Duration,
}

impl Default for FriendarazziSchedulerConfig {
    fn default() -> Self {
        Self {
            timezone: chrono_tz::Europe::Vienna,
            grace_period: Duration::from_secs(READY_GRACE_PERIOD_SECONDS as u64),
        }
    }
}

pub struct FriendarazziScheduler<S: StateStore = InMemoryStateStore> {
    app: App,
    store: S,
}

impl FriendarazziScheduler {
    pub fn new(app: App) -> Self {
        Self {
            app,
            store: InMemoryStateStore::new(),
        }
    }
}

impl<S: StateStore> FriendarazziScheduler<S> {
    pub fn state_store<SS: StateStore>(self, store: SS) -> FriendarazziScheduler<SS> {
        FriendarazziScheduler {
            app: self.app,
            store,
        }
    }
}

pub type SchedulerFuture =
    Pin<Box<dyn Future<Output = Result<SchedulerReport, SchedulerError>> + Send>>;

impl<S: StateStore + Send + Sync + 'static> FriendarazziScheduler<S> {
    pub fn start_with_handle(
        self,
        config: FriendarazziSchedulerConfig,
    ) -> (SchedulerHandle, SchedulerFuture) {
        let scheduler_config = SchedulerConfig {
            timezone: config.timezone,
            ..SchedulerConfig::default()
        };
        let scheduler = Scheduler::with_observer(scheduler_config, self.store, TracingObserver);
        let handle = scheduler.handle();
        info!(
            timezone = %config.timezone,
            grace_period = ?config.grace_period,
            "starting friendarazzi scheduler"
        );
        (handle, run(scheduler, self.app, config.grace_period))
    }

    pub fn start(self, config: FriendarazziSchedulerConfig) -> SchedulerFuture {
        self.start_with_handle(config).1
    }
}
