//! OpenAPI schemas for player badges, public profiles, and photo galleries.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::capture::CaptureFeedItem;

/// Compact player badge used for crosslinked avatars across lobbies, boards, and feeds.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerBadge {
    /// Player user ID.
    pub id: Uuid,
    /// Display name.
    pub display_name: String,
    /// Avatar storage key, if present.
    pub avatar_storage_key: Option<String>,
    /// Lifetime Gold Medals earned.
    pub gold_medal_count: u32,
    /// Active rank title.
    pub effective_title: String,
    /// Whether this player has locked their draft in the current game.
    pub draft_locked: bool,
    /// Whether this player is the host of the current game.
    pub is_host: bool,
}

/// Full player profile with prestige stats and unlocked titles.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    /// Player user ID.
    pub id: Uuid,
    /// Display name.
    pub display_name: String,
    /// Avatar storage key, if present.
    pub avatar_storage_key: Option<String>,
    /// Lifetime Gold Medals earned.
    pub gold_medal_count: u32,
    /// Explicitly equipped rank title, if any.
    pub equipped_title: Option<String>,
    /// Effective rank title currently shown.
    pub effective_title: String,
    /// All rank titles unlocked for this user.
    pub unlocked_titles: Vec<String>,
    /// Total games participated in.
    pub games_played: u32,
    /// Total captures taken by this user.
    pub captures_made: u32,
    /// Total times this user was caught in a photo.
    pub times_caught: u32,
}

/// One game section in a user's Caught / Captured photo gallery.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryGameGroup {
    /// Game ID.
    pub game_id: Uuid,
    /// Game title.
    pub game_title: String,
    /// Timestamp when game started.
    pub started_at: Option<DateTime<Utc>>,
    /// Timestamp when game ended.
    pub ended_at: Option<DateTime<Utc>>,
    /// Winner badge if completed.
    pub winner: Option<PlayerBadge>,
    /// Visible photos belonging to this game.
    pub photos: Vec<CaptureFeedItem>,
}

/// User profile photo galleries grouped by game.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserGallery {
    /// Profile owner summary.
    pub user: UserProfile,
    /// Photos where this user was the target, grouped by game.
    pub caught: Vec<GalleryGameGroup>,
    /// Photos taken by this user as the catcher, grouped by game.
    pub captured: Vec<GalleryGameGroup>,
}
