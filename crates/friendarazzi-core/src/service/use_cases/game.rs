//! Game lobby, joining, settings, drafting, and board assembly use cases.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, ResultExt};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::{
    application::{ExecutionContext, uow::UnitOfWork, uow::Versioned},
    domain::{
        ActiveGame, BenutzerId, DraftStartOutcome, Game, GameError, GameId, GridSize, InviteCode,
        MIN_PLAYERS, NewBoard, NewGame, NonEmptyString, PromptSource, PromptSubmission,
        PromptSubmissionId, VisibilityMode,
    },
    ports::RepositoryError,
    service::UseCase,
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

            let game = uow
                .games()
                .create(NewGame {
                    invite_code,
                    host_user_id: host_id,
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

            lobby.join_player(user_id, now);
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
                .change_context(GameUseCaseError::Persistence)?;

            Ok::<_, Report<GameUseCaseError>>(updated.into_data())
        }
        .await;

        uow.finish(result, GameUseCaseError::Persistence).await
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
                .change_context(GameUseCaseError::Persistence)?;

            Ok::<_, Report<GameUseCaseError>>(())
        }
        .await;

        uow.finish(result, GameUseCaseError::Persistence).await
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
                .change_context(GameUseCaseError::Persistence)?;

            Ok::<_, Report<GameUseCaseError>>(updated.into_data())
        }
        .await;

        uow.finish(result, GameUseCaseError::Persistence).await
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
                .change_context(GameUseCaseError::Persistence)?;

            Ok::<_, Report<GameUseCaseError>>(updated.into_data())
        }
        .await;

        uow.finish(result, GameUseCaseError::Persistence).await
    }
}

#[derive(Debug, Clone)]
pub struct FinalizeReadyGame {
    pub game_id: GameId,
}

#[async_trait]
impl UseCase<Game> for FinalizeReadyGame {
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
            let game_data = game.into_data();
            let Game::Drafting(drafting) = game_data else {
                return Ok(game_data);
            };

            let outcome = {
                let mut rng = ctx.rng();
                drafting.finalize_ready_start(now, &mut *rng).map_err(|e| {
                    let mapped = map_game_error(e.current_context());
                    e.change_context(mapped)
                })?
            };

            match outcome {
                DraftStartOutcome::StillDrafting(d) => Ok(Game::Drafting(d)),
                DraftStartOutcome::Started {
                    game: active,
                    boards,
                } => persist_started_game(&uow, version, active, boards).await,
            }
        }
        .await;

        uow.finish(result, GameUseCaseError::Persistence).await
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
            if matches!(
                err.current_context(),
                RepositoryError::VersionMismatch { .. } | RepositoryError::Conflict
            ) {
                err.change_context(GameUseCaseError::Conflict)
            } else {
                err.change_context(GameUseCaseError::Persistence)
            }
        })?;
    Ok(updated.into_data())
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StartReadyGamesOutcome {
    pub started_games: Vec<GameId>,
    pub next_auto_start_at: Option<DateTime<Utc>>,
}

/// Finalize all `Drafting` games whose countdown (`auto_start_at <= now`) has expired,
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
        let uow = ctx
            .enter()
            .await
            .change_context(GameUseCaseError::Persistence)?;

        let result = async {
            let candidates = uow
                .games()
                .find_drafting_with_countdown()
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            let mut started_games = Vec::new();
            let mut next_auto_start_at: Option<DateTime<Utc>> = None;

            for versioned in candidates {
                let version = versioned.v();
                let Game::Drafting(drafting) = versioned.into_data() else {
                    continue;
                };
                if !drafting.is_ready_to_start(now) {
                    if let Some(at) = drafting.auto_start_at() {
                        next_auto_start_at = Some(match next_auto_start_at {
                            Some(existing) if existing <= at => existing,
                            _ => at,
                        });
                    }
                    continue;
                }

                let game_id = drafting.id();
                let outcome = {
                    let mut rng = ctx.rng();
                    drafting.finalize_ready_start(now, &mut *rng).map_err(|e| {
                        let mapped = map_game_error(e.current_context());
                        e.change_context(mapped)
                    })?
                };
                if let DraftStartOutcome::Started {
                    game: active,
                    boards,
                } = outcome
                {
                    match persist_started_game(&uow, version, active, boards).await {
                        Ok(_) => started_games.push(game_id),
                        Err(err) if matches!(err.current_context(), GameUseCaseError::Conflict) => {
                            debug!(
                                ?game_id,
                                "ready game start raced with concurrent update; skipping"
                            );
                        }
                        Err(err) => return Err(err),
                    }
                }
            }

            if !started_games.is_empty() {
                info!(count = started_games.len(), "started ready games");
            }

            Ok::<_, Report<GameUseCaseError>>(StartReadyGamesOutcome {
                started_games,
                next_auto_start_at,
            })
        }
        .await;

        uow.finish(result, GameUseCaseError::Persistence).await
    }
}
