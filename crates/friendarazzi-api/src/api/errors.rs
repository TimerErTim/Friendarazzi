//! Refusals of reads that run inline in [`FriendarazziAppApi`](super::FriendarazziAppApi).

/// Refusals when reading games or game details.
#[derive(Debug, thiserror::Error)]
pub enum GameReadError {
    #[error("für diese Aktion ist eine Anmeldung erforderlich")]
    Unauthorized,
    #[error("du bist nicht berechtigt, dieses Spiel einzusehen")]
    Forbidden,
    #[error("das Spiel wurde nicht gefunden")]
    NotFound,
    #[error("ungültige Anfrage")]
    BadRequest,
    #[error("Spieldaten konnten nicht geladen werden")]
    Persistence,
}

/// Refusals when reading user profiles or galleries.
#[derive(Debug, thiserror::Error)]
pub enum ProfileReadError {
    #[error("für diese Aktion ist eine Anmeldung erforderlich")]
    Unauthorized,
    #[error("das Profil wurde nicht gefunden")]
    NotFound,
    #[error("Profildaten konnten nicht geladen werden")]
    Persistence,
}

/// Refusals when streaming stored media objects.
#[derive(Debug, thiserror::Error)]
pub enum MediaReadError {
    #[error("für diese Aktion ist eine Anmeldung erforderlich")]
    Unauthorized,
    #[error("dieses Foto darfst du aktuell nicht sehen")]
    Forbidden,
    #[error("das Medium wurde nicht gefunden")]
    NotFound,
    #[error("Medium konnte nicht geladen werden")]
    Storage,
}
