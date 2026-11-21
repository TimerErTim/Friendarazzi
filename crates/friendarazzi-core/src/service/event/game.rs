use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use tracing::{debug, info};

use crate::{
    application::ExecutionContext,
    domain::{DraftStartOutcome, Game, GameId},
    ports::RepositoryError,
    service::{EventHandler, GameUseCaseError, map_game_error, persist_started_game},
};

/// Queued background event that transitions a locked game from `Drafting` to `Active`
/// once its ready countdown (`scheduled_for`) expires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelayedGameStart {
    pub game_id: GameId,
    pub scheduled_for: DateTime<Utc>,
}

#[async_trait]
impl EventHandler for DelayedGameStart {
    type Error = Report<GameUseCaseError>;

    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        let now = ctx.clock().now();
        let uow = ctx
            .enter()
            .await
            .change_context(GameUseCaseError::Persistence)?;

        let result = async {
            let game = match uow.games().find_by_id(self.game_id).await {
                Ok(g) => g,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    debug!(game_id = ?self.game_id, "delayed game start skipped: game not found");
                    return Ok(());
                }
                Err(err) => return Err(err).change_context(GameUseCaseError::Persistence),
            };

            let version = game.v();
            let Game::Drafting(drafting) = game.into_data() else {
                debug!(game_id = ?self.game_id, "delayed game start skipped: game no longer drafting");
                return Ok(());
            };

            // Ignore stale events whose countdown was cancelled or superseded by a re-lock.
            if drafting.auto_start_at() != Some(self.scheduled_for) {
                debug!(
                    game_id = ?self.game_id,
                    expected = %self.scheduled_for,
                    actual = ?drafting.auto_start_at(),
                    "delayed game start skipped: countdown changed"
                );
                return Ok(());
            }

            let outcome = {
                let mut rng = ctx.rng();
                drafting.finalize_ready_start(now, &mut *rng).map_err(|e| {
                    let mapped = map_game_error(e.current_context());
                    e.change_context(mapped)
                })?
            };

            match outcome {
                DraftStartOutcome::StillDrafting(_) => {
                    debug!(game_id = ?self.game_id, %now, "delayed game start skipped: not yet due");
                    Ok(())
                }
                DraftStartOutcome::Started {
                    game: active,
                    boards,
                } => match persist_started_game(&uow, version, active, boards).await {
                    Ok(_) => {
                        info!(game_id = ?self.game_id, "delayed game start finalized");
                        Ok(())
                    }
                    Err(err) if matches!(err.current_context(), GameUseCaseError::Conflict) => {
                        debug!(
                            game_id = ?self.game_id,
                            "delayed game start lost optimistic lock race; ignoring"
                        );
                        Ok(())
                    }
                    Err(err) => Err(err),
                },
            }
        }
        .await;

        uow.finish(result, GameUseCaseError::Persistence).await
    }
}
