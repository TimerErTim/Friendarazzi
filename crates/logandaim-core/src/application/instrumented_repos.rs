//! Repository tracing wrappers.

use std::pin::Pin;

use async_trait::async_trait;
use tracing::instrument;

use crate::application::uow::UnitOfWorkImpl;
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
use crate::ports::{
    BenutzerRepository, BoardRepository, CaptureRepository, DeckRepository,
    EmailBestätigungRepository, GameRepository, KontoRepository,
    PasswortWiederherstellungRepository, RepositoryResult, SessionRepository,
    UserProfileRepository,
};

macro_rules! impl_instrumented_repo {
    (
        $wrapper:ident,
        $trait:ident,
        $accessor:ident,
        methods: [
            $(#[$meta:meta] fn $name:ident(&self $(, $arg:ident : $arg_ty:ty)*) -> $ret:ty;)*
        ]
    ) => {
        pub(crate) struct $wrapper {
            uow: Option<*const dyn UnitOfWorkImpl>,
        }

        impl $wrapper {
            pub(crate) fn new(uow: *const dyn UnitOfWorkImpl) -> Self {
                Self { uow: Some(uow) }
            }

            pub(crate) fn disabled() -> Self {
                Self { uow: None }
            }

            fn uow(&self) -> &dyn UnitOfWorkImpl {
                let uow = self
                    .uow
                    .expect("instrumented repository without unit of work");
                unsafe { &*uow }
            }
        }

        unsafe impl Send for $wrapper {}
        unsafe impl Sync for $wrapper {}

        #[async_trait]
        impl $trait for $wrapper {
            $(#[$meta]
            async fn $name(&self $(, $arg : $arg_ty)*) -> $ret {
                self.uow().$accessor().$name($($arg),*).await
            })*
        }
    };
}

impl_instrumented_repo!(
    InstrumentedUserProfileRepository,
    UserProfileRepository,
    profiles,
    methods: [
        #[instrument(skip(self, profile), level = "debug", err(Debug))]
        fn create(&self, profile: NewUserProfile) -> RepositoryResult<Versioned<UserProfile>>;
        #[instrument(skip(self, user_id), fields(user_id = ?user_id), level = "trace", err(Debug))]
        fn find_by_user_id(&self, user_id: BenutzerId) -> RepositoryResult<Versioned<UserProfile>>;
        #[instrument(skip(self, user_ids), level = "trace", err(Debug))]
        fn find_by_user_ids(&self, user_ids: &[BenutzerId]) -> RepositoryResult<Vec<Versioned<UserProfile>>>;
        #[instrument(skip(self, username), level = "trace", err(Debug))]
        fn find_by_username(&self, username: &str) -> RepositoryResult<Versioned<UserProfile>>;
        #[instrument(skip(self, profile), level = "debug", err(Debug))]
        fn update(&self, profile: &mut Versioned<UserProfile>) -> RepositoryResult<()>;
        #[instrument(skip(self, user_id), fields(user_id = ?user_id), level = "debug", err(Debug))]
        fn delete_by_user_id(&self, user_id: BenutzerId) -> RepositoryResult<()>;
    ]
);

impl_instrumented_repo!(
    InstrumentedGameRepository,
    GameRepository,
    games,
    methods: [
        #[instrument(skip(self, game), level = "debug", err(Debug))]
        fn create(&self, game: NewGame) -> RepositoryResult<Versioned<Game>>;
        #[instrument(skip(self, id), fields(id = ?id), level = "trace", err(Debug))]
        fn find_by_id(&self, id: GameId) -> RepositoryResult<Versioned<Game>>;
        #[instrument(skip(self, code), level = "trace", err(Debug))]
        fn find_by_invite_code(&self, code: &InviteCode) -> RepositoryResult<Versioned<Game>>;
        #[instrument(skip(self, user_id), fields(user_id = ?user_id), level = "trace", err(Debug))]
        fn find_for_user(&self, user_id: BenutzerId) -> RepositoryResult<Vec<Versioned<Game>>>;
        #[instrument(skip(self, game), level = "debug", err(Debug))]
        fn update(&self, game: &mut Versioned<Game>) -> RepositoryResult<()>;
        #[instrument(skip(self, player), level = "debug", err(Debug))]
        fn add_player(&self, player: GamePlayer) -> RepositoryResult<()>;
        #[instrument(skip(self, game_id), fields(game_id = ?game_id), level = "trace", err(Debug))]
        fn find_players(&self, game_id: GameId) -> RepositoryResult<Vec<GamePlayer>>;
        #[instrument(skip(self, game_id, user_id), level = "debug", err(Debug))]
        fn set_player_draft_locked(&self, game_id: GameId, user_id: BenutzerId, locked: bool) -> RepositoryResult<()>;
        #[instrument(skip(self, game_id, author_user_id, prompts), level = "debug", err(Debug))]
        fn replace_author_prompts(&self, game_id: GameId, author_user_id: BenutzerId, prompts: Vec<PromptSubmission>) -> RepositoryResult<()>;
        #[instrument(skip(self, game_id), fields(game_id = ?game_id), level = "trace", err(Debug))]
        fn find_prompts_by_game(&self, game_id: GameId) -> RepositoryResult<Vec<PromptSubmission>>;
        #[instrument(skip(self, game_id, author_user_id), level = "trace", err(Debug))]
        fn find_prompts_by_author(&self, game_id: GameId, author_user_id: BenutzerId) -> RepositoryResult<Vec<PromptSubmission>>;
    ]
);

impl_instrumented_repo!(
    InstrumentedBoardRepository,
    BoardRepository,
    boards,
    methods: [
        #[instrument(skip(self, board, tiles), level = "debug", err(Debug))]
        fn create_board_with_tiles(&self, board: Board, tiles: Vec<BoardTile>) -> RepositoryResult<()>;
        #[instrument(skip(self, game_id), fields(game_id = ?game_id), level = "trace", err(Debug))]
        fn find_boards_by_game(&self, game_id: GameId) -> RepositoryResult<Vec<Versioned<Board>>>;
        #[instrument(skip(self, game_id, owner), level = "trace", err(Debug))]
        fn find_board_by_game_and_owner(&self, game_id: GameId, owner: BenutzerId) -> RepositoryResult<Versioned<Board>>;
        #[instrument(skip(self, board_id), fields(board_id = ?board_id), level = "trace", err(Debug))]
        fn find_tiles_by_board(&self, board_id: BoardId) -> RepositoryResult<Vec<BoardTile>>;
        #[instrument(skip(self, game_id), fields(game_id = ?game_id), level = "trace", err(Debug))]
        fn find_tiles_by_game(&self, game_id: GameId) -> RepositoryResult<Vec<BoardTile>>;
        #[instrument(skip(self, tile_id), fields(tile_id = ?tile_id), level = "trace", err(Debug))]
        fn find_tile_by_id(&self, tile_id: BoardTileId) -> RepositoryResult<BoardTile>;
        #[instrument(skip(self, board), level = "debug", err(Debug))]
        fn update_board(&self, board: &mut Versioned<Board>) -> RepositoryResult<()>;
        #[instrument(skip(self, tile_id, capture_id), level = "debug", err(Debug))]
        fn attach_tile_capture(&self, tile_id: BoardTileId, capture_id: CaptureId) -> RepositoryResult<()>;
    ]
);

impl_instrumented_repo!(
    InstrumentedCaptureRepository,
    CaptureRepository,
    captures,
    methods: [
        #[instrument(skip(self, capture), level = "debug", err(Debug))]
        fn create(&self, capture: NewCapture) -> RepositoryResult<Versioned<Capture>>;
        #[instrument(skip(self, id), fields(id = ?id), level = "trace", err(Debug))]
        fn find_by_id(&self, id: CaptureId) -> RepositoryResult<Versioned<Capture>>;
        #[instrument(skip(self, game_id), fields(game_id = ?game_id), level = "trace", err(Debug))]
        fn find_by_game(&self, game_id: GameId) -> RepositoryResult<Vec<Versioned<Capture>>>;
        #[instrument(skip(self, user_id), fields(user_id = ?user_id), level = "trace", err(Debug))]
        fn find_by_target_user(&self, user_id: BenutzerId) -> RepositoryResult<Vec<Versioned<Capture>>>;
        #[instrument(skip(self, user_id), fields(user_id = ?user_id), level = "trace", err(Debug))]
        fn find_by_catcher_user(&self, user_id: BenutzerId) -> RepositoryResult<Vec<Versioned<Capture>>>;
        #[instrument(skip(self, key), level = "trace", err(Debug))]
        fn find_by_storage_key(&self, key: &str) -> RepositoryResult<Versioned<Capture>>;
    ]
);

impl_instrumented_repo!(
    InstrumentedDeckRepository,
    DeckRepository,
    decks,
    methods: [
        #[instrument(skip(self), level = "trace", err(Debug))]
        fn list_all(&self) -> RepositoryResult<Vec<StarterDeck>>;
    ]
);

impl_instrumented_repo!(
    InstrumentedBenutzerRepository,
    BenutzerRepository,
    benutzer,
    methods: [
        #[instrument(skip(self, benutzer), level = "debug", err(Debug))]
        fn create(&self, benutzer: NeuerBenutzer) -> RepositoryResult<Versioned<Benutzer>>;
        #[instrument(skip(self, id), fields(id = ?id), level = "trace", err(Debug))]
        fn find_by_id(&self, id: BenutzerId) -> RepositoryResult<Versioned<Benutzer>>;
        #[instrument(skip(self, benutzer), level = "debug", err(Debug))]
        fn update(&self, benutzer: &mut Versioned<Benutzer>) -> RepositoryResult<()>;
        #[instrument(skip(self, benutzer), level = "debug", err(Debug))]
        fn delete(&self, benutzer: Versioned<Benutzer>) -> RepositoryResult<()>;
    ]
);

impl_instrumented_repo!(
    InstrumentedKontoRepository,
    KontoRepository,
    konten,
    methods: [
        #[instrument(skip(self, konto), level = "debug", err(Debug))]
        fn create(&self, konto: NeuesKonto) -> RepositoryResult<Versioned<Konto>>;
        #[instrument(skip(self, id), fields(id = ?id), level = "trace", err(Debug))]
        fn find_by_id(&self, id: KontoId) -> RepositoryResult<Versioned<Konto>>;
        #[instrument(skip(self, email), fields(email = email.as_ref()), level = "trace", err(Debug))]
        fn find_by_email(&self, email: &EmailAdresse) -> RepositoryResult<Versioned<Konto>>;
        #[instrument(skip(self, benutzer_id), fields(benutzer_id = ?benutzer_id), level = "trace", err(Debug))]
        fn find_by_benutzer(&self, benutzer_id: BenutzerId) -> RepositoryResult<Vec<Versioned<Konto>>>;
        #[instrument(skip(self, konto), level = "debug", err(Debug))]
        fn update(&self, konto: &mut Versioned<Konto>) -> RepositoryResult<()>;
        #[instrument(skip(self, konto), level = "debug", err(Debug))]
        fn delete(&self, konto: Versioned<Konto>) -> RepositoryResult<()>;
    ]
);

impl_instrumented_repo!(
    InstrumentedSessionRepository,
    SessionRepository,
    sessions,
    methods: [
        #[instrument(skip(self, session), level = "debug", err(Debug))]
        fn create(&self, session: NeueSession) -> RepositoryResult<Versioned<Session>>;
        #[instrument(skip(self, id), fields(id = ?id), level = "trace", err(Debug))]
        fn find_by_id(&self, id: SessionId) -> RepositoryResult<Versioned<Session>>;
        #[instrument(skip(self, hash), level = "trace", err(Debug))]
        fn find_by_refresh_hash(&self, hash: &SecretTokenHash) -> RepositoryResult<Versioned<Session>>;
        #[instrument(skip(self, session), level = "debug", err(Debug))]
        fn update(&self, session: &mut Versioned<Session>) -> RepositoryResult<()>;
        #[instrument(skip(self, benutzer_id), fields(benutzer_id = ?benutzer_id), level = "trace", err(Debug))]
        fn find_by_benutzer(&self, benutzer_id: BenutzerId) -> RepositoryResult<Vec<Versioned<Session>>>;
        #[instrument(skip(self, session), level = "debug", err(Debug))]
        fn delete(&self, session: Versioned<Session>) -> RepositoryResult<()>;
        #[instrument(skip(self, benutzer_id), fields(benutzer_id = ?benutzer_id), level = "debug", err(Debug))]
        fn delete_by_benutzer(&self, benutzer_id: BenutzerId) -> RepositoryResult<()>;
    ]
);

impl_instrumented_repo!(
    InstrumentedEmailBestätigungRepository,
    EmailBestätigungRepository,
    bestätigungen,
    methods: [
        #[instrument(skip(self, bestätigung), level = "debug", err(Debug))]
        fn create(&self, bestätigung: NeueBestätigung) -> RepositoryResult<Versioned<EmailBestätigung>>;
        #[instrument(skip(self, hash), level = "trace", err(Debug))]
        fn find_by_hash(&self, hash: &SecretTokenHash) -> RepositoryResult<Versioned<EmailBestätigung>>;
        #[instrument(skip(self, id), fields(id = ?id), level = "trace", err(Debug))]
        fn find_by_id(&self, id: BestätigungId) -> RepositoryResult<Versioned<EmailBestätigung>>;
        #[instrument(skip(self, email), fields(email = email.as_ref()), level = "trace", err(Debug))]
        fn find_by_email(&self, email: &EmailAdresse) -> RepositoryResult<Vec<Versioned<EmailBestätigung>>>;
        #[instrument(skip(self, bestätigung), level = "debug", err(Debug))]
        fn update(&self, bestätigung: &mut Versioned<EmailBestätigung>) -> RepositoryResult<()>;
        #[instrument(skip(self, bestätigung), level = "debug", err(Debug))]
        fn delete(&self, bestätigung: Versioned<EmailBestätigung>) -> RepositoryResult<()>;
        #[instrument(skip(self, email), fields(email = email.as_ref()), level = "debug", err(Debug))]
        fn delete_by_email(&self, email: &EmailAdresse) -> RepositoryResult<()>;
    ]
);

impl_instrumented_repo!(
    InstrumentedPasswortWiederherstellungRepository,
    PasswortWiederherstellungRepository,
    wiederherstellungen,
    methods: [
        #[instrument(skip(self, wiederherstellung), level = "debug", err(Debug))]
        fn create(&self, wiederherstellung: NeuePasswortWiederherstellung) -> RepositoryResult<Versioned<PasswortWiederherstellung>>;
        #[instrument(skip(self, id), fields(id = ?id), level = "trace", err(Debug))]
        fn find_by_id(&self, id: PasswortWiederherstellungId) -> RepositoryResult<Versioned<PasswortWiederherstellung>>;
        #[instrument(skip(self, hash), level = "trace", err(Debug))]
        fn find_by_hash(&self, hash: &SecretTokenHash) -> RepositoryResult<Versioned<PasswortWiederherstellung>>;
        #[instrument(skip(self, konto_id), fields(konto_id = ?konto_id), level = "trace", err(Debug))]
        fn find_by_konto(&self, konto_id: KontoId) -> RepositoryResult<Vec<Versioned<PasswortWiederherstellung>>>;
        #[instrument(skip(self, wiederherstellung), level = "debug", err(Debug))]
        fn update(&self, wiederherstellung: &mut Versioned<PasswortWiederherstellung>) -> RepositoryResult<()>;
        #[instrument(skip(self, wiederherstellung), level = "debug", err(Debug))]
        fn delete(&self, wiederherstellung: Versioned<PasswortWiederherstellung>) -> RepositoryResult<()>;
        #[instrument(skip(self, konto_id), fields(konto_id = ?konto_id), level = "debug", err(Debug))]
        fn delete_by_konto(&self, konto_id: KontoId) -> RepositoryResult<()>;
    ]
);

/// Pinned bundle: instrumented repos hold pointers into `uow` (field order keeps drop safe).
pub(crate) struct RepoStorage {
    pub(crate) profiles: InstrumentedUserProfileRepository,
    pub(crate) games: InstrumentedGameRepository,
    pub(crate) boards: InstrumentedBoardRepository,
    pub(crate) captures: InstrumentedCaptureRepository,
    pub(crate) decks: InstrumentedDeckRepository,
    pub(crate) benutzer: InstrumentedBenutzerRepository,
    pub(crate) konten: InstrumentedKontoRepository,
    pub(crate) sessions: InstrumentedSessionRepository,
    pub(crate) bestätigungen: InstrumentedEmailBestätigungRepository,
    pub(crate) wiederherstellungen: InstrumentedPasswortWiederherstellungRepository,
    pub(crate) uow: Box<dyn UnitOfWorkImpl>,
}

impl RepoStorage {
    pub(crate) fn new(uow: Box<dyn UnitOfWorkImpl>) -> Pin<Box<Self>> {
        let mut pinned = Box::pin(RepoStorage {
            profiles: InstrumentedUserProfileRepository::disabled(),
            games: InstrumentedGameRepository::disabled(),
            boards: InstrumentedBoardRepository::disabled(),
            captures: InstrumentedCaptureRepository::disabled(),
            decks: InstrumentedDeckRepository::disabled(),
            benutzer: InstrumentedBenutzerRepository::disabled(),
            konten: InstrumentedKontoRepository::disabled(),
            sessions: InstrumentedSessionRepository::disabled(),
            bestätigungen: InstrumentedEmailBestätigungRepository::disabled(),
            wiederherstellungen: InstrumentedPasswortWiederherstellungRepository::disabled(),
            uow,
        });

        unsafe {
            let this = pinned.as_mut().get_unchecked_mut();
            let uow_ptr = std::ptr::from_ref(this.uow.as_ref());
            this.profiles = InstrumentedUserProfileRepository::new(uow_ptr);
            this.games = InstrumentedGameRepository::new(uow_ptr);
            this.boards = InstrumentedBoardRepository::new(uow_ptr);
            this.captures = InstrumentedCaptureRepository::new(uow_ptr);
            this.decks = InstrumentedDeckRepository::new(uow_ptr);
            this.benutzer = InstrumentedBenutzerRepository::new(uow_ptr);
            this.konten = InstrumentedKontoRepository::new(uow_ptr);
            this.sessions = InstrumentedSessionRepository::new(uow_ptr);
            this.bestätigungen = InstrumentedEmailBestätigungRepository::new(uow_ptr);
            this.wiederherstellungen =
                InstrumentedPasswortWiederherstellungRepository::new(uow_ptr);
        }

        pinned
    }
}
