use std::hash::Hash;
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use error_stack::{Report, ResultExt};
use rustc_hash::{FxHashMap, FxHashSet};
use uuid::Uuid;

use friendarazzi_core::{
    ResultReport,
    application::uow::Versioned,
    domain::{
        Benachrichtigung, BenachrichtigungId, Benutzer, BenutzerId, Board, BoardId, Capture,
        CaptureId, EmailAdresse, Game, GameId, InviteCode, Konto, KontoId, NeueBenachrichtigung,
        NeuerBenutzer, NewBoard, NewCapture, NewGame, StarterDeck, builtin_starter_decks,
        konto::{
            BestätigungId, EmailBestätigung, NeueBestätigung, NeuePasswortWiederherstellung,
            NeueSession, NeuesKonto, PasswortWiederherstellung, PasswortWiederherstellungId,
            RenewableSession, SecretTokenHash, Session, SessionId,
        },
    },
    ports::{
        BenachrichtigungRepository, BenutzerRepository, BoardRepository, CaptureRepository,
        DeckRepository, EmailBestätigungRepository, GameRepository, KontoRepository, Mail,
        MailError, MailSender, PasswortWiederherstellungRepository, RepositoryError,
        RepositoryResult, SessionRepository,
    },
};

/// In-memory backing store: one map per aggregate, all on the same snapshot.
pub struct FakeDatastore {
    pub games: Mutex<FxHashMap<GameId, Versioned<Game>>>,
    pub boards: Mutex<FxHashMap<BoardId, Versioned<Board>>>,
    pub captures: Mutex<FxHashMap<CaptureId, Versioned<Capture>>>,
    pub decks: Mutex<Vec<StarterDeck>>,
    pub benutzer: Mutex<FxHashMap<BenutzerId, Versioned<Benutzer>>>,
    pub konten: Mutex<FxHashMap<KontoId, Versioned<Konto>>>,
    pub sessions: Mutex<FxHashMap<SessionId, Versioned<Session>>>,
    pub bestätigungen: Mutex<FxHashMap<BestätigungId, Versioned<EmailBestätigung>>>,
    pub wiederherstellungen:
        Mutex<FxHashMap<PasswortWiederherstellungId, Versioned<PasswortWiederherstellung>>>,
    pub benachrichtigungen: Mutex<FxHashMap<BenachrichtigungId, Versioned<Benachrichtigung>>>,
}

impl Clone for FakeDatastore {
    fn clone(&self) -> Self {
        Self {
            games: Mutex::new(self.games().clone()),
            boards: Mutex::new(self.boards().clone()),
            captures: Mutex::new(self.captures().clone()),
            decks: Mutex::new(self.decks().clone()),
            benutzer: Mutex::new(self.benutzer().clone()),
            konten: Mutex::new(self.konten().clone()),
            sessions: Mutex::new(self.sessions().clone()),
            bestätigungen: Mutex::new(self.bestätigungen().clone()),
            wiederherstellungen: Mutex::new(self.wiederherstellungen().clone()),
            benachrichtigungen: Mutex::new(self.benachrichtigungen().clone()),
        }
    }
}

impl Default for FakeDatastore {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeDatastore {
    pub fn new() -> Self {
        Self {
            games: Mutex::new(FxHashMap::default()),
            boards: Mutex::new(FxHashMap::default()),
            captures: Mutex::new(FxHashMap::default()),
            decks: Mutex::new(builtin_starter_decks()),
            benutzer: Mutex::new(FxHashMap::default()),
            konten: Mutex::new(FxHashMap::default()),
            sessions: Mutex::new(FxHashMap::default()),
            bestätigungen: Mutex::new(FxHashMap::default()),
            wiederherstellungen: Mutex::new(FxHashMap::default()),
            benachrichtigungen: Mutex::new(FxHashMap::default()),
        }
    }

    pub fn games(&self) -> MutexGuard<'_, FxHashMap<GameId, Versioned<Game>>> {
        self.games.lock().unwrap()
    }

