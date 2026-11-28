use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::Report;
use friendarazzi_core::{
    ResultReport,
    domain::konto::{OidcProviderDescriptor, OidcProviderId, VerifiedOidcIdentity},
    ports::{OidcClientError, OidcProviderClient},
};

#[derive(Debug, Clone)]
pub struct FakeAuthorizationCall {
    pub provider_id: OidcProviderId,
    pub state: String,
    pub nonce: String,
    pub code_challenge: String,
}

#[derive(Default)]
pub struct FakeOidcProviderClient {
    providers: Mutex<Vec<OidcProviderDescriptor>>,
    responses: Mutex<HashMap<String, Result<VerifiedOidcIdentity, OidcClientError>>>,
    calls: Mutex<Vec<FakeAuthorizationCall>>,
}

impl FakeOidcProviderClient {
    pub fn new(providers: Vec<OidcProviderDescriptor>) -> Self {
        Self {
            providers: Mutex::new(providers),
            responses: Mutex::new(HashMap::new()),
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn set_providers(&self, providers: Vec<OidcProviderDescriptor>) {
        *self.providers.lock().unwrap() = providers;
    }

    pub fn stub_exchange(
        &self,
        authorization_code: impl Into<String>,
        outcome: Result<VerifiedOidcIdentity, OidcClientError>,
    ) {
        self.responses
            .lock()
            .unwrap()
            .insert(authorization_code.into(), outcome);
    }

    pub fn authorization_calls(&self) -> Vec<FakeAuthorizationCall> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl OidcProviderClient for FakeOidcProviderClient {
    fn configured_providers(&self) -> Vec<OidcProviderDescriptor> {
        self.providers.lock().unwrap().clone()
    }

    async fn build_authorization_url(
        &self,
        provider_id: &OidcProviderId,
        state: &str,
        nonce: &str,
        code_challenge: &str,
    ) -> ResultReport<String, OidcClientError> {
        let configured = self
            .providers
            .lock()
            .unwrap()
            .iter()
            .any(|p| &p.id == provider_id);
        if !configured {
            return Err(Report::new(OidcClientError::ProviderNotConfigured));
        }
        self.calls.lock().unwrap().push(FakeAuthorizationCall {
            provider_id: provider_id.clone(),
            state: state.to_owned(),
            nonce: nonce.to_owned(),
            code_challenge: code_challenge.to_owned(),
        });
        Ok(format!(
            "https://idp.example.test/{}/authorize?state={state}&nonce={nonce}&code_challenge={code_challenge}&code_challenge_method=S256",
            provider_id.as_str()
        ))
    }

    async fn exchange_and_verify(
        &self,
        provider_id: &OidcProviderId,
        authorization_code: &str,
        _pkce_verifier: &str,
        _expected_nonce: &str,
        _now: DateTime<Utc>,
    ) -> ResultReport<VerifiedOidcIdentity, OidcClientError> {
        let configured = self
            .providers
            .lock()
            .unwrap()
            .iter()
            .any(|p| &p.id == provider_id);
        if !configured {
            return Err(Report::new(OidcClientError::ProviderNotConfigured));
        }
        match self.responses.lock().unwrap().remove(authorization_code) {
            Some(Ok(identity)) => Ok(identity),
            Some(Err(err)) => Err(Report::new(err)),
            None => Err(Report::new(OidcClientError::TokenExchangeFailed)),
        }
    }
}
