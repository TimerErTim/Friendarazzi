//! Outbound schemas of the `konto` subdomain.

use uuid::Uuid;

use friendarazzi_core::domain;

use crate::schema::EmailAdresse;

/// The authenticated caller, as `GET /benutzer` answers it.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatedBenutzer {
    /// Identity of the Benutzer.
    pub id: Uuid,
    /// Its e-mail address.
    pub e_mail_adresse: EmailAdresse,
    /// Display name shown in lobbies and boards.
    pub display_name: String,
    /// Storage key of the player's avatar, if uploaded.
    pub avatar_storage_key: Option<String>,
    /// Lifetime Gold Medals earned.
    pub gold_medal_count: u32,
    /// Effective rank title displayed on badges.
    pub effective_title: String,
}

/// A known Benutzer.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Benutzer {
    /// Identity of the Benutzer.
    pub id: Uuid,
    /// E-mail address.
    pub e_mail_adresse: EmailAdresse,
}

impl From<domain::Benutzer> for Benutzer {
    fn from(benutzer: domain::Benutzer) -> Self {
        Self {
            id: benutzer.id().0,
            e_mail_adresse: benutzer.email_address().clone().into(),
        }
    }
}

impl From<domain::Benutzer> for AuthenticatedBenutzer {
    fn from(benutzer: domain::Benutzer) -> Self {
        let effective_title = benutzer.effective_title().to_owned();
        Self {
            id: benutzer.id().0,
            e_mail_adresse: benutzer.email_address().clone().into(),
            display_name: benutzer.display_name().as_str().to_owned(),
            avatar_storage_key: benutzer.avatar_storage_key().map(str::to_owned),
            gold_medal_count: benutzer.gold_medal_count(),
            effective_title,
        }
    }
}

/// What registration step 2, sign-in, and session renewal hand back.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeueSession {
    /// The token to send as `Authorization: Bearer <token>`.
    pub access_token: String,
    /// Kept for the response builder; it never leaves in a body.
    pub refresh_token: String,
}
