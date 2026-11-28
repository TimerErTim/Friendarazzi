use chrono::{DateTime, Utc};

use crate::application::ResultReport;
use crate::domain::konto::{OidcProviderDescriptor, OidcProviderId, VerifiedOidcIdentity};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OidcClientError {
    #[error("dieser Anmeldeanbieter ist nicht konfiguriert")]
    ProviderNotConfigured,
    #[error("OIDC-Discovery fehlgeschlagen")]
    DiscoveryFailed,
    #[error("Token-Austausch mit dem Anbieter fehlgeschlagen")]
    TokenExchangeFailed,
    #[error("ID-Token konnte nicht verifiziert werden")]
    InvalidIdToken,
    #[error("Nonce stimmt nicht überein")]
    NonceMismatch,
    #[error("der Anbieter bestätigt diese E-Mail-Adresse nicht")]
    EmailNotVerified,
    #[error("die vom Anbieter gelieferte E-Mail-Adresse ist ungültig")]
    InvalidEmail,
    #[error("kryptographische OIDC-Prüfung fehlgeschlagen")]
    CryptoError,
}

/// Transport adapter for external OpenID Connect identity providers.
#[async_trait::async_trait]
pub trait OidcProviderClient: Send + Sync {
    fn configured_providers(&self) -> Vec<OidcProviderDescriptor>;

    async fn build_authorization_url(
        &self,
        provider_id: &OidcProviderId,
        state: &str,
        nonce: &str,
        code_challenge: &str,
    ) -> ResultReport<String, OidcClientError>;

    async fn exchange_and_verify(
        &self,
        provider_id: &OidcProviderId,
        authorization_code: &str,
        pkce_verifier: &str,
        expected_nonce: &str,
        now: DateTime<Utc>,
    ) -> ResultReport<VerifiedOidcIdentity, OidcClientError>;
}
