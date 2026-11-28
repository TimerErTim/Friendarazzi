//! Inbound requests of the `konto` subdomain: transport DTOs whose conversion into a use
//! case performs the domain validation once, at the edge. A conversion failure reuses the
//! use case's own `UngültigeAnfrage` variant, so no route invents a second error type.

use error_stack::{Report, ResultExt};
use friendarazzi_core::{
    domain,
    ports::ClientKey,
    service::{
        Anmeldung, AnmeldungFehler, BenutzerLöschen, BenutzerLöschenFehler, EmailBestätigen,
        EmailBestätigenFehler, OidcExchangeCodeForSession, OidcStartAuth, OidcUseCaseError,
        PasswortErstmaligSetzen, PasswortErstmaligSetzenFehler, PasswortVergessen,
        PasswortVergessenFehler, PasswortWiederherstellen, PasswortWiederherstellenFehler,
        Registrieren, RegistrierenFehler,
    },
};

use crate::schema::EmailAdresse;

/// Single-step registration with e-mail address, password, and optional display name.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrierungRequest {
    /// E-mail address of the new account.
    pub e_mail_adresse: EmailAdresse,
    /// Chosen account password (checked against the password policy).
    pub passwort: String,
    /// Optional display name shown in lobbies and boards.
    #[serde(default)]
    pub display_name: Option<String>,
}

/// Confirm the caller's e-mail address with the single-use verification token.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailBestaetigenRequest {
    /// The single-use verification token from the confirmation mail.
    pub token: String,
}

/// Sign-in with the account's e-mail address and password.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnmeldungRequest {
    /// E-mail address of the account.
    pub e_mail_adresse: EmailAdresse,
    /// The account's password.
    pub passwort: String,
}

/// Request a password reset mail.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswortVergessenRequest {
    /// The address to send the reset mail to.
    pub e_mail_adresse: EmailAdresse,
}

/// Set a new password from a mailed reset token.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswortWiederherstellenRequest {
    /// The single-use token from the reset mail.
    pub token: String,
    /// The new password. It is checked against the password policy.
    pub neues_passwort: String,
}

/// Delete one's own Benutzer: proven either with password re-entry or an OIDC re-auth code.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenutzerLoeschungRequest {
    /// The account's password, re-entered when deleting a password-based account.
    #[serde(default)]
    pub passwort: Option<String>,
    /// Single-use OIDC re-authentication code for passwordless account deletion.
    #[serde(default)]
    pub reauth_code: Option<String>,
    /// Must be `true`; a missing flag is a refusal, never a default.
    pub bestaetigt: bool,
}

/// Exchange a single-use OIDC browser callback code for a live session.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcExchangeRequest {
    /// Single-use opaque code received on `/anmeldung/oidc`.
    pub code: String,
}

/// Mode for an authenticated OIDC account ceremony.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Enum))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OidcAccountActionMode {
    /// Link a new external identity provider to the signed-in user.
    Link,
    /// Re-authenticate via OIDC before deleting a passwordless account.
    DeleteReauth,
}

/// Start an authenticated OIDC ceremony (link provider or deletion re-auth).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartOidcAccountActionRequest {
    /// Which authenticated OIDC flow to initiate.
    pub mode: OidcAccountActionMode,
}

/// Set an initial password on a passwordless OIDC-created account.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetInitialPasswordRequest {
    /// Chosen account password (checked against password policy).
    pub passwort: String,
}

impl RegistrierungRequest {
    pub fn in_use_case(
        self,
        client_key: ClientKey,
    ) -> Result<Registrieren, Report<RegistrierenFehler>> {
        let email = domain::EmailAdresse::try_from(self.e_mail_adresse)
            .change_context(RegistrierenFehler::UngültigeAnfrage)?;
        let passwort = domain::konto::KlartextPasswort::try_from(self.passwort)
            .change_context(RegistrierenFehler::UngültigeAnfrage)?;
        let display_name = match self.display_name {
            Some(raw) if !raw.trim().is_empty() => Some(
                domain::NonEmptyString::neu(raw)
                    .change_context(RegistrierenFehler::UngültigeAnfrage)?,
            ),
            _ => None,
        };
        Ok(Registrieren {
            email,
            passwort,
            display_name,
            client_key,
        })
    }
}

