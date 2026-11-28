//! Bingo boards, tiles, coordinates, and win-line evaluation.

use std::collections::HashSet;

use error_stack::{Report, ResultExt};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    ResultReport,
    domain::{
        BenutzerId, NonEmptyString,
        capture::CaptureId,
        game::{GameId, GridSize, PromptSubmissionId},
    },
};

const CONSTRUCTING: &str = "while constructing board";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BoardId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BoardTileId(pub Uuid);

#[derive(Debug, Error)]
pub enum BoardError {
    #[error("Kacheltext darf nicht leer sein")]
    EmptyPromptText,
    #[error("Ungültige Kachelposition")]
    InvalidPosition,
    #[error("Die Anzahl der Kacheln passt nicht zur Brettgröße")]
    InvalidTileLayout,
    #[error("Kachel wurde nicht gefunden")]
    TileNotFound,
    #[error("Diese Kachel wurde bereits erfasst")]
    TileAlreadyCaptured,
    #[error("Diese Kachel hat kein Foto")]
    TileNotCaptured,
}

/// Validated zero-based `(row, col)` coordinate on a Bingo board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BoardCoordinates {
    row: u8,
    col: u8,
}

impl BoardCoordinates {
    pub fn new(row: u8, col: u8, grid_size: GridSize) -> Result<Self, BoardError> {
        let side = grid_size.side() as u8;
        if row >= side || col >= side {
            return Err(BoardError::InvalidPosition);
        }
        Ok(Self { row, col })
    }

    pub fn row(self) -> u8 {
        self.row
    }

    pub fn col(self) -> u8 {
        self.col
    }
}

/// Prompt assignment used when constructing a brand-new `Board` via `NewBoard::new`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardTileAssignment {
    pub prompt_submission_id: PromptSubmissionId,
    pub target_user_id: BenutzerId,
    pub prompt_text: NonEmptyString,
}

/// Creation input for a `Board` aggregate together with its full grid of tiles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBoard {
    game_id: GameId,
    owner_user_id: BenutzerId,
    grid_size: GridSize,
    assignments: Vec<BoardTileAssignment>,
}

impl NewBoard {
    /// Construct a validated board creation input; tiles must cover the full `grid_size x grid_size` grid.
    pub fn new(
        game_id: GameId,
        owner_user_id: BenutzerId,
        grid_size: GridSize,
        assignments: Vec<BoardTileAssignment>,
    ) -> ResultReport<Self, BoardError> {
        if assignments.len() != grid_size.playable_tile_count() {
            return Err(Report::new(BoardError::InvalidTileLayout).attach(CONSTRUCTING));
        }
        Ok(Self {
            game_id,
            owner_user_id,
            grid_size,
            assignments,
        })
    }

    pub fn game_id(&self) -> GameId {
        self.game_id
    }

    pub fn owner_user_id(&self) -> BenutzerId {
        self.owner_user_id
    }

    pub fn grid_size(&self) -> GridSize {
        self.grid_size
    }

    pub fn assignments(&self) -> &[BoardTileAssignment] {
        &self.assignments
    }
}

/// One player's Bingo board aggregate root, owning all of its `BoardTile` entities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    id: BoardId,
    game_id: GameId,
    owner_user_id: BenutzerId,
    grid_size: GridSize,
    tiles: Vec<BoardTile>,
    has_bingo: bool,
}

