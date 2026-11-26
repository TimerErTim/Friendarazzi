use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, ResultExt};
use tracing::warn;

use crate::{
    application::ExecutionContext,
    domain::{
        AbbestellToken, BenachrichtigungArt, Benutzer, BenutzerId, CaptureId, Game, GameId,
        NeueBenachrichtigung, VisibilityMode,
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

/// Emitted after a capture is submitted and committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureUpdated {
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
            warn!(fehler = %err, "Benachrichtigungsmail konnte nicht zugestellt werden");
        }
    }
}

async fn load_users_map(
    ctx: &ExecutionContext<'_>,
    uow: &crate::application::uow::UnitOfWork<'_>,
    ids: &[BenutzerId],
) -> Result<HashMap<BenutzerId, Benutzer>, Report<GameEventError>> {
    let _ = ctx;
    let users = uow
        .benutzer()
        .find_by_ids(ids)
        .await
        .map_err(IntoReport::into_report)
        .change_context(GameEventError::Persistence)?;
    Ok(users
        .into_iter()
        .map(|b| {
            let b = b.into_data();
            (b.id(), b)
        })
        .collect())
}

#[async_trait]
impl EventHandler for GameStarted {
    type Error = Report<GameEventError>;

    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        let now = ctx.clock().now();
        let uow = ctx
            .enter()
            .await
            .change_context(GameEventError::Persistence)?;

        let result = async {
            let game = match uow.games().find_by_id(self.game_id).await {
                Ok(g) => g.into_data(),
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Ok(Vec::new());
                }
                Err(err) => return Err(err).change_context(GameEventError::Persistence),
            };
            let Game::Active(active) = game else {
                return Ok(Vec::new());
            };

            let watching_ids: Vec<BenutzerId> = active
                .players()
                .iter()
                .filter(|p| p.watching)
                .map(|p| p.user_id)
                .collect();
            if watching_ids.is_empty() {
                return Ok(Vec::new());
            }

            for &uid in &watching_ids {
                create_notification_idempotent(
                    &uow,
                    NeueBenachrichtigung::spiel_gestartet(uid, active.id(), self.started_at),
                )
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameEventError::Persistence)?;
            }

            let user_map = load_users_map(&ctx, &uow, &watching_ids).await?;
            let mut mails = Vec::new();
            for uid in watching_ids {
                let Some(user) = user_map.get(&uid) else {
                    continue;
                };
                if user.allows_email(BenachrichtigungArt::SpielGestartet) {
                    let token = AbbestellToken::erzeugen(
                        uid,
                        BenachrichtigungArt::SpielGestartet,
                        now,
                        ctx.signing(),
                    );
                    mails.push(Mail::spiel_gestartet(
                        user.email_address().clone(),
                        active.id(),
                        active.title().as_str(),
                        token,
                    ));
                }
            }

            Ok::<_, Report<GameEventError>>(mails)
        }
        .await;

        let mails = uow.finish(result, GameEventError::Persistence).await?;
        send_best_effort(&ctx, mails).await;
        Ok(())
    }
}

#[async_trait]
impl EventHandler for CaptureUpdated {
    type Error = Report<GameEventError>;

    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        let now = ctx.clock().now();
        let uow = ctx
            .enter()
            .await
            .change_context(GameEventError::Persistence)?;

        let result = async {
            let game = match uow.games().find_by_id(self.game_id).await {
                Ok(g) => g.into_data(),
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Ok(Vec::new());
                }
                Err(err) => return Err(err).change_context(GameEventError::Persistence),
            };
            if !matches!(game, Game::Active(_) | Game::Completed(_))
                || game.visibility_mode() == VisibilityMode::Undercover
            {
                return Ok(Vec::new());
            }

            let recipients: Vec<BenutzerId> = game
                .players()
                .iter()
                .filter(|p| p.user_id != self.uploader_id && p.watching)
                .map(|p| p.user_id)
                .collect();
            if recipients.is_empty() {
                return Ok(Vec::new());
            }

