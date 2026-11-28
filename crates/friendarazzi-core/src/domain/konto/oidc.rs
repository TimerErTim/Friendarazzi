//! External OpenID Connect identities, auth states, and short-lived exchange codes.

use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::domain::{BenutzerId, EmailAdresse};

use super::{KontoFehler, SecretTokenHash, token::basis64url};

/// Lifetime of one OIDC authorization request state/PKCE/nonce record.
pub const OIDC_AUTH_STATE_GÜLTIGKEIT: Duration = Duration::minutes(10);

/// Lifetime of one single-use OIDC browser exchange code (`302 -> POST /exchange`).
pub const OIDC_EXCHANGE_CODE_GÜLTIGKEIT: Duration = Duration::seconds(120);

/// Built-in and configurable external OIDC provider families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OidcProviderKind {
    Google,
    Apple,
    Generic,
}

impl OidcProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Apple => "apple",
            Self::Generic => "generic",
        }
    }
}

/// Validated lowercase ASCII identifier of an OIDC provider (`[a-z0-9_-]{1,32}`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OidcProviderId(String);

impl OidcProviderId {
    pub fn parse(raw: &str) -> Result<Self, KontoFehler> {
        let trimmed = raw.trim();
        let valid = !trimmed.is_empty()
            && trimmed.len() <= 32
            && trimmed
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_');
        if !valid {
            return Err(KontoFehler::OidcProviderUngültig);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for OidcProviderId {
    type Error = KontoFehler;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for OidcProviderId {
    type Error = KontoFehler;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

/// Public descriptor published by `GET /auth/providers` and login-method listings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcProviderDescriptor {
    pub id: OidcProviderId,
    pub kind: OidcProviderKind,
    pub display_name: String,
}

/// Surrogate key of a linked external identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExternalIdentityId(pub Uuid);

/// Creation payload for a linked external identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeueExternalIdentity {
    benutzer_id: BenutzerId,
    provider_id: OidcProviderId,
    subject: String,
    provider_email: Option<EmailAdresse>,
    created_at: DateTime<Utc>,
}

impl NeueExternalIdentity {
    pub fn neu(
        benutzer_id: BenutzerId,
        provider_id: OidcProviderId,
        subject: impl Into<String>,
        provider_email: Option<EmailAdresse>,
        created_at: DateTime<Utc>,
    ) -> Result<Self, KontoFehler> {
        let subject = subject.into().trim().to_owned();
        if subject.is_empty() {
            return Err(KontoFehler::OidcProviderUngültig);
        }
        Ok(Self {
            benutzer_id,
            provider_id,
            subject,
            provider_email,
            created_at,
        })
    }

    pub fn benutzer_id(&self) -> BenutzerId {
        self.benutzer_id
    }

    pub fn provider_id(&self) -> &OidcProviderId {
        &self.provider_id
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn provider_email(&self) -> Option<&EmailAdresse> {
        self.provider_email.as_ref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

/// Linked external OIDC identity belonging to one `Benutzer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalIdentity {
    id: ExternalIdentityId,
    benutzer_id: BenutzerId,
    provider_id: OidcProviderId,
    subject: String,
    provider_email: Option<EmailAdresse>,
    created_at: DateTime<Utc>,
}

impl ExternalIdentity {
    pub fn from_parts(
        id: ExternalIdentityId,
        benutzer_id: BenutzerId,
        provider_id: OidcProviderId,
        subject: String,
        provider_email: Option<EmailAdresse>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            benutzer_id,
            provider_id,
            subject,
            provider_email,
            created_at,
        }
    }

    pub fn id(&self) -> ExternalIdentityId {
        self.id
    }

    pub fn benutzer_id(&self) -> BenutzerId {
        self.benutzer_id
    }

    pub fn provider_id(&self) -> &OidcProviderId {
        &self.provider_id
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn provider_email(&self) -> Option<&EmailAdresse> {
        self.provider_email.as_ref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

/// Which ceremony started an `OidcAuthState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OidcFlowMode {
    Login,
    Link { benutzer_id: BenutzerId },
    DeleteReauth { benutzer_id: BenutzerId },
}

/// Surrogate key of one `OidcAuthState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OidcAuthStateId(pub Uuid);

/// Creation payload for a single-use OIDC auth state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeuerOidcAuthState {
    provider_id: OidcProviderId,
    state_hash: SecretTokenHash,
    nonce: String,
    pkce_verifier: String,
    mode: OidcFlowMode,
    return_to: Option<String>,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl NeuerOidcAuthState {
    pub fn neu(
        provider_id: OidcProviderId,
        state_hash: SecretTokenHash,
        nonce: impl Into<String>,
        pkce_verifier: impl Into<String>,
        mode: OidcFlowMode,
        return_to: Option<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            provider_id,
            state_hash,
            nonce: nonce.into(),
            pkce_verifier: pkce_verifier.into(),
            mode,
            return_to,
            created_at,
            expires_at: created_at + OIDC_AUTH_STATE_GÜLTIGKEIT,
        }
    }

    pub fn provider_id(&self) -> &OidcProviderId {
        &self.provider_id
    }

    pub fn state_hash(&self) -> &SecretTokenHash {
        &self.state_hash
    }

    pub fn nonce(&self) -> &str {
        &self.nonce
    }

    pub fn pkce_verifier(&self) -> &str {
        &self.pkce_verifier
    }

    pub fn mode(&self) -> OidcFlowMode {
        self.mode
    }

    pub fn return_to(&self) -> Option<&str> {
        self.return_to.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
}

/// Single-use, short-lived state/PKCE/nonce record for an in-flight OIDC redirect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcAuthState {
    id: OidcAuthStateId,
    provider_id: OidcProviderId,
    state_hash: SecretTokenHash,
    nonce: String,
    pkce_verifier: String,
    mode: OidcFlowMode,
    return_to: Option<String>,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    spent_at: Option<DateTime<Utc>>,
}

impl OidcAuthState {
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: OidcAuthStateId,
        provider_id: OidcProviderId,
        state_hash: SecretTokenHash,
        nonce: String,
        pkce_verifier: String,
        mode: OidcFlowMode,
        return_to: Option<String>,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        spent_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            provider_id,
            state_hash,
            nonce,
            pkce_verifier,
            mode,
            return_to,
            created_at,
            expires_at,
            spent_at,
        }
    }

    pub fn id(&self) -> OidcAuthStateId {
        self.id
    }

    pub fn provider_id(&self) -> &OidcProviderId {
        &self.provider_id
    }

    pub fn state_hash(&self) -> &SecretTokenHash {
        &self.state_hash
    }

    pub fn nonce(&self) -> &str {
        &self.nonce
    }

    pub fn pkce_verifier(&self) -> &str {
        &self.pkce_verifier
    }

    pub fn mode(&self) -> OidcFlowMode {
        self.mode
    }

    pub fn return_to(&self) -> Option<&str> {
        self.return_to.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    pub fn spent_at(&self) -> Option<DateTime<Utc>> {
        self.spent_at
    }

    pub fn validate(&self, now: DateTime<Utc>) -> Result<(), KontoFehler> {
        if self.spent_at.is_some() {
            return Err(KontoFehler::TokenVerbraucht);
        }
        if now >= self.expires_at {
            return Err(KontoFehler::TokenAbgelaufen);
        }
        Ok(())
    }

    pub fn verbrauchen(&mut self, now: DateTime<Utc>) -> Result<(), KontoFehler> {
        self.validate(now)?;
        self.spent_at = Some(now);
        Ok(())
    }
}

/// Purpose for which an `OidcExchangeCode` may be redeemed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OidcExchangePurpose {
    Session,
    DeleteReauth,
}

impl OidcExchangePurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::DeleteReauth => "delete_reauth",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, KontoFehler> {
        match raw {
            "session" => Ok(Self::Session),
            "delete_reauth" => Ok(Self::DeleteReauth),
            _ => Err(KontoFehler::TokenUngültig),
        }
    }
}

/// Surrogate key of one `OidcExchangeCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OidcExchangeCodeId(pub Uuid);

/// Creation payload for a single-use OIDC browser exchange code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeuerOidcExchangeCode {
    code_hash: SecretTokenHash,
    benutzer_id: BenutzerId,
    purpose: OidcExchangePurpose,
    expires_at: DateTime<Utc>,
}

impl NeuerOidcExchangeCode {
    pub fn neu(
        code_hash: SecretTokenHash,
        benutzer_id: BenutzerId,
        purpose: OidcExchangePurpose,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            code_hash,
            benutzer_id,
            purpose,
            expires_at: now + OIDC_EXCHANGE_CODE_GÜLTIGKEIT,
        }
    }

