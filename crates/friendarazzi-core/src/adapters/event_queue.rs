use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, error};

use crate::ResultReport;
use crate::application::WeakApp;
use crate::ports::{EventQueue, EventQueueError};
use crate::service::{Event, handle_event};

/// Default in-memory blocking [`EventQueue`] adapter that executes submitted events inline on [`App`](crate::App).
///
/// Events are dispatched sequentially after the submitting use case finishes and closes its
/// unit of work.
pub struct BlockingEventQueue {
    app: WeakApp,
}

impl BlockingEventQueue {
    pub fn new(app: WeakApp) -> Self {
        Self { app }
    }

    pub fn from_app(app: WeakApp) -> Arc<dyn EventQueue> {
        Arc::new(Self::new(app))
    }
}

#[async_trait]
impl EventQueue for BlockingEventQueue {
    async fn enqueue(&self, events: Vec<Event>) -> ResultReport<(), EventQueueError> {
        for event in events {
            let Some(app) = self.app.upgrade() else {
                return Err(error_stack::Report::new(EventQueueError::Unavailable));
            };
            if let Err(err) = handle_event(&app, event).await {
                error!(?err, "queued event execution failed");
            } else {
                debug!("queued event completed");
            }
        }
        Ok(())
    }
}
