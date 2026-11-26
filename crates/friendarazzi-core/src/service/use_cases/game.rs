//! Game lobby, joining, settings, drafting, and board assembly use cases.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, ResultExt};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::{
    application::{ExecutionContext, uow::UnitOfWork, uow::Versioned},
    domain::{
        AbbestellToken, ActiveGame, BenachrichtigungArt, Benutzer, BenutzerId, DraftStartOutcome,
        Game, GameError, GameId, GridSize, InviteCode, MIN_PLAYERS, NeueBenachrichtigung, NewBoard,
        NewGame, NonEmptyString, PromptSource, PromptSubmission, PromptSubmissionId,
        VisibilityMode,
    },
    ports::{Mail, RepositoryError},
    service::{
        UseCase,
        use_cases::benachrichtigung::{
            create_notification_idempotent, load_default_watch_for_user,
        },
    },
};

const MAX_INVITE_CODE_ATTEMPTS: usize = 8;

#[derive(Debug, thiserror::Error)]
pub enum GameUseCaseError {
    #[error("für diese Aktion ist eine Anmeldung erforderlich")]
    Unauthorized,
    #[error("du bist nicht berechtigt, diese Aktion auszuführen")]
    Forbidden,
    #[error("das Spiel wurde nicht gefunden")]
    NotFound,
    #[error("ungültige Eingabe")]
    BadRequest,
    #[error("die Aktion passt nicht zum aktuellen Spielstatus")]
    Conflict,
    #[error("mindestens {MIN_PLAYERS} Spieler sind erforderlich")]
    TooFewPlayers,
    #[error("Spiel konnte nicht geladen oder gespeichert werden")]
    Persistence,
}

fn map_update_repo_err(err: &Report<RepositoryError>) -> GameUseCaseError {
    if matches!(
        err.current_context(),
        RepositoryError::VersionMismatch { .. } | RepositoryError::Conflict
    ) {
        GameUseCaseError::Conflict
    } else {
        GameUseCaseError::Persistence
    }
}

pub(crate) fn map_game_error(err: &GameError) -> GameUseCaseError {
    match err {
        GameError::EmptyTitle
        | GameError::InvalidInviteCode
        | GameError::InvalidGridSize
        | GameError::InvalidVisibilityMode
        | GameError::InvalidPromptSource
        | GameError::SelfTargetNotAllowed
        | GameError::TargetNotInGame
        | GameError::InvalidPromptQuota => GameUseCaseError::BadRequest,
        GameError::PlayerNotInGame => GameUseCaseError::Forbidden,
        GameError::InvalidStateTransition
        | GameError::BoardAssemblyFailed
        | GameError::BoardNotCompleted => GameUseCaseError::Conflict,
        GameError::TooFewPlayers => GameUseCaseError::TooFewPlayers,
    }
}

#[derive(Debug, Clone)]
pub struct CreateGame {
    pub title: NonEmptyString,
    pub grid_size: GridSize,
    pub visibility_mode: VisibilityMode,
}

#[async_trait]
impl UseCase<Game> for CreateGame {
    type Error = Report<GameUseCaseError>;

    #[instrument(
        skip_all,
        fields(
            title = %self.title,
            grid_size = ?self.grid_size,
            visibility_mode = ?self.visibility_mode
        ),
        err(Debug)
    )]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<Game, Self::Error> {
        let host_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(GameUseCaseError::Unauthorized)?;
        let now = ctx.clock().now();
        let uow = ctx
            .enter()
            .await
            .change_context(GameUseCaseError::Persistence)?;

        let result = async {
            let mut invite_code = None;
            for attempt in 1..=MAX_INVITE_CODE_ATTEMPTS {
                let candidate = {
                    let mut rng = ctx.rng();
                    InviteCode::generate(&mut *rng)
                };
                match uow.games().find_by_invite_code(&candidate).await {
                    Ok(_) => {
                        warn!(
                            attempt,
                            max = MAX_INVITE_CODE_ATTEMPTS,
                            "invite code collision during game creation"
                        );
                    }
                    Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                        invite_code = Some(candidate);
                        break;
                    }
                    Err(err) => return Err(err).change_context(GameUseCaseError::Persistence),
                }
            }
            let invite_code = match invite_code {
                Some(code) => code,
                None => {
                    error!(
                        attempts = MAX_INVITE_CODE_ATTEMPTS,
                        "exhausted invite code generation retries"
                    );
                    return Err(Report::new(GameUseCaseError::Conflict));
                }
            };

            let host_watching = load_default_watch_for_user(&uow, host_id)
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            let game = uow
                .games()
                .create(NewGame {
                    invite_code,
                    host_user_id: host_id,
                    host_watching,
                    title: self.title,
                    grid_size: self.grid_size,
                    visibility_mode: self.visibility_mode,
                    created_at: now,
                })
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            Ok::<_, Report<GameUseCaseError>>(game.into_data())
        }
        .await;

        uow.finish(result, GameUseCaseError::Persistence).await
    }
}

