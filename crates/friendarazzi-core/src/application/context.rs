use std::sync::{Arc, Mutex};

use crate::{
    application::instrumented::{
        InstrumentedMailSender, InstrumentedObjectStore, InstrumentedOidcProviderClient,
        InstrumentedPdfRenderer, InstrumentedRngSource, InstrumentedThrottle,
        InstrumentedUnitOfWork,
    },
    application::uow::{UnitOfWork, UnitOfWorkImpl, UnitOfWorkProvider},
    ports::{
        Clock, MailSender, ObjectStore, OidcProviderClient, PdfRenderer, RepositoryResult,
        RngSource, Signing, Throttle,
    },
    service::Event,
};
use rand::Rng;
use tracing::{debug, instrument};

use super::auth::AuthContext;
use super::konfiguration::IdentityProviderConfiguration;

/// Per-execution buffer that collects events submitted by use cases before [`App`](crate::App)
/// dispatches them to [`EventQueue`](crate::ports::EventQueue) after execution succeeds.
#[derive(Clone, Default)]
pub struct EventCollector {
    events: Arc<Mutex<Vec<Event>>>,
}

impl EventCollector {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Stage an event to be enqueued after the current `App::execute` / `execute_fn`
    /// completes successfully.
    pub fn submit<T: Into<Event>>(&self, event: T) {
        let event = event.into();
        debug!(
            event = std::any::type_name::<T>(),
            "collected event for post-execution enqueuing"
        );
        self.events
            .lock()
            .expect("event stage mutex poisoned")
            .push(event);
    }

    pub(crate) fn take(&self) -> Vec<Event> {
        std::mem::take(&mut *self.events.lock().expect("event stage mutex poisoned"))
    }
}

#[derive(Clone, Copy)]
enum ExecutionSource<'a> {
    Root(&'a dyn UnitOfWorkProvider),
    Nested(&'a dyn UnitOfWorkImpl),
}

/// Fresh per `App::execute` / `execute_fn`. Holds ports, the executing identity, and a UoW
/// source (root provider or nested impl). Does not hold an active transaction — call
/// [`ExecutionContext::enter`].
pub struct ExecutionContext<'a> {
    source: ExecutionSource<'a>,
    auth: AuthContext,
    signing: Arc<dyn Signing>,
    clock: Arc<dyn Clock>,
    rng: Arc<dyn RngSource>,
    object_store: Arc<dyn ObjectStore>,
    pdf_renderer: Arc<dyn PdfRenderer>,
    throttle: Arc<dyn Throttle>,
    mail_sender: Arc<dyn MailSender>,
    oidc_client: Arc<dyn OidcProviderClient>,
    events: EventCollector,
    identity_provider_conf: IdentityProviderConfiguration,
}

/// Everything a context carries besides its UoW source, so `root`, `nested`, and `sub`
/// cannot drift apart.
pub(crate) struct ContextPorts {
    pub(crate) auth: AuthContext,
    pub(crate) signing: Arc<dyn Signing>,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) rng: Arc<dyn RngSource>,
    pub(crate) object_store: Arc<dyn ObjectStore>,
    pub(crate) pdf_renderer: Arc<dyn PdfRenderer>,
    pub(crate) throttle: Arc<dyn Throttle>,
    pub(crate) mail_sender: Arc<dyn MailSender>,
    pub(crate) oidc_client: Arc<dyn OidcProviderClient>,
    pub(crate) events: EventCollector,
    pub(crate) identity_provider_conf: IdentityProviderConfiguration,
}