            let capture = match uow.captures().find_by_id(self.capture_id).await {
                Ok(c) => c.into_data(),
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Ok(Vec::new());
                }
                Err(err) => return Err(err).change_context(GameEventError::Persistence),
            };

            for &uid in &recipients {
                create_notification_idempotent(
                    &uow,
                    NeueBenachrichtigung::aufnahme_hochgeladen(
                        uid,
                        game.id(),
                        self.capture_id,
                        self.uploader_id,
                        self.target_id,
                        self.occurred_at,
                    ),
                )
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameEventError::Persistence)?;
            }

            let mut lookup_ids = recipients.clone();
            if !lookup_ids.contains(&self.uploader_id) {
                lookup_ids.push(self.uploader_id);
            }
            if !lookup_ids.contains(&self.target_id) {
                lookup_ids.push(self.target_id);
            }
            let user_map = load_users_map(&ctx, &uow, &lookup_ids).await?;
            let (Some(catcher_user), Some(target_user)) = (
                user_map.get(&self.uploader_id),
                user_map.get(&self.target_id),
            ) else {
                return Ok(Vec::new());
            };

            let is_blurred_teaser = matches!(game, Game::Active(_))
                && game.visibility_mode() == VisibilityMode::BlurredTeaser;

            let mut mails = Vec::new();
            for uid in recipients {
                let Some(recipient) = user_map.get(&uid) else {
                    continue;
                };
                if !recipient.allows_email(BenachrichtigungArt::AufnahmeHochgeladen) {
                    continue;
                }
                let token = AbbestellToken::erzeugen(
                    uid,
                    BenachrichtigungArt::AufnahmeHochgeladen,
                    now,
                    ctx.signing(),
                );
                mails.push(Mail::aufnahme_hochgeladen(
                    recipient.email_address().clone(),
                    game.id(),
                    game.title().as_str(),
                    self.capture_id,
                    catcher_user.display_name().as_str(),
                    (!is_blurred_teaser).then(|| target_user.display_name().as_str().to_owned()),
                    (!is_blurred_teaser).then(|| capture.prompt_text().as_str().to_owned()),
                    is_blurred_teaser,
                    token,
                ));
            }

            Ok::<_, Report<GameEventError>>(mails)
        }
        .await;

        let mails = uow.finish(result, GameEventError::Persistence).await?;
        send_best_effort(&ctx, mails).await;
        Ok(())
    }
}

#[async_trait]
impl EventHandler for GameFinished {
    type Error = Report<GameEventError>;

    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        let now = ctx.clock().now();
        let uow = ctx
            .enter()
            .await
            .change_context(GameEventError::Persistence)?;

        let result = async {
            let game = match uow.games().find_by_id(self.game_id).await {
                Ok(g) => g.into_data(),
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Ok(Vec::new());
                }
                Err(err) => return Err(err).change_context(GameEventError::Persistence),
            };
            let Game::Completed(completed) = game else {
                return Ok(Vec::new());
            };

            let recipients: Vec<BenutzerId> = completed
                .players()
                .iter()
                .filter(|p| p.user_id != self.winner_id && p.watching)
                .map(|p| p.user_id)
                .collect();
            if recipients.is_empty() {
                return Ok(Vec::new());
            }

            for &uid in &recipients {
                create_notification_idempotent(
                    &uow,
                    NeueBenachrichtigung::spiel_beendet(
                        uid,
                        completed.id(),
                        self.winner_id,
                        self.finished_at,
                    ),
                )
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameEventError::Persistence)?;
            }

            let mut lookup_ids = recipients.clone();
            lookup_ids.push(self.winner_id);
            let user_map = load_users_map(&ctx, &uow, &lookup_ids).await?;
            let winner_name = user_map
                .get(&self.winner_id)
                .map(|u| u.display_name().as_str().to_owned())
                .unwrap_or_else(|| "Spieler".to_owned());

            let mut mails = Vec::new();
            for uid in recipients {
                let Some(user) = user_map.get(&uid) else {
                    continue;
                };
                if user.allows_email(BenachrichtigungArt::SpielBeendet) {
                    let token = AbbestellToken::erzeugen(
                        uid,
                        BenachrichtigungArt::SpielBeendet,
                        now,
                        ctx.signing(),
                    );
                    mails.push(Mail::spiel_beendet(
                        user.email_address().clone(),
                        completed.id(),
                        completed.title().as_str(),
                        winner_name.clone(),
                        token,
                    ));
                }
            }

            Ok::<_, Report<GameEventError>>(mails)
        }
        .await;

        let mails = uow.finish(result, GameEventError::Persistence).await?;
        send_best_effort(&ctx, mails).await;
        Ok(())
    }
}
