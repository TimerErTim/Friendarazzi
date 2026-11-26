//! OpenAPI schemas.

mod benachrichtigung;
mod board;
mod capture;
mod deck;
mod game;
mod health;
mod kontakt;
mod konto;
mod profile;

pub use benachrichtigung::*;
pub use board::*;
pub use capture::*;
pub use deck::*;
pub use game::*;
pub use health::*;
pub use kontakt::*;
pub use konto::*;
pub use profile::*;

/// Acknowledgement of a request that has nothing to return.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase", example))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ok {
    /// Always `true`; the presence of the body is the acknowledgement.
    pub ok: bool,
}

impl Default for Ok {
    fn default() -> Self {
        Self { ok: true }
    }
}

#[cfg(feature = "openapi")]
impl poem_openapi::types::Example for Ok {
    fn example() -> Self {
        Self::default()
    }
}
