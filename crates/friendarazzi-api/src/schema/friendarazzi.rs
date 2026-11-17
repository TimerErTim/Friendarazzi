//! OpenAPI schemas for Friendarazzi games, boards, captures, decks, and user profiles.

use chrono::{DateTime, Utc};
use friendarazzi_core::domain;
use uuid::Uuid;

/// Proof visibility mode of a Friendarazzi game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Enum))]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum VisibilityMode {
    Undercover,
    LiveFeed,
    BlurredTeaser,
}

impl From<domain::VisibilityMode> for VisibilityMode {
    fn from(value: domain::VisibilityMode) -> Self {
        match value {
            domain::VisibilityMode::Undercover => Self::Undercover,
            domain::VisibilityMode::LiveFeed => Self::LiveFeed,
            domain::VisibilityMode::BlurredTeaser => Self::BlurredTeaser,
        }
    }
}

impl From<VisibilityMode> for domain::VisibilityMode {
    fn from(value: VisibilityMode) -> Self {
        match value {
            VisibilityMode::Undercover => Self::Undercover,
            VisibilityMode::LiveFeed => Self::LiveFeed,
            VisibilityMode::BlurredTeaser => Self::BlurredTeaser,
        }
    }
}

/// Lifecycle state of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Enum))]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum GameStatus {
    Lobby,
    Drafting,
    Active,
    Completed,
}

impl From<domain::GameStatus> for GameStatus {
    fn from(value: domain::GameStatus) -> Self {
        match value {
            domain::GameStatus::Lobby => Self::Lobby,
            domain::GameStatus::Drafting => Self::Drafting,
            domain::GameStatus::Active => Self::Active,
            domain::GameStatus::Completed => Self::Completed,
        }
    }
}

/// Source of a submitted prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Enum))]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum PromptSource {
    Custom,
    StarterDeck,
}

impl From<domain::PromptSource> for PromptSource {
    fn from(value: domain::PromptSource) -> Self {
        match value {
            domain::PromptSource::Custom => Self::Custom,
            domain::PromptSource::StarterDeck => Self::StarterDeck,
        }
    }
}

impl From<PromptSource> for domain::PromptSource {
    fn from(value: PromptSource) -> Self {
        match value {
            PromptSource::Custom => Self::Custom,
            PromptSource::StarterDeck => Self::StarterDeck,
        }
    }
}

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

/// Compact player badge used for crosslinked avatars across lobbies, boards, and feeds.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerBadge {
    /// Player user ID.
    pub id: Uuid,
    /// Public handle.
    pub username: String,
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
    /// Public handle.
    pub username: String,
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

/// Quick-Start deck template collection.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StarterDeck {
    /// Deck identifier.
    pub id: Uuid,
    /// URL-safe slug.
    pub slug: String,
    /// Display name of the deck.
    pub name: String,
    /// Short description of the deck theme.
    pub description: String,
    /// Prompt templates containing `{name}` placeholder.
    pub prompts: Vec<String>,
}

/// Required submission count for one target friend during drafting.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftTargetQuota {
    /// Target friend badge.
    pub target_user: PlayerBadge,
    /// Exact number of prompts required for this friend.
    pub required_count: u32,
}

/// One prompt submission authored by the caller.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptSubmissionView {
    /// Prompt submission ID.
    pub id: Uuid,
    /// Friend targeted by this prompt.
    pub target_user_id: Uuid,
    /// Prompt text.
    pub text: String,
    /// Custom or StarterDeck.
    pub source: PromptSource,
}

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
    /// Target friend badge, or null for Free Space.
    pub target_user: Option<PlayerBadge>,
    /// True if this is the center Free Space tile.
    pub is_free_space: bool,
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

/// Summary card for a game in lists and invite previews.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSummary {
    /// Game ID.
    pub id: Uuid,
    /// Short invite code.
    pub invite_code: String,
    /// Game title.
    pub title: String,
    /// Board side length (3, 4, or 5).
    pub grid_size: u8,
    /// Proof visibility mode.
    pub visibility_mode: VisibilityMode,
    /// Whether center Free Space is enabled on odd boards.
    pub include_free_space: bool,
    /// Current lifecycle state.
    pub status: GameStatus,
    /// Number of joined players.
    pub player_count: u32,
    /// Winner badge if completed.
    pub winner: Option<PlayerBadge>,
    /// Timestamp when active play started.
    pub started_at: Option<DateTime<Utc>>,
    /// Timestamp when Bingo was achieved.
    pub ended_at: Option<DateTime<Utc>>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Complete adaptive state of a game for `/games?id=...`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDetail {
    /// Game ID.
    pub id: Uuid,
    /// Short invite code.
    pub invite_code: String,
    /// Host user ID.
    pub host_user_id: Uuid,
    /// Game title.
    pub title: String,
    /// Board side length (3, 4, or 5).
    pub grid_size: u8,
    /// Proof visibility mode.
    pub visibility_mode: VisibilityMode,
    /// Whether center Free Space is enabled on odd boards.
    pub include_free_space: bool,
    /// Current lifecycle state.
    pub status: GameStatus,
    /// Playable tile count per player.
    pub playable_tile_count: u32,
    /// Timestamp when active play started.
    pub started_at: Option<DateTime<Utc>>,
    /// Timestamp when Bingo was achieved.
    pub ended_at: Option<DateTime<Utc>>,
    /// Winner badge if completed.
    pub winner: Option<PlayerBadge>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Joined players in deterministic turn order.
    pub players: Vec<PlayerBadge>,
    /// Equal-distribution prompt quotas for the caller during Drafting.
    pub my_quotas: Vec<DraftTargetQuota>,
    /// Caller's saved prompt drafts during Drafting.
    pub my_drafts: Vec<PromptSubmissionView>,
    /// Caller's own Bingo board during Active and Completed play.
    pub my_board: Option<BoardView>,
    /// All players' unlocked boards once Completed.
    pub all_boards: Vec<BoardView>,
    /// Group activity / timeline captures visible according to game mode & status.
    pub feed: Vec<CaptureFeedItem>,
    /// Automated post-game superlatives once Completed.
    pub superlatives: Vec<SuperlativeView>,
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

/// Response returned after uploading a photo capture for a tile.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureOutcome {
    /// Created capture record.
    pub capture: CaptureFeedItem,
    /// True if this capture triggered Bingo and ended the game.
    pub game_completed: bool,
    /// Winning coordinates if Bingo was achieved.
    pub winning_line: Vec<GridCoordinate>,
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
