mod finish_expired_games;
mod start_ready_games;

use std::future::Future;
use std::pin::Pin;

use ::scheduler::{
    Job, Schedule, Scheduler, SchedulerError, SchedulerReport, StateStore, Task, TaskContext,
};
use friendarazzi_core::App;
use friendarazzi_core::domain::READY_GRACE_PERIOD_SECONDS;

pub use finish_expired_games::run_finish_expired_games;
pub use start_ready_games::{START_READY_GAMES_JOB_ID, run_start_ready_games};

pub fn run<S: StateStore + Send + Sync + 'static>(
    scheduler: Scheduler<S>,
    app: App,
) -> Pin<Box<dyn Future<Output = Result<SchedulerReport, SchedulerError>> + Send>> {
    let job = Job::new(
        START_READY_GAMES_JOB_ID,
        Schedule::Interval(std::time::Duration::from_secs(
            READY_GRACE_PERIOD_SECONDS as u64,
        )),
        app,
        Task::from_async(|context: TaskContext<App>| async move {
            tokio::try_join!(
                run_start_ready_games(&context.deps),
                run_finish_expired_games(&context.deps),
            )
            .map(|_| ())
            .map_err(|err| err.to_string())
        }),
    );
    Box::pin(async move { scheduler.run(job).await })
}