    pub fn code_hash(&self) -> &SecretTokenHash {
        &self.code_hash
    }

    pub fn benutzer_id(&self) -> BenutzerId {
        self.benutzer_id
    }

    pub fn purpose(&self) -> OidcExchangePurpose {
        self.purpose
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
}

/// Single-use, short-lived opaque code exchanged by the browser after OIDC callback redirect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcExchangeCode {
    id: OidcExchangeCodeId,
    code_hash: SecretTokenHash,
    benutzer_id: BenutzerId,
    purpose: OidcExchangePurpose,
    expires_at: DateTime<Utc>,
    spent_at: Option<DateTime<Utc>>,
}

impl OidcExchangeCode {
    pub fn from_parts(
        id: OidcExchangeCodeId,
        code_hash: SecretTokenHash,
        benutzer_id: BenutzerId,
        purpose: OidcExchangePurpose,
        expires_at: DateTime<Utc>,
        spent_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            code_hash,
            benutzer_id,
            purpose,
            expires_at,
            spent_at,
        }
    }

    pub fn id(&self) -> OidcExchangeCodeId {
        self.id
    }

    pub fn code_hash(&self) -> &SecretTokenHash {
        &self.code_hash
    }

    pub fn benutzer_id(&self) -> BenutzerId {
        self.benutzer_id
    }

    pub fn purpose(&self) -> OidcExchangePurpose {
        self.purpose
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    pub fn spent_at(&self) -> Option<DateTime<Utc>> {
        self.spent_at
    }

    pub fn einlösen(
        &mut self,
        expected_purpose: OidcExchangePurpose,
        now: DateTime<Utc>,
    ) -> Result<(), KontoFehler> {
        if self.purpose != expected_purpose {
            return Err(KontoFehler::TokenUngültig);
        }
        if self.spent_at.is_some() {
            return Err(KontoFehler::TokenVerbraucht);
        }
        if now >= self.expires_at {
            return Err(KontoFehler::TokenAbgelaufen);
        }
        self.spent_at = Some(now);
        Ok(())
    }
}

