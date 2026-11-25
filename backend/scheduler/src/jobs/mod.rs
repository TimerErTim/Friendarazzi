mod start_ready_games;

use std::future::Future;
use std::pin::Pin;

use ::scheduler::{Scheduler, SchedulerError, SchedulerReport, StateStore};
use friendarazzi_core::App;

pub use start_ready_games::{
    START_READY_GAMES_JOB_ID, build_start_ready_games_job, run_start_ready_games,
};

pub fn run<S: StateStore + Send + Sync + 'static>(
    scheduler: Scheduler<S>,
    app: App,
) -> Pin<Box<dyn Future<Output = Result<SchedulerReport, SchedulerError>> + Send>> {
    let job = build_start_ready_games_job(app);
    Box::pin(async move { scheduler.run(job).await })
}
