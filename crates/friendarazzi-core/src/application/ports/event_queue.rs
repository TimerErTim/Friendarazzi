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
/// Use cases gather events during execution via [`ExecutionContext::event_queue`], and
/// [`App`](crate::App) invokes [`EventQueue::enqueue_submitted`] only after the execution
/// (and any open `UnitOfWork`) succeeds.
///
/// [`ExecutionContext::event_queue`]: crate::application::ExecutionContext::event_queue
#[async_trait::async_trait]
pub trait EventQueue: Send + Sync {
    async fn enqueue_submitted(&self, events: Vec<Event>) -> ResultReport<(), EventQueueError>;
}
