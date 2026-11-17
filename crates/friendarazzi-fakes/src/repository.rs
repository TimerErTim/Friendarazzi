use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use error_stack::Report;
use rustc_hash::FxHashMap;
use uuid::Uuid;

use friendarazzi_core::{
    ResultReport,
    application::uow::Versioned,
    domain::{
        Board, BoardId, BoardTile, BoardTileId, Capture, CaptureId, EmailAdresse, Game, GameId,
        GamePlayer, InviteCode, NewCapture, NewGame, NewUserProfile, PromptSubmission, StarterDeck,
        UserProfile, builtin_starter_decks,
        konto::{
            Benutzer, BenutzerId, BestätigungId, EmailBestätigung, Konto, KontoId, NeueBestätigung,
            NeuePasswortWiederherstellung, NeueSession, NeuerBenutzer, NeuesKonto,
            PasswortWiederherstellung, PasswortWiederherstellungId, RenewableSession,
            SecretTokenHash, Session, SessionId,
        },
    },
    ports::{
        BenutzerRepository, BoardRepository, CaptureRepository, DeckRepository,
        EmailBestätigungRepository, GameRepository, KontoRepository, Mail, MailError, MailSender,
        PasswortWiederherstellungRepository, RepositoryError, RepositoryResult, SessionRepository,
        UserProfileRepository,
    },
};

/// In-memory backing store: one map per aggregate, all on the same snapshot.
pub struct FakeDatastore {
    pub profiles: Mutex<FxHashMap<BenutzerId, Versioned<UserProfile>>>,
    pub games: Mutex<FxHashMap<GameId, Versioned<Game>>>,
    pub game_players: Mutex<Vec<GamePlayer>>,
    pub prompt_submissions: Mutex<Vec<PromptSubmission>>,
    pub boards: Mutex<FxHashMap<BoardId, Versioned<Board>>>,
    pub board_tiles: Mutex<FxHashMap<BoardTileId, BoardTile>>,
    pub captures: Mutex<FxHashMap<CaptureId, Versioned<Capture>>>,
    pub decks: Mutex<Vec<StarterDeck>>,
    pub benutzer: Mutex<FxHashMap<BenutzerId, Versioned<Benutzer>>>,
    pub konten: Mutex<FxHashMap<KontoId, Versioned<Konto>>>,
    pub sessions: Mutex<FxHashMap<SessionId, Versioned<Session>>>,
    pub bestätigungen: Mutex<FxHashMap<BestätigungId, Versioned<EmailBestätigung>>>,
    pub wiederherstellungen:
        Mutex<FxHashMap<PasswortWiederherstellungId, Versioned<PasswortWiederherstellung>>>,
}

impl Clone for FakeDatastore {
    fn clone(&self) -> Self {
        Self {
            profiles: Mutex::new(self.profiles().clone()),
            games: Mutex::new(self.games().clone()),
            game_players: Mutex::new(self.game_players().clone()),
            prompt_submissions: Mutex::new(self.prompt_submissions().clone()),
            boards: Mutex::new(self.boards().clone()),
            board_tiles: Mutex::new(self.board_tiles().clone()),
            captures: Mutex::new(self.captures().clone()),
            decks: Mutex::new(self.decks().clone()),
            benutzer: Mutex::new(self.benutzer().clone()),
            konten: Mutex::new(self.konten().clone()),
            sessions: Mutex::new(self.sessions().clone()),
            bestätigungen: Mutex::new(self.bestätigungen().clone()),
            wiederherstellungen: Mutex::new(self.wiederherstellungen().clone()),
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
            profiles: Mutex::new(FxHashMap::default()),
            games: Mutex::new(FxHashMap::default()),
            game_players: Mutex::new(Vec::new()),
            prompt_submissions: Mutex::new(Vec::new()),
            boards: Mutex::new(FxHashMap::default()),
            board_tiles: Mutex::new(FxHashMap::default()),
            captures: Mutex::new(FxHashMap::default()),
            decks: Mutex::new(builtin_starter_decks()),
            benutzer: Mutex::new(FxHashMap::default()),
            konten: Mutex::new(FxHashMap::default()),
            sessions: Mutex::new(FxHashMap::default()),
            bestätigungen: Mutex::new(FxHashMap::default()),
            wiederherstellungen: Mutex::new(FxHashMap::default()),
        }
    }

