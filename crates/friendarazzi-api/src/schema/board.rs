//! OpenAPI schemas for Bingo boards, tiles, and coordinates.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::profile::PlayerBadge;

/// One tile on a Bingo board.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardTileView {
    /// Tile ID.
    pub id: Uuid,
    /// Zero-based grid row.
    pub row: u8,
    /// Zero-based grid column.
    pub col: u8,
    /// Prompt text on the tile.
    pub prompt_text: String,
    /// Target friend badge.
    pub target_user: PlayerBadge,
    /// True if a photo capture is attached.
    pub is_captured: bool,
    /// Attached capture ID if visible to the caller.
    pub capture_id: Option<Uuid>,
    /// Media storage key for the tile photo if visible to the caller.
    pub image_storage_key: Option<String>,
    /// Timestamp when the tile was captured.
    pub captured_at: Option<DateTime<Utc>>,
}

/// One player's Bingo board.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardView {
    /// Board ID.
    pub id: Uuid,
    /// Owner player badge.
    pub owner: PlayerBadge,
    /// True if this board completed a Bingo line.
    pub has_bingo: bool,
    /// Grid tiles ordered by row then column.
    pub tiles: Vec<BoardTileView>,
}

/// Grid coordinate on a Bingo board.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridCoordinate {
    /// Zero-based row.
    pub row: u8,
    /// Zero-based column.
    pub col: u8,
}

impl From<friendarazzi_core::domain::BoardCoordinates> for GridCoordinate {
    fn from(coords: friendarazzi_core::domain::BoardCoordinates) -> Self {
        Self {
            row: coords.row(),
            col: coords.col(),
        }
    }
}
