mod game;

pub use game::*;

use crate::App;
use crate::application::AuthContext;

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
pub async fn handle_event(app: &App, event: Event) -> Result<(), String> {
    match event {
        Event::CaptureUploaded(event) => app
            .execute(AuthContext::Anonymous, *event)
            .await
            .map_err(|err| format!("{err:?}")),
    }
}
