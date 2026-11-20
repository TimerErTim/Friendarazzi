use chrono::{DateTime, Utc};

use crate::application::ResultReport;
use crate::service::Event;

#[derive(Debug, thiserror::Error)]
pub enum EventQueueError {
    #[error("event queue unavailable")]
    Unavailable,
    #[error("event serialization failed")]
    Serialization,
}

/// Application port for enqueuing and scheduling background [`Event`]s.
///
/// Implementations may execute inline, dispatch to in-memory worker threads, or persist to
/// durable storage for a background worker process.
#[async_trait::async_trait]
pub trait EventQueue: Send + Sync {
    /// Enqueue an event to run as soon as possible.
    async fn enqueue(&self, event: Event) -> ResultReport<(), EventQueueError>;

    /// Schedule an event to run at or after `run_at`.
    ///
    /// When `key` is `Some`, any pending (not yet started) event with the same key is
    /// replaced so at most one pending event exists for that key.
    async fn schedule(
        &self,
        event: Event,
        run_at: DateTime<Utc>,
        key: Option<&str>,
    ) -> ResultReport<(), EventQueueError>;

    /// Cancel any pending (not yet started) event matching `key`.
    async fn cancel(&self, key: &str) -> ResultReport<(), EventQueueError>;
}
