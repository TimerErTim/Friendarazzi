//! Game lobby, joining, settings, drafting, and board assembly use cases.

use async_trait::async_trait;
use error_stack::{IntoReport, Report, ResultExt};
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::{
    application::ExecutionContext,
    domain::{
        DraftLockOutcome, Game, GameError, GameId, GridSize, InviteCode, MIN_PLAYERS, NewGame,
        NonEmptyString, PromptSource, PromptSubmission, PromptSubmissionId, VisibilityMode,
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

fn map_game_error(err: &GameError) -> GameUseCaseError {
    match err {
        GameError::EmptyTitle
        | GameError::InvalidInviteCode
        | GameError::InvalidGridSize
        | GameError::InvalidVisibilityMode
        | GameError::InvalidStatus
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
            let mut rng = ctx.rng();
            let mut invite_code = None;
            for attempt in 1..=MAX_INVITE_CODE_ATTEMPTS {
                let candidate = InviteCode::generate(&mut rng);
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
                    warn!(
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
            let mut updated = crate::application::uow::Versioned::new(version, Game::Lobby(lobby));
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

            let mut updated = crate::application::uow::Versioned::new(version, Game::Lobby(lobby));
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

            let mut updated =
                crate::application::uow::Versioned::new(version, Game::Drafting(drafting));
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
    pub target_user_id: crate::domain::BenutzerId,
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
                .save_author_prompts(user_id, submissions)
                .map_err(|e| {
                    let mapped = map_game_error(e.current_context());
                    e.change_context(mapped)
                })?;

            let mut updated =
                crate::application::uow::Versioned::new(version, Game::Drafting(drafting));
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
            let Game::Drafting(drafting) = game.into_data() else {
                return Err(Report::new(GameUseCaseError::Conflict));
            };

            let mut rng = ctx.rng();
            let outcome = drafting
                .lock_author_prompts(user_id, now, &mut rng)
                .map_err(|e| {
                    let mapped = map_game_error(e.current_context());
                    e.change_context(mapped)
                })?;

            let next_game = match outcome {
                DraftLockOutcome::StillDrafting(d) => Game::Drafting(d),
                DraftLockOutcome::Started {
                    game: active,
                    boards,
                } => {
                    for new_board in boards {
                        uow.boards()
                            .create(new_board)
                            .await
                            .map_err(IntoReport::into_report)
                            .change_context(GameUseCaseError::Persistence)?;
                    }
                    Game::Active(active)
                }
            };

            let mut updated = crate::application::uow::Versioned::new(version, next_game);
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
