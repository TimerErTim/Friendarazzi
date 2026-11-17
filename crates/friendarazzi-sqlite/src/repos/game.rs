use std::sync::Arc;

use async_lock::Mutex;
use async_trait::async_trait;
use error_stack::ResultExt;
use friendarazzi_core::{
    ErrorReportExt,
    application::uow::Versioned,
    domain::{
        Game, GameId, GamePlayer, InviteCode, NewGame, NonEmptyString, PromptSource,
        PromptSubmission, PromptSubmissionId, konto::BenutzerId,
    },
    ports::{GameRepository, RepositoryError, RepositoryResult},
};
use libsql::{Row, Transaction};
use uuid::Uuid;

use crate::{
    errors::libsql_error_to_persistence_error,
    repos::common::{format_datetime, parse_datetime, parse_uuid, parse_version, update_versioned},
};

const SELECT_GAME: &str = r#"SELECT id, invite_code, host_user_id, title, grid_size, visibility_mode, include_free_space, status, started_at, ended_at, winner_user_id, created_at, _version FROM "games""#;

fn game_from_row(row: &Row) -> RepositoryResult<Versioned<Game>> {
    let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let invite_code: String = row.get(1).contextualize(RepositoryError::Data)?;
    let host_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
    let title: String = row.get(3).contextualize(RepositoryError::Data)?;
    let grid_size_raw: i64 = row.get(4).contextualize(RepositoryError::Data)?;
    let visibility_mode: String = row.get(5).contextualize(RepositoryError::Data)?;
    let include_free_raw: i64 = row.get(6).contextualize(RepositoryError::Data)?;
    let status: String = row.get(7).contextualize(RepositoryError::Data)?;
    let started_at_raw: Option<String> = row.get(8).contextualize(RepositoryError::Data)?;
    let ended_at_raw: Option<String> = row.get(9).contextualize(RepositoryError::Data)?;
    let winner_raw: Option<String> = row.get(10).contextualize(RepositoryError::Data)?;
    let created_at_raw: String = row.get(11).contextualize(RepositoryError::Data)?;
    let version = parse_version(row, 12)?;

    let id = GameId(parse_uuid(&id_raw)?);
    let host_user_id = BenutzerId(parse_uuid(&host_raw)?);
    let grid_size = u8::try_from(grid_size_raw).contextualize(RepositoryError::Data)?;
    let started_at = started_at_raw.map(|s| parse_datetime(&s)).transpose()?;
    let ended_at = ended_at_raw.map(|s| parse_datetime(&s)).transpose()?;
    let winner_user_id = winner_raw
        .map(|s| parse_uuid(&s))
        .transpose()?
        .map(BenutzerId);
    let created_at = parse_datetime(&created_at_raw)?;

    let game = Game::from_parts(
        id,
        invite_code,
        host_user_id,
        title,
        grid_size,
        visibility_mode,
        include_free_raw != 0,
        status,
        started_at,
        ended_at,
        winner_user_id,
        created_at,
    )
    .change_context(RepositoryError::Data)?;

    Ok(Versioned::new(version, game))
}

