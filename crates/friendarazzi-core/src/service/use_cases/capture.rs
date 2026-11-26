//! Photo capture submission, image processing, and atomic Bingo win + Gold Medal award.

use async_trait::async_trait;
use error_stack::{IntoReport, Report, ResultExt};
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::{
    application::{ExecutionContext, uow::Versioned},
    domain::{BoardCoordinates, BoardTileId, Capture, CaptureId, Game, GameId, NewCapture},
    ports::RepositoryError,
    service::{CaptureUploaded, GameFinished, UseCase},
};

#[derive(Debug, thiserror::Error)]
pub enum CaptureUseCaseError {
    #[error("für diese Aktion ist eine Anmeldung erforderlich")]
    Unauthorized,
    #[error("du darfst für diese Kachel kein Foto hochladen")]
    Forbidden,
    #[error("Spiel oder Kachel wurde nicht gefunden")]
    NotFound,
    #[error("ungültige Bilddatei")]
    InvalidImage,
    #[error("das Spiel ist nicht aktiv oder diese Kachel wurde bereits erfasst")]
    Conflict,
    #[error("Foto konnte nicht gespeichert werden")]
    Persistence,
}

#[derive(Debug, Clone)]
pub struct SubmitCapture {
    pub game_id: GameId,
    pub tile_id: BoardTileId,
    pub image_bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SubmitCaptureOutcome {
    pub capture: Capture,
    pub winning_line: Option<Vec<BoardCoordinates>>,
}

#[async_trait]
impl UseCase<SubmitCaptureOutcome> for SubmitCapture {
    type Error = Report<CaptureUseCaseError>;

    #[instrument(skip_all, fields(game_id = ?self.game_id, tile_id = ?self.tile_id), err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<SubmitCaptureOutcome, Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(CaptureUseCaseError::Unauthorized)?;
        let now = ctx.clock().now();

        let processed = Capture::process_photo(&self.image_bytes)
            .change_context(CaptureUseCaseError::InvalidImage)?;

        let capture_uuid = Uuid::new_v4();
        let full_key = format!("capture_{capture_uuid}.webp");
        let thumb_key = format!("thumb_{capture_uuid}.webp");

        let uow = ctx
            .enter()
            .await
            .change_context(CaptureUseCaseError::Persistence)?;

        let result = async {
            let game = match uow.games().find_by_id(self.game_id).await {
                Ok(g) => g,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(CaptureUseCaseError::NotFound));
                }
                Err(err) => return Err(err).change_context(CaptureUseCaseError::Persistence),
            };

            let game_version = game.v();
            let Game::Active(active_game) = game.into_data() else {
                return Err(Report::new(CaptureUseCaseError::Conflict));
            };

            let mut board = match uow
                .boards()
                .find_by_game_and_owner(self.game_id, user_id)
                .await
            {
                Ok(b) => b,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(CaptureUseCaseError::Forbidden));
                }
                Err(err) => return Err(err).change_context(CaptureUseCaseError::Persistence),
            };

            let Some(tile) = board.tile_by_id(self.tile_id).cloned() else {
                return Err(Report::new(CaptureUseCaseError::NotFound));
            };

            if tile.capture_id().is_some() {
                return Err(Report::new(CaptureUseCaseError::Conflict));
            }

            let capture = uow
                .captures()
                .create(NewCapture {
                    game_id: self.game_id,
                    tile_id: self.tile_id,
                    catcher_user_id: user_id,
                    target_user_id: tile.target_user_id(),
                    prompt_text: tile.prompt_text().clone(),
                    image_storage_key: full_key.clone(),
                    thumbnail_storage_key: thumb_key.clone(),
                    captured_at: now,
                })
                .await
                .map_err(IntoReport::into_report)
                .change_context(CaptureUseCaseError::Persistence)?;

            let capture_id: CaptureId = capture.id();
            let winning_line = board
                .record_capture(self.tile_id, capture_id)
                .change_context(CaptureUseCaseError::Conflict)?;

            uow.boards()
                .update(&mut board)
                .await
                .map_err(IntoReport::into_report)
                .change_context(CaptureUseCaseError::Persistence)?;

            if winning_line.is_some() {
                let completed = active_game
                    .complete(&board, now)
                    .change_context(CaptureUseCaseError::Conflict)?;
                let mut updated_game = Versioned::new(game_version, Game::Completed(completed));
                uow.games()
                    .update(&mut updated_game)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(CaptureUseCaseError::Persistence)?;

                let mut winner = uow
                    .benutzer()
                    .find_by_id(user_id)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(CaptureUseCaseError::Persistence)?;
                winner.award_gold_medal();
                uow.benutzer()
                    .update(&mut winner)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(CaptureUseCaseError::Persistence)?;
            }

            // Upload media right before committing so a rollback only needs to clean up on commit failure.
            ctx.object_store()
                .put(&full_key, &processed.full_webp)
                .await
                .change_context(CaptureUseCaseError::Persistence)?;
            ctx.object_store()
                .put(&thumb_key, &processed.blurred_webp)
                .await
                .change_context(CaptureUseCaseError::Persistence)?;

            ctx.events().submit(CaptureUploaded {
                game_id: self.game_id,
                capture_id,
                uploader_id: user_id,
                target_id: tile.target_user_id(),
                occurred_at: now,
            });
            if winning_line.is_some() {
                ctx.events().submit(GameFinished {
                    game_id: self.game_id,
                    winner_id: user_id,
                    finished_at: now,
                });
            }

            Ok::<_, Report<CaptureUseCaseError>>(SubmitCaptureOutcome {
                capture: capture.into_data(),
                winning_line,
            })
        }
        .await;

        let committed = uow.finish(result, CaptureUseCaseError::Persistence).await;
        match committed {
            Ok(outcome) => Ok(outcome),
            Err(err) => {
                for key in [&full_key, &thumb_key] {
                    if let Err(cleanup_err) = ctx.object_store().ensure_deleted(key).await {
                        warn!(%key, fehler = %cleanup_err, "failed to roll back uploaded capture media");
                    }
                }
                Err(err)
            }
        }
    }
}