impl<'a> ExecutionContext<'a> {
    pub(super) fn root(
        provider: &'a dyn UnitOfWorkProvider,
        ports: ContextPorts,
    ) -> ExecutionContext<'a> {
        Self {
            source: ExecutionSource::Root(provider),
            auth: ports.auth,
            signing: ports.signing,
            clock: ports.clock,
            rng: InstrumentedRngSource::new(ports.rng),
            object_store: InstrumentedObjectStore::new(ports.object_store),
            pdf_renderer: InstrumentedPdfRenderer::new(ports.pdf_renderer),
            throttle: InstrumentedThrottle::new(ports.throttle),
            mail_sender: InstrumentedMailSender::new(ports.mail_sender),
            oidc_client: InstrumentedOidcProviderClient::new(ports.oidc_client),
            events: ports.events,
            identity_provider_conf: ports.identity_provider_conf,
        }
    }

    pub(super) fn nested(uow: &'a dyn UnitOfWorkImpl, ports: ContextPorts) -> ExecutionContext<'a> {
        Self {
            source: ExecutionSource::Nested(uow),
            auth: ports.auth,
            signing: ports.signing,
            clock: ports.clock,
            rng: InstrumentedRngSource::new(ports.rng),
            object_store: InstrumentedObjectStore::new(ports.object_store),
            pdf_renderer: InstrumentedPdfRenderer::new(ports.pdf_renderer),
            throttle: InstrumentedThrottle::new(ports.throttle),
            mail_sender: InstrumentedMailSender::new(ports.mail_sender),
            oidc_client: InstrumentedOidcProviderClient::new(ports.oidc_client),
            events: ports.events,
            identity_provider_conf: ports.identity_provider_conf,
        }
    }

    pub fn clock(&self) -> &dyn Clock {
        self.clock.as_ref()
    }

    pub fn rng(&self) -> Box<dyn Rng + '_> {
        self.rng.rng()
    }

    pub fn object_store(&self) -> &dyn ObjectStore {
        self.object_store.as_ref()
    }

    pub fn pdf_renderer(&self) -> &dyn PdfRenderer {
        self.pdf_renderer.as_ref()
    }

    /// Client-scoped rate limiting. Use cases call it directly and map a refusal to their
    /// own `Throttled` variant.
    pub fn throttle(&self) -> &dyn Throttle {
        self.throttle.as_ref()
    }

    pub fn mail_sender(&self) -> &dyn MailSender {
        self.mail_sender.as_ref()
    }

    pub fn oidc_client(&self) -> &dyn OidcProviderClient {
        self.oidc_client.as_ref()
    }

    /// Staged event submission handle for the current execution.
    pub fn events(&self) -> &EventCollector {
        &self.events
    }

    /// Mailed link targets and token identity of this deployment.
    pub fn identity_provider_conf(&self) -> &IdentityProviderConfiguration {
        &self.identity_provider_conf
    }

    /// Who is executing this use case. Injected by the API layer, never by a use case.
    pub fn auth(&self) -> AuthContext {
        self.auth
    }

    /// The configured signing key as a port. The algorithm below it is domain code; this
    /// accessor is how the application composes and checks a token.
    pub fn signing(&self) -> &dyn Signing {
        self.signing.as_ref()
    }

    /// Nested context that joins `uow`'s transaction and copies this context's ports,
    /// event stage, and identity.
    pub fn sub<'b>(&self, uow: &'b UnitOfWork<'b>) -> ExecutionContext<'b> {
        ExecutionContext::nested(
            uow.as_impl(),
            ContextPorts {
                auth: self.auth,
                signing: Arc::clone(&self.signing),
                clock: Arc::clone(&self.clock),
                rng: Arc::clone(&self.rng),
                object_store: Arc::clone(&self.object_store),
                pdf_renderer: Arc::clone(&self.pdf_renderer),
                throttle: Arc::clone(&self.throttle),
                mail_sender: Arc::clone(&self.mail_sender),
                oidc_client: Arc::clone(&self.oidc_client),
                events: self.events.clone(),
                identity_provider_conf: self.identity_provider_conf.clone(),
            },
        )
    }

    /// Start a unit of work. Returned UoW borrows `self`, so this context cannot be
    /// moved into a nested `perform` until the UoW is committed or rolled back.
    /// Nested contexts (from [`ExecutionContext::sub`]) join the outer transaction.
    #[instrument(skip(self), level = "debug", err(Debug))]
    pub async fn enter(&self) -> RepositoryResult<UnitOfWork<'_>> {
        match self.source {
            ExecutionSource::Root(provider) => {
                let implementation = InstrumentedUnitOfWork::wrap(provider.begin().await?);
                Ok(UnitOfWork::owned(implementation))
            }
            ExecutionSource::Nested(inner) => Ok(UnitOfWork::locked(inner)),
        }
    }
}