    pub fn boards(&self) -> MutexGuard<'_, FxHashMap<BoardId, Versioned<Board>>> {
        self.boards.lock().unwrap()
    }

    pub fn captures(&self) -> MutexGuard<'_, FxHashMap<CaptureId, Versioned<Capture>>> {
        self.captures.lock().unwrap()
    }

    pub fn decks(&self) -> MutexGuard<'_, Vec<StarterDeck>> {
        self.decks.lock().unwrap()
    }

    pub fn benutzer(&self) -> MutexGuard<'_, FxHashMap<BenutzerId, Versioned<Benutzer>>> {
        self.benutzer.lock().unwrap()
    }

    pub fn konten(&self) -> MutexGuard<'_, FxHashMap<KontoId, Versioned<Konto>>> {
        self.konten.lock().unwrap()
    }

    pub fn sessions(&self) -> MutexGuard<'_, FxHashMap<SessionId, Versioned<Session>>> {
        self.sessions.lock().unwrap()
    }

    pub fn bestätigungen(
        &self,
    ) -> MutexGuard<'_, FxHashMap<BestätigungId, Versioned<EmailBestätigung>>> {
        self.bestätigungen.lock().unwrap()
    }

    pub fn wiederherstellungen(
        &self,
    ) -> MutexGuard<'_, FxHashMap<PasswortWiederherstellungId, Versioned<PasswortWiederherstellung>>>
    {
        self.wiederherstellungen.lock().unwrap()
    }

    pub fn benachrichtigungen(
        &self,
    ) -> MutexGuard<'_, FxHashMap<BenachrichtigungId, Versioned<Benachrichtigung>>> {
        self.benachrichtigungen.lock().unwrap()
    }

    pub fn replace_with(&self, other: &FakeDatastore) {
        *self.games() = other.games().clone();
        *self.boards() = other.boards().clone();
        *self.captures() = other.captures().clone();
        *self.decks() = other.decks().clone();
        *self.benutzer() = other.benutzer().clone();
        *self.konten() = other.konten().clone();
        *self.sessions() = other.sessions().clone();
        *self.bestätigungen() = other.bestätigungen().clone();
        *self.wiederherstellungen() = other.wiederherstellungen().clone();
        *self.benachrichtigungen() = other.benachrichtigungen().clone();
    }

    /// 3-way merge of `tx` against `reference` into `target`.
    ///
    /// Computes all merged aggregate maps before mutating any `target` map so a conflict in
    /// one aggregate never leaves earlier aggregates partially committed.
    pub fn merge(
        target: &FakeDatastore,
        reference: &FakeDatastore,
        tx: &FakeDatastore,
    ) -> Result<FakeDatastore, RepositoryError> {
        fn lock_map<K, T>(
            map: &Mutex<FxHashMap<K, Versioned<T>>>,
        ) -> Result<MutexGuard<'_, FxHashMap<K, Versioned<T>>>, RepositoryError> {
            map.lock().map_err(|_| RepositoryError::OperationFailed)
        }

        let mut target_games = lock_map(&target.games)?;
        let mut target_boards = lock_map(&target.boards)?;
        let mut target_captures = lock_map(&target.captures)?;
        let target_decks = target
            .decks
            .lock()
            .map_err(|_| RepositoryError::OperationFailed)?;
        let mut target_benutzer = lock_map(&target.benutzer)?;
        let mut target_konten = lock_map(&target.konten)?;
        let mut target_sessions = lock_map(&target.sessions)?;
        let mut target_bestätigungen = lock_map(&target.bestätigungen)?;
        let mut target_wiederherstellungen = lock_map(&target.wiederherstellungen)?;
        let mut target_benachrichtigungen = lock_map(&target.benachrichtigungen)?;

        let merged_games = Self::merge_single_aggregate(
            &target_games,
            &*lock_map(&reference.games)?,
            &*lock_map(&tx.games)?,
        )?;
        let merged_boards = Self::merge_single_aggregate(
            &target_boards,
            &*lock_map(&reference.boards)?,
            &*lock_map(&tx.boards)?,
        )?;
        let merged_captures = Self::merge_single_aggregate(
            &target_captures,
            &*lock_map(&reference.captures)?,
            &*lock_map(&tx.captures)?,
        )?;
        let merged_benutzer = Self::merge_single_aggregate(
            &target_benutzer,
            &*lock_map(&reference.benutzer)?,
            &*lock_map(&tx.benutzer)?,
        )?;
        let merged_konten = Self::merge_single_aggregate(
            &target_konten,
            &*lock_map(&reference.konten)?,
            &*lock_map(&tx.konten)?,
        )?;
        let merged_sessions = Self::merge_single_aggregate(
            &target_sessions,
            &*lock_map(&reference.sessions)?,
            &*lock_map(&tx.sessions)?,
        )?;
        let merged_bestätigungen = Self::merge_single_aggregate(
            &target_bestätigungen,
            &*lock_map(&reference.bestätigungen)?,
            &*lock_map(&tx.bestätigungen)?,
        )?;
        let merged_wiederherstellungen = Self::merge_single_aggregate(
            &target_wiederherstellungen,
            &*lock_map(&reference.wiederherstellungen)?,
            &*lock_map(&tx.wiederherstellungen)?,
        )?;
        let merged_benachrichtigungen = Self::merge_single_aggregate(
            &target_benachrichtigungen,
            &*lock_map(&reference.benachrichtigungen)?,
            &*lock_map(&tx.benachrichtigungen)?,
        )?;

        *target_games = merged_games.clone();
        *target_boards = merged_boards.clone();
        *target_captures = merged_captures.clone();
        *target_benutzer = merged_benutzer.clone();
        *target_konten = merged_konten.clone();
        *target_sessions = merged_sessions.clone();
        *target_bestätigungen = merged_bestätigungen.clone();
        *target_wiederherstellungen = merged_wiederherstellungen.clone();
        *target_benachrichtigungen = merged_benachrichtigungen.clone();

        Ok(FakeDatastore {
            games: Mutex::new(merged_games),
            boards: Mutex::new(merged_boards),
            captures: Mutex::new(merged_captures),
            decks: Mutex::new(target_decks.clone()),
            benutzer: Mutex::new(merged_benutzer),
            konten: Mutex::new(merged_konten),
            sessions: Mutex::new(merged_sessions),
            bestätigungen: Mutex::new(merged_bestätigungen),
            wiederherstellungen: Mutex::new(merged_wiederherstellungen),
            benachrichtigungen: Mutex::new(merged_benachrichtigungen),
        })
    }

    fn merge_single_aggregate<K, T>(
        target: &FxHashMap<K, Versioned<T>>,
        reference: &FxHashMap<K, Versioned<T>>,
        tx: &FxHashMap<K, Versioned<T>>,
    ) -> Result<FxHashMap<K, Versioned<T>>, RepositoryError>
    where
        K: Hash + Eq + Copy,
        T: Clone,
    {
        let mut merged = target.clone();
        let all_ids: FxHashSet<K> = reference.keys().chain(tx.keys()).copied().collect();

        for id in all_ids {
            let target_versioned = target.get(&id).cloned();
            let reference_versioned = reference.get(&id).cloned();
            let tx_versioned = tx.get(&id).cloned();

            match (reference_versioned, tx_versioned) {
                (Some(reference_versioned), Some(tx_versioned)) => {
                    if reference_versioned.v() != tx_versioned.v() {
                        let Some(target_versioned) = target_versioned else {
                            return Err(RepositoryError::Conflict);
                        };
                        if target_versioned.v() > reference_versioned.v() {
                            return Err(RepositoryError::VersionMismatch {
                                expected: reference_versioned.v(),
                                actual: Some(target_versioned.v()),
                            });
                        }
                        merged.insert(id, tx_versioned);
                    }
                }
                (Some(reference_versioned), None) => {
                    if let Some(target_v) = target_versioned
                        && target_v.v() > reference_versioned.v()
                    {
                        return Err(RepositoryError::VersionMismatch {
                            expected: reference_versioned.v(),
                            actual: Some(target_v.v()),
                        });
                    }
                    merged.remove(&id);
                }
                (None, Some(tx_versioned)) => {
                    if target_versioned.is_some() {
                        return Err(RepositoryError::Conflict);
                    }
                    merged.insert(id, tx_versioned);
                }
                (None, None) => {}
            }
        }
        Ok(merged)
    }
}

