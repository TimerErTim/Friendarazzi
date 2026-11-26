use async_trait::async_trait;

use crate::application::ResultReport;
use crate::application::uow::Versioned;
use crate::domain::konto::{
    BestätigungId, EmailBestätigung, Konto, KontoId, NeueBestätigung,
    NeuePasswortWiederherstellung, NeueSession, NeuesKonto, PasswortWiederherstellung,
    PasswortWiederherstellungId, SecretTokenHash, Session, SessionId,
};
use crate::domain::{
    Benachrichtigung, BenachrichtigungId, Benutzer, BenutzerId, Board, BoardId, Capture, CaptureId,
    EmailAdresse, Game, GameId, InviteCode, NeueBenachrichtigung, NeuerBenutzer, NewBoard,
    NewCapture, NewGame, StarterDeck,
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
pub trait GameRepository: Send + Sync {
    async fn create(&self, game: NewGame) -> RepositoryResult<Versioned<Game>>;
    async fn find_by_id(&self, id: GameId) -> RepositoryResult<Versioned<Game>>;
    async fn find_by_invite_code(&self, code: &InviteCode) -> RepositoryResult<Versioned<Game>>;
    async fn find_by_participating_user(
        &self,
        user_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Game>>>;
    async fn find_drafting_with_countdown(&self) -> RepositoryResult<Vec<Versioned<Game>>>;
    async fn update(&self, game: &mut Versioned<Game>) -> RepositoryResult<()>;
}

#[async_trait]
pub trait BoardRepository: Send + Sync {
    async fn create(&self, board: NewBoard) -> RepositoryResult<Versioned<Board>>;
    async fn find_by_id(&self, id: BoardId) -> RepositoryResult<Versioned<Board>>;
    async fn find_by_game(&self, game_id: GameId) -> RepositoryResult<Vec<Versioned<Board>>>;
    async fn find_by_game_and_owner(
        &self,
        game_id: GameId,
        owner: BenutzerId,
    ) -> RepositoryResult<Versioned<Board>>;
    async fn update(&self, board: &mut Versioned<Board>) -> RepositoryResult<()>;
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
    async fn find_all(&self) -> RepositoryResult<Vec<StarterDeck>>;
}

#[async_trait]
pub trait BenutzerRepository: Send + Sync {
    async fn create(&self, benutzer: NeuerBenutzer) -> RepositoryResult<Versioned<Benutzer>>;

    async fn find_by_id(&self, id: BenutzerId) -> RepositoryResult<Versioned<Benutzer>>;

    async fn find_by_ids(&self, ids: &[BenutzerId]) -> RepositoryResult<Vec<Versioned<Benutzer>>>;

    async fn find_by_email(&self, email: &EmailAdresse) -> RepositoryResult<Versioned<Benutzer>>;

    async fn update(&self, benutzer: &mut Versioned<Benutzer>) -> RepositoryResult<()>;

    async fn delete(&self, benutzer: Versioned<Benutzer>) -> RepositoryResult<()>;
}

#[async_trait]
pub trait KontoRepository: Send + Sync {
    async fn create(&self, konto: NeuesKonto) -> RepositoryResult<Versioned<Konto>>;

    async fn find_by_id(&self, id: KontoId) -> RepositoryResult<Versioned<Konto>>;

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

#[async_trait]
pub trait BenachrichtigungRepository: Send + Sync {
    async fn create(
        &self,
        benachrichtigung: NeueBenachrichtigung,
    ) -> RepositoryResult<Versioned<Benachrichtigung>>;

    async fn find_by_id(
        &self,
        id: BenachrichtigungId,
    ) -> RepositoryResult<Versioned<Benachrichtigung>>;

    async fn find_by_empfänger(
        &self,
        empfänger_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Benachrichtigung>>>;

    async fn find_by_abbestell_token(
        &self,
        token_hash: &crate::domain::konto::SecretTokenHash,
    ) -> RepositoryResult<Versioned<Benachrichtigung>>;

    async fn update(
        &self,
        benachrichtigung: &mut Versioned<Benachrichtigung>,
    ) -> RepositoryResult<()>;

    async fn delete(&self, benachrichtigung: Versioned<Benachrichtigung>) -> RepositoryResult<()>;

    async fn delete_by_empfänger(&self, empfänger_id: BenutzerId) -> RepositoryResult<()>;
}
