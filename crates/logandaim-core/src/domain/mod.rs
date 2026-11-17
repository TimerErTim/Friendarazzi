#![doc = include_str!("README.md")]

pub mod board;
pub mod capture;
pub mod common;
pub mod deck;
pub mod game;
pub mod kontakt;
pub mod konto;
pub mod medal;
pub mod media;

pub use board::{
    Board, BoardError, BoardId, BoardTile, BoardTileId, NewBoard, NewBoardTile, evaluate_bingo,
};
pub use capture::{
    Capture, CaptureError, CaptureId, NewCapture, Superlative, SuperlativeKind,
    compute_superlatives,
};
pub use common::NonEmptyString;
pub use deck::{DeckId, StarterDeck, builtin_starter_decks};
pub use game::{
    Game, GameError, GameId, GamePlayer, GameStatus, GridSize, InviteCode, MIN_PLAYERS, NewGame,
    PromptSource, PromptSubmission, PromptSubmissionId, VisibilityMode, assemble_boards,
    target_quotas_for_author, validate_author_prompts,
};
pub use kontakt::EmailAdresse;
pub use kontakt::Mobilnummer;
pub use konto::Konto;
pub use konto::KontoFehler;
pub use konto::KontoId;
pub use konto::Rolle;
pub use medal::{
    NewUserProfile, ProfileError, RANK_TIERS, UserProfile, default_username_from_email,
};
pub use media::{
    MediaProcessingError, ProcessedCaptureMedia, process_avatar_photo, process_capture_photo,
};
