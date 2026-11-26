//! Inbound request DTOs for Friendarazzi games, drafting, and user profiles.

use error_stack::{Report, ResultExt};
use friendarazzi_core::{
    domain::{BenutzerId, GameId, GridSize, InviteCode, NonEmptyString},
    service::{
        BenachrichtigungsEinstellungenAktualisieren, CreateGame, DraftPromptInput,
        GameUseCaseError, JoinGame, ProfileUseCaseError, SaveDraftPrompts, SpielBeobachtungSetzen,
        UpdateGameSettings, UpdateProfile,
    },
};
use uuid::Uuid;

use crate::schema::{PromptSource, VisibilityMode};

/// Create a new Friendarazzi game lobby.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGameRequest {
    /// Name of the game (e.g., "Friday Night Out").
    pub title: String,
    /// Board side length: 3, 4, or 5.
    pub grid_size: u8,
    /// Proof visibility mode.
    pub visibility_mode: VisibilityMode,
}

impl CreateGameRequest {
    pub fn into_use_case(self) -> Result<CreateGame, Report<GameUseCaseError>> {
        let title = NonEmptyString::neu(self.title).change_context(GameUseCaseError::BadRequest)?;
        let grid_size =
            GridSize::from_u8(self.grid_size).change_context(GameUseCaseError::BadRequest)?;
        Ok(CreateGame {
            title,
            grid_size,
            visibility_mode: self.visibility_mode.into(),
        })
    }
}

/// Update lobby settings before drafting starts.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGameSettingsRequest {
    /// Updated game title.
    pub title: String,
    /// Board side length: 3, 4, or 5.
    pub grid_size: u8,
    /// Proof visibility mode.
    pub visibility_mode: VisibilityMode,
}

impl UpdateGameSettingsRequest {
    pub fn into_use_case(
        self,
        game_id: GameId,
    ) -> Result<UpdateGameSettings, Report<GameUseCaseError>> {
        let title = NonEmptyString::neu(self.title).change_context(GameUseCaseError::BadRequest)?;
        let grid_size =
            GridSize::from_u8(self.grid_size).change_context(GameUseCaseError::BadRequest)?;
        Ok(UpdateGameSettings {
            game_id,
            title,
            grid_size,
            visibility_mode: self.visibility_mode.into(),
        })
    }
}

/// Join a game lobby using an invite code.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinGameRequest {
    /// Short URL-safe invite code.
    pub invite_code: String,
}

impl JoinGameRequest {
    pub fn into_use_case(self) -> Result<JoinGame, Report<GameUseCaseError>> {
        let invite_code =
            InviteCode::new(self.invite_code).change_context(GameUseCaseError::BadRequest)?;
        Ok(JoinGame { invite_code })
    }
}

/// Single prompt item in a player's draft submission batch.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftPromptRequestItem {
    /// Target friend user ID.
    pub target_user_id: Uuid,
    /// Prompt text describing the habit/action.
    pub text: String,
    /// Custom or StarterDeck.
    pub source: PromptSource,
}

/// Save or replace the caller's draft prompt submissions for a game.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDraftPromptsRequest {
    /// Full list of prompts authored by the caller.
    pub prompts: Vec<DraftPromptRequestItem>,
}

impl SaveDraftPromptsRequest {
    pub fn into_use_case(
        self,
        game_id: GameId,
    ) -> Result<SaveDraftPrompts, Report<GameUseCaseError>> {
        let mut prompts = Vec::with_capacity(self.prompts.len());
        for item in self.prompts {
            let text =
                NonEmptyString::neu(item.text).change_context(GameUseCaseError::BadRequest)?;
            prompts.push(DraftPromptInput {
                target_user_id: BenutzerId(item.target_user_id),
                text,
                source: item.source.into(),
            });
        }
        Ok(SaveDraftPrompts { game_id, prompts })
    }
}

/// Update the caller's profile details and equipped Gold Medal title.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    /// Display name shown to friends.
    pub display_name: String,
    /// Optional unlocked rank title to equip.
    pub equipped_title: Option<String>,
}

impl UpdateProfileRequest {
    pub fn into_use_case(self) -> Result<UpdateProfile, Report<ProfileUseCaseError>> {
        let display_name = NonEmptyString::neu(self.display_name)
            .change_context(ProfileUseCaseError::BadRequest)?;
        let equipped_title = self
            .equipped_title
            .and_then(|t| NonEmptyString::neu(t).ok());
        Ok(UpdateProfile {
            display_name,
            equipped_title,
        })
    }
}

/// Update account-wide notification preferences and per-event email opt-outs.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotificationPreferencesRequest {
    /// Default watching state when creating or joining a game.
    pub default_watch_games: Option<bool>,
    /// Email notification on game start.
    pub email_game_started: Option<bool>,
    /// Email notification on new capture.
    pub email_capture_posted: Option<bool>,
    /// Email notification on game completion.
    pub email_game_completed: Option<bool>,
}

impl From<UpdateNotificationPreferencesRequest> for BenachrichtigungsEinstellungenAktualisieren {
    fn from(value: UpdateNotificationPreferencesRequest) -> Self {
        Self {
            default_watch_games: value.default_watch_games,
            email_game_started: value.email_game_started,
            email_capture_posted: value.email_capture_posted,
            email_game_completed: value.email_game_completed,
        }
    }
}

/// Toggle notification subscription for one game.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetGameWatchRequest {
    /// Whether the caller wants to watch notifications for this game.
    pub watching: bool,
}

impl SetGameWatchRequest {
    pub fn into_use_case(self, game_id: GameId) -> SpielBeobachtungSetzen {
        SpielBeobachtungSetzen {
            game_id,
            watching: self.watching,
        }
    }
}

/// Token-based one-click email unsubscribe payload.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribeNotificationRequest {
    /// Expiring unsubscribe token from the email footer or `List-Unsubscribe` header.
    pub token: String,
}
