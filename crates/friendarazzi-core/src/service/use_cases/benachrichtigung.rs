//! In-app notification read-state, preference updates, game watch toggles, and token unsubscribe.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, ResultExt};
use tracing::{info, instrument};

use crate::{
    application::{ExecutionContext, uow::UnitOfWork, uow::Versioned},
    domain::{
        AbbestellToken, BenachrichtigungArt, BenachrichtigungId, BenachrichtigungsEinstellungen,
        BenutzerId, Game, GameId, NeueBenachrichtigung, NeueBenachrichtigungsEinstellungen,
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

pub(crate) async fn load_or_init_preferences(
    uow: &UnitOfWork<'_>,
    benutzer_id: BenutzerId,
    fallback_now: DateTime<Utc>,
) -> Result<(Versioned<BenachrichtigungsEinstellungen>, bool), Report<BenachrichtigungUseCaseError>>
{
    match uow
        .benachrichtigungs_einstellungen()
        .find_by_benutzer(benutzer_id)
        .await
    {
        Ok(existing) => Ok((existing, true)),
        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => Ok((
            Versioned::init(BenachrichtigungsEinstellungen::standard(
                benutzer_id,
                fallback_now,
            )),
            false,
        )),
        Err(err) => Err(err).change_context(BenachrichtigungUseCaseError::Persistence),
    }
}

pub(crate) async fn load_default_watch_for_user(
    uow: &UnitOfWork<'_>,
    benutzer_id: BenutzerId,
) -> Result<bool, Report<RepositoryError>> {
    match uow
        .benachrichtigungs_einstellungen()
        .find_by_benutzer(benutzer_id)
        .await
    {
        Ok(prefs) => Ok(prefs.standard_spiel_beobachten().aktiv),
        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => Ok(true),
        Err(err) => Err(err),
    }
}

pub(crate) async fn create_notification_idempotent(
    uow: &UnitOfWork<'_>,
    neu: NeueBenachrichtigung,
) -> Result<(), Report<RepositoryError>> {
    match uow.benachrichtigungen().create(neu).await {
        Ok(_) => Ok(()),
        Err(err) if matches!(err.current_context(), RepositoryError::Conflict) => Ok(()),
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
            let (mut prefs, exists) = load_or_init_preferences(&uow, user_id, now).await?;

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

            let saved = if exists {
                uow.benachrichtigungs_einstellungen()
                    .update(&mut prefs)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(BenachrichtigungUseCaseError::Persistence)?;
                prefs.into_data()
            } else {
                uow.benachrichtigungs_einstellungen()
                    .create(NeueBenachrichtigungsEinstellungen {
                        benutzer_id: user_id,
                        standard_spiel_beobachten: prefs.standard_spiel_beobachten(),
                        email_spiel_gestartet: prefs.email_spiel_gestartet(),
                        email_aufnahme_hochgeladen: prefs.email_aufnahme_hochgeladen(),
                        email_spiel_beendet: prefs.email_spiel_beendet(),
                    })
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(BenachrichtigungUseCaseError::Persistence)?
                    .into_data()
            };

            Ok::<_, Report<BenachrichtigungUseCaseError>>(saved)
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
        let now = ctx.clock().now();
        let (benutzer_id, art) = AbbestellToken::prüfen(&self.token, ctx.signing(), now)
            .change_context(BenachrichtigungUseCaseError::InvalidToken)?;

        let uow = ctx
            .enter()
            .await
            .change_context(BenachrichtigungUseCaseError::Persistence)?;

        let result = async {
            // Confirm the user still exists; return the generic InvalidToken so deleted accounts cannot be enumerated.
            match uow.benutzer().find_by_id(benutzer_id).await {
                Ok(_) => {}
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(BenachrichtigungUseCaseError::InvalidToken));
                }
                Err(err) => {
                    return Err(err).change_context(BenachrichtigungUseCaseError::Persistence);
                }
            }

            let (mut prefs, exists) = load_or_init_preferences(&uow, benutzer_id, now).await?;
            prefs.setze_email(art, false, now);

            if exists {
                uow.benachrichtigungs_einstellungen()
                    .update(&mut prefs)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(BenachrichtigungUseCaseError::Persistence)?;
            } else {
                uow.benachrichtigungs_einstellungen()
                    .create(NeueBenachrichtigungsEinstellungen {
                        benutzer_id,
                        standard_spiel_beobachten: prefs.standard_spiel_beobachten(),
                        email_spiel_gestartet: prefs.email_spiel_gestartet(),
                        email_aufnahme_hochgeladen: prefs.email_aufnahme_hochgeladen(),
                        email_spiel_beendet: prefs.email_spiel_beendet(),
                    })
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(BenachrichtigungUseCaseError::Persistence)?;
            }

            Ok::<_, Report<BenachrichtigungUseCaseError>>(art)
        }
        .await;

        let art = uow
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
