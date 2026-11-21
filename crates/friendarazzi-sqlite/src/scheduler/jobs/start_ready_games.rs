use std::sync::{Arc, Mutex};
use std::time::Duration;

use ::scheduler::{Job, Schedule, Task, TaskContext};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use friendarazzi_core::App;
use friendarazzi_core::application::AuthContext;
use friendarazzi_core::service::StartReadyGames;
use tracing::{debug, error, info};

pub const START_READY_GAMES_JOB_ID: &str = "start_ready_games";

#[derive(Clone)]
pub struct StartReadyGamesDeps {
    pub app: App,
    pub next_auto_start_at: Arc<Mutex<Option<DateTime<Utc>>>>,
}

/// Compute how long to wait until the next check: `next_game_start_at - now` when a
/// drafting countdown is already pending, capped by `grace_period` so newly-locked
/// games are picked up by the end of their grace period.
pub fn next_check_delay(
    now: DateTime<Utc>,
    next_auto_start_at: Option<DateTime<Utc>>,
    grace_period: Duration,
) -> Duration {
    match next_auto_start_at {
        Some(next_at) if next_at > now => (next_at - now)
            .to_std()
            .unwrap_or(Duration::ZERO)
            .min(grace_period),
        Some(_) => Duration::ZERO,
        None => grace_period,
    }
}

pub fn build_start_ready_games_job(
    app: App,
    run_at: DateTime<Tz>,
    next_auto_start_at: Arc<Mutex<Option<DateTime<Utc>>>>,
) -> Job<StartReadyGamesDeps> {
    let deps = StartReadyGamesDeps {
        app,
        next_auto_start_at,
    };
    Job::new(
        START_READY_GAMES_JOB_ID,
        Schedule::AtTimes(vec![run_at]),
        deps,
        Task::from_async(|context: TaskContext<StartReadyGamesDeps>| async move {
            debug!("scheduled start_ready_games check started");
            match context
                .deps
                .app
                .execute(AuthContext::Anonymous, StartReadyGames)
                .await
            {
                Ok(outcome) => {
                    *context
                        .deps
                        .next_auto_start_at
                        .lock()
                        .expect("next_auto_start_at mutex poisoned") = outcome.next_auto_start_at;
                    if !outcome.started_games.is_empty() {
                        info!(
                            count = outcome.started_games.len(),
                            next_auto_start_at = ?outcome.next_auto_start_at,
                            "scheduled start_ready_games started games"
                        );
                    }
                    Ok(())
                }
                Err(err) => {
                    error!(?err, "scheduled start_ready_games failed");
                    Err(err.to_string())
                }
            }
        }),
    )
}
