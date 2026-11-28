use friendarazzi_core::App;
use friendarazzi_core::ResultReport;
use friendarazzi_core::application::AuthContext;
use friendarazzi_core::service::{FinishExpiredGames, GameUseCaseError};
use tracing::info;

pub async fn run_finish_expired_games(app: &App) -> ResultReport<(), GameUseCaseError> {
    let outcome = app
        .execute(AuthContext::Anonymous, FinishExpiredGames)
        .await?;
    if !outcome.finished_games.is_empty() {
        info!(
            count = outcome.finished_games.len(),
            next_ends_at = ?outcome.next_ends_at,
            "scheduled finish_expired_games finalized expired active games"
        );
    }
    Ok(())
}
