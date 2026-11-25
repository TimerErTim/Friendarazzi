mod start_ready_games;

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use ::scheduler::{Scheduler, SchedulerError, SchedulerReport, StateStore};
use friendarazzi_core::App;

pub use start_ready_games::{START_READY_GAMES_JOB_ID, build_start_ready_games_job};

pub fn run<S: StateStore + Send + Sync + 'static>(
    scheduler: Scheduler<S>,
    app: App,
    grace_period: Duration,
) -> Pin<Box<dyn Future<Output = Result<SchedulerReport, SchedulerError>> + Send>> {
    let job = build_start_ready_games_job(app, grace_period);
    Box::pin(async move { scheduler.run(job).await })
}
