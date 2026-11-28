use std::time::Duration;

use friendarazzi_core::App;
use friendarazzi_core::ResultReport;
use friendarazzi_core::application::AuthContext;
use friendarazzi_core::domain::READY_GRACE_PERIOD_SECONDS;
use friendarazzi_core::service::{GameUseCaseError, StartReadyGames};
use tracing::{debug, info};

pub const START_READY_GAMES_JOB_ID: &str = "start_ready_games";

/// Execute one scheduled tick of `StartReadyGames`, sleeping in-task when another locked
/// drafting game will close before the next grace-period tick.
pub async fn run_start_ready_games(app: &App) -> ResultReport<(), GameUseCaseError> {
    let grace_chrono = chrono::Duration::seconds(READY_GRACE_PERIOD_SECONDS);
    let window_end = app.clock.now() + grace_chrono;

    loop {
        let now = app.clock.now();
        let outcome = app.execute(AuthContext::Anonymous, StartReadyGames).await?;

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
        // Intentional tradeoff: sleep ignores shutdown, so shutdown hits the 250ms abort timeout during countdowns.
        tokio::time::sleep(sleep_for).await;
        if app.clock.now() <= now && !sleep_for.is_zero() {
            break;
        }
    }

    Ok(())
}
