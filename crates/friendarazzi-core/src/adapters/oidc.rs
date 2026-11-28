use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::Report;

use crate::{
    ResultReport,
    domain::konto::{OidcProviderDescriptor, OidcProviderId, VerifiedOidcIdentity},
    ports::{OidcClientError, OidcProviderClient},
};

/// Default OIDC adapter when no external identity provider is configured.
pub struct NullOidcProviderClient;

#[async_trait]
impl OidcProviderClient for NullOidcProviderClient {
    fn configured_providers(&self) -> Vec<OidcProviderDescriptor> {
        Vec::new()
    }

    async fn build_authorization_url(
        &self,
        _provider_id: &OidcProviderId,
        _state: &str,
        _nonce: &str,
        _code_challenge: &str,
    ) -> ResultReport<String, OidcClientError> {
        Err(Report::new(OidcClientError::ProviderNotConfigured))
    }

    async fn exchange_and_verify(
        &self,
        _provider_id: &OidcProviderId,
        _authorization_code: &str,
        _pkce_verifier: &str,
        _expected_nonce: &str,
        _now: DateTime<Utc>,
    ) -> ResultReport<VerifiedOidcIdentity, OidcClientError> {
        Err(Report::new(OidcClientError::ProviderNotConfigured))
    }
}
