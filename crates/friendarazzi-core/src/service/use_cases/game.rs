//! Game lobby, joining, settings, drafting, and board assembly use cases.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, ResultExt};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::{
    application::{ExecutionContext, uow::UnitOfWork, uow::Versioned},
    domain::{
        ActiveGame, BenutzerId, Board, Capture, CompletionReason, DraftStartOutcome, Game,
        GameError, GameId, GridSize, InviteCode, MIN_PLAYERS, NewBoard, NewGame, NonEmptyString,
        PromptSource, PromptSubmission, PromptSubmissionId, PromptText, VisibilityMode,
    },
    ports::RepositoryError,
    service::{
        GameFinished, GameStarted, UseCase,
        use_cases::benachrichtigung::load_default_watch_for_user,
    },
};

const MAX_INVITE_CODE_ATTEMPTS: usize = 8;

#[derive(Debug, thiserror::Error)]
pub enum GameUseCaseError {
    #[error("für diese Aktion ist eine Anmeldung erforderlich")]
    Unauthorized,
    #[error("du bist nicht berechtigt, diese Aktion auszuführen")]
    Forbidden,
    #[error(
        "Bitte bestätige zuerst deine E-Mail-Adresse, um ein weiteres Spiel zu starten oder einem weiteren Spiel beizutreten"
    )]
    EmailVerificationRequired,
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
        | GameError::EmptyPromptText
        | GameError::PromptTooLong
        | GameError::DeadlineInPast
        | GameError::InvalidInviteCode
        | GameError::InvalidGridSize
        | GameError::InvalidVisibilityMode
        | GameError::InvalidPromptSource
        | GameError::InvalidCompletionReason
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
    pub draft_deadline_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
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
        if let Some(deadline) = self.draft_deadline_at
            && deadline <= now
        {
            return Err(Report::new(GameUseCaseError::BadRequest));
        }
        if let Some(end) = self.ends_at
            && (end <= now || self.draft_deadline_at.is_some_and(|draft| end <= draft))
        {
            return Err(Report::new(GameUseCaseError::BadRequest));
        }
        let uow = ctx
            .enter()
            .await
            .change_context(GameUseCaseError::Persistence)?;

        let result = async {
            ensure_unverified_game_quota(&uow, host_id).await?;
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
                    draft_deadline_at: self.draft_deadline_at,
                    ends_at: self.ends_at,
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

            ensure_unverified_game_quota(&uow, user_id).await?;

            let watching = load_default_watch_for_user(&uow, user_id)
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            let version = game.v();
            let updated = match game.into_data() {
                Game::Lobby(mut lobby) => {
                    lobby.join_player(user_id, now, watching);
                    let mut updated = Versioned::new(version, Game::Lobby(lobby));
                    uow.games()
                        .update(&mut updated)
                        .await
                        .map_err(IntoReport::into_report)
                        .map_err(|err| {
                            let mapped = map_update_repo_err(&err);
                            err.change_context(mapped)
                        })?;
                    updated.into_data()
                }
                Game::Active(mut active) => {
                    if active.is_deadline_reached(now) {
                        return Err(Report::new(GameUseCaseError::Conflict));
                    }
                    let decks = uow
                        .decks()
                        .find_all()
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(GameUseCaseError::Persistence)?;
                    let maybe_board = {
                        let mut rng = ctx.rng();
                        active
                            .join_late_player(user_id, now, watching, &decks, &mut *rng)
                            .map_err(|e| {
                                let mapped = map_game_error(e.current_context());
                                e.change_context(mapped)
                            })?
                    };
                    let mut updated = Versioned::new(version, Game::Active(active));
                    uow.games()
                        .update(&mut updated)
                        .await
                        .map_err(IntoReport::into_report)
                        .map_err(|err| {
                            let mapped = map_update_repo_err(&err);
                            err.change_context(mapped)
                        })?;
                    if let Some(new_board) = maybe_board {
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
                    updated.into_data()
                }
                Game::Drafting(_) | Game::Completed(_) => {
                    return Err(Report::new(GameUseCaseError::Conflict));
                }
            };

            Ok::<_, Report<GameUseCaseError>>(updated)
        }
        .await;

        uow.finish_with(result, map_update_repo_err).await
    }
}