impl Board {
    pub fn new(id: BoardId, input: NewBoard) -> ResultReport<Self, BoardError> {
        let side = input.grid_size.side() as u8;
        if input.assignments.len() != (side as usize) * (side as usize) {
            return Err(Report::new(BoardError::InvalidTileLayout).attach(CONSTRUCTING));
        }

        let mut tiles = Vec::with_capacity(input.assignments.len());
        let mut iter = input.assignments.into_iter();
        for r in 0..side {
            for c in 0..side {
                let assignment = iter
                    .next()
                    .ok_or_else(|| Report::new(BoardError::InvalidTileLayout))?;
                let coordinates = BoardCoordinates::new(r, c, input.grid_size)
                    .change_context(BoardError::InvalidPosition)
                    .attach(CONSTRUCTING)?;
                tiles.push(BoardTile {
                    id: BoardTileId(Uuid::new_v4()),
                    coordinates,
                    prompt_submission_id: assignment.prompt_submission_id,
                    target_user_id: assignment.target_user_id,
                    prompt_text: assignment.prompt_text,
                    capture_id: None,
                });
            }
        }

        Ok(Self {
            id,
            game_id: input.game_id,
            owner_user_id: input.owner_user_id,
            grid_size: input.grid_size,
            tiles,
            has_bingo: false,
        })
    }

    pub fn from_parts(
        id: BoardId,
        game_id: GameId,
        owner_user_id: BenutzerId,
        grid_size: GridSize,
        mut tiles: Vec<BoardTile>,
        has_bingo: bool,
    ) -> ResultReport<Self, BoardError> {
        let side = grid_size.side();
        if tiles.len() != side * side {
            return Err(Report::new(BoardError::InvalidTileLayout).attach(CONSTRUCTING));
        }
        let mut seen = HashSet::with_capacity(tiles.len());
        for tile in &tiles {
            if usize::from(tile.row()) >= side
                || usize::from(tile.col()) >= side
                || !seen.insert(tile.coordinates())
            {
                return Err(Report::new(BoardError::InvalidPosition).attach(CONSTRUCTING));
            }
        }
        tiles.sort_by_key(|t| (t.row(), t.col()));

        Ok(Self {
            id,
            game_id,
            owner_user_id,
            grid_size,
            tiles,
            has_bingo,
        })
    }

    pub fn id(&self) -> BoardId {
        self.id
    }

    pub fn game_id(&self) -> GameId {
        self.game_id
    }

    pub fn owner_user_id(&self) -> BenutzerId {
        self.owner_user_id
    }

    pub fn grid_size(&self) -> GridSize {
        self.grid_size
    }

    pub fn tiles(&self) -> &[BoardTile] {
        &self.tiles
    }

    pub fn tile(&self, coords: BoardCoordinates) -> Option<&BoardTile> {
        self.tiles.iter().find(|t| t.coordinates == coords)
    }

    pub fn tile_by_id(&self, tile_id: BoardTileId) -> Option<&BoardTile> {
        self.tiles.iter().find(|t| t.id == tile_id)
    }

    pub fn has_bingo(&self) -> bool {
        self.has_bingo
    }

    /// Attach a photo capture to `tile_id`, evaluate Bingo, and return the winning line if completed.
    pub fn record_capture(
        &mut self,
        tile_id: BoardTileId,
        capture_id: CaptureId,
    ) -> ResultReport<Option<Vec<BoardCoordinates>>, BoardError> {
        let Some(tile) = self.tiles.iter_mut().find(|t| t.id == tile_id) else {
            return Err(Report::new(BoardError::TileNotFound));
        };
        if tile.capture_id.is_some() {
            return Err(Report::new(BoardError::TileAlreadyCaptured));
        }
        tile.capture_id = Some(capture_id);

        let winning_line = self.evaluate_bingo();
        if winning_line.is_some() {
            self.has_bingo = true;
        }
        Ok(winning_line)
    }

    /// Remove a previously attached capture from a tile and recompute win state.
    pub fn retract_capture(
        &mut self,
        tile_id: BoardTileId,
        expected_capture_id: CaptureId,
    ) -> ResultReport<(), BoardError> {
        let Some(tile) = self.tiles.iter_mut().find(|t| t.id == tile_id) else {
            return Err(Report::new(BoardError::TileNotFound));
        };
        if tile.capture_id != Some(expected_capture_id) {
            return Err(Report::new(BoardError::TileNotCaptured));
        }
        tile.capture_id = None;
        self.has_bingo = self.evaluate_bingo().is_some();
        Ok(())
    }