#[derive(Debug, Clone)]
pub struct JoinGame {
    pub invite_code: InviteCode,
}

#[async_trait]
impl UseCase<Game> for JoinGame {
    type Error = Report<GameUseCaseError>;

    #[instrument(skip_all, fields(invite_code = self.invite_code.as_str()), err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<Game, Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(GameUseCaseError::Unauthorized)?;
        let now = ctx.clock().now();
        let uow = ctx
            .enter()
            .await
            .change_context(GameUseCaseError::Persistence)?;

        let result = async {
            let game = match uow.games().find_by_invite_code(&self.invite_code).await {
                Ok(g) => g,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(GameUseCaseError::NotFound));
                }
                Err(err) => return Err(err).change_context(GameUseCaseError::Persistence),
            };

            if game.has_player(user_id) {
                return Ok(game.into_data());
            }

            let version = game.v();
            let Game::Lobby(mut lobby) = game.into_data() else {
                return Err(Report::new(GameUseCaseError::Conflict));
            };

            let watching = load_default_watch_for_user(&uow, user_id)
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            lobby.join_player(user_id, now, watching);
            let mut updated = Versioned::new(version, Game::Lobby(lobby));
            uow.games()
                .update(&mut updated)
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            Ok::<_, Report<GameUseCaseError>>(updated.into_data())
        }
        .await;

        uow.finish(result, GameUseCaseError::Persistence).await
    }
}

#[derive(Debug, Clone)]
pub struct UpdateGameSettings {
    pub game_id: GameId,
    pub title: NonEmptyString,
    pub grid_size: GridSize,
    pub visibility_mode: VisibilityMode,
}

#[async_trait]
impl UseCase<Game> for UpdateGameSettings {
    type Error = Report<GameUseCaseError>;

    #[instrument(
        skip_all,
        fields(
            game_id = ?self.game_id,
            title = %self.title,
            grid_size = ?self.grid_size,
            visibility_mode = ?self.visibility_mode
        ),
        err(Debug)
    )]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<Game, Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(GameUseCaseError::Unauthorized)?;
        let uow = ctx
            .enter()
            .await
            .change_context(GameUseCaseError::Persistence)?;

        let result = async {
            let game = match uow.games().find_by_id(self.game_id).await {
                Ok(g) => g,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(GameUseCaseError::NotFound));
                }
                Err(err) => return Err(err).change_context(GameUseCaseError::Persistence),
            };

            if game.host_user_id() != user_id {
                return Err(Report::new(GameUseCaseError::Forbidden));
            }

            let version = game.v();
            let Game::Lobby(mut lobby) = game.into_data() else {
                return Err(Report::new(GameUseCaseError::Conflict));
            };

            lobby.update_settings(self.title, self.grid_size, self.visibility_mode);

            let mut updated = Versioned::new(version, Game::Lobby(lobby));
            uow.games()
                .update(&mut updated)
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            Ok::<_, Report<GameUseCaseError>>(updated.into_data())
        }
        .await;

        uow.finish(result, GameUseCaseError::Persistence).await
    }
}

#[derive(Debug, Clone)]
pub struct StartDrafting {
    pub game_id: GameId,
}

#[async_trait]
impl UseCase<Game> for StartDrafting {
    type Error = Report<GameUseCaseError>;

    #[instrument(skip_all, fields(game_id = ?self.game_id), err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<Game, Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(GameUseCaseError::Unauthorized)?;
        let uow = ctx
            .enter()
            .await
            .change_context(GameUseCaseError::Persistence)?;

        let result = async {
            let game = match uow.games().find_by_id(self.game_id).await {
                Ok(g) => g,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(GameUseCaseError::NotFound));
                }
                Err(err) => return Err(err).change_context(GameUseCaseError::Persistence),
            };

            if game.host_user_id() != user_id {
                return Err(Report::new(GameUseCaseError::Forbidden));
            }

            let version = game.v();
            let Game::Lobby(lobby) = game.into_data() else {
                return Err(Report::new(GameUseCaseError::Conflict));
            };

            let drafting = lobby.start_drafting().map_err(|e| {
                let mapped = map_game_error(e.current_context());
                e.change_context(mapped)
            })?;

            let mut updated = Versioned::new(version, Game::Drafting(drafting));
            uow.games()
                .update(&mut updated)
                .await
                .map_err(IntoReport::into_report)
                .map_err(|err| {
                    let mapped = map_update_repo_err(&err);
                    err.change_context(mapped)
                })?;

            Ok::<_, Report<GameUseCaseError>>(updated.into_data())
        }
        .await;

        uow.finish_with(result, map_update_repo_err).await
    }
}

