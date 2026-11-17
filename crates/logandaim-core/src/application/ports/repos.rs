use async_trait::async_trait;

use crate::application::ResultReport;
use crate::application::uow::Versioned;
use crate::domain::konto::{
    Benutzer, BenutzerId, BestätigungId, EmailBestätigung, Konto, KontoId, NeueBestätigung,
    NeuePasswortWiederherstellung, NeueSession, NeuerBenutzer, NeuesKonto,
    PasswortWiederherstellung, PasswortWiederherstellungId, SecretTokenHash, Session, SessionId,
};
use crate::domain::{
    Board, BoardId, BoardTile, BoardTileId, Capture, CaptureId, EmailAdresse, Game, GameId,
    GamePlayer, InviteCode, NewCapture, NewGame, NewUserProfile, PromptSubmission, StarterDeck,
    UserProfile,
};

pub type RepositoryResult<T> = ResultReport<T, RepositoryError>;

#[derive(thiserror::Error, Debug)]
pub enum RepositoryError {
    #[error("entity not found")]
    NotFound,
    #[error("version mismatch - entity was modified by another process {expected} != {actual:?}")]
    VersionMismatch { expected: u64, actual: Option<u64> },
    #[error("conflict occurred")]
    Conflict,
    #[error("connection failed")]
    Connection,
    #[error("operation failed")]
    OperationFailed,
    #[error("permissions error")]
    Permission,
    #[error("storage error")]
    Storage,
    #[error("data error")]
    Data,
    #[error("unknown repository error")]
    Unknown,
}

#[async_trait]
pub trait UserProfileRepository: Send + Sync {
    async fn create(&self, profile: NewUserProfile) -> RepositoryResult<Versioned<UserProfile>>;
    async fn find_by_user_id(
        &self,
        user_id: BenutzerId,
    ) -> RepositoryResult<Versioned<UserProfile>>;
    async fn find_by_user_ids(
        &self,
        user_ids: &[BenutzerId],
    ) -> RepositoryResult<Vec<Versioned<UserProfile>>>;
    async fn find_by_username(&self, username: &str) -> RepositoryResult<Versioned<UserProfile>>;
    async fn update(&self, profile: &mut Versioned<UserProfile>) -> RepositoryResult<()>;
    async fn delete_by_user_id(&self, user_id: BenutzerId) -> RepositoryResult<()>;
}

#[async_trait]
pub trait GameRepository: Send + Sync {
    async fn create(&self, game: NewGame) -> RepositoryResult<Versioned<Game>>;
    async fn find_by_id(&self, id: GameId) -> RepositoryResult<Versioned<Game>>;
    async fn find_by_invite_code(&self, code: &InviteCode) -> RepositoryResult<Versioned<Game>>;
    async fn find_for_user(&self, user_id: BenutzerId) -> RepositoryResult<Vec<Versioned<Game>>>;
    async fn update(&self, game: &mut Versioned<Game>) -> RepositoryResult<()>;
    async fn add_player(&self, player: GamePlayer) -> RepositoryResult<()>;
    async fn find_players(&self, game_id: GameId) -> RepositoryResult<Vec<GamePlayer>>;
    async fn set_player_draft_locked(
        &self,
        game_id: GameId,
        user_id: BenutzerId,
        locked: bool,
    ) -> RepositoryResult<()>;
    async fn replace_author_prompts(
        &self,
        game_id: GameId,
        author_user_id: BenutzerId,
        prompts: Vec<PromptSubmission>,
    ) -> RepositoryResult<()>;
    async fn find_prompts_by_game(
        &self,
        game_id: GameId,
    ) -> RepositoryResult<Vec<PromptSubmission>>;
    async fn find_prompts_by_author(
        &self,
        game_id: GameId,
        author_user_id: BenutzerId,
    ) -> RepositoryResult<Vec<PromptSubmission>>;
}

#[async_trait]
pub trait BoardRepository: Send + Sync {
    async fn create_board_with_tiles(
        &self,
        board: Board,
        tiles: Vec<BoardTile>,
    ) -> RepositoryResult<()>;
    async fn find_boards_by_game(&self, game_id: GameId)
    -> RepositoryResult<Vec<Versioned<Board>>>;
    async fn find_board_by_game_and_owner(
        &self,
        game_id: GameId,
        owner: BenutzerId,
    ) -> RepositoryResult<Versioned<Board>>;
    async fn find_tiles_by_board(&self, board_id: BoardId) -> RepositoryResult<Vec<BoardTile>>;
    async fn find_tiles_by_game(&self, game_id: GameId) -> RepositoryResult<Vec<BoardTile>>;
    async fn find_tile_by_id(&self, tile_id: BoardTileId) -> RepositoryResult<BoardTile>;
    async fn update_board(&self, board: &mut Versioned<Board>) -> RepositoryResult<()>;
    async fn attach_tile_capture(
        &self,
        tile_id: BoardTileId,
        capture_id: CaptureId,
    ) -> RepositoryResult<()>;
}

