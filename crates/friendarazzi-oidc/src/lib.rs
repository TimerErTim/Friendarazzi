use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use friendarazzi_core::{
    ResultReport,
    domain::konto::{
        OidcProviderDescriptor, OidcProviderId, OidcProviderKind, VerifiedOidcIdentity,
    },
    ports::{OidcClientError, OidcProviderClient},
};
use serde::Deserialize;
use tokio::sync::RwLock;
use url::Url;

mod apple;
mod jwks;

use jwks::{JwkSet, VerifyStatus, verify_id_token};

#[derive(Debug, Clone)]
pub struct GoogleOidcConfig {
    pub client_id: String,
    pub client_secret: String,
    pub issuer_url: String,
}

impl GoogleOidcConfig {
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            issuer_url: "https://accounts.google.com".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppleOidcConfig {
    pub client_id: String,
    pub team_id: String,
    pub key_id: String,
    pub private_key_pem: String,
    pub issuer_url: String,
}

impl AppleOidcConfig {
    pub fn new(
        client_id: impl Into<String>,
        team_id: impl Into<String>,
        key_id: impl Into<String>,
        private_key_pem: impl Into<String>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            team_id: team_id.into(),
            key_id: key_id.into(),
            private_key_pem: private_key_pem.into(),
            issuer_url: "https://appleid.apple.com".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenericOidcConfig {
    pub id: OidcProviderId,
    pub display_name: String,
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OidcProvidersConfig {
    pub callback_base_url: String,
    pub google: Option<GoogleOidcConfig>,
    pub apple: Option<AppleOidcConfig>,
    pub generic: Vec<GenericOidcConfig>,
}

#[derive(Debug, Clone)]
enum ProviderCredentials {
    Secret(String),
    Apple {
        team_id: String,
        key_id: String,
        private_key_pem: String,
    },
}

#[derive(Debug, Clone)]
struct ConfiguredProvider {
    descriptor: OidcProviderDescriptor,
    issuer_url: String,
    client_id: String,
    credentials: ProviderCredentials,
    scopes: String,
    response_mode: &'static str,
}

#[derive(Debug, Clone, Deserialize)]
struct DiscoveryDocument {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    jwks_uri: String,
}

#[derive(Debug, Deserialize)]
struct TokenEndpointResponse {
    id_token: String,
}

pub struct HttpOidcProviderClient {
    http: reqwest::Client,
    callback_base_url: String,
    providers: Vec<ConfiguredProvider>,
    discovery_cache: RwLock<HashMap<OidcProviderId, DiscoveryDocument>>,
    jwks_cache: RwLock<HashMap<OidcProviderId, JwkSet>>,
}

impl HttpOidcProviderClient {
    pub fn new(config: OidcProvidersConfig) -> Self {
        let mut providers = Vec::new();
        if let Some(google) = config.google {
            providers.push(ConfiguredProvider {
                descriptor: OidcProviderDescriptor {
                    id: OidcProviderId::parse("google").expect("static slug"),
                    kind: OidcProviderKind::Google,
                    display_name: "Google".to_owned(),
                },
                issuer_url: google.issuer_url,
                client_id: google.client_id,
                credentials: ProviderCredentials::Secret(google.client_secret),
                scopes: "openid email profile".to_owned(),
                response_mode: "query",
            });
        }
        if let Some(apple) = config.apple {
            providers.push(ConfiguredProvider {
                descriptor: OidcProviderDescriptor {
                    id: OidcProviderId::parse("apple").expect("static slug"),
                    kind: OidcProviderKind::Apple,
                    display_name: "Apple".to_owned(),
                },
                issuer_url: apple.issuer_url,
                client_id: apple.client_id,
                credentials: ProviderCredentials::Apple {
                    team_id: apple.team_id,
                    key_id: apple.key_id,
                    private_key_pem: apple.private_key_pem,
                },
                scopes: "openid email name".to_owned(),
                response_mode: "form_post",
            });
        }
        for generic in config.generic {
            providers.push(ConfiguredProvider {
                descriptor: OidcProviderDescriptor {
                    id: generic.id,
                    kind: OidcProviderKind::Generic,
                    display_name: generic.display_name,
                },
                issuer_url: generic.issuer_url,
                client_id: generic.client_id,
                credentials: ProviderCredentials::Secret(generic.client_secret),
                scopes: generic
                    .scopes
                    .unwrap_or_else(|| "openid email profile".to_owned()),
                response_mode: "query",
            });
        }

        Self {
            http: reqwest::Client::new(),
            callback_base_url: config.callback_base_url.trim_end_matches('/').to_owned(),
            providers,
            discovery_cache: RwLock::new(HashMap::new()),
            jwks_cache: RwLock::new(HashMap::new()),
        }
    }

    fn provider(
        &self,
        provider_id: &OidcProviderId,
    ) -> Result<&ConfiguredProvider, Report<OidcClientError>> {
        self.providers
            .iter()
            .find(|p| &p.descriptor.id == provider_id)
            .ok_or_else(|| Report::new(OidcClientError::ProviderNotConfigured))
    }

    fn redirect_uri(&self, provider_id: &OidcProviderId) -> String {
        format!(
            "{}/auth/oidc/{}/callback",
            self.callback_base_url,
            provider_id.as_str()
        )
    }

    async fn discovery(
        &self,
        provider: &ConfiguredProvider,
    ) -> Result<DiscoveryDocument, Report<OidcClientError>> {
        if let Some(cached) = self
            .discovery_cache
            .read()
            .await
            .get(&provider.descriptor.id)
            .cloned()
        {
            return Ok(cached);
        }
        let url = format!(
            "{}/.well-known/openid-configuration",
            provider.issuer_url.trim_end_matches('/')
        );
        let doc: DiscoveryDocument = self
            .http
            .get(&url)
            .send()
            .await
            .change_context(OidcClientError::DiscoveryFailed)?
            .error_for_status()
            .change_context(OidcClientError::DiscoveryFailed)?
            .json()
            .await
            .change_context(OidcClientError::DiscoveryFailed)?;
        self.discovery_cache
            .write()
            .await
            .insert(provider.descriptor.id.clone(), doc.clone());
        Ok(doc)
    }

    async fn jwks(
        &self,
        provider_id: &OidcProviderId,
        jwks_uri: &str,
        force_refresh: bool,
    ) -> Result<JwkSet, Report<OidcClientError>> {
        if !force_refresh
            && let Some(cached) = self.jwks_cache.read().await.get(provider_id).cloned()
        {
            return Ok(cached);
        }
        let set: JwkSet = self
            .http
            .get(jwks_uri)
            .send()
            .await
            .change_context(OidcClientError::DiscoveryFailed)?
            .error_for_status()
            .change_context(OidcClientError::DiscoveryFailed)?
            .json()
            .await
            .change_context(OidcClientError::DiscoveryFailed)?;
        self.jwks_cache
            .write()
            .await
            .insert(provider_id.clone(), set.clone());
        Ok(set)
    }
}

#[async_trait]
impl OidcProviderClient for HttpOidcProviderClient {
    fn configured_providers(&self) -> Vec<OidcProviderDescriptor> {
        self.providers
            .iter()
            .map(|p| p.descriptor.clone())
            .collect()
    }

    async fn build_authorization_url(
        &self,
        provider_id: &OidcProviderId,
        state: &str,
        nonce: &str,
        code_challenge: &str,
    ) -> ResultReport<String, OidcClientError> {
        let provider = self.provider(provider_id)?;
        let discovery = self.discovery(provider).await?;
        let mut url = Url::parse(&discovery.authorization_endpoint)
            .change_context(OidcClientError::DiscoveryFailed)?;
        url.query_pairs_mut()
            .append_pair("client_id", &provider.client_id)
            .append_pair("redirect_uri", &self.redirect_uri(provider_id))
            .append_pair("response_type", "code")
            .append_pair("response_mode", provider.response_mode)
            .append_pair("scope", &provider.scopes)
            .append_pair("state", state)
            .append_pair("nonce", nonce)
            .append_pair("code_challenge", code_challenge)
            .append_pair("code_challenge_method", "S256");
        Ok(url.into())
    }

    async fn exchange_and_verify(
        &self,
        provider_id: &OidcProviderId,
        authorization_code: &str,
        pkce_verifier: &str,
        expected_nonce: &str,
        now: DateTime<Utc>,
    ) -> ResultReport<VerifiedOidcIdentity, OidcClientError> {
        let provider = self.provider(provider_id)?;
        let discovery = self.discovery(provider).await?;
        let client_secret = match &provider.credentials {
            ProviderCredentials::Secret(secret) => secret.clone(),
            ProviderCredentials::Apple {
                team_id,
                key_id,
                private_key_pem,
            } => apple::build_apple_client_secret(
                team_id,
                &provider.client_id,
                key_id,
                private_key_pem,
                &discovery.issuer,
                now,
            )?,
        };

        let redirect_uri = self.redirect_uri(provider_id);
        let form = [
            ("grant_type", "authorization_code"),
            ("code", authorization_code),
            ("redirect_uri", redirect_uri.as_str()),
            ("client_id", provider.client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code_verifier", pkce_verifier),
        ];

        let token_res: TokenEndpointResponse = self
            .http
            .post(&discovery.token_endpoint)
            .form(&form)
            .send()
            .await
            .change_context(OidcClientError::TokenExchangeFailed)?
            .error_for_status()
            .change_context(OidcClientError::TokenExchangeFailed)?
            .json()
            .await
            .change_context(OidcClientError::TokenExchangeFailed)?;

        let jwks = self.jwks(provider_id, &discovery.jwks_uri, false).await?;
        match verify_id_token(
            &token_res.id_token,
            &jwks,
            provider_id,
            &discovery.issuer,
            &provider.client_id,
            expected_nonce,
            now,
        )? {
            VerifyStatus::Verified(identity) => Ok(identity),
            VerifyStatus::UnknownKid => {
                let refreshed = self.jwks(provider_id, &discovery.jwks_uri, true).await?;
                match verify_id_token(
                    &token_res.id_token,
                    &refreshed,
                    provider_id,
                    &discovery.issuer,
                    &provider.client_id,
                    expected_nonce,
                    now,
                )? {
                    VerifyStatus::Verified(identity) => Ok(identity),
                    VerifyStatus::UnknownKid => Err(Report::new(OidcClientError::InvalidIdToken)),
                }
            }
        }
    }
}