fn prompt_from_row(row: &Row) -> RepositoryResult<PromptSubmission> {
    let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let game_id_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
    let author_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
    let target_raw: String = row.get(3).contextualize(RepositoryError::Data)?;
    let text_raw: String = row.get(4).contextualize(RepositoryError::Data)?;
    let source_raw: String = row.get(5).contextualize(RepositoryError::Data)?;

    let text = NonEmptyString::neu(text_raw).contextualize(RepositoryError::Data)?;
    let source: PromptSource = source_raw.parse().change_context(RepositoryError::Data)?;

    PromptSubmission::new(
        PromptSubmissionId(parse_uuid(&id_raw)?),
        GameId(parse_uuid(&game_id_raw)?),
        BenutzerId(parse_uuid(&author_raw)?),
        BenutzerId(parse_uuid(&target_raw)?),
        text,
        source,
    )
    .change_context(RepositoryError::Data)
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
            r#"INSERT INTO "games" (id, invite_code, host_user_id, title, grid_size, visibility_mode, include_free_space, status, started_at, ended_at, winner_user_id, created_at, _version)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0)"#,
            (
                id.0.to_string(),
                domain.invite_code().as_str(),
                domain.host_user_id().0.to_string(),
                domain.title().as_str(),
                i64::from(domain.grid_size() as u8),
                domain.visibility_mode().as_str(),
                i64::from(domain.include_free_space()),
                domain.status().as_str(),
                domain.started_at().map(format_datetime),
                domain.ended_at().map(format_datetime),
                domain.winner_user_id().map(|u| u.0.to_string()),
                format_datetime(domain.created_at()),
            ),
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;

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
        game_from_row(&row)
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
        game_from_row(&row)
    }

    async fn find_for_user(&self, user_id: BenutzerId) -> RepositoryResult<Vec<Versioned<Game>>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                r#"SELECT g.id, g.invite_code, g.host_user_id, g.title, g.grid_size, g.visibility_mode, g.include_free_space, g.status, g.started_at, g.ended_at, g.winner_user_id, g.created_at, g._version
                   FROM "games" g
                   INNER JOIN "game_players" gp ON gp.game_id = g.id
                   WHERE gp.user_id = ?1
                   ORDER BY g.created_at DESC"#,
                [user_id.0.to_string()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        {
            out.push(game_from_row(&row)?);
        }
        Ok(out)
    }

    async fn update(&self, game: &mut Versioned<Game>) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        update_versioned(
            tx,
            r#"UPDATE "games"
               SET title = ?2, grid_size = ?3, visibility_mode = ?4, include_free_space = ?5, status = ?6, started_at = ?7, ended_at = ?8, winner_user_id = ?9, _version = ?10 + 1
               WHERE id = ?1 AND _version = ?10"#,
            (
                game.id().0.to_string(),
                game.title().as_str(),
                i64::from(game.grid_size() as u8),
                game.visibility_mode().as_str(),
                i64::from(game.include_free_space()),
                game.status().as_str(),
                game.started_at().map(format_datetime),
                game.ended_at().map(format_datetime),
                game.winner_user_id().map(|u| u.0.to_string()),
                game.v() as i64,
            ),
            game.v(),
        )
        .await?;
        game.increment();
        Ok(())
    }

    async fn add_player(&self, player: GamePlayer) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"INSERT INTO "game_players" (game_id, user_id, joined_at, draft_locked) VALUES (?1, ?2, ?3, ?4)"#,
            (
                player.game_id.0.to_string(),
                player.user_id.0.to_string(),
                format_datetime(player.joined_at),
                i64::from(player.draft_locked),
            ),
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;
        Ok(())
    }

    async fn find_players(&self, game_id: GameId) -> RepositoryResult<Vec<GamePlayer>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                r#"SELECT game_id, user_id, joined_at, draft_locked FROM "game_players" WHERE game_id = ?1 ORDER BY joined_at ASC, user_id ASC"#,
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
            let gid_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
            let uid_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
            let joined_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
            let locked_raw: i64 = row.get(3).contextualize(RepositoryError::Data)?;
            out.push(GamePlayer {
                game_id: GameId(parse_uuid(&gid_raw)?),
                user_id: BenutzerId(parse_uuid(&uid_raw)?),
                joined_at: parse_datetime(&joined_raw)?,
                draft_locked: locked_raw != 0,
            });
        }
        Ok(out)
    }

    async fn set_player_draft_locked(
        &self,
        game_id: GameId,
        user_id: BenutzerId,
        locked: bool,
    ) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let affected = tx
            .execute(
                r#"UPDATE "game_players" SET draft_locked = ?3 WHERE game_id = ?1 AND user_id = ?2"#,
                (
                    game_id.0.to_string(),
                    user_id.0.to_string(),
                    i64::from(locked),
                ),
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        if affected == 0 {
            return Err(RepositoryError::NotFound.into());
        }
        Ok(())
    }

    async fn replace_author_prompts(
        &self,
        game_id: GameId,
        author_user_id: BenutzerId,
        prompts: Vec<PromptSubmission>,
    ) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"DELETE FROM "prompt_submissions" WHERE game_id = ?1 AND author_user_id = ?2"#,
            (game_id.0.to_string(), author_user_id.0.to_string()),
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;

        for p in prompts {
            tx.execute(
                r#"INSERT INTO "prompt_submissions" (id, game_id, author_user_id, target_user_id, text, source)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
                (
                    p.id.0.to_string(),
                    p.game_id.0.to_string(),
                    p.author_user_id.0.to_string(),
                    p.target_user_id.0.to_string(),
                    p.text.as_str(),
                    p.source.as_str(),
                ),
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;
        }
        Ok(())
    }

    async fn find_prompts_by_game(
        &self,
        game_id: GameId,
    ) -> RepositoryResult<Vec<PromptSubmission>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                r#"SELECT id, game_id, author_user_id, target_user_id, text, source FROM "prompt_submissions" WHERE game_id = ?1"#,
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
            out.push(prompt_from_row(&row)?);
        }
        Ok(out)
    }

    async fn find_prompts_by_author(
        &self,
        game_id: GameId,
        author_user_id: BenutzerId,
    ) -> RepositoryResult<Vec<PromptSubmission>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                r#"SELECT id, game_id, author_user_id, target_user_id, text, source FROM "prompt_submissions" WHERE game_id = ?1 AND author_user_id = ?2"#,
                (game_id.0.to_string(), author_user_id.0.to_string()),
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        {
            out.push(prompt_from_row(&row)?);
        }
        Ok(out)
    }
}
