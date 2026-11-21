mod game;

pub use game::*;

#[derive(Debug, Clone)]
pub enum Event {
    DelayedGameStart(Box<DelayedGameStart>),
}

impl From<DelayedGameStart> for Event {
    fn from(event: DelayedGameStart) -> Self {
        Self::DelayedGameStart(Box::new(event))
    }
}
