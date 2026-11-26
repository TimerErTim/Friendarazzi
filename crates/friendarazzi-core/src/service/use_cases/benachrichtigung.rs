//! In-app notification read-state, preference updates, game watch toggles, and token unsubscribe.

use async_trait::async_trait;
use error_stack::{IntoReport, Report, ResultExt};
use tracing::{info, instrument};

use crate::{
    application::{ExecutionContext, uow::UnitOfWork, uow::Versioned},
    domain::{
        Benachrichtigung, BenachrichtigungArt, BenachrichtigungId, BenachrichtigungsEinstellungen,
        BenutzerId, Game, GameId, NeueBenachrichtigung,
    },
    ports::RepositoryError,
    service::{UseCase, map_game_error},
};

#[derive(Debug, thiserror::Error)]
pub enum BenachrichtigungUseCaseError {
    #[error("für diese Aktion ist eine Anmeldung erforderlich")]
    Unauthorized,
    #[error("du darfst diese Aktion nicht ausführen")]
    Forbidden,
    #[error("Eintrag oder Spiel wurde nicht gefunden")]
    NotFound,
    #[error("der Abmelde-Link ist ungültig oder abgelaufen")]
    InvalidToken,
    #[error("Benachrichtigungen konnten nicht gespeichert werden")]
    Persistence,
}

pub(crate) async fn load_default_watch_for_user(
    uow: &UnitOfWork<'_>,
    benutzer_id: BenutzerId,
) -> Result<bool, Report<RepositoryError>> {
    match uow.benutzer().find_by_id(benutzer_id).await {
        Ok(benutzer) => Ok(benutzer.default_watch_games()),
        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => Ok(true),
        Err(err) => Err(err),
    }
}

/// Persist `neu` once; returns `(row, created)` so retries never resend emails for existing rows.
pub(crate) async fn create_notification_idempotent(
    uow: &UnitOfWork<'_>,
    neu: NeueBenachrichtigung,
) -> Result<(Versioned<Benachrichtigung>, bool), Report<RepositoryError>> {
    let empfänger_id = neu.empfänger_id;
    let dedupe_key = neu.dedupe_key.clone();
    match uow.benachrichtigungen().create(neu).await {
        Ok(created) => Ok((created, true)),
        Err(err) if matches!(err.current_context(), RepositoryError::Conflict) => {
            let existing = uow
                .benachrichtigungen()
                .find_by_empfänger(empfänger_id)
                .await?
                .into_iter()
                .find(|n| n.dedupe_key() == &dedupe_key)
                .ok_or_else(|| Report::new(RepositoryError::NotFound))?;
            Ok((existing, false))
        }
        Err(err) => Err(err),
    }
}

#[derive(Debug, Clone)]
pub struct BenachrichtigungAlsGelesenMarkieren {
    pub benachrichtigung_id: BenachrichtigungId,
}

#[async_trait]
impl UseCase<()> for BenachrichtigungAlsGelesenMarkieren {
    type Error = Report<BenachrichtigungUseCaseError>;

    #[instrument(skip_all, fields(id = ?self.benachrichtigung_id), err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(BenachrichtigungUseCaseError::Unauthorized)?;
        let now = ctx.clock().now();
        let uow = ctx
            .enter()
            .await
            .change_context(BenachrichtigungUseCaseError::Persistence)?;

        let result = async {
            let mut notif = match uow
                .benachrichtigungen()
                .find_by_id(self.benachrichtigung_id)
                .await
            {
                Ok(n) => n,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(BenachrichtigungUseCaseError::NotFound));
                }
                Err(err) => {
                    return Err(err).change_context(BenachrichtigungUseCaseError::Persistence);
                }
            };

            if notif.empfänger_id() != user_id {
                return Err(Report::new(BenachrichtigungUseCaseError::Forbidden));
            }

            if notif.ist_ungelesen() {
                notif.als_gelesen_markieren(now);
                uow.benachrichtigungen()
                    .update(&mut notif)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(BenachrichtigungUseCaseError::Persistence)?;
            }

            Ok::<_, Report<BenachrichtigungUseCaseError>>(())
        }
        .await;

        uow.finish(result, BenachrichtigungUseCaseError::Persistence)
            .await
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AlleBenachrichtigungenAlsGelesenMarkieren;

#[async_trait]
impl UseCase<()> for AlleBenachrichtigungenAlsGelesenMarkieren {
    type Error = Report<BenachrichtigungUseCaseError>;

    #[instrument(skip_all, err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(BenachrichtigungUseCaseError::Unauthorized)?;
        let now = ctx.clock().now();
        let uow = ctx
            .enter()
            .await
            .change_context(BenachrichtigungUseCaseError::Persistence)?;

        let result = async {
            let list = uow
                .benachrichtigungen()
                .find_by_empfänger(user_id)
                .await
                .map_err(IntoReport::into_report)
                .change_context(BenachrichtigungUseCaseError::Persistence)?;

            for mut notif in list {
                if notif.ist_ungelesen() {
                    notif.als_gelesen_markieren(now);
                    uow.benachrichtigungen()
                        .update(&mut notif)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(BenachrichtigungUseCaseError::Persistence)?;
                }
            }

            Ok::<_, Report<BenachrichtigungUseCaseError>>(())
        }
        .await;

        uow.finish(result, BenachrichtigungUseCaseError::Persistence)
            .await
    }
}

#[derive(Debug, Clone)]
pub struct BenachrichtigungsEinstellungenAktualisieren {
    pub default_watch_games: Option<bool>,
    pub email_game_started: Option<bool>,
    pub email_capture_posted: Option<bool>,
    pub email_game_completed: Option<bool>,
}

