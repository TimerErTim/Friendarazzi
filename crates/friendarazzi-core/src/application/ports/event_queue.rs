use crate::application::ResultReport;
use crate::service::Event;

#[derive(Debug, thiserror::Error)]
pub enum EventQueueError {
    #[error("event queue unavailable")]
    Unavailable,
    #[error("event serialization failed")]
    Serialization,
}

/// Application port for dispatching staged background [`Event`]s.
///
/// Use cases gather events during execution via [`ExecutionContext::events`], and
/// [`App`](crate::App) invokes [`EventQueue::enqueue`] only after the execution
/// (and any open `UnitOfWork`) succeeds.
///
/// [`ExecutionContext::events`]: crate::application::ExecutionContext::events
#[async_trait::async_trait]
pub trait EventQueue: Send + Sync {
    async fn enqueue(&self, events: Vec<Event>) -> ResultReport<(), EventQueueError>;
}