    /// Evaluate whether this board contains any completed Bingo line (row, column, or diagonal).
    pub fn evaluate_bingo(&self) -> Option<Vec<BoardCoordinates>> {
        let side = self.grid_size.side() as u8;
        let is_filled = |r: u8, c: u8| -> bool {
            self.tiles
                .iter()
                .find(|t| t.row() == r && t.col() == c)
                .is_some_and(BoardTile::is_filled)
        };
        let coord = |r: u8, c: u8| BoardCoordinates { row: r, col: c };

        for r in 0..side {
            if (0..side).all(|c| is_filled(r, c)) {
                return Some((0..side).map(|c| coord(r, c)).collect());
            }
        }

        for c in 0..side {
            if (0..side).all(|r| is_filled(r, c)) {
                return Some((0..side).map(|r| coord(r, c)).collect());
            }
        }

        if (0..side).all(|i| is_filled(i, i)) {
            return Some((0..side).map(|i| coord(i, i)).collect());
        }

        if (0..side).all(|i| is_filled(i, side - 1 - i)) {
            return Some((0..side).map(|i| coord(i, side - 1 - i)).collect());
        }

        None
    }
}

/// One tile entity owned by a `Board` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardTile {
    id: BoardTileId,
    coordinates: BoardCoordinates,
    prompt_submission_id: PromptSubmissionId,
    target_user_id: BenutzerId,
    prompt_text: NonEmptyString,
    capture_id: Option<CaptureId>,
}