fn store_update<K, T>(
    map: &mut FxHashMap<K, Versioned<T>>,
    id: K,
    neu: &mut Versioned<T>,
) -> RepositoryResult<()>
where
    K: std::hash::Hash + Eq + Copy,
    T: Clone,
{
    let vorhanden = map
        .get(&id)
        .ok_or_else(|| Report::new(RepositoryError::NotFound))?;
    if vorhanden.v() != neu.v() {
        return Err(Report::new(RepositoryError::VersionMismatch {
            expected: neu.v(),
            actual: Some(vorhanden.v()),
        }));
    }
    neu.increment();
    map.insert(id, neu.clone());
    Ok(())
}

fn store_delete<K, T>(
    map: &mut FxHashMap<K, Versioned<T>>,
    id: K,
    geladen: &Versioned<T>,
) -> RepositoryResult<()>
where
    K: std::hash::Hash + Eq + Copy,
{
    let vorhanden = map
        .get(&id)
        .ok_or_else(|| Report::new(RepositoryError::NotFound))?;
    if vorhanden.v() != geladen.v() {
        return Err(Report::new(RepositoryError::VersionMismatch {
            expected: geladen.v(),
            actual: Some(vorhanden.v()),
        }));
    }
    map.remove(&id);
    Ok(())
}

pub struct FakeGameRepository {
    datastore: Arc<FakeDatastore>,
}

impl FakeGameRepository {
    pub fn new(datastore: Arc<FakeDatastore>) -> Self {
        Self { datastore }
    }
}

#[async_trait]
impl GameRepository for FakeGameRepository {
    async fn create(&self, game: NewGame) -> RepositoryResult<Versioned<Game>> {
        let mut games = self.datastore.games();
        if games.values().any(|g| g.invite_code() == &game.invite_code) {
            return Err(Report::new(RepositoryError::Conflict));
        }
        let id = GameId(Uuid::new_v4());
        let created = Versioned::init(Game::new(id, game));
        games.insert(id, created.clone());
        Ok(created)
    }