#[async_trait]
pub trait CaptureRepository: Send + Sync {
    async fn create(&self, capture: NewCapture) -> RepositoryResult<Versioned<Capture>>;
    async fn find_by_id(&self, id: CaptureId) -> RepositoryResult<Versioned<Capture>>;
    async fn find_by_game(&self, game_id: GameId) -> RepositoryResult<Vec<Versioned<Capture>>>;
    async fn find_by_target_user(
        &self,
        user_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Capture>>>;
    async fn find_by_catcher_user(
        &self,
        user_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Capture>>>;
    async fn find_by_storage_key(&self, key: &str) -> RepositoryResult<Versioned<Capture>>;
}

#[async_trait]
pub trait DeckRepository: Send + Sync {
    async fn list_all(&self) -> RepositoryResult<Vec<StarterDeck>>;
}

#[async_trait]
pub trait BenutzerRepository: Send + Sync {
    async fn create(&self, benutzer: NeuerBenutzer) -> RepositoryResult<Versioned<Benutzer>>;

    async fn find_by_id(&self, id: BenutzerId) -> RepositoryResult<Versioned<Benutzer>>;

    async fn update(&self, benutzer: &mut Versioned<Benutzer>) -> RepositoryResult<()>;

    async fn delete(&self, benutzer: Versioned<Benutzer>) -> RepositoryResult<()>;
}

#[async_trait]
pub trait KontoRepository: Send + Sync {
    async fn create(&self, konto: NeuesKonto) -> RepositoryResult<Versioned<Konto>>;

    async fn find_by_id(&self, id: KontoId) -> RepositoryResult<Versioned<Konto>>;

    /// `NotFound` when no Konto holds the address.
    async fn find_by_email(&self, email: &EmailAdresse) -> RepositoryResult<Versioned<Konto>>;

    async fn find_by_benutzer(
        &self,
        benutzer_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Konto>>>;

    async fn update(&self, konto: &mut Versioned<Konto>) -> RepositoryResult<()>;

    async fn delete(&self, konto: Versioned<Konto>) -> RepositoryResult<()>;
}

/// Rows are retained when spent, revoked, or expired.
#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, session: NeueSession) -> RepositoryResult<Versioned<Session>>;

    async fn find_by_id(&self, id: SessionId) -> RepositoryResult<Versioned<Session>>;

    /// `NotFound` for an unknown or already rotated refresh token.
    async fn find_by_refresh_hash(
        &self,
        hash: &SecretTokenHash,
    ) -> RepositoryResult<Versioned<Session>>;

    async fn find_by_benutzer(
        &self,
        benutzer_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Session>>>;

    async fn update(&self, session: &mut Versioned<Session>) -> RepositoryResult<()>;

    async fn delete(&self, session: Versioned<Session>) -> RepositoryResult<()>;

    /// Remove every session of one Benutzer, when the identity itself goes away.
    async fn delete_by_benutzer(&self, benutzer_id: BenutzerId) -> RepositoryResult<()>;
}

#[async_trait]
pub trait EmailBestätigungRepository: Send + Sync {
    async fn create(
        &self,
        bestätigung: NeueBestätigung,
    ) -> RepositoryResult<Versioned<EmailBestätigung>>;

    /// Keyed by the token hash, which is what confirmation step 2 presents.
    async fn find_by_hash(
        &self,
        hash: &SecretTokenHash,
    ) -> RepositoryResult<Versioned<EmailBestätigung>>;

    async fn find_by_id(
        &self, id: BestätigungId
    ) -> RepositoryResult<Versioned<EmailBestätigung>>;

    async fn find_by_email(
        &self,
        email: &EmailAdresse,
    ) -> RepositoryResult<Vec<Versioned<EmailBestätigung>>>;

    async fn update(&self, bestätigung: &mut Versioned<EmailBestätigung>)
    -> RepositoryResult<()>;

    async fn delete(&self, bestätigung: Versioned<EmailBestätigung>) -> RepositoryResult<()>;

    async fn delete_by_email(&self, email: &EmailAdresse) -> RepositoryResult<()>;
}

#[async_trait]
pub trait PasswortWiederherstellungRepository: Send + Sync {
    async fn create(
        &self,
        wiederherstellung: NeuePasswortWiederherstellung,
    ) -> RepositoryResult<Versioned<PasswortWiederherstellung>>;

    async fn find_by_id(
        &self,
        id: PasswortWiederherstellungId,
    ) -> RepositoryResult<Versioned<PasswortWiederherstellung>>;

    async fn find_by_hash(
        &self,
        hash: &SecretTokenHash,
    ) -> RepositoryResult<Versioned<PasswortWiederherstellung>>;

    async fn find_by_konto(
        &self,
        konto_id: KontoId,
    ) -> RepositoryResult<Vec<Versioned<PasswortWiederherstellung>>>;

    async fn update(
        &self,
        wiederherstellung: &mut Versioned<PasswortWiederherstellung>,
    ) -> RepositoryResult<()>;

    async fn delete(
        &self,
        wiederherstellung: Versioned<PasswortWiederherstellung>,
    ) -> RepositoryResult<()>;

    async fn delete_by_konto(&self, konto_id: KontoId) -> RepositoryResult<()>;
}
