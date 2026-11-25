use std::time::Duration;

use ::scheduler::{Job, Schedule, Task, TaskContext};
use friendarazzi_core::App;
use friendarazzi_core::application::AuthContext;
use friendarazzi_core::service::StartReadyGames;
use tracing::{debug, info};

pub const START_READY_GAMES_JOB_ID: &str = "start_ready_games";

pub fn build_start_ready_games_job(app: App, grace_period: Duration) -> Job<App> {
    Job::new(
        START_READY_GAMES_JOB_ID,
        Schedule::Interval(grace_period),
        app,
        Task::from_async(move |context: TaskContext<App>| async move {
            let app = &context.deps;
            let grace_chrono = chrono::Duration::from_std(grace_period).unwrap_or_default();
            let window_end = app.clock.now() + grace_chrono;

            loop {
                let now = app.clock.now();
                let outcome = app
                    .execute(AuthContext::Anonymous, StartReadyGames)
                    .await
                    .map_err(|err| err.to_string())?;

                if !outcome.started_games.is_empty() {
                    info!(
                        count = outcome.started_games.len(),
                        next_draft_closes_at = ?outcome.next_draft_closes_at,
                        "scheduled start_ready_games started games"
                    );
                }

                let Some(next_at) = outcome.next_draft_closes_at else {
                    break;
                };
                if next_at >= window_end {
                    break;
                }

                let sleep_for = (next_at - now).to_std().unwrap_or(Duration::ZERO);
                debug!(?sleep_for, %next_at, "sleeping until next draft closes");
                tokio::time::sleep(sleep_for).await;
                if app.clock.now() <= now && !sleep_for.is_zero() {
                    break;
                }
            }

            Ok(())
        }),
    )
}
