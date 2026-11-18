use std::sync::Arc;

use async_lock::Mutex;
use async_trait::async_trait;
use error_stack::ResultExt;
use friendarazzi_core::{
    ErrorReportExt,
    application::uow::Versioned,
    domain::{
        BenutzerId, Board, BoardId, BoardTile, BoardTileId, CaptureId, GameId, GridSize, NewBoard,
        PromptSubmissionId,
    },
    ports::{BoardRepository, RepositoryError, RepositoryResult},
};
use libsql::{Row, Transaction};
use uuid::Uuid;

use crate::{
    errors::libsql_error_to_persistence_error,
    repos::common::{parse_uuid, parse_version, update_versioned},
};

const SELECT_BOARD: &str = r#"SELECT b.id, b.game_id, b.owner_user_id, b.has_bingo, g.grid_size, b._version
    FROM "board" b
    INNER JOIN "game" g ON g.id = b.game_id"#;

async fn load_tiles(
    tx: &Transaction,
    board_id: BoardId,
    grid_size: GridSize,
) -> RepositoryResult<Vec<BoardTile>> {
    let mut rows = tx
        .query(
            r#"SELECT bt.id, bt.row, bt.col, bt.prompt_submission_id, bt.target_user_id, ps.text, bt.capture_id
               FROM "board_tile" bt
               INNER JOIN "prompt_submission" ps ON ps.id = bt.prompt_submission_id
               WHERE bt.board_id = ?1
               ORDER BY bt.row ASC, bt.col ASC"#,
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
        let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
        let row_raw: i64 = row.get(1).contextualize(RepositoryError::Data)?;
        let col_raw: i64 = row.get(2).contextualize(RepositoryError::Data)?;
        let prompt_sub_raw: String = row.get(3).contextualize(RepositoryError::Data)?;
        let target_raw: String = row.get(4).contextualize(RepositoryError::Data)?;
        let prompt_text: String = row.get(5).contextualize(RepositoryError::Data)?;
        let capture_raw: Option<String> = row.get(6).contextualize(RepositoryError::Data)?;

        let tile = BoardTile::from_parts(
            BoardTileId(parse_uuid(&id_raw)?),
            u8::try_from(row_raw).contextualize(RepositoryError::Data)?,
            u8::try_from(col_raw).contextualize(RepositoryError::Data)?,
            grid_size,
            PromptSubmissionId(parse_uuid(&prompt_sub_raw)?),
            BenutzerId(parse_uuid(&target_raw)?),
            prompt_text,
            capture_raw
                .map(|s| parse_uuid(&s))
                .transpose()?
                .map(CaptureId),
        )
        .change_context(RepositoryError::Data)?;
        out.push(tile);
    }
    Ok(out)
}

struct BoardRow {
    id: BoardId,
    game_id: GameId,
    owner_user_id: BenutzerId,
    has_bingo: bool,
    grid_size: GridSize,
    version: u64,
}

fn read_board_row(row: &Row) -> RepositoryResult<BoardRow> {
    let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let game_id_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
    let owner_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
    let has_bingo_raw: i64 = row.get(3).contextualize(RepositoryError::Data)?;
    let grid_size_raw: i64 = row.get(4).contextualize(RepositoryError::Data)?;
    let version = parse_version(row, 5)?;

    Ok(BoardRow {
        id: BoardId(parse_uuid(&id_raw)?),
        game_id: GameId(parse_uuid(&game_id_raw)?),
        owner_user_id: BenutzerId(parse_uuid(&owner_raw)?),
        has_bingo: has_bingo_raw != 0,
        grid_size: GridSize::from_u8(
            u8::try_from(grid_size_raw).contextualize(RepositoryError::Data)?,
        )
        .change_context(RepositoryError::Data)?,
        version,
    })
}

async fn hydrate_board(tx: &Transaction, row: BoardRow) -> RepositoryResult<Versioned<Board>> {
    let tiles = load_tiles(tx, row.id, row.grid_size).await?;
    let board = Board::from_parts(
        row.id,
        row.game_id,
        row.owner_user_id,
        row.grid_size,
        tiles,
        row.has_bingo,
    )
    .change_context(RepositoryError::Data)?;

    Ok(Versioned::new(row.version, board))
}

pub struct SQLiteBoardRepository {
    pub(crate) tx: Arc<Mutex<Option<Transaction>>>,
}

#[async_trait]
impl BoardRepository for SQLiteBoardRepository {
    async fn create(&self, board: NewBoard) -> RepositoryResult<Versioned<Board>> {
        let id = BoardId(Uuid::new_v4());
        let domain = Board::new(id, board).change_context(RepositoryError::Data)?;

        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"INSERT INTO "board" (id, game_id, owner_user_id, has_bingo, _version) VALUES (?1, ?2, ?3, ?4, 0)"#,
            (
                domain.id().0.to_string(),
                domain.game_id().0.to_string(),
                domain.owner_user_id().0.to_string(),
                i64::from(domain.has_bingo()),
            ),
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;

        for tile in domain.tiles() {
            tx.execute(
                r#"INSERT INTO "board_tile" (id, board_id, row, col, prompt_submission_id, target_user_id, capture_id)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
                (
                    tile.id().0.to_string(),
                    domain.id().0.to_string(),
                    i64::from(tile.row()),
                    i64::from(tile.col()),
                    tile.prompt_submission_id().0.to_string(),
                    tile.target_user_id().0.to_string(),
                    tile.capture_id().map(|c| c.0.to_string()),
                ),
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;
        }

        Ok(Versioned::init(domain))
    }

    async fn find_by_id(&self, id: BoardId) -> RepositoryResult<Versioned<Board>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(r#"{SELECT_BOARD} WHERE b.id = ?1"#),
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
        let board_row = read_board_row(&row)?;
        drop(rows);
        hydrate_board(tx, board_row).await
    }

    async fn find_by_game(&self, game_id: GameId) -> RepositoryResult<Vec<Versioned<Board>>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(r#"{SELECT_BOARD} WHERE b.game_id = ?1"#),
                [game_id.0.to_string()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let mut board_rows = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        {
            board_rows.push(read_board_row(&row)?);
        }
        drop(rows);

        let mut out = Vec::with_capacity(board_rows.len());
        for board_row in board_rows {
            out.push(hydrate_board(tx, board_row).await?);
        }
        Ok(out)
    }

    async fn find_by_game_and_owner(
        &self,
        game_id: GameId,
        owner: BenutzerId,
    ) -> RepositoryResult<Versioned<Board>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(r#"{SELECT_BOARD} WHERE b.game_id = ?1 AND b.owner_user_id = ?2"#),
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
        let board_row = read_board_row(&row)?;
        drop(rows);
        hydrate_board(tx, board_row).await
    }

    async fn update(&self, board: &mut Versioned<Board>) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        update_versioned(
            tx,
            r#"UPDATE "board" SET has_bingo = ?2, _version = ?3 + 1 WHERE id = ?1 AND _version = ?3"#,
            (
                board.id().0.to_string(),
                i64::from(board.has_bingo()),
                board.v() as i64,
            ),
            board.v(),
        )
        .await?;

        for tile in board.tiles() {
            if let Some(capture_id) = tile.capture_id() {
                tx.execute(
                    r#"UPDATE "board_tile" SET capture_id = ?2 WHERE id = ?1 AND capture_id IS NULL"#,
                    (tile.id().0.to_string(), capture_id.0.to_string()),
                )
                .await
                .contextualize_with(libsql_error_to_persistence_error)?;
            }
        }

        board.increment();
        Ok(())
    }
}