/// Verified claims returned by an external OIDC provider after ID-token validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedOidcIdentity {
    pub provider_id: OidcProviderId,
    pub subject: String,
    pub email: EmailAdresse,
    pub display_name: Option<String>,
}

/// Derive a PKCE S256 `code_challenge` from an ASCII `code_verifier`.
pub fn pkce_s256_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    basis64url(&digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::konto::SecretToken;

    #[test_log::test]
    fn provider_id_validates_lowercase_ascii_slugs() {
        assert_eq!(OidcProviderId::parse("google").unwrap().as_str(), "google");
        assert_eq!(
            OidcProviderId::parse("my_sso-1").unwrap().as_str(),
            "my_sso-1"
        );
        assert_eq!(
            OidcProviderId::parse("Google"),
            Err(KontoFehler::OidcProviderUngültig)
        );
        assert_eq!(
            OidcProviderId::parse(""),
            Err(KontoFehler::OidcProviderUngültig)
        );
        assert_eq!(
            OidcProviderId::parse("a/b"),
            Err(KontoFehler::OidcProviderUngültig)
        );
    }

    #[test_log::test]
    fn oidc_auth_state_enforces_expiry_and_single_use() {
        let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        let token = SecretToken::random().unwrap();
        let draft = NeuerOidcAuthState::neu(
            OidcProviderId::parse("google").unwrap(),
            token.hash(),
            "nonce-1",
            "verifier-1",
            OidcFlowMode::Login,
            Some("/join?code=ABC".to_owned()),
            now,
        );
        let mut state = OidcAuthState::from_parts(
            OidcAuthStateId(Uuid::new_v4()),
            draft.provider_id().clone(),
            draft.state_hash().clone(),
            draft.nonce().to_owned(),
            draft.pkce_verifier().to_owned(),
            draft.mode(),
            draft.return_to().map(str::to_owned),
            draft.created_at(),
            draft.expires_at(),
            None,
        );
        assert!(state.validate(now + Duration::minutes(9)).is_ok());
        assert_eq!(
            state.validate(now + OIDC_AUTH_STATE_GÜLTIGKEIT),
            Err(KontoFehler::TokenAbgelaufen)
        );
        state.verbrauchen(now + Duration::seconds(5)).unwrap();
        assert_eq!(
            state.verbrauchen(now + Duration::seconds(6)),
            Err(KontoFehler::TokenVerbraucht)
        );
    }

    #[test_log::test]
    fn oidc_exchange_code_enforces_purpose_expiry_and_single_use() {
        let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        let token = SecretToken::random().unwrap();
        let mut code = OidcExchangeCode::from_parts(
            OidcExchangeCodeId(Uuid::new_v4()),
            token.hash(),
            BenutzerId(Uuid::new_v4()),
            OidcExchangePurpose::Session,
            now + OIDC_EXCHANGE_CODE_GÜLTIGKEIT,
            None,
        );
        assert_eq!(
            code.einlösen(
                OidcExchangePurpose::DeleteReauth,
                now + Duration::seconds(1)
            ),
            Err(KontoFehler::TokenUngültig)
        );
        code.einlösen(OidcExchangePurpose::Session, now + Duration::seconds(10))
            .unwrap();
        assert_eq!(
            code.einlösen(OidcExchangePurpose::Session, now + Duration::seconds(11)),
            Err(KontoFehler::TokenVerbraucht)
        );

        let mut expired = OidcExchangeCode::from_parts(
            OidcExchangeCodeId(Uuid::new_v4()),
            token.hash(),
            BenutzerId(Uuid::new_v4()),
            OidcExchangePurpose::DeleteReauth,
            now + OIDC_EXCHANGE_CODE_GÜLTIGKEIT,
            None,
        );
        assert_eq!(
            expired.einlösen(
                OidcExchangePurpose::DeleteReauth,
                now + OIDC_EXCHANGE_CODE_GÜLTIGKEIT
            ),
            Err(KontoFehler::TokenAbgelaufen)
        );
    }

    #[test_log::test]
    fn pkce_s256_challenge_matches_rfc_7636_vector() {
        // RFC 7636 Appendix B test vector.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            pkce_s256_challenge(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }
}
