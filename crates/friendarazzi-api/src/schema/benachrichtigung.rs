//! OpenAPI schemas for in-app notifications and notification preferences.

use chrono::{DateTime, Utc};
use friendarazzi_core::domain;
use uuid::Uuid;

use super::profile::PlayerBadge;

/// Type of notification event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Enum))]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum NotificationKind {
    GameStarted,
    CapturePosted,
    GameCompleted,
    NearBingo,
    DraftReminder,
}

impl From<domain::BenachrichtigungArt> for NotificationKind {
    fn from(value: domain::BenachrichtigungArt) -> Self {
        match value {
            domain::BenachrichtigungArt::SpielGestartet => Self::GameStarted,
            domain::BenachrichtigungArt::AufnahmeHochgeladen => Self::CapturePosted,
            domain::BenachrichtigungArt::SpielBeendet => Self::GameCompleted,
            domain::BenachrichtigungArt::FastBingo => Self::NearBingo,
            domain::BenachrichtigungArt::EntwurfErinnerung => Self::DraftReminder,
        }
    }
}

/// One inbox notification item with visibility-mode-aware details.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationView {
    /// Notification ID.
    pub id: Uuid,
    /// Event kind.
    pub kind: NotificationKind,
    /// Affected game ID.
    pub game_id: Uuid,
    /// Title of the affected game.
    pub game_title: String,
    /// Capture ID for `CapturePosted` notifications, used to open the inspection dialog directly.
    pub capture_id: Option<Uuid>,
    /// Catcher (for `CapturePosted`) or winner (for `GameCompleted`).
    pub actor: Option<PlayerBadge>,
    /// Captured target player (redacted in `BlurredTeaser` during active play).
    pub target: Option<PlayerBadge>,
    /// Prompt/activity text (redacted in `BlurredTeaser` during active play).
    pub prompt_text: Option<String>,
    /// Whether the capture is still blurred/redacted under `BlurredTeaser`.
    pub is_blurred: bool,
    /// Timestamp when the notification was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp when the notification was marked read, if any.
    pub read_at: Option<DateTime<Utc>>,
}

/// Caller's persistent notification inbox and unread badge count.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationsInbox {
    /// Number of unread notifications in the inbox.
    pub unread_count: u32,
    /// Inbox items ordered newest first.
    pub notifications: Vec<NotificationView>,
}

/// Account-wide notification defaults and per-event email preferences.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPreferences {
    /// Default watch/subscription state applied when creating or joining a game.
    pub default_watch_games: bool,
    /// Timestamp of the last change to `defaultWatchGames`.
    pub default_watch_updated_at: DateTime<Utc>,
    /// Send email when a game starts (drafting phase finishes).
    pub email_game_started: bool,
    /// Timestamp of the last change to `emailGameStarted`.
    pub email_game_started_updated_at: DateTime<Utc>,
    /// Send email when a new capture is uploaded in a watched game.
    pub email_capture_posted: bool,
    /// Timestamp of the last change to `emailCapturePosted`.
    pub email_capture_posted_updated_at: DateTime<Utc>,
    /// Send email when another player completes a game.
    pub email_game_completed: bool,
    /// Timestamp of the last change to `emailGameCompleted`.
    pub email_game_completed_updated_at: DateTime<Utc>,
}

impl From<domain::BenachrichtigungsEinstellungen> for NotificationPreferences {
    fn from(value: domain::BenachrichtigungsEinstellungen) -> Self {
        Self {
            default_watch_games: value.standard_spiel_beobachten().aktiv(),
            default_watch_updated_at: value.standard_spiel_beobachten().aktualisiert_am(),
            email_game_started: value.email_spiel_gestartet().aktiv(),
            email_game_started_updated_at: value.email_spiel_gestartet().aktualisiert_am(),
            email_capture_posted: value.email_aufnahme_hochgeladen().aktiv(),
            email_capture_posted_updated_at: value.email_aufnahme_hochgeladen().aktualisiert_am(),
            email_game_completed: value.email_spiel_beendet().aktiv(),
            email_game_completed_updated_at: value.email_spiel_beendet().aktualisiert_am(),
        }
    }
}

/// Confirmation returned by one-click token-based email unsubscribe.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribeOutcome {
    /// Always `true` when the preference was unsubscribed.
    pub ok: bool,
    /// The notification kind that was unsubscribed.
    pub kind: NotificationKind,
}
