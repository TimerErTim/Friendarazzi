//! Game lobby, joining, settings, drafting, and board assembly use cases.

use async_trait::async_trait;
use error_stack::{IntoReport, Report, ResultExt};
use rand::rngs::OsRng;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    application::ExecutionContext,
    domain::{
        Game, GameError, GameId, GamePlayer, GameStatus, GridSize, InviteCode, MIN_PLAYERS,
        NewGame, NonEmptyString, PromptSource, PromptSubmission, PromptSubmissionId,
        VisibilityMode, assemble_boards, validate_author_prompts,
    },
    ports::RepositoryError,
    service::UseCase,
};

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
        GameError::InvalidStateTransition | GameError::BoardAssemblyFailed => {
            GameUseCaseError::Conflict
        }
        GameError::TooFewPlayers => GameUseCaseError::TooFewPlayers,
    }
}

#[derive(Debug, Clone)]
pub struct CreateGame {
    pub title: NonEmptyString,
    pub grid_size: GridSize,
    pub visibility_mode: VisibilityMode,
    pub include_free_space: bool,
}

#[async_trait]
impl UseCase<Game> for CreateGame {
    type Error = Report<GameUseCaseError>;

    #[instrument(skip_all, err(Debug))]
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
            let mut rng = OsRng;
            let mut invite_code = InviteCode::generate(&mut rng);
            for _ in 0..8 {
                if uow.games().find_by_invite_code(&invite_code).await.is_err() {
                    break;
                }
                invite_code = InviteCode::generate(&mut rng);
            }

            let game = uow
                .games()
                .create(NewGame {
                    invite_code,
                    host_user_id: host_id,
                    title: self.title,
                    grid_size: self.grid_size,
                    visibility_mode: self.visibility_mode,
                    include_free_space: self.include_free_space,
                    created_at: now,
                })
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            uow.games()
                .add_player(GamePlayer {
                    game_id: game.id(),
                    user_id: host_id,
                    joined_at: now,
                    draft_locked: false,
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

    #[instrument(skip_all, err(Debug))]
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

            let players = uow
                .games()
                .find_players(game.id())
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            if players.iter().any(|p| p.user_id == user_id) {
                return Ok(game.into_data());
            }

            if game.status() != GameStatus::Lobby {
                return Err(Report::new(GameUseCaseError::Conflict));
            }

            uow.games()
                .add_player(GamePlayer {
                    game_id: game.id(),
                    user_id,
                    joined_at: now,
                    draft_locked: false,
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
pub struct UpdateGameSettings {
    pub game_id: GameId,
    pub title: NonEmptyString,
    pub grid_size: GridSize,
    pub visibility_mode: VisibilityMode,
    pub include_free_space: bool,
}

#[async_trait]
impl UseCase<Game> for UpdateGameSettings {
    type Error = Report<GameUseCaseError>;

    #[instrument(skip_all, err(Debug))]
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

            game.update_settings(
                self.title,
                self.grid_size,
                self.visibility_mode,
                self.include_free_space,
            )
            .map_err(|e| {
                let mapped = map_game_error(e.current_context());
                e.change_context(mapped)
            })?;

            uow.games()
                .update(&mut game)
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
pub struct StartDrafting {
    pub game_id: GameId,
}

#[async_trait]
impl UseCase<Game> for StartDrafting {
    type Error = Report<GameUseCaseError>;

    #[instrument(skip_all, err(Debug))]
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

            let players = uow
                .games()
                .find_players(self.game_id)
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            game.start_drafting(players.len()).map_err(|e| {
                let mapped = map_game_error(e.current_context());
                e.change_context(mapped)
            })?;

            uow.games()
                .update(&mut game)
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
pub struct DraftPromptInput {
    pub target_user_id: crate::domain::konto::BenutzerId,
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

    #[instrument(skip_all, err(Debug))]
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

            if game.status() != GameStatus::Drafting {
                return Err(Report::new(GameUseCaseError::Conflict));
            }

            let players = uow
                .games()
                .find_players(self.game_id)
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            if !players.iter().any(|p| p.user_id == user_id) {
                return Err(Report::new(GameUseCaseError::Forbidden));
            }

            let player_ids: Vec<_> = players.iter().map(|p| p.user_id).collect();
            let mut submissions = Vec::with_capacity(self.prompts.len());
            for item in self.prompts {
                if !player_ids.contains(&item.target_user_id) {
                    return Err(Report::new(GameUseCaseError::BadRequest));
                }
                let sub = PromptSubmission::new(
                    PromptSubmissionId(Uuid::new_v4()),
                    self.game_id,
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

            uow.games()
                .replace_author_prompts(self.game_id, user_id, submissions)
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            uow.games()
                .set_player_draft_locked(self.game_id, user_id, false)
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

    #[instrument(skip_all, err(Debug))]
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
            let mut game = match uow.games().find_by_id(self.game_id).await {
                Ok(g) => g,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(GameUseCaseError::NotFound));
                }
                Err(err) => return Err(err).change_context(GameUseCaseError::Persistence),
            };

            if game.status() != GameStatus::Drafting {
                return Err(Report::new(GameUseCaseError::Conflict));
            }

            let mut players = uow
                .games()
                .find_players(self.game_id)
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            if !players.iter().any(|p| p.user_id == user_id) {
                return Err(Report::new(GameUseCaseError::Forbidden));
            }

            // Sort deterministically by (joined_at, user_id) so cyclic target quotas match UI & backend.
            players.sort_by(|a, b| {
                a.joined_at
                    .cmp(&b.joined_at)
                    .then(a.user_id.cmp(&b.user_id))
            });
            let player_ids: Vec<_> = players.iter().map(|p| p.user_id).collect();

            let my_prompts = uow
                .games()
                .find_prompts_by_author(self.game_id, user_id)
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            validate_author_prompts(
                &player_ids,
                user_id,
                game.playable_tile_count(),
                &my_prompts,
            )
            .map_err(|e| {
                let mapped = map_game_error(e.current_context());
                e.change_context(mapped)
            })?;

            uow.games()
                .set_player_draft_locked(self.game_id, user_id, true)
                .await
                .map_err(IntoReport::into_report)
                .change_context(GameUseCaseError::Persistence)?;

            for p in &mut players {
                if p.user_id == user_id {
                    p.draft_locked = true;
                }
            }

            if players.iter().all(|p| p.draft_locked) {
                let all_prompts = uow
                    .games()
                    .find_prompts_by_game(self.game_id)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(GameUseCaseError::Persistence)?;

                let mut rng = OsRng;
                let boards =
                    assemble_boards(&game, &player_ids, &all_prompts, &mut rng).map_err(|e| {
                        let mapped = map_game_error(e.current_context());
                        e.change_context(mapped)
                    })?;

                for (board, tiles) in boards {
                    uow.boards()
                        .create_board_with_tiles(board, tiles)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(GameUseCaseError::Persistence)?;
                }

                game.start_active(now).map_err(|e| {
                    let mapped = map_game_error(e.current_context());
                    e.change_context(mapped)
                })?;

                uow.games()
                    .update(&mut game)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(GameUseCaseError::Persistence)?;
            }

            Ok::<_, Report<GameUseCaseError>>(game.into_data())
        }
        .await;

        uow.finish(result, GameUseCaseError::Persistence).await
    }
}
