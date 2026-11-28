use std::{collections::HashSet, sync::Arc};

use async_lock::Mutex;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::ResultExt;
use friendarazzi_core::{
    ErrorReportExt,
    application::uow::Versioned,
    domain::{
        BenutzerId, CompletionReason, Game, GameId, GameIn, GamePlayer, GameState, InviteCode,
        NewGame, PromptSource, PromptSubmission, PromptSubmissionId, PromptText,
    },
    ports::{GameRepository, RepositoryError, RepositoryResult},
};
use libsql::{Row, Transaction};
use uuid::Uuid;

use crate::{
    errors::libsql_error_to_persistence_error,
    repos::common::{format_datetime, parse_datetime, parse_uuid, parse_version, update_versioned},
};

const SELECT_GAME: &str = r#"SELECT id, invite_code, host_user_id, title, grid_size, visibility_mode, status, draft_closes_at, draft_deadline_at, ends_at, started_at, ended_at, winner_user_id, completion_reason, created_at, _version FROM "game""#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DbGameStatus {
    Lobby,
    Drafting,
    Active,
    Completed,
}

impl DbGameStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Lobby => "Lobby",
            Self::Drafting => "Drafting",
            Self::Active => "Active",
            Self::Completed => "Completed",
        }
    }

    fn parse(raw: &str) -> RepositoryResult<Self> {
        match raw {
            "Lobby" => Ok(Self::Lobby),
            "Drafting" => Ok(Self::Drafting),
            "Active" => Ok(Self::Active),
            "Completed" => Ok(Self::Completed),
            _ => Err(RepositoryError::Data.into()),
        }
    }
}

fn state_columns(
    game: &Game,
) -> (
    &'static str,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<&'static str>,
) {
    match game {
        Game::Lobby(_) => (DbGameStatus::Lobby.as_str(), None, None, None, None, None),
        Game::Drafting(d) => (
            DbGameStatus::Drafting.as_str(),
            d.draft_closes_at().map(format_datetime),
            None,
            None,
            None,
            None,
        ),
        Game::Active(a) => (
            DbGameStatus::Active.as_str(),
            None,
            Some(format_datetime(a.started_at())),
            None,
            None,
            None,
        ),
        Game::Completed(c) => (
            DbGameStatus::Completed.as_str(),
            None,
            Some(format_datetime(c.started_at())),
            Some(format_datetime(c.ended_at())),
            c.winner_user_id().map(|id| id.0.to_string()),
            Some(c.completion_reason().as_str()),
        ),
    }
}