    async fn find_by_id(&self, id: GameId) -> RepositoryResult<Versioned<Game>> {
        self.datastore
            .games()
            .get(&id)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn find_by_invite_code(&self, code: &InviteCode) -> RepositoryResult<Versioned<Game>> {
        self.datastore
            .games()
            .values()
            .find(|g| g.invite_code() == code)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn find_by_participating_user(
        &self,
        user_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Game>>> {
        let mut out: Vec<_> = self
            .datastore
            .games()
            .values()
            .filter(|g| g.has_player(user_id))
            .cloned()
            .collect();
        out.sort_by_key(|g| std::cmp::Reverse(g.created_at()));
        Ok(out)
    }

    async fn find_drafting_with_countdown(&self) -> RepositoryResult<Vec<Versioned<Game>>> {
        let mut out: Vec<_> = self
            .datastore
            .games()
            .values()
            .filter(|g| matches!(***g, Game::Drafting(ref d) if d.draft_closes_at().is_some()))
            .cloned()
            .collect();
        out.sort_by_key(|g| match &**g {
            Game::Drafting(d) => d.draft_closes_at(),
            _ => None,
        });
        Ok(out)
    }

    async fn update(&self, game: &mut Versioned<Game>) -> RepositoryResult<()> {
        let mut map = self.datastore.games();
        store_update(&mut map, game.id(), game)
    }
}

pub struct FakeBoardRepository {
    datastore: Arc<FakeDatastore>,
}

impl FakeBoardRepository {
    pub fn new(datastore: Arc<FakeDatastore>) -> Self {
        Self { datastore }
    }
}

#[async_trait]
impl BoardRepository for FakeBoardRepository {
    async fn create(&self, board: NewBoard) -> RepositoryResult<Versioned<Board>> {
        let mut boards = self.datastore.boards();
        if boards
            .values()
            .any(|b| b.game_id() == board.game_id() && b.owner_user_id() == board.owner_user_id())
        {
            return Err(Report::new(RepositoryError::Conflict));
        }
        let id = BoardId(Uuid::new_v4());
        let created = Versioned::init(Board::new(id, board).change_context(RepositoryError::Data)?);
        boards.insert(id, created.clone());
        Ok(created)
    }

    async fn find_by_id(&self, id: BoardId) -> RepositoryResult<Versioned<Board>> {
        self.datastore
            .boards()
            .get(&id)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn find_by_game(&self, game_id: GameId) -> RepositoryResult<Vec<Versioned<Board>>> {
        Ok(self
            .datastore
            .boards()
            .values()
            .filter(|b| b.game_id() == game_id)
            .cloned()
            .collect())
    }

    async fn find_by_game_and_owner(
        &self,
        game_id: GameId,
        owner: BenutzerId,
    ) -> RepositoryResult<Versioned<Board>> {
        self.datastore
            .boards()
            .values()
            .find(|b| b.game_id() == game_id && b.owner_user_id() == owner)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn update(&self, board: &mut Versioned<Board>) -> RepositoryResult<()> {
        let mut map = self.datastore.boards();
        store_update(&mut map, board.id(), board)
    }
}

pub struct FakeCaptureRepository {
    datastore: Arc<FakeDatastore>,
}

impl FakeCaptureRepository {
    pub fn new(datastore: Arc<FakeDatastore>) -> Self {
        Self { datastore }
    }
}

#[async_trait]
impl CaptureRepository for FakeCaptureRepository {
    async fn create(&self, capture: NewCapture) -> RepositoryResult<Versioned<Capture>> {
        let mut captures = self.datastore.captures();
        if captures.values().any(|c| c.tile_id() == capture.tile_id) {
            return Err(Report::new(RepositoryError::Conflict));
        }
        let id = CaptureId(Uuid::new_v4());
        let created = Versioned::init(Capture::new(id, capture));
        captures.insert(id, created.clone());
        Ok(created)
    }

    async fn find_by_id(&self, id: CaptureId) -> RepositoryResult<Versioned<Capture>> {
        self.datastore
            .captures()
            .get(&id)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn find_by_game(&self, game_id: GameId) -> RepositoryResult<Vec<Versioned<Capture>>> {
        let mut out: Vec<_> = self
            .datastore
            .captures()
            .values()
            .filter(|c| c.game_id() == game_id)
            .cloned()
            .collect();
        out.sort_by_key(|c| c.captured_at());
        Ok(out)
    }

    async fn find_by_target_user(
        &self,
        user_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Capture>>> {
        let mut out: Vec<_> = self
            .datastore
            .captures()
            .values()
            .filter(|c| c.target_user_id() == user_id)
            .cloned()
            .collect();
        out.sort_by_key(|c| std::cmp::Reverse(c.captured_at()));
        Ok(out)
    }

    async fn find_by_catcher_user(
        &self,
        user_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Capture>>> {
        let mut out: Vec<_> = self
            .datastore
            .captures()
            .values()
            .filter(|c| c.catcher_user_id() == user_id)
            .cloned()
            .collect();
        out.sort_by_key(|c| std::cmp::Reverse(c.captured_at()));
        Ok(out)
    }

    async fn find_by_storage_key(&self, key: &str) -> RepositoryResult<Versioned<Capture>> {
        self.datastore
            .captures()
            .values()
            .find(|c| c.image_storage_key() == key || c.thumbnail_storage_key() == key)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }
}

pub struct FakeDeckRepository {
    datastore: Arc<FakeDatastore>,
}

impl FakeDeckRepository {
    pub fn new(datastore: Arc<FakeDatastore>) -> Self {
        Self { datastore }
    }
}

#[async_trait]
impl DeckRepository for FakeDeckRepository {
    async fn find_all(&self) -> RepositoryResult<Vec<StarterDeck>> {
        Ok(self.datastore.decks().clone())
    }
}

pub struct FakeBenutzerRepository {
    datastore: Arc<FakeDatastore>,
}

impl FakeBenutzerRepository {
    pub fn new(datastore: Arc<FakeDatastore>) -> Self {
        Self { datastore }
    }
}

#[async_trait]
impl BenutzerRepository for FakeBenutzerRepository {
    async fn create(&self, benutzer: NeuerBenutzer) -> RepositoryResult<Versioned<Benutzer>> {
        let mut map = self.datastore.benutzer();
        if map
            .values()
            .any(|b| b.email_address() == benutzer.email_address())
        {
            return Err(Report::new(RepositoryError::Conflict));
        }
        let id = BenutzerId(Uuid::new_v4());
        let erstellt = Versioned::init(Benutzer::neu(id, benutzer));
        map.insert(id, erstellt.clone());
        Ok(erstellt)
    }

    async fn find_by_id(&self, id: BenutzerId) -> RepositoryResult<Versioned<Benutzer>> {
        self.datastore
            .benutzer()
            .get(&id)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn find_by_ids(&self, ids: &[BenutzerId]) -> RepositoryResult<Vec<Versioned<Benutzer>>> {
        let map = self.datastore.benutzer();
        Ok(ids.iter().filter_map(|id| map.get(id).cloned()).collect())
    }

    async fn find_by_email(&self, email: &EmailAdresse) -> RepositoryResult<Versioned<Benutzer>> {
        self.datastore
            .benutzer()
            .values()
            .find(|b| b.email_address() == email)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn update(&self, benutzer: &mut Versioned<Benutzer>) -> RepositoryResult<()> {
        let mut map = self.datastore.benutzer();
        if map
            .values()
            .any(|b| b.id() != benutzer.id() && b.email_address() == benutzer.email_address())
        {
            return Err(Report::new(RepositoryError::Conflict));
        }
        store_update(&mut map, benutzer.id(), benutzer)
    }

    async fn delete(&self, benutzer: Versioned<Benutzer>) -> RepositoryResult<()> {
        let mut map = self.datastore.benutzer();
        store_delete(&mut map, benutzer.id(), &benutzer)
    }
}

pub struct FakeKontoRepository {
    datastore: Arc<FakeDatastore>,
}

impl FakeKontoRepository {
    pub fn new(datastore: Arc<FakeDatastore>) -> Self {
        Self { datastore }
    }
}

#[async_trait]
impl KontoRepository for FakeKontoRepository {
    async fn create(&self, konto: NeuesKonto) -> RepositoryResult<Versioned<Konto>> {
        let mut konten = self.datastore.konten();
        let id = KontoId(Uuid::new_v4());
        let erstellt = Versioned::init(Konto::neu(id, konto));
        konten.insert(id, erstellt.clone());
        Ok(erstellt)
    }

    async fn find_by_id(&self, id: KontoId) -> RepositoryResult<Versioned<Konto>> {
        self.datastore
            .konten()
            .get(&id)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn find_by_benutzer(
        &self,
        benutzer_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Konto>>> {
        Ok(self
            .datastore
            .konten()
            .values()
            .filter(|konto| konto.benutzer_id() == benutzer_id)
            .cloned()
            .collect())
    }

    async fn update(&self, konto: &mut Versioned<Konto>) -> RepositoryResult<()> {
        let mut map = self.datastore.konten();
        store_update(&mut map, konto.id(), konto)
    }

    async fn delete(&self, konto: Versioned<Konto>) -> RepositoryResult<()> {
        let mut map = self.datastore.konten();
        store_delete(&mut map, konto.id(), &konto)
    }
}

pub struct FakeSessionRepository {
    datastore: Arc<FakeDatastore>,
}

impl FakeSessionRepository {
    pub fn new(datastore: Arc<FakeDatastore>) -> Self {
        Self { datastore }
    }
}

#[async_trait]
impl SessionRepository for FakeSessionRepository {
    async fn create(&self, session: NeueSession) -> RepositoryResult<Versioned<Session>> {
        let id = SessionId(Uuid::new_v4());
        let erstellt = Versioned::init(Session::Renewable(RenewableSession::aus(id, session)));
        self.datastore.sessions().insert(id, erstellt.clone());
        Ok(erstellt)
    }

    async fn find_by_id(&self, id: SessionId) -> RepositoryResult<Versioned<Session>> {
        self.datastore
            .sessions()
            .get(&id)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn find_by_refresh_hash(
        &self,
        hash: &SecretTokenHash,
    ) -> RepositoryResult<Versioned<Session>> {
        self.datastore
            .sessions()
            .values()
            .find(|session| session.refresh_token_hash() == hash)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn find_by_benutzer(
        &self,
        benutzer_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Session>>> {
        Ok(self
            .datastore
            .sessions()
            .values()
            .filter(|session| session.benutzer_id() == benutzer_id)
            .cloned()
            .collect())
    }

    async fn update(&self, session: &mut Versioned<Session>) -> RepositoryResult<()> {
        let mut map = self.datastore.sessions();
        store_update(&mut map, session.id(), session)
    }

    async fn delete(&self, session: Versioned<Session>) -> RepositoryResult<()> {
        let mut map = self.datastore.sessions();
        store_delete(&mut map, session.id(), &session)
    }

    async fn delete_by_benutzer(&self, benutzer_id: BenutzerId) -> RepositoryResult<()> {
        self.datastore
            .sessions()
            .retain(|_, session| session.benutzer_id() != benutzer_id);
        Ok(())
    }
}

pub struct FakeEmailBestätigungRepository {
    datastore: Arc<FakeDatastore>,
}

impl FakeEmailBestätigungRepository {
    pub fn new(datastore: Arc<FakeDatastore>) -> Self {
        Self { datastore }
    }
}

#[async_trait]
impl EmailBestätigungRepository for FakeEmailBestätigungRepository {
    async fn create(
        &self,
        bestätigung: NeueBestätigung,
    ) -> RepositoryResult<Versioned<EmailBestätigung>> {
        let id = BestätigungId(Uuid::new_v4());
        let erstellt = Versioned::init(EmailBestätigung::neu(id, bestätigung));
        self.datastore.bestätigungen().insert(id, erstellt.clone());
        Ok(erstellt)
    }

    async fn find_by_hash(
        &self,
        hash: &SecretTokenHash,
    ) -> RepositoryResult<Versioned<EmailBestätigung>> {
        self.datastore
            .bestätigungen()
            .values()
            .find(|bestätigung| bestätigung.token_hash() == hash)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn find_by_id(
        &self,
        id: BestätigungId,
    ) -> RepositoryResult<Versioned<EmailBestätigung>> {
        self.datastore
            .bestätigungen()
            .get(&id)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn find_by_email(
        &self,
        email: &EmailAdresse,
    ) -> RepositoryResult<Vec<Versioned<EmailBestätigung>>> {
        Ok(self
            .datastore
            .bestätigungen()
            .values()
            .filter(|bestätigung| bestätigung.email() == email)
            .cloned()
            .collect())
    }

    async fn update(
        &self, bestätigung: &mut Versioned<EmailBestätigung>
    ) -> RepositoryResult<()> {
        let mut map = self.datastore.bestätigungen();
        store_update(&mut map, bestätigung.id(), bestätigung)
    }

    async fn delete(&self, bestätigung: Versioned<EmailBestätigung>) -> RepositoryResult<()> {
        let mut map = self.datastore.bestätigungen();
        store_delete(&mut map, bestätigung.id(), &bestätigung)
    }

    async fn delete_by_email(&self, email: &EmailAdresse) -> RepositoryResult<()> {
        self.datastore
            .bestätigungen()
            .retain(|_, bestätigung| bestätigung.email() != email);
        Ok(())
    }
}

pub struct FakePasswortWiederherstellungRepository {
    datastore: Arc<FakeDatastore>,
}

impl FakePasswortWiederherstellungRepository {
    pub fn new(datastore: Arc<FakeDatastore>) -> Self {
        Self { datastore }
    }
}

#[async_trait]
impl PasswortWiederherstellungRepository for FakePasswortWiederherstellungRepository {
    async fn create(
        &self,
        wiederherstellung: NeuePasswortWiederherstellung,
    ) -> RepositoryResult<Versioned<PasswortWiederherstellung>> {
        let id = PasswortWiederherstellungId(Uuid::new_v4());
        let erstellt = Versioned::init(PasswortWiederherstellung::neu(id, wiederherstellung));
        self.datastore
            .wiederherstellungen()
            .insert(id, erstellt.clone());
        Ok(erstellt)
    }

    async fn find_by_id(
        &self,
        id: PasswortWiederherstellungId,
    ) -> RepositoryResult<Versioned<PasswortWiederherstellung>> {
        self.datastore
            .wiederherstellungen()
            .get(&id)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn find_by_hash(
        &self,
        hash: &SecretTokenHash,
    ) -> RepositoryResult<Versioned<PasswortWiederherstellung>> {
        self.datastore
            .wiederherstellungen()
            .values()
            .find(|wiederherstellung| wiederherstellung.token_hash() == hash)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn find_by_konto(
        &self,
        konto_id: KontoId,
    ) -> RepositoryResult<Vec<Versioned<PasswortWiederherstellung>>> {
        Ok(self
            .datastore
            .wiederherstellungen()
            .values()
            .filter(|wiederherstellung| wiederherstellung.konto_id() == konto_id)
            .cloned()
            .collect())
    }

    async fn update(
        &self,
        wiederherstellung: &mut Versioned<PasswortWiederherstellung>,
    ) -> RepositoryResult<()> {
        let mut map = self.datastore.wiederherstellungen();
        store_update(&mut map, wiederherstellung.id(), wiederherstellung)
    }

    async fn delete(
        &self,
        wiederherstellung: Versioned<PasswortWiederherstellung>,
    ) -> RepositoryResult<()> {
        let mut map = self.datastore.wiederherstellungen();
        store_delete(&mut map, wiederherstellung.id(), &wiederherstellung)
    }

    async fn delete_by_konto(&self, konto_id: KontoId) -> RepositoryResult<()> {
        self.datastore
            .wiederherstellungen()
            .retain(|_, wiederherstellung| wiederherstellung.konto_id() != konto_id);
        Ok(())
    }
}

pub struct FakeBenachrichtigungRepository {
    datastore: Arc<FakeDatastore>,
}

impl FakeBenachrichtigungRepository {
    pub fn new(datastore: Arc<FakeDatastore>) -> Self {
        Self { datastore }
    }
}

#[async_trait]
impl BenachrichtigungRepository for FakeBenachrichtigungRepository {
    async fn create(
        &self,
        neu: NeueBenachrichtigung,
    ) -> RepositoryResult<Versioned<Benachrichtigung>> {
        let mut map = self.datastore.benachrichtigungen();
        if map.values().any(|existing| {
            existing.empfänger_id() == neu.empfänger_id && existing.dedupe_key() == &neu.dedupe_key
        }) {
            return Err(Report::new(RepositoryError::Conflict));
        }
        let id = BenachrichtigungId(Uuid::new_v4());
        let entity = Versioned::init(Benachrichtigung::neu(id, neu));
        map.insert(id, entity.clone());
        Ok(entity)
    }

    async fn find_by_id(
        &self,
        id: BenachrichtigungId,
    ) -> RepositoryResult<Versioned<Benachrichtigung>> {
        self.datastore
            .benachrichtigungen()
            .get(&id)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn find_by_empfänger(
        &self,
        empfänger_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Benachrichtigung>>> {
        let mut list: Vec<Versioned<Benachrichtigung>> = self
            .datastore
            .benachrichtigungen()
            .values()
            .filter(|n| n.empfänger_id() == empfänger_id)
            .cloned()
            .collect();
        list.sort_by(|a, b| {
            b.erstellt_am()
                .cmp(&a.erstellt_am())
                .then(b.id().cmp(&a.id()))
        });
        Ok(list)
    }

    async fn update(
        &self,
        benachrichtigung: &mut Versioned<Benachrichtigung>,
    ) -> RepositoryResult<()> {
        let mut map = self.datastore.benachrichtigungen();
        let Some(existing) = map.get(&benachrichtigung.id()) else {
            return Err(Report::new(RepositoryError::NotFound));
        };
        if existing.v() != benachrichtigung.v() {
            return Err(Report::new(RepositoryError::VersionMismatch {
                expected: benachrichtigung.v(),
                actual: Some(existing.v()),
            }));
        }
        benachrichtigung.increment();
        map.insert(benachrichtigung.id(), benachrichtigung.clone());
        Ok(())
    }

    async fn delete(&self, benachrichtigung: Versioned<Benachrichtigung>) -> RepositoryResult<()> {
        let mut map = self.datastore.benachrichtigungen();
        let Some(existing) = map.get(&benachrichtigung.id()) else {
            return Err(Report::new(RepositoryError::NotFound));
        };
        if existing.v() != benachrichtigung.v() {
            return Err(Report::new(RepositoryError::VersionMismatch {
                expected: benachrichtigung.v(),
                actual: Some(existing.v()),
            }));
        }
        map.remove(&benachrichtigung.id());
        Ok(())
    }

    async fn delete_by_benutzer(&self, empfänger_id: BenutzerId) -> RepositoryResult<()> {
        self.datastore
            .benachrichtigungen()
            .retain(|_, n| n.empfänger_id() != empfänger_id);
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeMailSender {
    gesendet: Mutex<Vec<Mail>>,
}

impl FakeMailSender {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sent(&self) -> Vec<Mail> {
        self.gesendet.lock().unwrap().clone()
    }

    pub fn single(&self) -> Mail {
        let gesendet = self.sent();
        assert_eq!(gesendet.len(), 1, "erwartet genau eine Mail");
        gesendet.into_iter().next().unwrap()
    }

    pub fn latest(&self) -> Mail {
        self.gesendet
            .lock()
            .unwrap()
            .last()
            .cloned()
            .expect("erwartet mindestens eine Mail")
    }

    pub fn clear(&self) {
        self.gesendet.lock().unwrap().clear();
    }
}

#[async_trait]
impl MailSender for FakeMailSender {
    async fn send(&self, mail: Mail) -> ResultReport<(), MailError> {
        self.gesendet.lock().unwrap().push(mail);
        Ok(())
    }

    async fn prüfe_verbindung(&self) -> ResultReport<(), MailError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use friendarazzi_core::{
        domain::{GridSize, InviteCode, NonEmptyString, VisibilityMode},
        ports::RepositoryError,
    };

    use super::*;

    fn sample_time() -> chrono::DateTime<chrono::Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).expect("valid timestamp")
    }

    fn sample_benutzer(id: BenutzerId, email: &str) -> Versioned<Benutzer> {
        Versioned::init(Benutzer::neu(
            id,
            NeuerBenutzer::neu(
                EmailAdresse::normalisiert(email).unwrap(),
                NonEmptyString::neu("Tester").unwrap(),
                sample_time(),
            ),
        ))
    }

    fn sample_game(id: GameId, host: BenutzerId, code: &str, title: &str) -> Versioned<Game> {
        Versioned::init(Game::new(
            id,
            NewGame {
                host_user_id: host,
                host_watching: true,
                invite_code: InviteCode::new(code).unwrap(),
                title: NonEmptyString::neu(title).unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
                created_at: sample_time(),
            },
        ))
    }

    #[test]
    fn merge_applies_disjoint_concurrent_changes() {
        let backing = FakeDatastore::new();
        let ref_a = backing.clone();
        let ref_b = backing.clone();
        let tx_a = ref_a.clone();
        let tx_b = ref_b.clone();

        let id_a = BenutzerId(Uuid::new_v4());
        let id_b = BenutzerId(Uuid::new_v4());
        tx_a.benutzer()
            .insert(id_a, sample_benutzer(id_a, "a@example.com"));
        tx_b.benutzer()
            .insert(id_b, sample_benutzer(id_b, "b@example.com"));

        FakeDatastore::merge(&backing, &ref_a, &tx_a).unwrap();
        FakeDatastore::merge(&backing, &ref_b, &tx_b).unwrap();

        let stored = backing.benutzer();
        assert!(stored.contains_key(&id_a));
        assert!(stored.contains_key(&id_b));
    }

    #[test]
    fn merge_conflict_is_atomic_across_aggregates() {
        let user_id = BenutzerId(Uuid::new_v4());
        let initial_user = sample_benutzer(user_id, "race@example.com");

        let backing = FakeDatastore::new();
        backing.benutzer().insert(user_id, initial_user);

        // Concurrent tx1 advances `user_id` from v0 -> v1 in backing.
        let reference = backing.clone();
        let mut bumped_user = backing.benutzer().get(&user_id).unwrap().clone();
        bumped_user.increment();
        backing.benutzer().insert(user_id, bumped_user);

        // tx2 creates a new game AND updates `user_id` from stale v0 -> v1.
        let tx = reference.clone();
        let game_id = GameId(Uuid::new_v4());
        tx.games().insert(
            game_id,
            sample_game(game_id, user_id, "ABCD23", "Race Game"),
        );
        let mut stale_update = tx.benutzer().get(&user_id).unwrap().clone();
        stale_update.increment();
        tx.benutzer().insert(user_id, stale_update);

        let Err(err) = FakeDatastore::merge(&backing, &reference, &tx) else {
            panic!("expected version mismatch");
        };
        assert!(matches!(
            err,
            RepositoryError::VersionMismatch {
                expected: 0,
                actual: Some(1),
            }
        ));
        // Game creation in an earlier aggregate must NOT be persisted when benutzer conflicts.
        assert!(!backing.games().contains_key(&game_id));
        assert_eq!(backing.benutzer().get(&user_id).unwrap().v(), 1);
    }
}
