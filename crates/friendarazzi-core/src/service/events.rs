use async_trait::async_trait;
use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};

use crate::application::ExecutionContext;
use crate::service::{DelayedGameStart, UseCase};

/// Marker trait for service-layer background events.
///
/// Every event is a [`UseCase<()>`] that can be serialized/enqueued on an [`EventQueue`]
/// and executed asynchronously against [`App`](crate::App).
///
/// [`EventQueue`]: crate::ports::EventQueue
pub trait EventUseCase: UseCase<()> + Send + Sync + 'static {}

impl<T: UseCase<()> + Send + Sync + 'static> EventUseCase for T {}

/// Error returned when executing a queued [`Event`].
#[derive(Debug, thiserror::Error)]
#[error("event execution failed")]
pub struct EventError;

/// Closed sum of all queued/scheduled service-layer events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum Event {
    DelayedGameStart(DelayedGameStart),
}

impl Event {
    /// Stable event kind name for logs, traces, and metrics.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::DelayedGameStart(_) => "delayed_game_start",
        }
    }
}

impl From<DelayedGameStart> for Event {
    fn from(event: DelayedGameStart) -> Self {
        Self::DelayedGameStart(event)
    }
}

#[async_trait]
impl UseCase<()> for Event {
    type Error = Report<EventError>;

    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        match self {
            Self::DelayedGameStart(event) => event.perform(ctx).await.change_context(EventError),
        }
    }
}