#[derive(Debug, Clone)]
pub struct DraftPromptInput {
    pub target_user_id: BenutzerId,
    pub text: NonEmptyString,
    pub source: PromptSource,
}

#[derive(Debug, Clone)]
pub struct SaveDraftPrompts {
    pub game_id: GameId,
    pub prompts: Vec<DraftPromptInput>,
}

#[async_trait]
impl UseCase<()> for SaveDraftPrompts {
    type Error = Report<GameUseCaseError>;

    #[instrument(skip_all, fields(game_id = ?self.game_id, prompt_count = self.prompts.len()), err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(GameUseCaseError::Unauthorized)?;
        let now = ctx.clock().now();
        let uow = ctx
            .enter()
            .await
            .change_context(GameUseCaseError::Persistence)?;

        let result = async {
            let game = match uow.games().find_by_id(self.game_id).await {
                Ok(g) => g,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(GameUseCaseError::NotFound));
                }
                Err(err) => return Err(err).change_context(GameUseCaseError::Persistence),
            };

            if !game.has_player(user_id) {
                return Err(Report::new(GameUseCaseError::Forbidden));
            }

            let version = game.v();
            let Game::Drafting(mut drafting) = game.into_data() else {
                return Err(Report::new(GameUseCaseError::Conflict));
            };

            let mut submissions = Vec::with_capacity(self.prompts.len());
            for item in self.prompts {
                if !drafting.has_player(item.target_user_id) {
                    return Err(Report::new(GameUseCaseError::BadRequest));
                }
                let sub = PromptSubmission::new(
                    PromptSubmissionId(Uuid::new_v4()),
                    user_id,
                    item.target_user_id,
                    item.text,
                    item.source,
                )
                .map_err(|e| {
                    let mapped = map_game_error(e.current_context());
                    e.change_context(mapped)
                })?;
                submissions.push(sub);
            }

            drafting
                .save_author_prompts(user_id, submissions, now)
                .map_err(|e| {
                    let mapped = map_game_error(e.current_context());
                    e.change_context(mapped)
                })?;

            let mut updated = Versioned::new(version, Game::Drafting(drafting));
            uow.games()
                .update(&mut updated)
                .await
                .map_err(IntoReport::into_report)
                .map_err(|err| {
                    let mapped = map_update_repo_err(&err);
                    err.change_context(mapped)
                })?;

            Ok::<_, Report<GameUseCaseError>>(())
        }
        .await;

        uow.finish_with(result, map_update_repo_err).await
    }
}

#[derive(Debug, Clone)]
pub struct LockDraftPrompts {
    pub game_id: GameId,
}

#[async_trait]
impl UseCase<Game> for LockDraftPrompts {
    type Error = Report<GameUseCaseError>;

    #[instrument(skip_all, fields(game_id = ?self.game_id), err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<Game, Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(GameUseCaseError::Unauthorized)?;
        let now = ctx.clock().now();
        let uow = ctx
            .enter()
            .await
            .change_context(GameUseCaseError::Persistence)?;

        let result = async {
            let game = match uow.games().find_by_id(self.game_id).await {
                Ok(g) => g,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(GameUseCaseError::NotFound));
                }
                Err(err) => return Err(err).change_context(GameUseCaseError::Persistence),
            };

            if !game.has_player(user_id) {
                return Err(Report::new(GameUseCaseError::Forbidden));
            }

            let version = game.v();
            let Game::Drafting(mut drafting) = game.into_data() else {
                return Err(Report::new(GameUseCaseError::Conflict));
            };

            drafting.lock_author_prompts(user_id, now).map_err(|e| {
                let mapped = map_game_error(e.current_context());
                e.change_context(mapped)
            })?;

            let mut updated = Versioned::new(version, Game::Drafting(drafting));
            uow.games()
                .update(&mut updated)
                .await
                .map_err(IntoReport::into_report)
                .map_err(|err| {
                    let mapped = map_update_repo_err(&err);
                    err.change_context(mapped)
                })?;

            Ok::<_, Report<GameUseCaseError>>(updated.into_data())
        }
        .await;

        uow.finish_with(result, map_update_repo_err).await
    }
}

