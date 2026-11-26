//! Photo capture submission, image processing, and atomic Bingo win + Gold Medal award.

use std::collections::HashMap;

use async_trait::async_trait;
use error_stack::{IntoReport, Report, ResultExt};
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::{
    application::{ExecutionContext, uow::UnitOfWork, uow::Versioned},
    domain::{
        AbbestellToken, ActiveGame, BenachrichtigungArt, Benutzer, BenutzerId, BoardCoordinates,
        BoardTileId, Capture, CaptureId, Game, GameId, NeueBenachrichtigung, NewCapture,
        VisibilityMode,
    },
    ports::{Mail, RepositoryError},
    service::{
        CaptureUploaded, UseCase, use_cases::benachrichtigung::create_notification_idempotent,
    },
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

            let mut winner_opt = None;
            if winning_line.is_some() {
                let completed = active_game
                    .clone()
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
                winner_opt = Some(winner.into_data());
            }

            let mails = create_capture_and_completion_notifications(
                &uow,
                &active_game,
                capture.id(),
                user_id,
                tile.target_user_id(),
                tile.prompt_text().as_str(),
                winning_line.is_some(),
                winner_opt.as_ref(),
                now,
                ctx.signing(),
            )
            .await?;

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
            });

            Ok::<_, Report<CaptureUseCaseError>>((
                SubmitCaptureOutcome {
                    capture: capture.into_data(),
                    winning_line,
                },
                mails,
            ))
        }
        .await;

        let committed = uow.finish(result, CaptureUseCaseError::Persistence).await;
        match committed {
            Ok((outcome, mails)) => {
                for mail in mails {
                    if let Err(err) = ctx.mail_sender().send(mail).await {
                        warn!(fehler = %err, "Benachrichtigungsmail konnte nicht zugestellt werden");
                    }
                }
                Ok(outcome)
            }
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

#[allow(clippy::too_many_arguments)]
async fn create_capture_and_completion_notifications(
    uow: &UnitOfWork<'_>,
    game: &ActiveGame,
    capture_id: CaptureId,
    catcher_id: BenutzerId,
    target_id: BenutzerId,
    prompt_text: &str,
    game_completed: bool,
    winner_override: Option<&Benutzer>,
    now: chrono::DateTime<chrono::Utc>,
    signing: &dyn crate::ports::Signing,
) -> Result<Vec<Mail>, Report<CaptureUseCaseError>> {
    let notify_capture = game.visibility_mode() != VisibilityMode::Undercover
        && target_id != catcher_id
        && game.player_watching(target_id).unwrap_or(false);

    let completion_recipients: Vec<BenutzerId> = if game_completed {
        game.players()
            .iter()
            .filter(|p| p.user_id != catcher_id && p.watching)
            .map(|p| p.user_id)
            .collect()
    } else {
        Vec::new()
    };

    if !notify_capture && completion_recipients.is_empty() {
        return Ok(Vec::new());
    }

    let player_ids = game.sorted_player_ids();
    let users = uow
        .benutzer()
        .find_by_ids(&player_ids)
        .await
        .map_err(IntoReport::into_report)
        .change_context(CaptureUseCaseError::Persistence)?;
    let mut user_map: HashMap<BenutzerId, Benutzer> = users
        .into_iter()
        .map(|b| {
            let b = b.into_data();
            (b.id(), b)
        })
        .collect();
    if let Some(winner) = winner_override {
        user_map.insert(winner.id(), winner.clone());
    }

    let mut mails = Vec::new();

    if notify_capture {
        create_notification_idempotent(
            uow,
            NeueBenachrichtigung::aufnahme_hochgeladen(
                target_id,
                game.id(),
                capture_id,
                catcher_id,
                target_id,
                now,
            ),
        )
        .await
        .map_err(IntoReport::into_report)
        .change_context(CaptureUseCaseError::Persistence)?;

        if let (Some(target_user), Some(catcher_user)) =
            (user_map.get(&target_id), user_map.get(&catcher_id))
        {
            let wants_mail =
                wants_email(uow, target_id, BenachrichtigungArt::AufnahmeHochgeladen).await?;
            if wants_mail {
                let is_blurred_teaser = game.visibility_mode() == VisibilityMode::BlurredTeaser;
                let token = AbbestellToken::erzeugen(
                    target_id,
                    BenachrichtigungArt::AufnahmeHochgeladen,
                    now,
                    signing,
                );
                mails.push(Mail::aufnahme_hochgeladen(
                    target_user.email_address().clone(),
                    game.id(),
                    game.title().as_str(),
                    capture_id,
                    catcher_user.display_name().as_str(),
                    if is_blurred_teaser {
                        None
                    } else {
                        Some(target_user.display_name().as_str().to_owned())
                    },
                    if is_blurred_teaser {
                        None
                    } else {
                        Some(prompt_text.to_owned())
                    },
                    is_blurred_teaser,
                    token,
                ));
            }
        }
    }

    if game_completed {
        let winner_name = user_map
            .get(&catcher_id)
            .map(|u| u.display_name().as_str().to_owned())
            .unwrap_or_else(|| "Spieler".to_owned());

        for uid in completion_recipients {
            create_notification_idempotent(
                uow,
                NeueBenachrichtigung::spiel_beendet(uid, game.id(), catcher_id, now),
            )
            .await
            .map_err(IntoReport::into_report)
            .change_context(CaptureUseCaseError::Persistence)?;

            let Some(user) = user_map.get(&uid) else {
                continue;
            };
            if wants_email(uow, uid, BenachrichtigungArt::SpielBeendet).await? {
                let token =
                    AbbestellToken::erzeugen(uid, BenachrichtigungArt::SpielBeendet, now, signing);
                mails.push(Mail::spiel_beendet(
                    user.email_address().clone(),
                    game.id(),
                    game.title().as_str(),
                    winner_name.clone(),
                    token,
                ));
            }
        }
    }

    Ok(mails)
}

async fn wants_email(
    uow: &UnitOfWork<'_>,
    user_id: BenutzerId,
    art: BenachrichtigungArt,
) -> Result<bool, Report<CaptureUseCaseError>> {
    match uow
        .benachrichtigungs_einstellungen()
        .find_by_benutzer(user_id)
        .await
    {
        Ok(prefs) => Ok(prefs.email_erlaubt(art)),
        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => Ok(true),
        Err(err) => Err(err).change_context(CaptureUseCaseError::Persistence),
    }
}
