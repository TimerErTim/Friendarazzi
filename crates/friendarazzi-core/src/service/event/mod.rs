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
    GameStarted(Box<GameStarted>),
    CaptureUploaded(Box<CaptureUploaded>),
    NearBingoReached(Box<NearBingoReached>),
    GameFinished(Box<GameFinished>),
}

impl From<GameStarted> for Event {
    fn from(event: GameStarted) -> Self {
        Self::GameStarted(Box::new(event))
    }
}

impl From<CaptureUploaded> for Event {
    fn from(event: CaptureUploaded) -> Self {
        Self::CaptureUploaded(Box::new(event))
    }
}

impl From<NearBingoReached> for Event {
    fn from(event: NearBingoReached) -> Self {
        Self::NearBingoReached(Box::new(event))
    }
}

impl From<GameFinished> for Event {
    fn from(event: GameFinished) -> Self {
        Self::GameFinished(Box::new(event))
    }
}

/// Dispatch a queued [`Event`] onto [`App`] through its registered handler strategy.
pub async fn handle_event(app: &App, event: Event) -> ResultReport<(), EventHandlingError> {
    match event {
        Event::GameStarted(event) => app
            .execute(AuthContext::Anonymous, *event)
            .await
            .change_context(EventHandlingError),
        Event::CaptureUploaded(event) => app
            .execute(AuthContext::Anonymous, *event)
            .await
            .change_context(EventHandlingError),
        Event::NearBingoReached(event) => app
            .execute(AuthContext::Anonymous, *event)
            .await
            .change_context(EventHandlingError),
        Event::GameFinished(event) => app
            .execute(AuthContext::Anonymous, *event)
            .await
            .change_context(EventHandlingError),
    }
}