#[derive(Debug, Clone)]
pub struct UnlockDraftPrompts {
    pub game_id: GameId,
}

#[async_trait]
impl UseCase<Game> for UnlockDraftPrompts {
    type Error = Report<GameUseCaseError>;

    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<Game, Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(GameUseCaseError::Unauthorized)?;
        let now = ctx.clock().now();
        let uow = ctx
            .enter()
            .await
            .change_context(GameUseCaseError::Persistence)?;

        let result = async {
            let game = match uow.games().find_by_id(self.game_id).await {
                Ok(g) => g,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(GameUseCaseError::NotFound));
                }
                Err(err) => return Err(err).change_context(GameUseCaseError::Persistence),
            };

            if !game.has_player(user_id) {
                return Err(Report::new(GameUseCaseError::Forbidden));
            }

            let version = game.v();
            let Game::Drafting(mut drafting) = game.into_data() else {
                return Err(Report::new(GameUseCaseError::Conflict));
            };

            drafting.unlock_author_prompts(user_id, now).map_err(|e| {
                let mapped = map_game_error(e.current_context());
                e.change_context(mapped)
            })?;

            let mut updated = Versioned::new(version, Game::Drafting(drafting));
            uow.games()
                .update(&mut updated)
                .await
                .map_err(IntoReport::into_report)
                .map_err(|err| {
                    let mapped = map_update_repo_err(&err);
                    err.change_context(mapped)
                })?;

            Ok::<_, Report<GameUseCaseError>>(updated.into_data())
        }
        .await;

        uow.finish_with(result, map_update_repo_err).await
    }
}

pub(crate) async fn persist_started_game(
    uow: &UnitOfWork<'_>,
    version: u64,
    active: ActiveGame,
    boards: Vec<NewBoard>,
) -> Result<Game, Report<GameUseCaseError>> {
    for new_board in boards {
        uow.boards()
            .create(new_board)
            .await
            .map_err(IntoReport::into_report)
            .map_err(|err| {
                if matches!(err.current_context(), RepositoryError::Conflict) {
                    err.change_context(GameUseCaseError::Conflict)
                } else {
                    err.change_context(GameUseCaseError::Persistence)
                }
            })?;
    }
    let mut updated = Versioned::new(version, Game::Active(active));
    uow.games()
        .update(&mut updated)
        .await
        .map_err(IntoReport::into_report)
        .map_err(|err| {
            let mapped = map_update_repo_err(&err);
            err.change_context(mapped)
        })?;
    Ok(updated.into_data())
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StartReadyGamesOutcome {
    pub started_games: Vec<GameId>,
    pub next_draft_closes_at: Option<DateTime<Utc>>,
}

enum StartCandidateOutcome {
    Started { mails: Vec<Mail> },
    Rescheduled(DateTime<Utc>),
    Skipped,
}

async fn collect_game_started_notifications(
    uow: &UnitOfWork<'_>,
    active: &ActiveGame,
    now: DateTime<Utc>,
    signing: &dyn crate::ports::Signing,
) -> Result<Vec<Mail>, Report<GameUseCaseError>> {
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
            uow,
            NeueBenachrichtigung::spiel_gestartet(uid, active.id(), now),
        )
        .await
        .map_err(IntoReport::into_report)
        .change_context(GameUseCaseError::Persistence)?;
    }

    let users = uow
        .benutzer()
        .find_by_ids(&watching_ids)
        .await
        .map_err(IntoReport::into_report)
        .change_context(GameUseCaseError::Persistence)?;
    let user_map: std::collections::HashMap<BenutzerId, Benutzer> = users
        .into_iter()
        .map(|b| {
            let b = b.into_data();
            (b.id(), b)
        })
        .collect();

    let mut mails = Vec::new();
    for uid in watching_ids {
        let Some(user) = user_map.get(&uid) else {
            continue;
        };
        let send_email = match uow
            .benachrichtigungs_einstellungen()
            .find_by_benutzer(uid)
            .await
        {
            Ok(prefs) => prefs.email_erlaubt(BenachrichtigungArt::SpielGestartet),
            Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => true,
            Err(err) => return Err(err).change_context(GameUseCaseError::Persistence),
        };
        if send_email {
            let token =
                AbbestellToken::erzeugen(uid, BenachrichtigungArt::SpielGestartet, now, signing);
            mails.push(Mail::spiel_gestartet(
                user.email_address().clone(),
                active.id(),
                active.title().as_str(),
                token,
            ));
        }
    }
    Ok(mails)
}

