// TODO: Simplify the WeakApp / EventQueueInit / Arc::new_cyclic wiring once App/queue lifecycle is unified.

use std::ops::Deref;
use std::sync::{Arc, Weak};

use tracing::{error, instrument};

use crate::adapters::{
    BlankPdfRenderer, BlockingEventQueue, InMemoryObjectStore, InMemorySigning, InMemoryThrottle,
    NullMailSender, SystemClock, SystemRng,
};
use crate::application::context::{ContextPorts, EventCollector};
use crate::application::instrumented::InstrumentedEventQueue;
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
pub use context::{EventCollector as EventStage, ExecutionContext};

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
pub struct WeakApp {
    inner: Weak<AppInner>,
}

impl WeakApp {
    pub fn upgrade(&self) -> Option<App> {
        self.inner.upgrade().map(|inner| App { inner })
    }
}

/// Initialization input for `App::builder().event_queue(...)`.
///
/// Accepts either a ready `Arc<T>` / `Arc<dyn EventQueue>` (for adapters like
/// `SqliteEventQueue` that do not need `App`) or a factory `FnOnce(WeakApp) -> Arc<dyn EventQueue>`
/// (for in-process adapters like [`BlockingEventQueue::from_app`](crate::adapters::BlockingEventQueue::from_app)).
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

impl AppInner {
    #[instrument(
        skip(self, use_case),
        fields(use_case = std::any::type_name::<U>(), auth = ?auth),
        err(Debug)
    )]
    pub(crate) async fn execute<U: UseCase<O> + Send, O>(
        &self,
        auth: AuthContext,
        use_case: U,
    ) -> ResultReport<O, <U::Error as error_stack::IntoReport>::Context> {
        let ctx = self.new_execution_context(auth);
        let events = ctx.events().clone();
        let result = use_case.perform(ctx).await;
        if result.is_ok() {
            self.enqueue_events(events).await;
        }
        result
    }

    #[instrument(skip(self, f), fields(execute_fn = true, auth = ?auth), err(Debug))]
    pub(crate) async fn execute_fn<F, O, E: ThreadSafeError>(
        &self,
        auth: AuthContext,
        f: F,
    ) -> ResultReport<O, E>
    where
        for<'a> F: AsyncFnOnce(ExecutionContext<'a>) -> ResultReport<O, E> + Send,
    {
        let ctx = self.new_execution_context(auth);
        let events = ctx.events().clone();
        let result = f(ctx).await;
        if result.is_ok() {
            self.enqueue_events(events).await;
        }
        result
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
                events: EventCollector::new(),
                identity_provider_conf: self.konto.clone(),
            },
        )
    }

    async fn enqueue_events(&self, events: EventCollector) {
        let events = events.take();
        if events.is_empty() {
            return;
        }
        if let Err(err) = self.event_queue.enqueue(events).await {
            error!(?err, "failed to enqueue collected events");
        }
    }
}

#[derive(Clone)]
pub struct App {
    inner: Arc<AppInner>,
}

impl Deref for App {
    type Target = AppInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
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
        let inner = Arc::new_cyclic(|weak| {
            let weak_app = WeakApp {
                inner: weak.clone(),
            };
            let raw_queue = match event_queue {
                Some(init) => init.build(weak_app),
                None => BlockingEventQueue::from_app(weak_app),
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
                event_queue: InstrumentedEventQueue::new(raw_queue),
            }
        });
        Self { inner }
    }
}

pub use app_builder::{SetClock, SetUowProvider};

impl App {
    pub async fn execute<U: UseCase<O> + Send, O>(
        &self,
        auth: AuthContext,
        use_case: U,
    ) -> ResultReport<O, <U::Error as error_stack::IntoReport>::Context> {
        self.inner.execute(auth, use_case).await
    }

    pub async fn execute_fn<F, O, E: ThreadSafeError>(
        &self,
        auth: AuthContext,
        f: F,
    ) -> ResultReport<O, E>
    where
        for<'a> F: AsyncFnOnce(ExecutionContext<'a>) -> ResultReport<O, E> + Send,
    {
        self.inner.execute_fn(auth, f).await
    }
}
