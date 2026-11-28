//! OpenAPI schemas for Quick-Start starter decks.

use uuid::Uuid;

/// Quick-Start deck template collection.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
#[cfg_attr(feature = "openapi", oai(rename_all = "camelCase"))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StarterDeck {
    /// Deck identifier.
    pub id: Uuid,
    /// URL-safe slug.
    pub slug: String,
    /// Display name of the deck.
    pub name: String,
    /// Short description of the deck theme.
    pub description: String,
    /// Target-agnostic prompt templates.
    pub prompts: Vec<String>,
}
