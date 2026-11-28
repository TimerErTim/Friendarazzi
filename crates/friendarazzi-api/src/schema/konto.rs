//! Outbound schemas of the `konto` subdomain.

use chrono::{DateTime, Utc};
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
    /// Whether the Benutzer has confirmed ownership of its e-mail address.
    pub email_verified: bool,
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
        let email_verified = benutzer.ist_verifiziert();
        Self {
            id: benutzer.id().0,
            e_mail_adresse: benutzer.email_address().clone().into(),
            display_name: benutzer.display_name().as_str().to_owned(),
            avatar_storage_key: benutzer.avatar_storage_key().map(str::to_owned),
            gold_medal_count: benutzer.gold_medal_count(),
            effective_title,
            email_verified,
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

/// Supported external OpenID Connect provider families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Enum))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OidcProviderKindSchema {
    /// Google Sign-In.
    Google,
    /// Sign in with Apple.
    Apple,
    /// Configured generic OIDC provider.
    Generic,
}

impl From<domain::konto::OidcProviderKind> for OidcProviderKindSchema {
    fn from(kind: domain::konto::OidcProviderKind) -> Self {
        match kind {
            domain::konto::OidcProviderKind::Google => Self::Google,
            domain::konto::OidcProviderKind::Apple => Self::Apple,
            domain::konto::OidcProviderKind::Generic => Self::Generic,
        }
    }
}

/// Public metadata for one configured external OIDC provider.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcProviderItem {
    /// URL-safe provider slug (`google`, `apple`, or generic id).
    pub id: String,
    /// Provider family kind.
    pub kind: OidcProviderKindSchema,
    /// Human-readable label rendered on login buttons.
    pub display_name: String,
}

impl From<domain::konto::OidcProviderDescriptor> for OidcProviderItem {
    fn from(descriptor: domain::konto::OidcProviderDescriptor) -> Self {
        Self {
            id: descriptor.id.as_str().to_owned(),
            kind: descriptor.kind.into(),
            display_name: descriptor.display_name,
        }
    }
}

/// List of configured OIDC sign-in providers available in this deployment.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcProvidersResponse {
    /// Enabled OIDC identity providers.
    pub providers: Vec<OidcProviderItem>,
}

/// Redirect URL returned when initiating an authenticated OIDC account flow.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcAuthorizationRedirect {
    /// Provider authorization URL the frontend navigates the browser to.
    pub authorization_url: String,
}

/// External identity linked to the authenticated Benutzer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedExternalIdentity {
    /// Identifier of this linked identity row.
    pub id: Uuid,
    /// Provider slug (`google`, `apple`, or generic id).
    pub provider_id: String,
    /// Display name of the provider.
    pub provider_display_name: String,
    /// Optional e-mail address asserted by the provider at link time.
    pub provider_email: Option<String>,
    /// Timestamp when the provider identity was linked.
    pub created_at: DateTime<Utc>,
}

/// Overview of all sign-in methods configured for the authenticated Benutzer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginMethodsResponse {
    /// Whether the account currently has a local password credential.
    pub has_password: bool,
    /// External OIDC identities linked to the account.
    pub external_identities: Vec<LinkedExternalIdentity>,
    /// All OIDC providers configured in this deployment.
    pub available_providers: Vec<OidcProviderItem>,
}
