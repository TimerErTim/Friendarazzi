//! In-memory fakes for friendarazzi.app ports — usable in integration tests, scripts, and seed tooling.

mod clock;
mod object_store;
mod pdf;
mod repository;
mod rng;
mod uow;

pub use clock::*;
pub use object_store::*;
pub use pdf::*;
pub use repository::*;
pub use rng::*;
pub use uow::*;