impl EmailBestaetigenRequest {
    pub fn in_use_case(
        self,
        client_key: ClientKey,
    ) -> Result<EmailBestätigen, Report<EmailBestätigenFehler>> {
        let token = domain::konto::SecretToken::try_from(self.token)
            .change_context(EmailBestätigenFehler::UngültigerLink)?;
        Ok(EmailBestätigen { token, client_key })
    }
}

impl AnmeldungRequest {
    pub fn in_use_case(self, client_key: ClientKey) -> Result<Anmeldung, Report<AnmeldungFehler>> {
        let email = domain::EmailAdresse::try_from(self.e_mail_adresse)
            .change_context(AnmeldungFehler::ZugangsdatenFalsch)?;
        // A Passwort that breaks the length rule cannot be right, but it is also not a
        // reason to answer differently from a wrong one.
        let passwort = domain::konto::KlartextPasswort::try_from(self.passwort)
            .change_context(AnmeldungFehler::ZugangsdatenFalsch)?;
        Ok(Anmeldung {
            email,
            passwort,
            client_key,
        })
    }
}

impl PasswortVergessenRequest {
    pub fn in_use_case(
        self,
        client_key: ClientKey,
    ) -> Result<PasswortVergessen, Report<PasswortVergessenFehler>> {
        // A malformed address is answered like a free one: no address oracle.
        let email = domain::EmailAdresse::try_from(self.e_mail_adresse)
            .change_context(PasswortVergessenFehler::UngültigeAnfrage)?;
        Ok(PasswortVergessen { email, client_key })
    }
}

impl PasswortWiederherstellenRequest {
    /// No throttle key: the token is single-use and short-lived, and a per-client limit
    /// would let one shared IP lock out a whole household.
    pub fn in_use_case(
        self,
    ) -> Result<PasswortWiederherstellen, Report<PasswortWiederherstellenFehler>> {
        let token = domain::konto::SecretToken::try_from(self.token)
            .change_context(PasswortWiederherstellenFehler::UngültigeAnfrage)?;
        let neues_passwort = domain::konto::KlartextPasswort::try_from(self.neues_passwort)
            .change_context(PasswortWiederherstellenFehler::UngültigeAnfrage)?;
        Ok(PasswortWiederherstellen {
            token,
            neues_passwort,
        })
    }
}

impl BenutzerLoeschungRequest {
    pub fn in_use_case(self) -> Result<BenutzerLöschen, Report<BenutzerLöschenFehler>> {
        let passwort = self
            .passwort
            .filter(|s| !s.is_empty())
            .map(domain::konto::KlartextPasswort::try_from)
            .transpose()
            .change_context(BenutzerLöschenFehler::UngültigeAnfrage)?;
        let reauth_code = self
            .reauth_code
            .filter(|s| !s.is_empty())
            .map(|raw| domain::konto::SecretToken::try_from(raw.as_str()))
            .transpose()
            .change_context(BenutzerLöschenFehler::UngültigeAnfrage)?;
        Ok(BenutzerLöschen {
            passwort,
            reauth_code,
            bestätigt: self.bestaetigt,
        })
    }
}

impl OidcExchangeRequest {
    pub fn in_use_case(
        self,
        client_key: ClientKey,
    ) -> Result<OidcExchangeCodeForSession, Report<OidcUseCaseError>> {
        let code = domain::konto::SecretToken::try_from(self.code.as_str())
            .change_context(OidcUseCaseError::CodeUngültig)?;
        Ok(OidcExchangeCodeForSession { code, client_key })
    }
}

impl StartOidcAccountActionRequest {
    pub fn in_use_case(
        self,
        provider_id: domain::konto::OidcProviderId,
        benutzer_id: domain::BenutzerId,
        client_key: ClientKey,
    ) -> OidcStartAuth {
        let mode = match self.mode {
            OidcAccountActionMode::Link => domain::konto::OidcFlowMode::Link { benutzer_id },
            OidcAccountActionMode::DeleteReauth => {
                domain::konto::OidcFlowMode::DeleteReauth { benutzer_id }
            }
        };
        OidcStartAuth {
            provider_id,
            mode,
            return_to: None,
            client_key,
        }
    }
}

impl SetInitialPasswordRequest {
    pub fn in_use_case(
        self,
    ) -> Result<PasswortErstmaligSetzen, Report<PasswortErstmaligSetzenFehler>> {
        let neues_passwort = domain::konto::KlartextPasswort::try_from(self.passwort)
            .change_context(PasswortErstmaligSetzenFehler::UngültigeAnfrage)?;
        Ok(PasswortErstmaligSetzen { neues_passwort })
    }
}
