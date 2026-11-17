//! Photo capture submission, image processing, and atomic Bingo win + Gold Medal award.

use async_trait::async_trait;
use error_stack::{IntoReport, Report, ResultExt};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    application::ExecutionContext,
    domain::{
        BoardTileId, Capture, CaptureId, GameId, GameStatus, NewCapture, evaluate_bingo,
        process_capture_photo,
    },
    ports::RepositoryError,
    service::UseCase,
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
    pub game_completed: bool,
    pub winning_line: Option<Vec<(u8, u8)>>,
}

#[async_trait]
impl UseCase<SubmitCaptureOutcome> for SubmitCapture {
    type Error = Report<CaptureUseCaseError>;

    #[instrument(skip_all, err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<SubmitCaptureOutcome, Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(CaptureUseCaseError::Unauthorized)?;
        let now = ctx.clock().now();

        let processed = process_capture_photo(&self.image_bytes)
            .change_context(CaptureUseCaseError::InvalidImage)?;

        let capture_uuid = Uuid::new_v4();
        let full_key = format!("capture_{capture_uuid}.jpg");
        let thumb_key = format!("thumb_{capture_uuid}.jpg");

        let uow = ctx
            .enter()
            .await
            .change_context(CaptureUseCaseError::Persistence)?;

        let result = async {
            let mut game = match uow.games().find_by_id(self.game_id).await {
                Ok(g) => g,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(CaptureUseCaseError::NotFound));
                }
                Err(err) => return Err(err).change_context(CaptureUseCaseError::Persistence),
            };

            if game.status() != GameStatus::Active {
                return Err(Report::new(CaptureUseCaseError::Conflict));
            }

            let mut board = match uow
                .boards()
                .find_board_by_game_and_owner(self.game_id, user_id)
                .await
            {
                Ok(b) => b,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(CaptureUseCaseError::Forbidden));
                }
                Err(err) => return Err(err).change_context(CaptureUseCaseError::Persistence),
            };

            let mut tiles = uow
                .boards()
                .find_tiles_by_board(board.id())
                .await
                .map_err(IntoReport::into_report)
                .change_context(CaptureUseCaseError::Persistence)?;

            let Some(tile) = tiles.iter_mut().find(|t| t.id() == self.tile_id) else {
                return Err(Report::new(CaptureUseCaseError::NotFound));
            };

            if tile.capture_id().is_some() {
                return Err(Report::new(CaptureUseCaseError::Conflict));
            }

            // Persist media blobs before committing DB rows.
            ctx.object_store()
                .put(&full_key, &processed.full_jpeg)
                .await
                .change_context(CaptureUseCaseError::Persistence)?;
            ctx.object_store()
                .put(&thumb_key, &processed.blurred_jpeg)
                .await
                .change_context(CaptureUseCaseError::Persistence)?;

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
            tile.attach_capture(capture_id);

            uow.boards()
                .attach_tile_capture(self.tile_id, capture_id)
                .await
                .map_err(IntoReport::into_report)
                .change_context(CaptureUseCaseError::Persistence)?;

            let winning_line = evaluate_bingo(game.grid_size(), &tiles);
            let game_completed = if winning_line.is_some() {
                board.mark_bingo();
                uow.boards()
                    .update_board(&mut board)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(CaptureUseCaseError::Persistence)?;

                game.complete(user_id, now)
                    .change_context(CaptureUseCaseError::Conflict)?;
                uow.games()
                    .update(&mut game)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(CaptureUseCaseError::Persistence)?;

                let mut winner_profile = uow
                    .profiles()
                    .find_by_user_id(user_id)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(CaptureUseCaseError::Persistence)?;
                winner_profile.award_gold_medal();
                uow.profiles()
                    .update(&mut winner_profile)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(CaptureUseCaseError::Persistence)?;
                true
            } else {
                false
            };

            Ok::<_, Report<CaptureUseCaseError>>(SubmitCaptureOutcome {
                capture: capture.into_data(),
                game_completed,
                winning_line,
            })
        }
        .await;

        uow.finish(result, CaptureUseCaseError::Persistence).await
    }
}