/// Finalize all `Drafting` games whose countdown (`draft_closes_at <= now`) has expired,
/// returning the started game IDs and the earliest remaining pending countdown deadline.
#[derive(Debug, Clone, Copy, Default)]
pub struct StartReadyGames;

#[async_trait]
impl UseCase<StartReadyGamesOutcome> for StartReadyGames {
    type Error = Report<GameUseCaseError>;

    async fn perform(
        self,
        ctx: ExecutionContext<'_>,
    ) -> Result<StartReadyGamesOutcome, Self::Error> {
        let now = ctx.clock().now();
        let read_uow = ctx
            .enter()
            .await
            .change_context(GameUseCaseError::Persistence)?;

        let scan_result = async {
            let candidates = read_uow
                .games()
                .find_drafting_with_countdown()
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            let mut ready_ids = Vec::new();
            let mut next_draft_closes_at: Option<DateTime<Utc>> = None;

            for versioned in candidates {
                let Game::Drafting(drafting) = versioned.into_data() else {
                    continue;
                };
                if drafting.is_ready_to_start(now) {
                    ready_ids.push(drafting.id());
                } else if let Some(at) = drafting.draft_closes_at() {
                    next_draft_closes_at = Some(match next_draft_closes_at {
                        Some(existing) if existing <= at => existing,
                        _ => at,
                    });
                }
            }

            Ok::<_, Report<GameUseCaseError>>((ready_ids, next_draft_closes_at))
        }
        .await;

        let (ready_ids, mut next_draft_closes_at) = read_uow
            .finish(scan_result, GameUseCaseError::Persistence)
            .await?;

        let mut started_games = Vec::new();

        for game_id in ready_ids {
            // Isolate each game's transition in its own UoW so a conflict rolls back only that game.
            let uow = ctx
                .enter()
                .await
                .change_context(GameUseCaseError::Persistence)?;

            let step_result = async {
                let versioned = match uow.games().find_by_id(game_id).await {
                    Ok(g) => g,
                    Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                        return Ok(StartCandidateOutcome::Skipped);
                    }
                    Err(err) => return Err(err).change_context(GameUseCaseError::Persistence),
                };

                let version = versioned.v();
                let Game::Drafting(drafting) = versioned.into_data() else {
                    return Ok(StartCandidateOutcome::Skipped);
                };
                if !drafting.is_ready_to_start(now) {
                    return Ok(match drafting.draft_closes_at() {
                        Some(at) => StartCandidateOutcome::Rescheduled(at),
                        None => StartCandidateOutcome::Skipped,
                    });
                }

                let outcome = {
                    let mut rng = ctx.rng();
                    drafting.finalize_ready_start(now, &mut *rng).map_err(|e| {
                        let mapped = map_game_error(e.current_context());
                        e.change_context(mapped)
                    })?
                };
                match outcome {
                    DraftStartOutcome::Started {
                        game: active,
                        boards,
                    } => {
                        let mails =
                            collect_game_started_notifications(&uow, &active, now, ctx.signing())
                                .await?;
                        persist_started_game(&uow, version, active, boards).await?;
                        Ok(StartCandidateOutcome::Started { mails })
                    }
                    _ => Ok(StartCandidateOutcome::Skipped),
                }
            }
            .await;

            match uow.finish_with(step_result, map_update_repo_err).await {
                Ok(StartCandidateOutcome::Started { mails }) => {
                    for mail in mails {
                        if let Err(err) = ctx.mail_sender().send(mail).await {
                            warn!(fehler = %err, "Benachrichtigungsmail zum Spielstart konnte nicht zugestellt werden");
                        }
                    }
                    started_games.push(game_id);
                }
                Ok(StartCandidateOutcome::Rescheduled(at)) => {
                    next_draft_closes_at = Some(match next_draft_closes_at {
                        Some(existing) if existing <= at => existing,
                        _ => at,
                    });
                }
                Ok(StartCandidateOutcome::Skipped) => {}
                Err(err) if matches!(err.current_context(), GameUseCaseError::Conflict) => {
                    debug!(
                        ?game_id,
                        "ready game start raced with concurrent update; skipping"
                    );
                }
                Err(err) => return Err(err),
            }
        }

        if !started_games.is_empty() {
            info!(count = started_games.len(), "started ready games");
        }

        Ok(StartReadyGamesOutcome {
            started_games,
            next_draft_closes_at,
        })
    }
}