    pub fn profiles(&self) -> MutexGuard<'_, FxHashMap<BenutzerId, Versioned<UserProfile>>> {
        self.profiles.lock().unwrap()
    }

    pub fn games(&self) -> MutexGuard<'_, FxHashMap<GameId, Versioned<Game>>> {
        self.games.lock().unwrap()
    }

    pub fn game_players(&self) -> MutexGuard<'_, Vec<GamePlayer>> {
        self.game_players.lock().unwrap()
    }

    pub fn prompt_submissions(&self) -> MutexGuard<'_, Vec<PromptSubmission>> {
        self.prompt_submissions.lock().unwrap()
    }

    pub fn boards(&self) -> MutexGuard<'_, FxHashMap<BoardId, Versioned<Board>>> {
        self.boards.lock().unwrap()
    }

    pub fn board_tiles(&self) -> MutexGuard<'_, FxHashMap<BoardTileId, BoardTile>> {
        self.board_tiles.lock().unwrap()
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

    pub fn replace_with(&self, other: &FakeDatastore) {
        *self.profiles() = other.profiles().clone();
        *self.games() = other.games().clone();
        *self.game_players() = other.game_players().clone();
        *self.prompt_submissions() = other.prompt_submissions().clone();
        *self.boards() = other.boards().clone();
        *self.board_tiles() = other.board_tiles().clone();
        *self.captures() = other.captures().clone();
        *self.decks() = other.decks().clone();
        *self.benutzer() = other.benutzer().clone();
        *self.konten() = other.konten().clone();
        *self.sessions() = other.sessions().clone();
        *self.bestätigungen() = other.bestätigungen().clone();
        *self.wiederherstellungen() = other.wiederherstellungen().clone();
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

pub struct FakeUserProfileRepository {
    datastore: Arc<FakeDatastore>,
}

impl FakeUserProfileRepository {
    pub fn new(datastore: Arc<FakeDatastore>) -> Self {
        Self { datastore }
    }
}

#[async_trait]
impl UserProfileRepository for FakeUserProfileRepository {
    async fn create(&self, profile: NewUserProfile) -> RepositoryResult<Versioned<UserProfile>> {
        let mut profiles = self.datastore.profiles();
        if profiles.contains_key(&profile.user_id) {
            return Err(Report::new(RepositoryError::Conflict));
        }
        let user_id = profile.user_id;
        let created = Versioned::init(UserProfile::new(profile));
        profiles.insert(user_id, created.clone());
        Ok(created)
    }

    async fn find_by_user_id(
        &self,
        user_id: BenutzerId,
    ) -> RepositoryResult<Versioned<UserProfile>> {
        self.datastore
            .profiles()
            .get(&user_id)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn find_by_user_ids(
        &self,
        user_ids: &[BenutzerId],
    ) -> RepositoryResult<Vec<Versioned<UserProfile>>> {
        let profiles = self.datastore.profiles();
        Ok(user_ids
            .iter()
            .filter_map(|id| profiles.get(id).cloned())
            .collect())
    }

    async fn find_by_username(&self, username: &str) -> RepositoryResult<Versioned<UserProfile>> {
        self.datastore
            .profiles()
            .values()
            .find(|p| p.username().as_str().eq_ignore_ascii_case(username))
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn update(&self, profile: &mut Versioned<UserProfile>) -> RepositoryResult<()> {
        let mut map = self.datastore.profiles();
        store_update(&mut map, profile.user_id(), profile)
    }

    async fn delete_by_user_id(&self, user_id: BenutzerId) -> RepositoryResult<()> {
        self.datastore.profiles().remove(&user_id);
        Ok(())
    }
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

    async fn find_for_user(&self, user_id: BenutzerId) -> RepositoryResult<Vec<Versioned<Game>>> {
        let game_ids: Vec<GameId> = self
            .datastore
            .game_players()
            .iter()
            .filter(|p| p.user_id == user_id)
            .map(|p| p.game_id)
            .collect();
        let games = self.datastore.games();
        let mut out: Vec<_> = game_ids
            .into_iter()
            .filter_map(|id| games.get(&id).cloned())
            .collect();
        out.sort_by_key(|g| std::cmp::Reverse(g.created_at()));
        Ok(out)
    }

    async fn update(&self, game: &mut Versioned<Game>) -> RepositoryResult<()> {
        let mut map = self.datastore.games();
        store_update(&mut map, game.id(), game)
    }

    async fn add_player(&self, player: GamePlayer) -> RepositoryResult<()> {
        let mut players = self.datastore.game_players();
        if players
            .iter()
            .any(|p| p.game_id == player.game_id && p.user_id == player.user_id)
        {
            return Err(Report::new(RepositoryError::Conflict));
        }
        players.push(player);
        Ok(())
    }

    async fn find_players(&self, game_id: GameId) -> RepositoryResult<Vec<GamePlayer>> {
        let mut out: Vec<_> = self
            .datastore
            .game_players()
            .iter()
            .filter(|p| p.game_id == game_id)
            .cloned()
            .collect();
        out.sort_by(|a, b| {
            a.joined_at
                .cmp(&b.joined_at)
                .then(a.user_id.cmp(&b.user_id))
        });
        Ok(out)
    }

    async fn set_player_draft_locked(
        &self,
        game_id: GameId,
        user_id: BenutzerId,
        locked: bool,
    ) -> RepositoryResult<()> {
        let mut players = self.datastore.game_players();
        let Some(p) = players
            .iter_mut()
            .find(|p| p.game_id == game_id && p.user_id == user_id)
        else {
            return Err(Report::new(RepositoryError::NotFound));
        };
        p.draft_locked = locked;
        Ok(())
    }

    async fn replace_author_prompts(
        &self,
        game_id: GameId,
        author_user_id: BenutzerId,
        prompts: Vec<PromptSubmission>,
    ) -> RepositoryResult<()> {
        let mut all = self.datastore.prompt_submissions();
        all.retain(|p| !(p.game_id == game_id && p.author_user_id == author_user_id));
        all.extend(prompts);
        Ok(())
    }

    async fn find_prompts_by_game(
        &self,
        game_id: GameId,
    ) -> RepositoryResult<Vec<PromptSubmission>> {
        Ok(self
            .datastore
            .prompt_submissions()
            .iter()
            .filter(|p| p.game_id == game_id)
            .cloned()
            .collect())
    }

    async fn find_prompts_by_author(
        &self,
        game_id: GameId,
        author_user_id: BenutzerId,
    ) -> RepositoryResult<Vec<PromptSubmission>> {
        Ok(self
            .datastore
            .prompt_submissions()
            .iter()
            .filter(|p| p.game_id == game_id && p.author_user_id == author_user_id)
            .cloned()
            .collect())
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
    async fn create_board_with_tiles(
        &self,
        board: Board,
        tiles: Vec<BoardTile>,
    ) -> RepositoryResult<()> {
        let id = board.id();
        self.datastore.boards().insert(id, Versioned::init(board));
        let mut tile_map = self.datastore.board_tiles();
        for tile in tiles {
            tile_map.insert(tile.id(), tile);
        }
        Ok(())
    }

    async fn find_boards_by_game(
        &self,
        game_id: GameId,
    ) -> RepositoryResult<Vec<Versioned<Board>>> {
        Ok(self
            .datastore
            .boards()
            .values()
            .filter(|b| b.game_id() == game_id)
            .cloned()
            .collect())
    }

    async fn find_board_by_game_and_owner(
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

    async fn find_tiles_by_board(&self, board_id: BoardId) -> RepositoryResult<Vec<BoardTile>> {
        let mut out: Vec<_> = self
            .datastore
            .board_tiles()
            .values()
            .filter(|t| t.board_id() == board_id)
            .cloned()
            .collect();
        out.sort_by_key(|t| (t.row(), t.col()));
        Ok(out)
    }

    async fn find_tiles_by_game(&self, game_id: GameId) -> RepositoryResult<Vec<BoardTile>> {
        let board_ids: Vec<BoardId> = self
            .datastore
            .boards()
            .values()
            .filter(|b| b.game_id() == game_id)
            .map(|b| b.id())
            .collect();
        let mut out: Vec<_> = self
            .datastore
            .board_tiles()
            .values()
            .filter(|t| board_ids.contains(&t.board_id()))
            .cloned()
            .collect();
        out.sort_by_key(|t| (t.row(), t.col()));
        Ok(out)
    }

    async fn find_tile_by_id(&self, tile_id: BoardTileId) -> RepositoryResult<BoardTile> {
        self.datastore
            .board_tiles()
            .get(&tile_id)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn update_board(&self, board: &mut Versioned<Board>) -> RepositoryResult<()> {
        let mut map = self.datastore.boards();
        store_update(&mut map, board.id(), board)
    }

    async fn attach_tile_capture(
        &self,
        tile_id: BoardTileId,
        capture_id: CaptureId,
    ) -> RepositoryResult<()> {
        let mut map = self.datastore.board_tiles();
        let tile = map
            .get_mut(&tile_id)
            .ok_or_else(|| Report::new(RepositoryError::NotFound))?;
        tile.attach_capture(capture_id);
        Ok(())
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
        let id = CaptureId(Uuid::new_v4());
        let created = Versioned::init(Capture::new(id, capture));
        self.datastore.captures().insert(id, created.clone());
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
            .filter(|c| c.target_user_id() == Some(user_id))
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
    async fn list_all(&self) -> RepositoryResult<Vec<StarterDeck>> {
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
        let id = BenutzerId(Uuid::new_v4());
        let erstellt = Versioned::init(Benutzer::neu(id, benutzer));
        self.datastore.benutzer().insert(id, erstellt.clone());
        Ok(erstellt)
    }

    async fn find_by_id(&self, id: BenutzerId) -> RepositoryResult<Versioned<Benutzer>> {
        self.datastore
            .benutzer()
            .get(&id)
            .cloned()
            .ok_or_else(|| Report::new(RepositoryError::NotFound))
    }

    async fn update(&self, benutzer: &mut Versioned<Benutzer>) -> RepositoryResult<()> {
        let mut map = self.datastore.benutzer();
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
        if konten
            .values()
            .any(|vorhanden| vorhanden.email() == konto.email())
        {
            return Err(Report::new(RepositoryError::Conflict));
        }
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

    async fn find_by_email(&self, email: &EmailAdresse) -> RepositoryResult<Versioned<Konto>> {
        self.datastore
            .konten()
            .values()
            .find(|konto| konto.email() == email)
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