async fn load_players(
    tx: &Transaction,
    game_id: GameId,
) -> RepositoryResult<(Vec<GamePlayer>, HashSet<BenutzerId>)> {
    let mut rows = tx
        .query(
            r#"SELECT user_id, joined_at, draft_locked, watching FROM "game_player" WHERE game_id = ?1 ORDER BY joined_at ASC, user_id ASC"#,
            [game_id.0.to_string()],
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;

    let mut players = Vec::new();
    let mut draft_locked = HashSet::new();
    while let Some(row) = rows
        .next()
        .await
        .contextualize_with(libsql_error_to_persistence_error)?
    {
        let uid_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
        let joined_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
        let locked_raw: i64 = row.get(2).contextualize(RepositoryError::Data)?;
        let watching_raw: i64 = row.get(3).contextualize(RepositoryError::Data)?;
        let user_id = BenutzerId(parse_uuid(&uid_raw)?);
        if locked_raw != 0 {
            draft_locked.insert(user_id);
        }
        players.push(GamePlayer {
            user_id,
            joined_at: parse_datetime(&joined_raw)?,
            watching: watching_raw != 0,
        });
    }
    Ok((players, draft_locked))
}

async fn load_prompts(
    tx: &Transaction,
    game_id: GameId,
) -> RepositoryResult<Vec<PromptSubmission>> {
    let mut rows = tx
        .query(
            r#"SELECT id, author_user_id, target_user_id, text, source FROM "prompt_submission" WHERE game_id = ?1"#,
            [game_id.0.to_string()],
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;

    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .await
        .contextualize_with(libsql_error_to_persistence_error)?
    {
        let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
        let author_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
        let target_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
        let text_raw: String = row.get(3).contextualize(RepositoryError::Data)?;
        let source_raw: String = row.get(4).contextualize(RepositoryError::Data)?;

        let text = PromptText::new(text_raw).change_context(RepositoryError::Data)?;
        let source: PromptSource = source_raw.parse().change_context(RepositoryError::Data)?;

        let sub = PromptSubmission::new(
            PromptSubmissionId(parse_uuid(&id_raw)?),
            BenutzerId(parse_uuid(&author_raw)?),
            BenutzerId(parse_uuid(&target_raw)?),
            text,
            source,
        )
        .change_context(RepositoryError::Data)?;
        out.push(sub);
    }
    Ok(out)
}

struct GameRow {
    id: GameId,
    invite_code: String,
    host_user_id: BenutzerId,
    title: String,
    grid_size: u8,
    visibility_mode: String,
    status: DbGameStatus,
    draft_closes_at: Option<DateTime<Utc>>,
    draft_deadline_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    winner_user_id: Option<BenutzerId>,
    completion_reason: Option<CompletionReason>,
    created_at: DateTime<Utc>,
    version: u64,
}

fn read_game_row(row: &Row) -> RepositoryResult<GameRow> {
    let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let invite_code: String = row.get(1).contextualize(RepositoryError::Data)?;
    let host_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
    let title: String = row.get(3).contextualize(RepositoryError::Data)?;
    let grid_size_raw: i64 = row.get(4).contextualize(RepositoryError::Data)?;
    let visibility_mode: String = row.get(5).contextualize(RepositoryError::Data)?;
    let status_raw: String = row.get(6).contextualize(RepositoryError::Data)?;
    let draft_closes_at_raw: Option<String> = row.get(7).contextualize(RepositoryError::Data)?;
    let draft_deadline_at_raw: Option<String> = row.get(8).contextualize(RepositoryError::Data)?;
    let ends_at_raw: Option<String> = row.get(9).contextualize(RepositoryError::Data)?;
    let started_at_raw: Option<String> = row.get(10).contextualize(RepositoryError::Data)?;
    let ended_at_raw: Option<String> = row.get(11).contextualize(RepositoryError::Data)?;
    let winner_raw: Option<String> = row.get(12).contextualize(RepositoryError::Data)?;
    let reason_raw: Option<String> = row.get(13).contextualize(RepositoryError::Data)?;
    let created_at_raw: String = row.get(14).contextualize(RepositoryError::Data)?;
    let version = parse_version(row, 15)?;

    Ok(GameRow {
        id: GameId(parse_uuid(&id_raw)?),
        invite_code,
        host_user_id: BenutzerId(parse_uuid(&host_raw)?),
        title,
        grid_size: u8::try_from(grid_size_raw).contextualize(RepositoryError::Data)?,
        visibility_mode,
        status: DbGameStatus::parse(&status_raw)?,
        draft_closes_at: draft_closes_at_raw
            .map(|s| parse_datetime(&s))
            .transpose()?,
        draft_deadline_at: draft_deadline_at_raw
            .map(|s| parse_datetime(&s))
            .transpose()?,
        ends_at: ends_at_raw.map(|s| parse_datetime(&s)).transpose()?,
        started_at: started_at_raw.map(|s| parse_datetime(&s)).transpose()?,
        ended_at: ended_at_raw.map(|s| parse_datetime(&s)).transpose()?,
        winner_user_id: winner_raw
            .map(|s| parse_uuid(&s))
            .transpose()?
            .map(BenutzerId),
        completion_reason: reason_raw
            .map(|s| s.parse())
            .transpose()
            .change_context(RepositoryError::Data)?,
        created_at: parse_datetime(&created_at_raw)?,
        version,
    })
}

async fn hydrate_game(tx: &Transaction, row: GameRow) -> RepositoryResult<Versioned<Game>> {
    let (players, draft_locked) = load_players(tx, row.id).await?;
    let prompts = if row.status == DbGameStatus::Lobby {
        Vec::new()
    } else {
        load_prompts(tx, row.id).await?
    };

    let game = match row.status {
        DbGameStatus::Lobby => Game::Lobby(
            GameIn::from_parts(
                row.id,
                row.invite_code,
                row.host_user_id,
                row.title,
                row.grid_size,
                row.visibility_mode,
                row.draft_deadline_at,
                row.ends_at,
                players,
                row.created_at,
                GameState::Lobby,
            )
            .change_context(RepositoryError::Data)?,
        ),
        DbGameStatus::Drafting => Game::Drafting(
            GameIn::from_parts(
                row.id,
                row.invite_code,
                row.host_user_id,
                row.title,
                row.grid_size,
                row.visibility_mode,
                row.draft_deadline_at,
                row.ends_at,
                players,
                row.created_at,
                GameState::Drafting {
                    prompts,
                    draft_locked,
                    draft_closes_at: row.draft_closes_at,
                },
            )
            .change_context(RepositoryError::Data)?,
        ),
        DbGameStatus::Active => {
            let started_at = row.started_at.ok_or(RepositoryError::Data)?;
            Game::Active(
                GameIn::from_parts(
                    row.id,
                    row.invite_code,
                    row.host_user_id,
                    row.title,
                    row.grid_size,
                    row.visibility_mode,
                    row.draft_deadline_at,
                    row.ends_at,
                    players,
                    row.created_at,
                    GameState::Active {
                        prompts,
                        started_at,
                    },
                )
                .change_context(RepositoryError::Data)?,
            )
        }
        DbGameStatus::Completed => {
            let started_at = row.started_at.ok_or(RepositoryError::Data)?;
            let ended_at = row.ended_at.ok_or(RepositoryError::Data)?;
            let completion_reason = row.completion_reason.unwrap_or(CompletionReason::Bingo);
            Game::Completed(
                GameIn::from_parts(
                    row.id,
                    row.invite_code,
                    row.host_user_id,
                    row.title,
                    row.grid_size,
                    row.visibility_mode,
                    row.draft_deadline_at,
                    row.ends_at,
                    players,
                    row.created_at,
                    GameState::Completed {
                        prompts,
                        started_at,
                        ended_at,
                        winner_user_id: row.winner_user_id,
                        completion_reason,
                    },
                )
                .change_context(RepositoryError::Data)?,
            )
        }
    };

    Ok(Versioned::new(row.version, game))
}

pub struct SQLiteGameRepository {
    pub(crate) tx: Arc<Mutex<Option<Transaction>>>,
}

#[async_trait]
impl GameRepository for SQLiteGameRepository {
    async fn create(&self, game: NewGame) -> RepositoryResult<Versioned<Game>> {
        let id = GameId(Uuid::new_v4());
        let domain = Game::new(id, game);
        let (status, draft_closes_at, started_at, ended_at, winner_user_id, completion_reason) =
            state_columns(&domain);
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"INSERT INTO "game" (id, invite_code, host_user_id, title, grid_size, visibility_mode, status, draft_closes_at, draft_deadline_at, ends_at, started_at, ended_at, winner_user_id, completion_reason, created_at, _version)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 0)"#,
            (
                id.0.to_string(),
                domain.invite_code().as_str(),
                domain.host_user_id().0.to_string(),
                domain.title().as_str(),
                i64::from(domain.grid_size() as u8),
                domain.visibility_mode().as_str(),
                status,
                draft_closes_at,
                domain.draft_deadline_at().map(format_datetime),
                domain.ends_at().map(format_datetime),
                started_at,
                ended_at,
                winner_user_id,
                completion_reason,
                format_datetime(domain.created_at()),
            ),
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;

        for player in domain.players() {
            tx.execute(
                r#"INSERT INTO "game_player" (game_id, user_id, joined_at, draft_locked, watching) VALUES (?1, ?2, ?3, 0, ?4)"#,
                (
                    id.0.to_string(),
                    player.user_id.0.to_string(),
                    format_datetime(player.joined_at),
                    i64::from(player.watching),
                ),
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;
        }

        Ok(Versioned::init(domain))
    }

    async fn find_by_id(&self, id: GameId) -> RepositoryResult<Versioned<Game>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(r#"{SELECT_GAME} WHERE id = ?1"#),
                [id.0.to_string()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        else {
            return Err(RepositoryError::NotFound.into());
        };
        let game_row = read_game_row(&row)?;
        drop(rows);
        hydrate_game(tx, game_row).await
    }

    async fn find_by_invite_code(&self, code: &InviteCode) -> RepositoryResult<Versioned<Game>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(r#"{SELECT_GAME} WHERE invite_code = ?1"#),
                [code.as_str()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        else {
            return Err(RepositoryError::NotFound.into());
        };
        let game_row = read_game_row(&row)?;
        drop(rows);
        hydrate_game(tx, game_row).await
    }

    async fn find_by_participating_user(
        &self,
        user_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Game>>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let rows = tx
            .query(
                r#"SELECT g.id, g.invite_code, g.host_user_id, g.title, g.grid_size, g.visibility_mode, g.status, g.draft_closes_at, g.draft_deadline_at, g.ends_at, g.started_at, g.ended_at, g.winner_user_id, g.completion_reason, g.created_at, g._version
                   FROM "game" g
                   INNER JOIN "game_player" gp ON gp.game_id = g.id
                   WHERE gp.user_id = ?1
                   ORDER BY g.created_at DESC"#,
                [user_id.0.to_string()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        hydrate_game_rows(tx, rows).await
    }

    async fn find_drafting_with_countdown(&self) -> RepositoryResult<Vec<Versioned<Game>>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let rows = tx
            .query(
                &format!(
                    r#"{SELECT_GAME} WHERE status = ?1 AND (draft_closes_at IS NOT NULL OR draft_deadline_at IS NOT NULL) ORDER BY COALESCE(draft_closes_at, draft_deadline_at) ASC"#
                ),
                [DbGameStatus::Drafting.as_str()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        hydrate_game_rows(tx, rows).await
    }

    async fn find_active_with_deadline(&self) -> RepositoryResult<Vec<Versioned<Game>>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let rows = tx
            .query(
                &format!(
                    r#"{SELECT_GAME} WHERE status = ?1 AND ends_at IS NOT NULL ORDER BY ends_at ASC"#
                ),
                [DbGameStatus::Active.as_str()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        hydrate_game_rows(tx, rows).await
    }

    async fn update(&self, game: &mut Versioned<Game>) -> RepositoryResult<()> {
        let (status, draft_closes_at, started_at, ended_at, winner_user_id, completion_reason) =
            state_columns(game);
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        update_versioned(
            tx,
            r#"UPDATE "game"
               SET host_user_id = ?2, title = ?3, grid_size = ?4, visibility_mode = ?5, status = ?6,
                   draft_closes_at = ?7, draft_deadline_at = ?8, ends_at = ?9, started_at = ?10,
                   ended_at = ?11, winner_user_id = ?12, completion_reason = ?13, _version = ?14 + 1
               WHERE id = ?1 AND _version = ?14"#,
            (
                game.id().0.to_string(),
                game.host_user_id().0.to_string(),
                game.title().as_str(),
                i64::from(game.grid_size() as u8),
                game.visibility_mode().as_str(),
                status,
                draft_closes_at,
                game.draft_deadline_at().map(format_datetime),
                game.ends_at().map(format_datetime),
                started_at,
                ended_at,
                winner_user_id,
                completion_reason,
                game.v() as i64,
            ),
            game.v(),
        )
        .await?;

        let draft_locked = match &**game {
            Game::Drafting(d) => Some(d.draft_locked()),
            _ => None,
        };

        let existing_ids: HashSet<BenutzerId> = load_players(tx, game.id())
            .await?
            .0
            .into_iter()
            .map(|p| p.user_id)
            .collect();
        let target_ids: HashSet<BenutzerId> = game.players().iter().map(|p| p.user_id).collect();
        for removed in existing_ids.difference(&target_ids) {
            tx.execute(
                r#"DELETE FROM "game_player" WHERE game_id = ?1 AND user_id = ?2"#,
                (game.id().0.to_string(), removed.0.to_string()),
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;
        }

        for player in game.players() {
            let is_locked = draft_locked.is_some_and(|set| set.contains(&player.user_id));
            tx.execute(
                r#"INSERT INTO "game_player" (game_id, user_id, joined_at, draft_locked, watching)
                   VALUES (?1, ?2, ?3, ?4, ?5)
                   ON CONFLICT (game_id, user_id) DO UPDATE SET draft_locked = excluded.draft_locked, watching = excluded.watching"#,
                (
                    game.id().0.to_string(),
                    player.user_id.0.to_string(),
                    format_datetime(player.joined_at),
                    i64::from(is_locked),
                    i64::from(player.watching),
                ),
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;
        }

        // Lobby clears drafts; Drafting replaces drafts; Active inserts newly created prompts (e.g. autofill or late join).
        match &**game {
            Game::Lobby(_) => {
                tx.execute(
                    r#"DELETE FROM "prompt_submission" WHERE game_id = ?1"#,
                    [game.id().0.to_string()],
                )
                .await
                .contextualize_with(libsql_error_to_persistence_error)?;
            }
            Game::Drafting(drafting) => {
                tx.execute(
                    r#"DELETE FROM "prompt_submission" WHERE game_id = ?1"#,
                    [game.id().0.to_string()],
                )
                .await
                .contextualize_with(libsql_error_to_persistence_error)?;

                for p in drafting.prompts() {
                    tx.execute(
                        r#"INSERT INTO "prompt_submission" (id, game_id, author_user_id, target_user_id, text, source)
                           VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
                        (
                            p.id.0.to_string(),
                            game.id().0.to_string(),
                            p.author_user_id.0.to_string(),
                            p.target_user_id.0.to_string(),
                            p.text.as_str(),
                            p.source.as_str(),
                        ),
                    )
                    .await
                    .contextualize_with(libsql_error_to_persistence_error)?;
                }
            }
            Game::Active(active) => {
                for p in active.prompts() {
                    tx.execute(
                        r#"INSERT OR IGNORE INTO "prompt_submission" (id, game_id, author_user_id, target_user_id, text, source)
                           VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
                        (
                            p.id.0.to_string(),
                            game.id().0.to_string(),
                            p.author_user_id.0.to_string(),
                            p.target_user_id.0.to_string(),
                            p.text.as_str(),
                            p.source.as_str(),
                        ),
                    )
                    .await
                    .contextualize_with(libsql_error_to_persistence_error)?;
                }
            }
            _ => {}
        }

        game.increment();
        Ok(())
    }
}

async fn hydrate_game_rows(
    tx: &Transaction,
    mut rows: libsql::Rows,
) -> RepositoryResult<Vec<Versioned<Game>>> {
    let mut game_rows = Vec::new();
    while let Some(row) = rows
        .next()
        .await
        .contextualize_with(libsql_error_to_persistence_error)?
    {
        game_rows.push(read_game_row(&row)?);
    }
    drop(rows);

    let mut out = Vec::with_capacity(game_rows.len());
    for game_row in game_rows {
        out.push(hydrate_game(tx, game_row).await?);
    }
    Ok(out)
}
