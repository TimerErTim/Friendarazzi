//! Outbound schemas of the `konto` subdomain.

use uuid::Uuid;

use friendarazzi_core::domain;
use friendarazzi_core::domain::konto::Rolle as DomainBenutzerRolle;

use crate::schema::EmailAdresse;

/// `Benutzer` or `Systemadministrator`: the role a Benutzer holds on the whole instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Enum))]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum BenutzerRolle {
    Benutzer,
    Systemadministrator,
}

impl From<DomainBenutzerRolle> for BenutzerRolle {
    fn from(rolle: DomainBenutzerRolle) -> Self {
        match rolle {
            DomainBenutzerRolle::Benutzer => Self::Benutzer,
            DomainBenutzerRolle::Systemadministrator => Self::Systemadministrator,
        }
    }
}

/// The authenticated caller, as `GET /benutzer` answers it.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatedBenutzer {
    /// Identity of the Benutzer.
    pub id: Uuid,
    /// Its e-mail address, which is also the sign-in name.
    pub e_mail_adresse: EmailAdresse,
    /// Its role on the whole instance.
    pub rolle: BenutzerRolle,
    /// Public handle of the player.
    pub username: String,
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

impl From<(domain::konto::Benutzer, domain::EmailAdresse)> for Benutzer {
    fn from((benutzer, email): (domain::konto::Benutzer, domain::EmailAdresse)) -> Self {
        Self {
            id: benutzer.id().0,
            e_mail_adresse: email.into(),
        }
    }
}

impl
    From<(
        domain::konto::Benutzer,
        domain::EmailAdresse,
        domain::UserProfile,
    )> for AuthenticatedBenutzer
{
    fn from(
        (benutzer, email, profile): (
            domain::konto::Benutzer,
            domain::EmailAdresse,
            domain::UserProfile,
        ),
    ) -> Self {
        Self {
            id: benutzer.id().0,
            e_mail_adresse: email.into(),
            rolle: benutzer.rolle().into(),
            username: profile.username().as_str().to_owned(),
            display_name: profile.display_name().as_str().to_owned(),
            avatar_storage_key: profile.avatar_storage_key().map(str::to_owned),
            gold_medal_count: profile.gold_medal_count(),
            effective_title: profile.effective_title().to_owned(),
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
