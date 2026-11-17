use std::sync::Arc;

use async_lock::Mutex;
use async_trait::async_trait;
use error_stack::ResultExt;
use libsql::{Row, Transaction};
use logandaim_core::{
    ErrorReportExt,
    application::uow::Versioned,
    domain::{
        Board, BoardId, BoardTile, BoardTileId, CaptureId, GameId, PromptSubmissionId,
        konto::BenutzerId,
    },
    ports::{BoardRepository, RepositoryError, RepositoryResult},
};

use crate::{
    errors::libsql_error_to_persistence_error,
    repos::common::{parse_uuid, parse_version, update_versioned},
};

fn board_from_row(row: &Row) -> RepositoryResult<Versioned<Board>> {
    let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let game_id_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
    let owner_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
    let has_bingo_raw: i64 = row.get(3).contextualize(RepositoryError::Data)?;
    let version = parse_version(row, 4)?;

    Ok(Versioned::new(
        version,
        Board::from_parts(
            BoardId(parse_uuid(&id_raw)?),
            GameId(parse_uuid(&game_id_raw)?),
            BenutzerId(parse_uuid(&owner_raw)?),
            has_bingo_raw != 0,
        ),
    ))
}

fn tile_from_row(row: &Row) -> RepositoryResult<BoardTile> {
    let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let board_id_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
    let row_raw: i64 = row.get(2).contextualize(RepositoryError::Data)?;
    let col_raw: i64 = row.get(3).contextualize(RepositoryError::Data)?;
    let prompt_sub_raw: Option<String> = row.get(4).contextualize(RepositoryError::Data)?;
    let target_raw: Option<String> = row.get(5).contextualize(RepositoryError::Data)?;
    let prompt_text: String = row.get(6).contextualize(RepositoryError::Data)?;
    let is_free_raw: i64 = row.get(7).contextualize(RepositoryError::Data)?;
    let capture_raw: Option<String> = row.get(8).contextualize(RepositoryError::Data)?;

    BoardTile::from_parts(
        BoardTileId(parse_uuid(&id_raw)?),
        BoardId(parse_uuid(&board_id_raw)?),
        u8::try_from(row_raw).contextualize(RepositoryError::Data)?,
        u8::try_from(col_raw).contextualize(RepositoryError::Data)?,
        prompt_sub_raw
            .map(|s| parse_uuid(&s))
            .transpose()?
            .map(PromptSubmissionId),
        target_raw
            .map(|s| parse_uuid(&s))
            .transpose()?
            .map(BenutzerId),
        prompt_text,
        is_free_raw != 0,
        capture_raw
            .map(|s| parse_uuid(&s))
            .transpose()?
            .map(CaptureId),
    )
    .change_context(RepositoryError::Data)
}

pub struct SQLiteBoardRepository {
    pub(crate) tx: Arc<Mutex<Option<Transaction>>>,
}

#[async_trait]
impl BoardRepository for SQLiteBoardRepository {
    async fn create_board_with_tiles(
        &self,
        board: Board,
        tiles: Vec<BoardTile>,
    ) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"INSERT INTO "boards" (id, game_id, owner_user_id, has_bingo, _version) VALUES (?1, ?2, ?3, ?4, 0)"#,
            (
                board.id().0.to_string(),
                board.game_id().0.to_string(),
                board.owner_user_id().0.to_string(),
                i64::from(board.has_bingo()),
            ),
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;

        for tile in tiles {
            tx.execute(
                r#"INSERT INTO "board_tiles" (id, board_id, row, col, prompt_submission_id, target_user_id, prompt_text, is_free_space, capture_id)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
                (
                    tile.id().0.to_string(),
                    tile.board_id().0.to_string(),
                    i64::from(tile.row()),
                    i64::from(tile.col()),
                    tile.prompt_submission_id().map(|p| p.0.to_string()),
                    tile.target_user_id().map(|u| u.0.to_string()),
                    tile.prompt_text().as_str(),
                    i64::from(tile.is_free_space()),
                    tile.capture_id().map(|c| c.0.to_string()),
                ),
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;
        }

        Ok(())
    }

    async fn find_boards_by_game(
        &self,
        game_id: GameId,
    ) -> RepositoryResult<Vec<Versioned<Board>>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                r#"SELECT id, game_id, owner_user_id, has_bingo, _version FROM "boards" WHERE game_id = ?1"#,
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
            out.push(board_from_row(&row)?);
        }
        Ok(out)
    }

    async fn find_board_by_game_and_owner(
        &self,
        game_id: GameId,
        owner: BenutzerId,
    ) -> RepositoryResult<Versioned<Board>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                r#"SELECT id, game_id, owner_user_id, has_bingo, _version FROM "boards" WHERE game_id = ?1 AND owner_user_id = ?2"#,
                (game_id.0.to_string(), owner.0.to_string()),
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
        board_from_row(&row)
    }

    async fn find_tiles_by_board(&self, board_id: BoardId) -> RepositoryResult<Vec<BoardTile>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                r#"SELECT id, board_id, row, col, prompt_submission_id, target_user_id, prompt_text, is_free_space, capture_id
                   FROM "board_tiles" WHERE board_id = ?1 ORDER BY row ASC, col ASC"#,
                [board_id.0.to_string()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        {
            out.push(tile_from_row(&row)?);
        }
        Ok(out)
    }

    async fn find_tiles_by_game(&self, game_id: GameId) -> RepositoryResult<Vec<BoardTile>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                r#"SELECT bt.id, bt.board_id, bt.row, bt.col, bt.prompt_submission_id, bt.target_user_id, bt.prompt_text, bt.is_free_space, bt.capture_id
                   FROM "board_tiles" bt
                   INNER JOIN "boards" b ON b.id = bt.board_id
                   WHERE b.game_id = ?1
                   ORDER BY bt.row ASC, bt.col ASC"#,
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
            out.push(tile_from_row(&row)?);
        }
        Ok(out)
    }

    async fn find_tile_by_id(&self, tile_id: BoardTileId) -> RepositoryResult<BoardTile> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                r#"SELECT id, board_id, row, col, prompt_submission_id, target_user_id, prompt_text, is_free_space, capture_id
                   FROM "board_tiles" WHERE id = ?1"#,
                [tile_id.0.to_string()],
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
        tile_from_row(&row)
    }

    async fn update_board(&self, board: &mut Versioned<Board>) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        update_versioned(
            tx,
            r#"UPDATE "boards" SET has_bingo = ?2, _version = ?3 + 1 WHERE id = ?1 AND _version = ?3"#,
            (
                board.id().0.to_string(),
                i64::from(board.has_bingo()),
                board.v() as i64,
            ),
            board.v(),
        )
        .await?;
        board.increment();
        Ok(())
    }

    async fn attach_tile_capture(
        &self,
        tile_id: BoardTileId,
        capture_id: CaptureId,
    ) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let affected = tx
            .execute(
                r#"UPDATE "board_tiles" SET capture_id = ?2 WHERE id = ?1 AND capture_id IS NULL"#,
                (tile_id.0.to_string(), capture_id.0.to_string()),
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        if affected == 0 {
            return Err(RepositoryError::Conflict.into());
        }
        Ok(())
    }
}
