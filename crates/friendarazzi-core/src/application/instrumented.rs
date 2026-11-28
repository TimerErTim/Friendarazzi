use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;
use tracing::{instrument, trace};

use crate::ResultReport;
use crate::application::instrumented_repos::RepoStorage;
use crate::application::uow::UnitOfWorkImpl;
use crate::domain::konto::{OidcProviderDescriptor, OidcProviderId, VerifiedOidcIdentity};
use crate::ports::{
    BenachrichtigungRepository, BenutzerRepository, BoardRepository, CaptureRepository, ClientKey,
    DeckRepository, EmailBestätigungRepository, EventQueue, EventQueueError,
    ExternalIdentityRepository, GameRepository, KontoRepository, Mail, MailError, MailSender,
    ObjectStore, ObjectStoreError, ObjectStream, OidcAuthStateRepository, OidcClientError,
    OidcExchangeCodeRepository, OidcProviderClient, PasswortWiederherstellungRepository,
    PdfDokument, PdfRenderError, PdfRenderer, RepositoryResult, RngSource, SessionRepository,
    Throttle, ThrottleAction, ThrottleError,
};
use crate::service::Event;

#[derive(Clone)]
pub struct InstrumentedObjectStore {
    inner: Arc<dyn ObjectStore>,
}

impl InstrumentedObjectStore {
    pub fn new(inner: Arc<dyn ObjectStore>) -> Arc<dyn ObjectStore> {
        Arc::new(Self { inner })
    }
}

#[async_trait]
impl ObjectStore for InstrumentedObjectStore {
    #[instrument(skip(self, bytes), fields(key, bytes_len = bytes.len()), level = "debug", err(Debug))]
    async fn put(&self, key: &str, bytes: &[u8]) -> ResultReport<(), ObjectStoreError> {
        self.inner.put(key, bytes).await
    }

    #[instrument(skip(self), fields(key), level = "debug", err(Debug))]
    async fn get(&self, key: &str) -> ResultReport<Option<ObjectStream>, ObjectStoreError> {
        self.inner.get(key).await
    }

    #[instrument(skip(self), fields(key), level = "debug", err(Debug))]
    async fn delete(&self, key: &str) -> ResultReport<(), ObjectStoreError> {
        self.inner.delete(key).await
    }
}

#[derive(Clone)]
pub struct InstrumentedPdfRenderer {
    inner: Arc<dyn PdfRenderer>,
}

impl InstrumentedPdfRenderer {
    pub fn new(inner: Arc<dyn PdfRenderer>) -> Arc<dyn PdfRenderer> {
        Arc::new(Self { inner })
    }
}

#[async_trait]
impl PdfRenderer for InstrumentedPdfRenderer {
    #[instrument(skip(self, dokument), level = "debug", err(Debug))]
    async fn rendern(&self, dokument: &PdfDokument) -> ResultReport<Vec<u8>, PdfRenderError> {
        self.inner.rendern(dokument).await
    }
}

#[derive(Clone)]
pub struct InstrumentedThrottle {
    inner: Arc<dyn Throttle>,
}

impl InstrumentedThrottle {
    pub fn new(inner: Arc<dyn Throttle>) -> Arc<dyn Throttle> {
        Arc::new(Self { inner })
    }
}

#[async_trait]
impl Throttle for InstrumentedThrottle {
    #[instrument(skip(self, client), fields(action = action.as_str(), ip = %client.ip), level = "debug", err(Debug))]
    async fn check(
        &self,
        client: &ClientKey,
        action: ThrottleAction,
        now: chrono::DateTime<chrono::Utc>,
    ) -> ResultReport<(), ThrottleError> {
        self.inner.check(client, action, now).await
    }

    #[instrument(skip(self, client), fields(action = action.as_str(), ip = %client.ip), level = "debug", err(Debug))]
    async fn increment(
        &self,
        client: &ClientKey,
        action: ThrottleAction,
        now: chrono::DateTime<chrono::Utc>,
    ) -> ResultReport<(), ThrottleError> {
        self.inner.increment(client, action, now).await
    }

    #[instrument(skip(self, client), fields(action = action.as_str(), ip = %client.ip), level = "debug", err(Debug))]
    async fn check_and_increment(
        &self,
        client: &ClientKey,
        action: ThrottleAction,
        now: chrono::DateTime<chrono::Utc>,
    ) -> ResultReport<(), ThrottleError> {
        self.inner.check_and_increment(client, action, now).await
    }

    #[instrument(skip(self, client), fields(action = action.as_str(), ip = %client.ip), level = "debug", err(Debug))]
    async fn reset(
        &self,
        client: &ClientKey,
        action: ThrottleAction,
    ) -> ResultReport<(), ThrottleError> {
        self.inner.reset(client, action).await
    }
}

#[derive(Clone)]
pub struct InstrumentedMailSender {
    inner: Arc<dyn MailSender>,
}

impl InstrumentedMailSender {
    pub fn new(inner: Arc<dyn MailSender>) -> Arc<dyn MailSender> {
        Arc::new(Self { inner })
    }
}

#[async_trait]
impl MailSender for InstrumentedMailSender {
    // The body carries a single-use token, so it is never recorded in a span.
    #[instrument(skip(self, mail), fields(betreff = mail.betreff()), level = "debug", err(Debug))]
    async fn send(&self, mail: Mail) -> ResultReport<(), MailError> {
        self.inner.send(mail).await
    }

