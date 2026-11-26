use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, ResultExt};
use tracing::error;

use crate::{
    application::ExecutionContext,
    domain::{
        BenachrichtigungArt, Benutzer, BenutzerId, CaptureId, Game, GameId, NeueBenachrichtigung,
        VisibilityMode,
    },
    ports::{Mail, RepositoryError},
    service::{EventHandler, create_notification_idempotent},
};

#[derive(Debug, thiserror::Error)]
pub enum GameEventError {
    #[error("Benachrichtigungen konnten nicht verarbeitet werden")]
    Persistence,
}

/// Emitted after a drafting game transitions into active play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameStarted {
    pub game_id: GameId,
    pub started_at: DateTime<Utc>,
}

/// Emitted after a capture is uploaded and committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureUploaded {
    pub game_id: GameId,
    pub capture_id: CaptureId,
    pub uploader_id: BenutzerId,
    pub target_id: BenutzerId,
    pub occurred_at: DateTime<Utc>,
}

/// Emitted after a winning capture transitions an active game to completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameFinished {
    pub game_id: GameId,
    pub winner_id: BenutzerId,
    pub finished_at: DateTime<Utc>,
}

async fn send_best_effort(ctx: &ExecutionContext<'_>, mails: Vec<Mail>) {
    for mail in mails {
        if let Err(err) = ctx.mail_sender().send(mail).await {
            error!(fehler = %err, "Benachrichtigungsmail konnte nicht zugestellt werden");
        }
    }
}

/// Persist and mail one notification per recipient in its own UoW transaction so an error for
/// one player never rolls back or skips deliveries for other players.
async fn notify_recipients<F, M>(
    ctx: &ExecutionContext<'_>,
    game_id: GameId,
    recipients: Vec<BenutzerId>,
    mut build_notif: F,
    mut build_mail: M,
) -> Result<(), Report<GameEventError>>
where
    F: FnMut(BenutzerId) -> (NeueBenachrichtigung, String),
    M: FnMut(&Benutzer, &str) -> Option<Mail>,
{
    let mut first_error: Option<Report<GameEventError>> = None;

    for uid in recipients {
        let per_user_res: Result<Option<Mail>, Report<GameEventError>> = async {
            let uow = ctx
                .enter()
                .await
                .change_context(GameEventError::Persistence)?;

            let work = async {
                // Verify both game and recipient exist inside this UoW before inserting so a
                // concurrent deletion skips cleanly instead of creating orphans or FK conflicts.
                match uow.games().find_by_id(game_id).await {
                    Ok(_) => {}
                    Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                        return Ok::<_, Report<GameEventError>>(None);
                    }
                    Err(err) => return Err(err).change_context(GameEventError::Persistence),
                }
                let user = match uow.benutzer().find_by_id(uid).await {
                    Ok(u) => u.into_data(),
                    Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                        return Ok(None);
                    }
                    Err(err) => return Err(err).change_context(GameEventError::Persistence),
                };

                let (neu, raw_token) = build_notif(uid);
                let (_notif, created) = create_notification_idempotent(&uow, neu)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(GameEventError::Persistence)?;
                if !created {
                    return Ok(None);
                }
                Ok(build_mail(&user, &raw_token))
            }
            .await;

            uow.finish(work, GameEventError::Persistence).await
        }
        .await;

        match per_user_res {
            Ok(Some(mail)) => send_best_effort(ctx, vec![mail]).await,
            Ok(None) => {}
            Err(err) => {
                error!(?uid, fehler = %err, "Benachrichtigung für Spieler fehlgeschlagen");
                if first_error.is_none() {
                    first_error = Some(err);
                }
            }
        }
    }

    match first_error {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

#[async_trait]
impl EventHandler for GameStarted {
    type Error = Report<GameEventError>;

    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        let read_uow = ctx
            .enter()
            .await
            .change_context(GameEventError::Persistence)?;
        let load_res = async {
            let game = match read_uow.games().find_by_id(self.game_id).await {
                Ok(g) => g.into_data(),
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Ok(None);
                }
                Err(err) => return Err(err).change_context(GameEventError::Persistence),
            };
            let Game::Active(active) = game else {
                return Ok(None);
            };
            let recipients: Vec<BenutzerId> = active
                .players()
                .iter()
                .filter(|p| p.watching)
                .map(|p| p.user_id)
                .collect();
            if recipients.is_empty() {
                return Ok(None);
            }
            Ok::<_, Report<GameEventError>>(Some((
                active.id(),
                active.title().as_str().to_owned(),
                recipients,
            )))
        }
        .await;

        let Some((game_id, game_title, recipients)) = read_uow
            .finish(load_res, GameEventError::Persistence)
            .await?
        else {
            return Ok(());
        };

        notify_recipients(
            &ctx,
            game_id,
            recipients,
            |uid| NeueBenachrichtigung::spiel_gestartet(uid, game_id, self.started_at),
            |user, token| {
                user.allows_email(BenachrichtigungArt::SpielGestartet)
                    .then(|| {
                        Mail::spiel_gestartet(
                            user.email_address().clone(),
                            game_id,
                            game_title.clone(),
                            token.to_owned(),
                        )
                    })
            },
        )
        .await
    }
}