#[derive(Debug, Clone)]
pub struct UpdateGameSettings {
    pub game_id: GameId,
    pub title: NonEmptyString,
    pub grid_size: GridSize,
    pub visibility_mode: VisibilityMode,
    pub draft_deadline_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
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

            if game.host_user_id() != user_id {
                return Err(Report::new(GameUseCaseError::Forbidden));
            }

            let version = game.v();
            let Game::Lobby(mut lobby) = game.into_data() else {
                return Err(Report::new(GameUseCaseError::Conflict));
            };

            lobby
                .update_settings(
                    self.title,
                    self.grid_size,
                    self.visibility_mode,
                    self.draft_deadline_at,
                    self.ends_at,
                    now,
                )
                .map_err(|e| {
                    let mapped = map_game_error(e.current_context());
                    e.change_context(mapped)
                })?;

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
    pub text: PromptText,
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

/// Host-only override to fill open prompt slots for all unlocked players using starter decks.
#[derive(Debug, Clone)]
pub struct HostFillRemainingDrafts {
    pub game_id: GameId,
    pub start_immediately: bool,
}

#[async_trait]
impl UseCase<Game> for HostFillRemainingDrafts {
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

            if game.host_user_id() != user_id {
                return Err(Report::new(GameUseCaseError::Forbidden));
            }

            let version = game.v();
            let Game::Drafting(mut drafting) = game.into_data() else {
                return Err(Report::new(GameUseCaseError::Conflict));
            };

            let decks = uow
                .decks()
                .find_all()
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            if self.start_immediately {
                let (active, boards) = {
                    let mut rng = ctx.rng();
                    drafting
                        .finalize_with_autofill(&decks, now, &mut *rng)
                        .map_err(|e| {
                            let mapped = map_game_error(e.current_context());
                            e.change_context(mapped)
                        })?
                };
                let started = persist_started_game(&uow, version, active, boards).await?;
                Ok::<_, Report<GameUseCaseError>>((started, true))
            } else {
                {
                    let mut rng = ctx.rng();
                    drafting
                        .autofill_missing_prompts(&decks, now, &mut *rng)
                        .map_err(|e| {
                            let mapped = map_game_error(e.current_context());
                            e.change_context(mapped)
                        })?;
                }
                let mut updated = Versioned::new(version, Game::Drafting(drafting));
                uow.games()
                    .update(&mut updated)
                    .await
                    .map_err(IntoReport::into_report)
                    .map_err(|err| {
                        let mapped = map_update_repo_err(&err);
                        err.change_context(mapped)
                    })?;
                Ok((updated.into_data(), false))
            }
        }
        .await;

        let (game, started_now) = uow.finish_with(result, map_update_repo_err).await?;
        if started_now {
            ctx.events().submit(GameStarted {
                game_id: self.game_id,
                started_at: now,
            });
        }
        Ok(game)
    }
}

/// Host-only update of the drafting deadline while in `Drafting`.
#[derive(Debug, Clone)]
pub struct SetDraftDeadline {
    pub game_id: GameId,
    pub draft_deadline_at: Option<DateTime<Utc>>,
}

#[async_trait]
impl UseCase<Game> for SetDraftDeadline {
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

            if game.host_user_id() != user_id {
                return Err(Report::new(GameUseCaseError::Forbidden));
            }

            let version = game.v();
            let Game::Drafting(mut drafting) = game.into_data() else {
                return Err(Report::new(GameUseCaseError::Conflict));
            };