    #[instrument(skip(self), level = "debug", err(Debug))]
    async fn prüfe_verbindung(&self) -> ResultReport<(), MailError> {
        self.inner.prüfe_verbindung().await
    }
}

pub struct InstrumentedUnitOfWork {
    storage: Pin<Box<RepoStorage>>,
}

impl InstrumentedUnitOfWork {
    pub fn wrap(inner: Box<dyn UnitOfWorkImpl>) -> Box<dyn UnitOfWorkImpl> {
        Box::new(Self {
            storage: RepoStorage::new(inner),
        })
    }
}

#[async_trait]
impl UnitOfWorkImpl for InstrumentedUnitOfWork {
    #[instrument(skip(self), level = "debug", err(Debug))]
    async fn commit(self: Box<Self>) -> RepositoryResult<()> {
        let RepoStorage { uow, .. } = *Pin::into_inner(self.storage);
        uow.commit().await
    }

    #[instrument(skip(self), level = "debug", err(Debug))]
    async fn rollback(self: Box<Self>) -> RepositoryResult<()> {
        let RepoStorage { uow, .. } = *Pin::into_inner(self.storage);
        uow.rollback().await
    }

    fn games(&self) -> &dyn GameRepository {
        &self.storage.games
    }

    fn boards(&self) -> &dyn BoardRepository {
        &self.storage.boards
    }

    fn captures(&self) -> &dyn CaptureRepository {
        &self.storage.captures
    }

    fn decks(&self) -> &dyn DeckRepository {
        &self.storage.decks
    }

    fn benutzer(&self) -> &dyn BenutzerRepository {
        &self.storage.benutzer
    }

    fn konten(&self) -> &dyn KontoRepository {
        &self.storage.konten
    }

    fn sessions(&self) -> &dyn SessionRepository {
        &self.storage.sessions
    }

    fn bestätigungen(&self) -> &dyn EmailBestätigungRepository {
        &self.storage.bestätigungen
    }

    fn wiederherstellungen(&self) -> &dyn PasswortWiederherstellungRepository {
        &self.storage.wiederherstellungen
    }

    fn benachrichtigungen(&self) -> &dyn BenachrichtigungRepository {
        &self.storage.benachrichtigungen
    }

    fn external_identities(&self) -> &dyn ExternalIdentityRepository {
        &self.storage.external_identities
    }

    fn oidc_auth_states(&self) -> &dyn OidcAuthStateRepository {
        &self.storage.oidc_auth_states
    }

    fn oidc_exchange_codes(&self) -> &dyn OidcExchangeCodeRepository {
        &self.storage.oidc_exchange_codes
    }
}

#[derive(Clone)]
pub struct InstrumentedOidcProviderClient {
    inner: Arc<dyn OidcProviderClient>,
}

impl InstrumentedOidcProviderClient {
    pub fn new(inner: Arc<dyn OidcProviderClient>) -> Arc<dyn OidcProviderClient> {
        Arc::new(Self { inner })
    }
}

#[async_trait]
impl OidcProviderClient for InstrumentedOidcProviderClient {
    fn configured_providers(&self) -> Vec<OidcProviderDescriptor> {
        self.inner.configured_providers()
    }

    #[instrument(skip(self, state, nonce, code_challenge), fields(provider = provider_id.as_str()), level = "debug", err(Debug))]
    async fn build_authorization_url(
        &self,
        provider_id: &OidcProviderId,
        state: &str,
        nonce: &str,
        code_challenge: &str,
    ) -> ResultReport<String, OidcClientError> {
        self.inner
            .build_authorization_url(provider_id, state, nonce, code_challenge)
            .await
    }

    #[instrument(skip(self, authorization_code, pkce_verifier, expected_nonce), fields(provider = provider_id.as_str()), level = "debug", err(Debug))]
    async fn exchange_and_verify(
        &self,
        provider_id: &OidcProviderId,
        authorization_code: &str,
        pkce_verifier: &str,
        expected_nonce: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> ResultReport<VerifiedOidcIdentity, OidcClientError> {
        self.inner
            .exchange_and_verify(
                provider_id,
                authorization_code,
                pkce_verifier,
                expected_nonce,
                now,
            )
            .await
    }
}

#[derive(Clone)]
pub struct InstrumentedRngSource {
    inner: Arc<dyn RngSource>,
}

impl InstrumentedRngSource {
    pub fn new(inner: Arc<dyn RngSource>) -> Arc<dyn RngSource> {
        Arc::new(Self { inner })
    }
}

impl RngSource for InstrumentedRngSource {
    fn rng(&self) -> Box<dyn Rng + '_> {
        trace!("rng requested");
        self.inner.rng()
    }
}

#[derive(Clone)]
pub struct InstrumentedEventQueue {
    inner: Arc<dyn EventQueue>,
}

impl InstrumentedEventQueue {
    pub fn new(inner: Arc<dyn EventQueue>) -> Arc<dyn EventQueue> {
        Arc::new(Self { inner })
    }
}

#[async_trait]
impl EventQueue for InstrumentedEventQueue {
    #[instrument(skip(self, events), fields(count = events.len()), level = "debug", err(Debug))]
    async fn enqueue(&self, events: Vec<Event>) -> ResultReport<(), EventQueueError> {
        self.inner.enqueue(events).await
    }
}
