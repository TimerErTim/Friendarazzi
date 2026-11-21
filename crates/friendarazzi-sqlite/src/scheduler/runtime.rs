use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ::scheduler::{
    InMemoryStateStore, Scheduler, SchedulerConfig, SchedulerError, SchedulerReport, StateStore,
    TerminalStatePolicy,
};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use friendarazzi_core::App;
use friendarazzi_core::domain::READY_GRACE_PERIOD_SECONDS;
use tracing::{debug, info};

use crate::scheduler::jobs::{build_start_ready_games_job, next_check_delay};
use crate::scheduler::observer::TracingObserver;

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
    pub fn state_store<SS: StateStore + Clone>(self, store: SS) -> FriendarazziScheduler<SS> {
        FriendarazziScheduler {
            app: self.app,
            store,
        }
    }
}

impl<S: StateStore + Clone + Send + Sync + 'static> FriendarazziScheduler<S> {
    pub fn run_until(
        self,
        config: FriendarazziSchedulerConfig,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Pin<Box<dyn Future<Output = Result<SchedulerReport, SchedulerError>> + Send>> {
        Box::pin(async move {
            info!(
                timezone = %config.timezone,
                grace_period = ?config.grace_period,
                "starting friendarazzi scheduler"
            );

            let scheduler_config = SchedulerConfig {
                timezone: config.timezone,
                terminal_state_policy: TerminalStatePolicy::Delete,
                ..SchedulerConfig::default()
            };

            let next_auto_start_at: Arc<Mutex<Option<DateTime<Utc>>>> = Arc::new(Mutex::new(None));
            let mut next_run_utc = Utc::now();
            let mut last_report: Option<SchedulerReport>;

            tokio::pin!(shutdown);
            let mut shutdown_triggered = false;

            loop {
                let scheduler = Scheduler::with_observer(
                    scheduler_config.clone(),
                    self.store.clone(),
                    TracingObserver,
                );
                let handle = scheduler.handle();
                let run_at_tz = next_run_utc.with_timezone(&config.timezone);
                let job = build_start_ready_games_job(
                    self.app.clone(),
                    run_at_tz,
                    Arc::clone(&next_auto_start_at),
                );
                let run_fut = scheduler.run(job);
                tokio::pin!(run_fut);

                let cycle_report = tokio::select! {
                    biased;
                    () = &mut shutdown, if !shutdown_triggered => {
                        shutdown_triggered = true;
                        info!("friendarazzi scheduler shutdown signal received");
                        handle.shutdown();
                        run_fut.await?
                    }
                    res = &mut run_fut => res?,
                };
                last_report = Some(cycle_report);

                if shutdown_triggered {
                    break;
                }

                let now = self.app.clock.now();
                let next_deadline = *next_auto_start_at
                    .lock()
                    .expect("next_auto_start_at mutex poisoned");
                let delay = next_check_delay(now, next_deadline, config.grace_period);
                let chrono_delay = chrono::Duration::from_std(delay).unwrap_or_default();
                next_run_utc = Utc::now() + chrono_delay;
                debug!(
                    ?delay,
                    next_run_at = %next_run_utc,
                    ?next_deadline,
                    "friendarazzi scheduler computed next check"
                );
            }

            info!("friendarazzi scheduler stopped gracefully");
            Ok(last_report.expect("at least one scheduler cycle ran or stopped"))
        })
    }
}