#[async_trait]
impl EventHandler for CaptureUploaded {
    type Error = Report<GameEventError>;

    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        let read_uow = ctx
            .enter()
            .await
            .change_context(GameEventError::Persistence)?;
        let load_res = async {
            let game = match read_uow.games().find_by_id(self.game_id).await {
                Ok(g) => g.into_data(),
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Ok(None);
                }
                Err(err) => return Err(err).change_context(GameEventError::Persistence),
            };
            if !matches!(game, Game::Active(_) | Game::Completed(_))
                || game.visibility_mode() == VisibilityMode::Undercover
            {
                return Ok(None);
            }

            let recipients: Vec<BenutzerId> = game
                .players()
                .iter()
                .filter(|p| p.user_id != self.uploader_id && p.watching)
                .map(|p| p.user_id)
                .collect();
            if recipients.is_empty() {
                return Ok(None);
            }

            let capture = match read_uow.captures().find_by_id(self.capture_id).await {
                Ok(c) => c.into_data(),
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Ok(None);
                }
                Err(err) => return Err(err).change_context(GameEventError::Persistence),
            };
            let users = read_uow
                .benutzer()
                .find_by_ids(&[self.uploader_id, self.target_id])
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameEventError::Persistence)?;
            let catcher_name = users
                .iter()
                .find(|u| u.id() == self.uploader_id)
                .map(|u| u.display_name().as_str().to_owned());
            let target_name = users
                .iter()
                .find(|u| u.id() == self.target_id)
                .map(|u| u.display_name().as_str().to_owned());
            let (Some(catcher_name), Some(target_name)) = (catcher_name, target_name) else {
                return Ok(None);
            };

            let is_blurred_teaser = matches!(game, Game::Active(_))
                && game.visibility_mode() == VisibilityMode::BlurredTeaser;

            Ok::<_, Report<GameEventError>>(Some((
                game.id(),
                game.title().as_str().to_owned(),
                catcher_name,
                (!is_blurred_teaser).then_some(target_name),
                (!is_blurred_teaser).then(|| capture.prompt_text().as_str().to_owned()),
                is_blurred_teaser,
                recipients,
            )))
        }
        .await;

        let Some((
            game_id,
            game_title,
            catcher_name,
            target_name,
            prompt_text,
            is_blurred_teaser,
            recipients,
        )) = read_uow
            .finish(load_res, GameEventError::Persistence)
            .await?
        else {
            return Ok(());
        };

        notify_recipients(
            &ctx,
            game_id,
            recipients,
            |uid| {
                NeueBenachrichtigung::aufnahme_hochgeladen(
                    uid,
                    game_id,
                    self.capture_id,
                    self.uploader_id,
                    self.target_id,
                    self.occurred_at,
                )
            },
            |user, token| {
                user.allows_email(BenachrichtigungArt::AufnahmeHochgeladen)
                    .then(|| {
                        Mail::aufnahme_hochgeladen(
                            user.email_address().clone(),
                            game_id,
                            game_title.clone(),
                            self.capture_id,
                            catcher_name.clone(),
                            target_name.clone(),
                            prompt_text.clone(),
                            is_blurred_teaser,
                            token.to_owned(),
                        )
                    })
            },
        )
        .await
    }
}

#[async_trait]
impl EventHandler for GameFinished {
    type Error = Report<GameEventError>;

    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        let read_uow = ctx
            .enter()
            .await
            .change_context(GameEventError::Persistence)?;
        let load_res = async {
            let game = match read_uow.games().find_by_id(self.game_id).await {
                Ok(g) => g.into_data(),
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Ok(None);
                }
                Err(err) => return Err(err).change_context(GameEventError::Persistence),
            };
            let Game::Completed(completed) = game else {
                return Ok(None);
            };
            let recipients: Vec<BenutzerId> = completed
                .players()
                .iter()
                .filter(|p| p.user_id != self.winner_id && p.watching)
                .map(|p| p.user_id)
                .collect();
            if recipients.is_empty() {
                return Ok(None);
            }
            let winner_name = read_uow
                .benutzer()
                .find_by_ids(&[self.winner_id])
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameEventError::Persistence)?
                .into_iter()
                .next()
                .map(|u| u.display_name().as_str().to_owned())
                .unwrap_or_else(|| "Spieler".to_owned());

            Ok::<_, Report<GameEventError>>(Some((
                completed.id(),
                completed.title().as_str().to_owned(),
                winner_name,
                recipients,
            )))
        }
        .await;

        let Some((game_id, game_title, winner_name, recipients)) = read_uow
            .finish(load_res, GameEventError::Persistence)
            .await?
        else {
            return Ok(());
        };

        notify_recipients(
            &ctx,
            game_id,
            recipients,
            |uid| {
                NeueBenachrichtigung::spiel_beendet(uid, game_id, self.winner_id, self.finished_at)
            },
            |user, token| {
                user.allows_email(BenachrichtigungArt::SpielBeendet)
                    .then(|| {
                        Mail::spiel_beendet(
                            user.email_address().clone(),
                            game_id,
                            game_title.clone(),
                            winner_name.clone(),
                            token.to_owned(),
                        )
                    })
            },
        )
        .await
    }
}
