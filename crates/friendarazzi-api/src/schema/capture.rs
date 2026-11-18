//! OpenAPI schemas for photo captures, activity feeds, and post-game superlatives.

use chrono::{DateTime, Utc};
use friendarazzi_core::domain;
use uuid::Uuid;

use super::{board::GridCoordinate, profile::PlayerBadge};

/// Automated post-game superlative category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Enum))]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum SuperlativeKind {
    MostWanted,
    Ghost,
    QuickestDraw,
    Sniper,
}

impl From<domain::SuperlativeKind> for SuperlativeKind {
    fn from(value: domain::SuperlativeKind) -> Self {
        match value {
            domain::SuperlativeKind::MostWanted => Self::MostWanted,
            domain::SuperlativeKind::Ghost => Self::Ghost,
            domain::SuperlativeKind::QuickestDraw => Self::QuickestDraw,
            domain::SuperlativeKind::Sniper => Self::Sniper,
        }
    }
}

/// One capture entry shown in the game feed, 1:1 timeline, or galleries.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFeedItem {
    /// Capture ID.
    pub id: Uuid,
    /// Tile ID on the catcher's board.
    pub tile_id: Uuid,
    /// Catcher player badge.
    pub catcher: PlayerBadge,
    /// Target player badge, redacted as null during BlurredTeaser active play.
    pub target: Option<PlayerBadge>,
    /// Prompt text, redacted as null during BlurredTeaser active play.
    pub prompt_text: Option<String>,
    /// Full image storage key, redacted as null during BlurredTeaser active play.
    pub image_storage_key: Option<String>,
    /// Blurred 64px thumbnail storage key.
    pub thumbnail_storage_key: String,
    /// Capture timestamp.
    pub captured_at: DateTime<Utc>,
    /// True when the item is currently blurred/redacted.
    pub is_blurred: bool,
}

/// Post-game automated superlative award.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuperlativeView {
    /// Superlative category.
    pub kind: SuperlativeKind,
    /// Awarded player badge.
    pub user: PlayerBadge,
    /// Numeric metric value (count or elapsed seconds).
    pub metric_value: i64,
    /// Associated capture ID for QuickestDraw.
    pub capture_id: Option<Uuid>,
}

/// Response returned after uploading a photo capture for a tile.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureOutcome {
    /// Created capture record.
    pub capture: CaptureFeedItem,
    /// Winning coordinates if Bingo was achieved and the game completed; null otherwise.
    pub winning_line: Option<Vec<GridCoordinate>>,
}
