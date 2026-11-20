use std::ops::Deref;
use std::sync::{Arc, Weak};

use tracing::instrument;

use crate::adapters::{
    BlankPdfRenderer, InMemoryObjectStore, InMemorySigning, InMemoryThrottle,
    MultithreadedEventQueue, NullMailSender, SystemClock, SystemRng,
};
use crate::application::context::ContextPorts;
use crate::ports::{
    Clock, EventQueue, MailSender, ObjectStore, PdfRenderer, RngSource, Signing, Throttle,
};
use crate::service::UseCase;
use crate::uow::UnitOfWorkProvider;

pub mod uow;

pub mod ports;

pub mod auth;
pub use auth::{AccessError, AuthContext};

mod context;
pub use context::ExecutionContext;

mod konfiguration;
pub use konfiguration::IdentityProviderConfiguration;

mod instrumented;
mod instrumented_repos;

mod errors;
pub use errors::ErrorReportExt;
pub use errors::ResultReport;
pub use errors::ThreadSafeError;

/// Weak handle to an [`App`] used by in-process adapters (such as the core event queues)
/// that live inside `App` and execute queued work back onto it without creating an `Arc`
/// reference cycle.
#[derive(Clone, Default)]
pub struct WeakApp(Weak<AppInner>);

impl WeakApp {
    pub fn upgrade(&self) -> Option<App> {
        self.0.upgrade().map(App)
    }
}

/// Initialization input for `App::builder().event_queue(...)`.
///
/// Accepts either a ready `Arc<T>` / `Arc<dyn EventQueue>` (for adapters like
/// `SqliteEventQueue` that do not need `App`) or a factory `FnOnce(WeakApp) -> Arc<dyn EventQueue>`
/// (for in-process adapters like [`BlockingEventQueue`](crate::adapters::BlockingEventQueue)).
pub struct EventQueueInit(Box<dyn FnOnce(WeakApp) -> Arc<dyn EventQueue> + Send>);

impl EventQueueInit {
    pub fn build(self, app: WeakApp) -> Arc<dyn EventQueue> {
        (self.0)(app)
    }
}

impl<T: EventQueue + 'static> From<Arc<T>> for EventQueueInit {
    fn from(queue: Arc<T>) -> Self {
        Self(Box::new(move |_| queue))
    }
}

impl From<Arc<dyn EventQueue>> for EventQueueInit {
    fn from(queue: Arc<dyn EventQueue>) -> Self {
        Self(Box::new(move |_| queue))
    }
}

impl<F> From<F> for EventQueueInit
where
    F: FnOnce(WeakApp) -> Arc<dyn EventQueue> + Send + 'static,
{
    fn from(factory: F) -> Self {
        Self(Box::new(factory))
    }
}

pub struct AppInner {
    pub uow_provider: Box<dyn UnitOfWorkProvider>,
    /// Key material behind an application port; the algorithm beneath it is domain code.
    /// The default is a fresh keypair per process, which is right for a throwaway run and
    /// wrong for a deployment that must not end every session on restart.
    pub signing: Arc<dyn Signing>,
    /// Client-scoped rate limiting. `InMemoryThrottle` is the production adapter for a
    /// single-process deployment, so it is also the default.
    pub throttle: Arc<dyn Throttle>,
    /// Transaction mail. No relay configured fails every send and logs it (NFR-5).
    pub mail_sender: Arc<dyn MailSender>,
    /// Mailed link targets and token identity. Deployment data, not behaviour.
    pub konto: IdentityProviderConfiguration,
    pub clock: Arc<dyn Clock>,
    pub rng: Arc<dyn RngSource>,
    pub object_store: Arc<dyn ObjectStore>,
    pub pdf_renderer: Arc<dyn PdfRenderer>,
    pub event_queue: Arc<dyn EventQueue>,
}

#[derive(Clone)]
pub struct App(Arc<AppInner>);

impl Deref for App {
    type Target = AppInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[bon::bon]
impl App {
    #[builder(builder_type = AppBuilder, state_mod = app_builder)]
    pub fn builder(
        uow_provider: Box<dyn UnitOfWorkProvider>,
        #[builder(default = Arc::new(InMemorySigning::random()))] signing: Arc<dyn Signing>,
        #[builder(default = Arc::new(InMemoryThrottle::new()))] throttle: Arc<dyn Throttle>,
        #[builder(default = Arc::new(NullMailSender))] mail_sender: Arc<dyn MailSender>,
        konto: IdentityProviderConfiguration,
        #[builder(default = Arc::new(SystemClock))] clock: Arc<dyn Clock>,
        #[builder(default = Arc::new(SystemRng))] rng: Arc<dyn RngSource>,
        #[builder(default = Arc::new(InMemoryObjectStore::new()))] object_store: Arc<
            dyn ObjectStore,
        >,
        #[builder(default = Arc::new(BlankPdfRenderer))] pdf_renderer: Arc<dyn PdfRenderer>,
        #[builder(into)] event_queue: Option<EventQueueInit>,
    ) -> Self {
        Self(Arc::new_cyclic(|weak| {
            let weak_app = WeakApp(weak.clone());
            let event_queue = match event_queue {
                Some(init) => init.build(weak_app),
                None => Arc::new(MultithreadedEventQueue::new(weak_app)),
            };
            AppInner {
                uow_provider,
                signing,
                throttle,
                mail_sender,
                konto,
                clock,
                rng,
                object_store,
                pdf_renderer,
                event_queue,
            }
        }))
    }
}

pub use app_builder::{SetClock, SetUowProvider};

impl App {
    #[instrument(
        skip(self, use_case),
        fields(use_case = std::any::type_name::<U>(), auth = ?auth),
        err(Debug)
    )]
    pub async fn execute<U: UseCase<O> + Send, O>(
        &self,
        auth: AuthContext,
        use_case: U,
    ) -> ResultReport<O, <U::Error as error_stack::IntoReport>::Context> {
        use_case.perform(self.new_execution_context(auth)).await
    }

    #[instrument(skip(self, f), fields(execute_fn = true, auth = ?auth), err(Debug))]
    pub async fn execute_fn<F, O, E: ThreadSafeError>(
        &self,
        auth: AuthContext,
        f: F,
    ) -> ResultReport<O, E>
    where
        for<'a> F: AsyncFnOnce(ExecutionContext<'a>) -> ResultReport<O, E> + Send,
    {
        f(self.new_execution_context(auth)).await
    }

    fn new_execution_context(&self, auth: AuthContext) -> ExecutionContext<'_> {
        ExecutionContext::root(
            self.uow_provider.as_ref(),
            ContextPorts {
                auth,
                signing: Arc::clone(&self.signing),
                clock: Arc::clone(&self.clock),
                rng: Arc::clone(&self.rng),
                object_store: Arc::clone(&self.object_store),
                pdf_renderer: Arc::clone(&self.pdf_renderer),
                throttle: Arc::clone(&self.throttle),
                mail_sender: Arc::clone(&self.mail_sender),
                event_queue: Arc::clone(&self.event_queue),
                identity_provider_conf: self.konto.clone(),
            },
        )
    }
}