impl BoardTile {
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: BoardTileId,
        row: u8,
        col: u8,
        grid_size: GridSize,
        prompt_submission_id: PromptSubmissionId,
        target_user_id: BenutzerId,
        prompt_text: String,
        capture_id: Option<CaptureId>,
    ) -> ResultReport<Self, BoardError> {
        let coordinates = BoardCoordinates::new(row, col, grid_size)
            .change_context(BoardError::InvalidPosition)
            .attach(CONSTRUCTING)?;
        let prompt_text = NonEmptyString::neu(prompt_text)
            .change_context(BoardError::EmptyPromptText)
            .attach(CONSTRUCTING)?;
        Ok(Self {
            id,
            coordinates,
            prompt_submission_id,
            target_user_id,
            prompt_text,
            capture_id,
        })
    }

    pub fn id(&self) -> BoardTileId {
        self.id
    }

    pub fn coordinates(&self) -> BoardCoordinates {
        self.coordinates
    }

    pub fn row(&self) -> u8 {
        self.coordinates.row
    }

    pub fn col(&self) -> u8 {
        self.coordinates.col
    }

    pub fn prompt_submission_id(&self) -> PromptSubmissionId {
        self.prompt_submission_id
    }

    pub fn target_user_id(&self) -> BenutzerId {
        self.target_user_id
    }

    pub fn prompt_text(&self) -> &NonEmptyString {
        &self.prompt_text
    }

    pub fn capture_id(&self) -> Option<CaptureId> {
        self.capture_id
    }

    pub fn is_filled(&self) -> bool {
        self.capture_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_board(grid_size: GridSize) -> Board {
        let count = grid_size.playable_tile_count();
        let assignments = (0..count)
            .map(|i| BoardTileAssignment {
                prompt_submission_id: PromptSubmissionId(Uuid::new_v4()),
                target_user_id: BenutzerId(Uuid::new_v4()),
                prompt_text: NonEmptyString::neu(format!("Prompt #{i}")).unwrap(),
            })
            .collect();
        let new_board = NewBoard::new(
            GameId(Uuid::new_v4()),
            BenutzerId(Uuid::new_v4()),
            grid_size,
            assignments,
        )
        .unwrap();
        Board::new(BoardId(Uuid::new_v4()), new_board).unwrap()
    }

    #[test_log::test]
    fn bingo_detects_row_col_and_diagonal() {
        let mut board = mk_board(GridSize::Three);
        let ids: Vec<BoardTileId> = board.tiles().iter().map(BoardTile::id).collect();
        // Main diagonal: (0,0), (1,1), (2,2) -> indices 0, 4, 8
        assert_eq!(
            board
                .record_capture(ids[0], CaptureId(Uuid::new_v4()))
                .unwrap(),
            None
        );
        assert_eq!(
            board
                .record_capture(ids[4], CaptureId(Uuid::new_v4()))
                .unwrap(),
            None
        );
        assert!(!board.has_bingo());

        let line = board
            .record_capture(ids[8], CaptureId(Uuid::new_v4()))
            .unwrap()
            .unwrap();
        assert!(board.has_bingo());
        assert_eq!(
            line.into_iter()
                .map(|c| (c.row(), c.col()))
                .collect::<Vec<_>>(),
            vec![(0, 0), (1, 1), (2, 2)]
        );
    }

    #[test_log::test]
    fn bingo_detects_anti_diagonal_on_4x4() {
        let mut board = mk_board(GridSize::Four);
        let anti_ids: Vec<BoardTileId> = board
            .tiles()
            .iter()
            .filter(|t| t.row() + t.col() == 3)
            .map(BoardTile::id)
            .collect();
        for (idx, tile_id) in anti_ids.iter().enumerate() {
            let res = board
                .record_capture(*tile_id, CaptureId(Uuid::new_v4()))
                .unwrap();
            if idx < 3 {
                assert!(res.is_none());
            } else {
                let coords = res.unwrap();
                assert_eq!(
                    coords
                        .into_iter()
                        .map(|c| (c.row(), c.col()))
                        .collect::<Vec<_>>(),
                    vec![(0, 3), (1, 2), (2, 1), (3, 0)]
                );
            }
        }
        assert!(board.has_bingo());
    }

    #[test_log::test]
    fn duplicate_capture_on_same_tile_is_rejected() {
        let mut board = mk_board(GridSize::Three);
        let tile_id = board.tiles()[0].id();
        board
            .record_capture(tile_id, CaptureId(Uuid::new_v4()))
            .unwrap();
        let err = board
            .record_capture(tile_id, CaptureId(Uuid::new_v4()))
            .unwrap_err();
        assert!(matches!(
            err.current_context(),
            BoardError::TileAlreadyCaptured
        ));
    }

    #[test_log::test]
    fn new_board_rejects_mismatched_tile_count() {
        let err = NewBoard::new(
            GameId(Uuid::new_v4()),
            BenutzerId(Uuid::new_v4()),
            GridSize::Three,
            vec![],
        )
        .unwrap_err();
        assert!(matches!(
            err.current_context(),
            BoardError::InvalidTileLayout
        ));
    }

    #[test_log::test]
    fn retract_capture_clears_tile_and_rejects_mismatched_or_empty_tile() {
        let mut board = mk_board(GridSize::Three);
        let tile_id = board.tiles()[0].id();
        let cap_id = CaptureId(Uuid::new_v4());
        board.record_capture(tile_id, cap_id).unwrap();
        assert_eq!(board.tiles().iter().filter(|t| t.is_filled()).count(), 1);

        let wrong_err = board
            .retract_capture(tile_id, CaptureId(Uuid::new_v4()))
            .unwrap_err();
        assert!(matches!(
            wrong_err.current_context(),
            BoardError::TileNotCaptured
        ));

        board.retract_capture(tile_id, cap_id).unwrap();
        assert_eq!(board.tiles().iter().filter(|t| t.is_filled()).count(), 0);
        assert!(board.tiles()[0].capture_id().is_none());

        let empty_err = board.retract_capture(tile_id, cap_id).unwrap_err();
        assert!(matches!(
            empty_err.current_context(),
            BoardError::TileNotCaptured
        ));
    }
}