            drafting
                .set_draft_deadline(self.draft_deadline_at, now)
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
    // Persist the game (and any auto-filled prompts) before creating boards whose tiles FK-reference prompt_submission(id).
    let mut updated = Versioned::new(version, Game::Active(active));
    uow.games()
        .update(&mut updated)
        .await
        .map_err(IntoReport::into_report)
        .map_err(|err| {
            let mapped = map_update_repo_err(&err);
            err.change_context(mapped)
        })?;
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
    Ok(updated.into_data())
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StartReadyGamesOutcome {
    pub started_games: Vec<GameId>,
    pub next_draft_closes_at: Option<DateTime<Utc>>,
}

enum StartCandidateOutcome {
    Started,
    Rescheduled(DateTime<Utc>),
    Skipped,
}

fn earliest_timestamp(
    current: Option<DateTime<Utc>>,
    candidate: Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    match (current, candidate) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

async fn ensure_unverified_game_quota(
    uow: &UnitOfWork<'_>,
    user_id: BenutzerId,
) -> Result<(), Report<GameUseCaseError>> {
    let benutzer = match uow.benutzer().find_by_id(user_id).await {
        Ok(b) => b,
        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
            return Err(Report::new(GameUseCaseError::Unauthorized));
        }
        Err(err) => return Err(err).change_context(GameUseCaseError::Persistence),
    };
    if benutzer.ist_verifiziert() {
        return Ok(());
    }
    let existing = uow
        .games()
        .find_by_participating_user(user_id)
        .await
        .map_err(IntoReport::into_report)
        .change_context(GameUseCaseError::Persistence)?;
    if !existing.is_empty() {
        return Err(Report::new(GameUseCaseError::EmailVerificationRequired));
    }
    Ok(())
}

/// Finalize all `Drafting` games whose countdown (`draft_closes_at <= now`) or draft deadline
/// (`draft_deadline_at <= now`) has expired, returning the started game IDs and earliest pending
/// deadline.
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
                if drafting.is_ready_to_start(now) || drafting.is_draft_deadline_reached(now) {
                    ready_ids.push(drafting.id());
                } else {
                    next_draft_closes_at =
                        earliest_timestamp(next_draft_closes_at, drafting.draft_closes_at());
                    next_draft_closes_at =
                        earliest_timestamp(next_draft_closes_at, drafting.draft_deadline_at());
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

                if drafting.is_draft_deadline_reached(now) {
                    let decks = uow
                        .decks()
                        .find_all()
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(GameUseCaseError::Persistence)?;
                    let (active, boards) = {
                        let mut rng = ctx.rng();
                        drafting
                            .finalize_with_autofill(&decks, now, &mut *rng)
                            .map_err(|e| {
                                let mapped = map_game_error(e.current_context());
                                e.change_context(mapped)
                            })?
                    };
                    persist_started_game(&uow, version, active, boards).await?;
                    return Ok(StartCandidateOutcome::Started);
                }

                if !drafting.is_ready_to_start(now) {
                    let next = earliest_timestamp(
                        drafting.draft_closes_at(),
                        drafting.draft_deadline_at(),
                    );
                    return Ok(match next {
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
                        persist_started_game(&uow, version, active, boards).await?;
                        Ok(StartCandidateOutcome::Started)
                    }
                    _ => Ok(StartCandidateOutcome::Skipped),
                }
            }
            .await;

            match uow.finish_with(step_result, map_update_repo_err).await {
                Ok(StartCandidateOutcome::Started) => {
                    ctx.events().submit(GameStarted {
                        game_id,
                        started_at: now,
                    });
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

/// Remove the caller from a game while still in Lobby or Drafting (auto-transferring host if needed).
#[derive(Debug, Clone)]
pub struct LeaveGame {
    pub game_id: GameId,
}

#[async_trait]
impl UseCase<Game> for LeaveGame {
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
            let next_game = match game.into_data() {
                Game::Lobby(mut lobby) => {
                    lobby.leave_player(user_id).map_err(|e| {
                        let mapped = map_game_error(e.current_context());
                        e.change_context(mapped)
                    })?;
                    Game::Lobby(lobby)
                }
                Game::Drafting(drafting) => drafting.leave_player(user_id, now).map_err(|e| {
                    let mapped = map_game_error(e.current_context());
                    e.change_context(mapped)
                })?,
                Game::Active(_) | Game::Completed(_) => {
                    return Err(Report::new(GameUseCaseError::Conflict));
                }
            };
            let mut updated = Versioned::new(version, next_game);
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

/// Host-only action removing a guest player while in Lobby or Drafting.
#[derive(Debug, Clone)]
pub struct KickPlayer {
    pub game_id: GameId,
    pub target_user_id: BenutzerId,
}

#[async_trait]
impl UseCase<Game> for KickPlayer {
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
            if game.host_user_id() != user_id {
                return Err(Report::new(GameUseCaseError::Forbidden));
            }
            let version = game.v();
            let next_game = match game.into_data() {
                Game::Lobby(mut lobby) => {
                    lobby.kick_player(self.target_user_id).map_err(|e| {
                        let mapped = match e.current_context() {
                            crate::domain::GameError::PlayerNotInGame => GameUseCaseError::NotFound,
                            other => map_game_error(other),
                        };
                        e.change_context(mapped)
                    })?;
                    Game::Lobby(lobby)
                }
                Game::Drafting(drafting) => drafting
                    .kick_player(self.target_user_id, now)
                    .map_err(|e| {
                        let mapped = match e.current_context() {
                            crate::domain::GameError::PlayerNotInGame => GameUseCaseError::NotFound,
                            other => map_game_error(other),
                        };
                        e.change_context(mapped)
                    })?,
                Game::Active(_) | Game::Completed(_) => {
                    return Err(Report::new(GameUseCaseError::Conflict));
                }
            };
            let mut updated = Versioned::new(version, next_game);
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

/// Host-only transfer of host privileges to another participant in Lobby, Drafting, or Active play.
#[derive(Debug, Clone)]
pub struct TransferHost {
    pub game_id: GameId,
    pub new_host_user_id: BenutzerId,
}

#[async_trait]
impl UseCase<Game> for TransferHost {
    type Error = Report<GameUseCaseError>;

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
            let mut game = match uow.games().find_by_id(self.game_id).await {
                Ok(g) => g,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(GameUseCaseError::NotFound));
                }
                Err(err) => return Err(err).change_context(GameUseCaseError::Persistence),
            };
            if game.host_user_id() != user_id {
                return Err(Report::new(GameUseCaseError::Forbidden));
            }
            if matches!(&*game, Game::Completed(_)) {
                return Err(Report::new(GameUseCaseError::Conflict));
            }
            if !game.has_player(self.new_host_user_id) {
                return Err(Report::new(GameUseCaseError::BadRequest));
            }
            game.transfer_host(user_id, self.new_host_user_id)
                .map_err(|e| {
                    let mapped = map_game_error(e.current_context());
                    e.change_context(mapped)
                })?;
            uow.games()
                .update(&mut game)
                .await
                .map_err(IntoReport::into_report)
                .map_err(|err| {
                    let mapped = map_update_repo_err(&err);
                    err.change_context(mapped)
                })?;
            Ok::<_, Report<GameUseCaseError>>(game.into_data())
        }
        .await;

        uow.finish_with(result, map_update_repo_err).await
    }
}

/// Host-only termination of a Lobby, Drafting, or Active game as `Abandoned` (no Gold Medal).
#[derive(Debug, Clone)]
pub struct AbandonGame {
    pub game_id: GameId,
}

#[async_trait]
impl UseCase<Game> for AbandonGame {
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
            if game.host_user_id() != user_id {
                return Err(Report::new(GameUseCaseError::Forbidden));
            }
            let version = game.v();
            let completed = match game.into_data() {
                Game::Lobby(g) => g.abandon(now),
                Game::Drafting(g) => g.abandon(now),
                Game::Active(g) => g.abandon(now),
                Game::Completed(_) => return Err(Report::new(GameUseCaseError::Conflict)),
            };
            let mut updated = Versioned::new(version, Game::Completed(completed));
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

        let out = uow.finish_with(result, map_update_repo_err).await?;
        ctx.events().submit(GameFinished {
            game_id: self.game_id,
            winner_id: None,
            completion_reason: CompletionReason::Abandoned,
            finished_at: now,
        });
        Ok(out)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FinishExpiredGamesOutcome {
    pub finished_games: Vec<GameId>,
    pub next_ends_at: Option<DateTime<Utc>>,
}

/// Finalize all `Active` games whose `ends_at <= now` by ranking filled tiles.
#[derive(Debug, Clone, Copy, Default)]
pub struct FinishExpiredGames;

#[async_trait]
impl UseCase<FinishExpiredGamesOutcome> for FinishExpiredGames {
    type Error = Report<GameUseCaseError>;

    async fn perform(
        self,
        ctx: ExecutionContext<'_>,
    ) -> Result<FinishExpiredGamesOutcome, Self::Error> {
        let now = ctx.clock().now();
        let read_uow = ctx
            .enter()
            .await
            .change_context(GameUseCaseError::Persistence)?;

        let scan = async {
            let candidates = read_uow
                .games()
                .find_active_with_deadline()
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            let mut expired = Vec::new();
            let mut next_ends_at: Option<DateTime<Utc>> = None;
            for versioned in candidates {
                let Game::Active(active) = versioned.into_data() else {
                    continue;
                };
                if active.is_deadline_reached(now) {
                    expired.push(active.id());
                } else {
                    next_ends_at = earliest_timestamp(next_ends_at, active.ends_at());
                }
            }
            Ok::<_, Report<GameUseCaseError>>((expired, next_ends_at))
        }
        .await;

        let (expired_ids, mut next_ends_at) =
            read_uow.finish(scan, GameUseCaseError::Persistence).await?;

        let mut finished_games = Vec::new();
        for game_id in expired_ids {
            let uow = ctx
                .enter()
                .await
                .change_context(GameUseCaseError::Persistence)?;

            let step = async {
                let versioned = match uow.games().find_by_id(game_id).await {
                    Ok(g) => g,
                    Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                        return Ok(None);
                    }
                    Err(err) => return Err(err).change_context(GameUseCaseError::Persistence),
                };
                let version = versioned.v();
                let Game::Active(active) = versioned.into_data() else {
                    return Ok(None);
                };
                if !active.is_deadline_reached(now) {
                    next_ends_at = earliest_timestamp(next_ends_at, active.ends_at());
                    return Ok(None);
                }

                let boards: Vec<Board> = uow
                    .boards()
                    .find_by_game(game_id)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(GameUseCaseError::Persistence)?
                    .into_iter()
                    .map(|b| b.into_data())
                    .collect();
                let captures: Vec<Capture> = uow
                    .captures()
                    .find_by_game(game_id)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(GameUseCaseError::Persistence)?
                    .into_iter()
                    .map(|c| c.into_data())
                    .collect();

                let completed = active.complete_by_deadline(&boards, &captures, now);
                let winner_id = completed.winner_user_id();
                let mut updated = Versioned::new(version, Game::Completed(completed));
                uow.games()
                    .update(&mut updated)
                    .await
                    .map_err(IntoReport::into_report)
                    .map_err(|err| {
                        let mapped = map_update_repo_err(&err);
                        err.change_context(mapped)
                    })?;

                if let Some(uid) = winner_id {
                    let mut winner = uow
                        .benutzer()
                        .find_by_id(uid)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(GameUseCaseError::Persistence)?;
                    winner.award_gold_medal();
                    uow.benutzer()
                        .update(&mut winner)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(GameUseCaseError::Persistence)?;
                }

                Ok::<_, Report<GameUseCaseError>>(Some(winner_id))
            }
            .await;

            match uow.finish_with(step, map_update_repo_err).await {
                Ok(Some(winner_id)) => {
                    ctx.events().submit(GameFinished {
                        game_id,
                        winner_id,
                        completion_reason: CompletionReason::Deadline,
                        finished_at: now,
                    });
                    finished_games.push(game_id);
                }
                Ok(None) => {}
                Err(err) if matches!(err.current_context(), GameUseCaseError::Conflict) => {
                    debug!(
                        ?game_id,
                        "expired game completion raced with update; skipping"
                    );
                }
                Err(err) => return Err(err),
            }
        }

        Ok(FinishExpiredGamesOutcome {
            finished_games,
            next_ends_at,
        })
    }
}
