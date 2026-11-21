#![doc = include_str!("README.md")]

pub mod benutzer;
pub mod board;
pub mod capture;
pub mod common;
pub mod deck;
pub mod game;
pub mod kontakt;
pub mod konto;
pub mod media;

pub use benutzer::{Benutzer, BenutzerId, NeuerBenutzer, ProfileError, RANK_TIERS};
pub use board::{
    Board, BoardCoordinates, BoardError, BoardId, BoardTile, BoardTileAssignment, BoardTileId,
    NewBoard,
};
pub use capture::{
    Capture, CaptureError, CaptureId, GameAwards, GhostAward, MostWantedAward, NewCapture,
    ProcessedCaptureMedia, QuickestDrawAward, SniperAward, can_view_blurred_capture,
    can_view_full_capture, compute_game_awards,
};
pub use common::NonEmptyString;
pub use deck::{DeckId, StarterDeck, builtin_starter_decks};
pub use game::{
    ActiveGame, CompletedGame, DraftStartOutcome, DraftingGame, Game, GameError, GameId, GameIn,
    GamePlayer, GridSize, InviteCode, LobbyGame, MIN_PLAYERS, NewGame, PromptSource,
    PromptSubmission, PromptSubmissionId, READY_GRACE_PERIOD_SECONDS, VisibilityMode,
    assemble_boards, state as GameState, target_quotas_for_author, validate_author_prompts,
};
pub use kontakt::EmailAdresse;
pub use kontakt::Mobilnummer;
pub use konto::Konto;
pub use konto::KontoFehler;
pub use konto::KontoId;
pub use media::MediaProcessingError;
