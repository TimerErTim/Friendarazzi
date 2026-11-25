use std::sync::Mutex;

use error_stack::Report;
use friendarazzi_core::{
    ports::{EventQueue, EventQueueError},
    service::Event,
};

/// In-memory [`EventQueue`] test spy that records enqueued [`Event`] batches.
#[derive(Default)]
pub struct RecordingEventQueue {
    batches: Mutex<Vec<Vec<Event>>>,
}

impl RecordingEventQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn batches(&self) -> Vec<Vec<Event>> {
        self.batches.lock().unwrap().clone()
    }

    pub fn total_events(&self) -> usize {
        self.batches.lock().unwrap().iter().map(Vec::len).sum()
    }

    pub fn clear(&self) {
        self.batches.lock().unwrap().clear();
    }
}

#[async_trait::async_trait]
impl EventQueue for RecordingEventQueue {
    async fn enqueue(&self, events: Vec<Event>) -> Result<(), Report<EventQueueError>> {
        self.batches.lock().unwrap().push(events);
        Ok(())
    }
}
