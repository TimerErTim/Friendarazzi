use std::{ops::Deref, sync::Arc};

use async_lock::Mutex;
use async_trait::async_trait;
use error_stack::ResultExt;
use friendarazzi_core::{
    ErrorReportExt,
    application::uow::Versioned,
    domain::{
        BenutzerId, Game, GameId, GamePlayer, InviteCode, NewGame, NonEmptyString, PromptSource,
        PromptSubmission, PromptSubmissionId,
    },
    ports::{GameRepository, RepositoryError, RepositoryResult},
};
use libsql::{Row, Transaction};
use uuid::Uuid;

use crate::{
    errors::libsql_error_to_persistence_error,
    repos::common::{format_datetime, parse_datetime, parse_uuid, parse_version, update_versioned},
};

const SELECT_GAME: &str = r#"SELECT id, invite_code, host_user_id, title, grid_size, visibility_mode, status, started_at, ended_at, winner_user_id, created_at, _version FROM "game""#;

async fn load_players(tx: &Transaction, game_id: GameId) -> RepositoryResult<Vec<GamePlayer>> {
    let mut rows = tx
        .query(
            r#"SELECT user_id, joined_at, draft_locked FROM "game_player" WHERE game_id = ?1 ORDER BY joined_at ASC, user_id ASC"#,
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
        let uid_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
        let joined_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
        let locked_raw: i64 = row.get(2).contextualize(RepositoryError::Data)?;
        out.push(GamePlayer {
            user_id: BenutzerId(parse_uuid(&uid_raw)?),
            joined_at: parse_datetime(&joined_raw)?,
            draft_locked: locked_raw != 0,
        });
    }
    Ok(out)
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

        let text = NonEmptyString::neu(text_raw).contextualize(RepositoryError::Data)?;
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
    status: String,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    ended_at: Option<chrono::DateTime<chrono::Utc>>,
    winner_user_id: Option<BenutzerId>,
    created_at: chrono::DateTime<chrono::Utc>,
    version: u64,
}

fn read_game_row(row: &Row) -> RepositoryResult<GameRow> {
    let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let invite_code: String = row.get(1).contextualize(RepositoryError::Data)?;
    let host_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
    let title: String = row.get(3).contextualize(RepositoryError::Data)?;
    let grid_size_raw: i64 = row.get(4).contextualize(RepositoryError::Data)?;
    let visibility_mode: String = row.get(5).contextualize(RepositoryError::Data)?;
    let status: String = row.get(6).contextualize(RepositoryError::Data)?;
    let started_at_raw: Option<String> = row.get(7).contextualize(RepositoryError::Data)?;
    let ended_at_raw: Option<String> = row.get(8).contextualize(RepositoryError::Data)?;
    let winner_raw: Option<String> = row.get(9).contextualize(RepositoryError::Data)?;
    let created_at_raw: String = row.get(10).contextualize(RepositoryError::Data)?;
    let version = parse_version(row, 11)?;

    Ok(GameRow {
        id: GameId(parse_uuid(&id_raw)?),
        invite_code,
        host_user_id: BenutzerId(parse_uuid(&host_raw)?),
        title,
        grid_size: u8::try_from(grid_size_raw).contextualize(RepositoryError::Data)?,
        visibility_mode,
        status,
        started_at: started_at_raw.map(|s| parse_datetime(&s)).transpose()?,
        ended_at: ended_at_raw.map(|s| parse_datetime(&s)).transpose()?,
        winner_user_id: winner_raw
            .map(|s| parse_uuid(&s))
            .transpose()?
            .map(BenutzerId),
        created_at: parse_datetime(&created_at_raw)?,
        version,
    })
}

async fn hydrate_game(tx: &Transaction, row: GameRow) -> RepositoryResult<Versioned<Game>> {
    let players = load_players(tx, row.id).await?;
    let prompts = load_prompts(tx, row.id).await?;

    let game = Game::from_parts(
        row.id,
        row.invite_code,
        row.host_user_id,
        row.title,
        row.grid_size,
        row.visibility_mode,
        row.status,
        row.started_at,
        row.ended_at,
        row.winner_user_id,
        players,
        prompts,
        row.created_at,
    )
    .change_context(RepositoryError::Data)?;

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
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"INSERT INTO "game" (id, invite_code, host_user_id, title, grid_size, visibility_mode, status, started_at, ended_at, winner_user_id, created_at, _version)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0)"#,
            (
                id.0.to_string(),
                domain.invite_code().as_str(),
                domain.host_user_id().0.to_string(),
                domain.title().as_str(),
                i64::from(domain.grid_size() as u8),
                domain.visibility_mode().as_str(),
                domain.status().as_str(),
                domain.started_at().map(format_datetime),
                domain.ended_at().map(format_datetime),
                domain.winner_user_id().map(|u| u.0.to_string()),
                format_datetime(domain.created_at()),
            ),
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;

        for player in domain.players() {
            tx.execute(
                r#"INSERT INTO "game_player" (game_id, user_id, joined_at, draft_locked) VALUES (?1, ?2, ?3, ?4)"#,
                (
                    id.0.to_string(),
                    player.user_id.0.to_string(),
                    format_datetime(player.joined_at),
                    i64::from(player.draft_locked),
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

        let mut rows = tx
            .query(
                r#"SELECT g.id, g.invite_code, g.host_user_id, g.title, g.grid_size, g.visibility_mode, g.status, g.started_at, g.ended_at, g.winner_user_id, g.created_at, g._version
                   FROM "game" g
                   INNER JOIN "game_player" gp ON gp.game_id = g.id
                   WHERE gp.user_id = ?1
                   ORDER BY g.created_at DESC"#,
                [user_id.0.to_string()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

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

    async fn update(&self, game: &mut Versioned<Game>) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        update_versioned(
            tx,
            r#"UPDATE "game"
               SET title = ?2, grid_size = ?3, visibility_mode = ?4, status = ?5,
                   started_at = ?6, ended_at = ?7, winner_user_id = ?8, _version = ?9 + 1
               WHERE id = ?1 AND _version = ?9"#,
            (
                game.id().0.to_string(),
                game.title().as_str(),
                i64::from(game.grid_size() as u8),
                game.visibility_mode().as_str(),
                game.status().as_str(),
                game.started_at().map(format_datetime),
                game.ended_at().map(format_datetime),
                game.winner_user_id().map(|u| u.0.to_string()),
                game.v() as i64,
            ),
            game.v(),
        )
        .await?;

        for player in game.players() {
            tx.execute(
                r#"INSERT INTO "game_player" (game_id, user_id, joined_at, draft_locked)
                   VALUES (?1, ?2, ?3, ?4)
                   ON CONFLICT (game_id, user_id) DO UPDATE SET draft_locked = excluded.draft_locked"#,
                (
                    game.id().0.to_string(),
                    player.user_id.0.to_string(),
                    format_datetime(player.joined_at),
                    i64::from(player.draft_locked),
                ),
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;
        }

        // Only Drafting can mutate prompts; once Active, board_tile references prompt_submission(id).
        if matches!((*game).deref(), Game::Drafting(_)) {
            tx.execute(
                r#"DELETE FROM "prompt_submission" WHERE game_id = ?1"#,
                [game.id().0.to_string()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

            for p in game.prompts() {
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

        game.increment();
        Ok(())
    }
}