#[async_trait]
impl UseCase<BenachrichtigungsEinstellungen> for BenachrichtigungsEinstellungenAktualisieren {
    type Error = Report<BenachrichtigungUseCaseError>;

    #[instrument(skip_all, err(Debug))]
    async fn perform(
        self,
        ctx: ExecutionContext<'_>,
    ) -> Result<BenachrichtigungsEinstellungen, Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(BenachrichtigungUseCaseError::Unauthorized)?;
        let now = ctx.clock().now();
        let uow = ctx
            .enter()
            .await
            .change_context(BenachrichtigungUseCaseError::Persistence)?;

        let result = async {
            let mut benutzer = match uow.benutzer().find_by_id(user_id).await {
                Ok(b) => b,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(BenachrichtigungUseCaseError::NotFound));
                }
                Err(err) => {
                    return Err(err).change_context(BenachrichtigungUseCaseError::Persistence);
                }
            };

            let prefs = benutzer.notification_preferences_mut();
            if let Some(v) = self.default_watch_games {
                prefs.setze_standard_spiel_beobachten(v, now);
            }
            if let Some(v) = self.email_game_started {
                prefs.setze_email(BenachrichtigungArt::SpielGestartet, v, now);
            }
            if let Some(v) = self.email_capture_posted {
                prefs.setze_email(BenachrichtigungArt::AufnahmeHochgeladen, v, now);
            }
            if let Some(v) = self.email_game_completed {
                prefs.setze_email(BenachrichtigungArt::SpielBeendet, v, now);
            }

            uow.benutzer()
                .update(&mut benutzer)
                .await
                .map_err(IntoReport::into_report)
                .change_context(BenachrichtigungUseCaseError::Persistence)?;

            Ok::<_, Report<BenachrichtigungUseCaseError>>(benutzer.notification_preferences())
        }
        .await;

        uow.finish(result, BenachrichtigungUseCaseError::Persistence)
            .await
    }
}

#[derive(Debug, Clone)]
pub struct BenachrichtigungAbbestellenMitToken {
    pub token: String,
}

#[async_trait]
impl UseCase<BenachrichtigungArt> for BenachrichtigungAbbestellenMitToken {
    type Error = Report<BenachrichtigungUseCaseError>;

    #[instrument(skip_all, err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<BenachrichtigungArt, Self::Error> {
        let token = self.token.trim();
        if token.is_empty() {
            return Err(Report::new(BenachrichtigungUseCaseError::InvalidToken));
        }

        let now = ctx.clock().now();
        let uow = ctx
            .enter()
            .await
            .change_context(BenachrichtigungUseCaseError::Persistence)?;

        let result = async {
            let notif = match uow
                .benachrichtigungen()
                .find_by_abbestell_token(token)
                .await
            {
                Ok(n) => n,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(BenachrichtigungUseCaseError::InvalidToken));
                }
                Err(err) => {
                    return Err(err).change_context(BenachrichtigungUseCaseError::Persistence);
                }
            };
            if notif.abbestell_token_abgelaufen(now) {
                return Err(Report::new(BenachrichtigungUseCaseError::InvalidToken));
            }

            let benutzer_id = notif.empfänger_id();
            let art = notif.art();
            let mut benutzer = match uow.benutzer().find_by_id(benutzer_id).await {
                Ok(b) => b,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(BenachrichtigungUseCaseError::InvalidToken));
                }
                Err(err) => {
                    return Err(err).change_context(BenachrichtigungUseCaseError::Persistence);
                }
            };

            benutzer
                .notification_preferences_mut()
                .setze_email(art, false, now);
            uow.benutzer()
                .update(&mut benutzer)
                .await
                .map_err(IntoReport::into_report)
                .change_context(BenachrichtigungUseCaseError::Persistence)?;

            Ok::<_, Report<BenachrichtigungUseCaseError>>((benutzer_id, art))
        }
        .await;

        let (benutzer_id, art) = uow
            .finish(result, BenachrichtigungUseCaseError::Persistence)
            .await?;
        info!(
            ?benutzer_id,
            ?art,
            "notification mail preference unsubscribed"
        );
        Ok(art)
    }
}

#[derive(Debug, Clone)]
pub struct SpielBeobachtungSetzen {
    pub game_id: GameId,
    pub watching: bool,
}

#[async_trait]
impl UseCase<Game> for SpielBeobachtungSetzen {
    type Error = Report<BenachrichtigungUseCaseError>;

    #[instrument(skip_all, fields(game_id = ?self.game_id, watching = self.watching), err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<Game, Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(BenachrichtigungUseCaseError::Unauthorized)?;
        let uow = ctx
            .enter()
            .await
            .change_context(BenachrichtigungUseCaseError::Persistence)?;

        let result = async {
            let mut game = match uow.games().find_by_id(self.game_id).await {
                Ok(g) => g,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(BenachrichtigungUseCaseError::NotFound));
                }
                Err(err) => {
                    return Err(err).change_context(BenachrichtigungUseCaseError::Persistence);
                }
            };

            if !game.has_player(user_id) {
                return Err(Report::new(BenachrichtigungUseCaseError::Forbidden));
            }

            game.set_player_watching(user_id, self.watching)
                .map_err(|e| {
                    let _ = map_game_error(e.current_context());
                    e.change_context(BenachrichtigungUseCaseError::Forbidden)
                })?;

            uow.games()
                .update(&mut game)
                .await
                .map_err(IntoReport::into_report)
                .change_context(BenachrichtigungUseCaseError::Persistence)?;

            Ok::<_, Report<BenachrichtigungUseCaseError>>(game.into_data())
        }
        .await;

        uow.finish(result, BenachrichtigungUseCaseError::Persistence)
            .await
    }
}
