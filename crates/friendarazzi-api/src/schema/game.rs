//! OpenAPI schemas for Friendarazzi games, drafting quotas, and prompt submissions.

use chrono::{DateTime, Utc};
use friendarazzi_core::domain;
use uuid::Uuid;

use super::{
    board::BoardView,
    capture::{CaptureFeedItem, SuperlativeView},
    profile::PlayerBadge,
};

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

impl From<&domain::Game> for GameStatus {
    fn from(value: &domain::Game) -> Self {
        match value {
            domain::Game::Lobby(_) => Self::Lobby,
            domain::Game::Drafting(_) => Self::Drafting,
            domain::Game::Active(_) => Self::Active,
            domain::Game::Completed(_) => Self::Completed,
        }
    }
}

/// How a completed game ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Enum))]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum CompletionReason {
    Bingo,
    Deadline,
    Abandoned,
}

impl From<domain::CompletionReason> for CompletionReason {
    fn from(value: domain::CompletionReason) -> Self {
        match value {
            domain::CompletionReason::Bingo => Self::Bingo,
            domain::CompletionReason::Deadline => Self::Deadline,
            domain::CompletionReason::Abandoned => Self::Abandoned,
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
    /// Current lifecycle state.
    pub status: GameStatus,
    /// Whether the caller is subscribed to notifications for this game.
    pub watching: bool,
    /// Playable tile count per player.
    pub playable_tile_count: u32,
    /// Number of joined players.
    pub player_count: u32,
    /// Caller's captured tile count during Active/Completed play (own board only).
    pub my_captured_tiles: Option<u32>,
    /// Caller's saved prompt count during Drafting.
    pub my_draft_filled: Option<u32>,
    /// Whether the caller has locked prompts during Drafting.
    pub my_draft_locked: Option<bool>,
    /// Number of players locked in during Drafting.
    pub ready_player_count: Option<u32>,
    /// Whether the caller currently has a pending action (e.g. start drafting or lock prompts).
    pub needs_action: bool,
    /// Shared countdown deadline once all players are ready during Drafting.
    pub draft_closes_at: Option<DateTime<Utc>>,
    /// Optional host-configured drafting deadline after which missing prompts auto-fill.
    pub draft_deadline_at: Option<DateTime<Utc>>,
    /// Optional deadline at which active play concludes automatically by tile count.
    pub ends_at: Option<DateTime<Utc>>,
    /// Reason the game ended, once completed.
    pub completion_reason: Option<CompletionReason>,
    /// Server clock timestamp when this summary was generated.
    pub server_now: DateTime<Utc>,
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
    /// Current lifecycle state.
    pub status: GameStatus,
    /// Whether the caller is subscribed to notifications for this game.
    pub watching: bool,
    /// Playable tile count per player.
    pub playable_tile_count: u32,
    /// Shared countdown deadline once all players are ready during Drafting.
    pub draft_closes_at: Option<DateTime<Utc>>,
    /// Optional host-configured drafting deadline after which missing prompts auto-fill.
    pub draft_deadline_at: Option<DateTime<Utc>>,
    /// Optional deadline at which active play concludes automatically by tile count.
    pub ends_at: Option<DateTime<Utc>>,
    /// Reason the game ended, once completed.
    pub completion_reason: Option<CompletionReason>,
    /// Server clock timestamp when this detail was generated.
    pub server_now: DateTime<Utc>,
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
