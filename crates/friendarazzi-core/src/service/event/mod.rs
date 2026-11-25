mod game;

pub use game::*;

use error_stack::ResultExt;

use crate::application::AuthContext;
use crate::{App, ResultReport};

#[derive(Debug, thiserror::Error)]
#[error("event handling failed")]
pub struct EventHandlingError;

#[derive(Debug, Clone)]
pub enum Event {
    CaptureUploaded(Box<CaptureUploaded>),
}

impl From<CaptureUploaded> for Event {
    fn from(event: CaptureUploaded) -> Self {
        Self::CaptureUploaded(Box::new(event))
    }
}

/// Dispatch a queued [`Event`] onto [`App`] through its registered handler strategy.
pub async fn handle_event(app: &App, event: Event) -> ResultReport<(), EventHandlingError> {
    match event {
        Event::CaptureUploaded(event) => app
            .execute(AuthContext::Anonymous, *event)
            .await
            .change_context(EventHandlingError),
    }
}
