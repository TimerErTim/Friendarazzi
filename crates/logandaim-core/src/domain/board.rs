//! Bingo boards, tiles, and win-line evaluation.

use uuid::Uuid;

use crate::{
    ResultReport,
    domain::{
        NonEmptyString,
        capture::CaptureId,
        game::{GameId, GridSize, PromptSubmissionId},
        konto::BenutzerId,
    },
};

use error_stack::ResultExt;
use thiserror::Error;

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
}

/// One player's Bingo board in a game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    id: BoardId,
    game_id: GameId,
    owner_user_id: BenutzerId,
    has_bingo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBoard {
    pub game_id: GameId,
    pub owner_user_id: BenutzerId,
}

impl Board {
    pub fn new(id: BoardId, input: NewBoard) -> Self {
        Self {
            id,
            game_id: input.game_id,
            owner_user_id: input.owner_user_id,
            has_bingo: false,
        }
    }

    pub fn from_parts(
        id: BoardId,
        game_id: GameId,
        owner_user_id: BenutzerId,
        has_bingo: bool,
    ) -> Self {
        Self {
            id,
            game_id,
            owner_user_id,
            has_bingo,
        }
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

    pub fn has_bingo(&self) -> bool {
        self.has_bingo
    }

    pub fn mark_bingo(&mut self) {
        self.has_bingo = true;
    }
}

/// One tile on a player's Bingo grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardTile {
    id: BoardTileId,
    board_id: BoardId,
    row: u8,
    col: u8,
    prompt_submission_id: Option<PromptSubmissionId>,
    target_user_id: Option<BenutzerId>,
    prompt_text: NonEmptyString,
    is_free_space: bool,
    capture_id: Option<CaptureId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBoardTile {
    pub board_id: BoardId,
    pub row: u8,
    pub col: u8,
    pub prompt_submission_id: Option<PromptSubmissionId>,
    pub target_user_id: Option<BenutzerId>,
    pub prompt_text: NonEmptyString,
    pub is_free_space: bool,
}

impl BoardTile {
    pub fn new(id: BoardTileId, input: NewBoardTile) -> Self {
        Self {
            id,
            board_id: input.board_id,
            row: input.row,
            col: input.col,
            prompt_submission_id: input.prompt_submission_id,
            target_user_id: input.target_user_id,
            prompt_text: input.prompt_text,
            is_free_space: input.is_free_space,
            capture_id: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: BoardTileId,
        board_id: BoardId,
        row: u8,
        col: u8,
        prompt_submission_id: Option<PromptSubmissionId>,
        target_user_id: Option<BenutzerId>,
        prompt_text: String,
        is_free_space: bool,
        capture_id: Option<CaptureId>,
    ) -> ResultReport<Self, BoardError> {
        let prompt_text = NonEmptyString::neu(prompt_text)
            .change_context(BoardError::EmptyPromptText)
            .attach(CONSTRUCTING)?;
        Ok(Self {
            id,
            board_id,
            row,
            col,
            prompt_submission_id,
            target_user_id,
            prompt_text,
            is_free_space,
            capture_id,
        })
    }

    pub fn id(&self) -> BoardTileId {
        self.id
    }

    pub fn board_id(&self) -> BoardId {
        self.board_id
    }

    pub fn row(&self) -> u8 {
        self.row
    }

    pub fn col(&self) -> u8 {
        self.col
    }

    pub fn prompt_submission_id(&self) -> Option<PromptSubmissionId> {
        self.prompt_submission_id
    }

    pub fn target_user_id(&self) -> Option<BenutzerId> {
        self.target_user_id
    }

    pub fn prompt_text(&self) -> &NonEmptyString {
        &self.prompt_text
    }

    pub fn is_free_space(&self) -> bool {
        self.is_free_space
    }

    pub fn capture_id(&self) -> Option<CaptureId> {
        self.capture_id
    }

    pub fn is_filled(&self) -> bool {
        self.is_free_space || self.capture_id.is_some()
    }

    pub fn attach_capture(&mut self, capture_id: CaptureId) {
        self.capture_id = Some(capture_id);
    }
}

/// Evaluate whether `tiles` contain any completed Bingo line (row, column, or diagonal).
///
/// Returns the winning coordinates `Vec<(row, col)>` on the first completed line.
pub fn evaluate_bingo(grid_size: GridSize, tiles: &[BoardTile]) -> Option<Vec<(u8, u8)>> {
    let side = grid_size.side() as u8;
    let is_filled = |r: u8, c: u8| -> bool {
        tiles
            .iter()
            .find(|t| t.row() == r && t.col() == c)
            .is_some_and(BoardTile::is_filled)
    };

    // Rows
    for r in 0..side {
        if (0..side).all(|c| is_filled(r, c)) {
            return Some((0..side).map(|c| (r, c)).collect());
        }
    }

    // Columns
    for c in 0..side {
        if (0..side).all(|r| is_filled(r, c)) {
            return Some((0..side).map(|r| (r, c)).collect());
        }
    }

    // Main diagonal
    if (0..side).all(|i| is_filled(i, i)) {
        return Some((0..side).map(|i| (i, i)).collect());
    }

    // Anti-diagonal
    if (0..side).all(|i| is_filled(i, side - 1 - i)) {
        return Some((0..side).map(|i| (i, side - 1 - i)).collect());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(board_id: BoardId, row: u8, col: u8, captured: bool, free: bool) -> BoardTile {
        let mut t = BoardTile::new(
            BoardTileId(Uuid::new_v4()),
            NewBoardTile {
                board_id,
                row,
                col,
                prompt_submission_id: None,
                target_user_id: None,
                prompt_text: NonEmptyString::neu("Test").unwrap(),
                is_free_space: free,
            },
        );
        if captured {
            t.attach_capture(CaptureId(Uuid::new_v4()));
        }
        t
    }

    #[test_log::test]
    fn bingo_detects_row_col_and_diagonal_with_free_space() {
        let b = BoardId(Uuid::new_v4());
        // 3x3 with center free space (1,1) and (0,0), (2,2) -> main diagonal!
        let mut tiles = Vec::new();
        for r in 0..3 {
            for c in 0..3 {
                let free = r == 1 && c == 1;
                let cap = (r == 0 && c == 0) || (r == 2 && c == 2);
                tiles.push(tile(b, r, c, cap, free));
            }
        }
        assert_eq!(
            evaluate_bingo(GridSize::Three, &tiles),
            Some(vec![(0, 0), (1, 1), (2, 2)])
        );

        // Remove (2,2) -> no bingo
        tiles.last_mut().unwrap().capture_id = None;
        assert_eq!(evaluate_bingo(GridSize::Three, &tiles), None);
    }

    #[test_log::test]
    fn bingo_detects_anti_diagonal_and_column() {
        let b = BoardId(Uuid::new_v4());
        let mut tiles = Vec::new();
        for r in 0..4 {
            for c in 0..4 {
                tiles.push(tile(b, r, c, r + c == 3, false));
            }
        }
        assert_eq!(
            evaluate_bingo(GridSize::Four, &tiles),
            Some(vec![(0, 3), (1, 2), (2, 1), (3, 0)])
        );
    }
}
