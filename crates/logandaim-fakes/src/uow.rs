use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use logandaim_core::{
    ports::{
        BenutzerRepository, BoardRepository, CaptureRepository, DeckRepository,
        EmailBestätigungRepository, GameRepository, KontoRepository,
        PasswortWiederherstellungRepository, RepositoryResult, SessionRepository,
        UserProfileRepository,
    },
    uow::{UnitOfWorkImpl, UnitOfWorkProvider},
};

use crate::repository::{
    FakeBenutzerRepository, FakeBoardRepository, FakeCaptureRepository, FakeDatastore,
    FakeDeckRepository, FakeEmailBestätigungRepository, FakeGameRepository, FakeKontoRepository,
    FakePasswortWiederherstellungRepository, FakeSessionRepository, FakeUserProfileRepository,
};

pub struct FakeUnitOfWorkProvider {
    pub log: Arc<Mutex<Vec<UoWEvent>>>,
    pub datastore: Arc<FakeDatastore>,
}

impl FakeUnitOfWorkProvider {
    pub fn new(datastore: Arc<FakeDatastore>) -> Self {
        Self {
            log: Arc::new(Mutex::new(Vec::new())),
            datastore,
        }
    }

    pub fn empty() -> Self {
        Self::new(Arc::new(FakeDatastore::new()))
    }
}

impl Default for FakeUnitOfWorkProvider {
    fn default() -> Self {
        Self::empty()
    }
}

#[async_trait]
impl UnitOfWorkProvider for FakeUnitOfWorkProvider {
    async fn begin(&self) -> RepositoryResult<Box<dyn UnitOfWorkImpl>> {
        self.log.lock().unwrap().push(UoWEvent::Begin);
        Ok(Box::new(FakeUnitOfWork::new(
            Arc::clone(&self.log),
            Arc::clone(&self.datastore),
        )))
    }
}

pub struct FakeUnitOfWork {
    log: Arc<Mutex<Vec<UoWEvent>>>,
    backing_datastore: Arc<FakeDatastore>,
    transaction_datastore: Arc<FakeDatastore>,
    profiles: FakeUserProfileRepository,
    games: FakeGameRepository,
    boards: FakeBoardRepository,
    captures: FakeCaptureRepository,
    decks: FakeDeckRepository,
    benutzer: FakeBenutzerRepository,
    konten: FakeKontoRepository,
    sessions: FakeSessionRepository,
    bestätigungen: FakeEmailBestätigungRepository,
    wiederherstellungen: FakePasswortWiederherstellungRepository,
}

impl FakeUnitOfWork {
    pub fn new(log: Arc<Mutex<Vec<UoWEvent>>>, datastore: Arc<FakeDatastore>) -> Self {
        let transaction_datastore = Arc::new(FakeDatastore::clone(datastore.as_ref()));
        Self {
            log,
            backing_datastore: datastore,
            profiles: FakeUserProfileRepository::new(Arc::clone(&transaction_datastore)),
            games: FakeGameRepository::new(Arc::clone(&transaction_datastore)),
            boards: FakeBoardRepository::new(Arc::clone(&transaction_datastore)),
            captures: FakeCaptureRepository::new(Arc::clone(&transaction_datastore)),
            decks: FakeDeckRepository::new(Arc::clone(&transaction_datastore)),
            benutzer: FakeBenutzerRepository::new(Arc::clone(&transaction_datastore)),
            konten: FakeKontoRepository::new(Arc::clone(&transaction_datastore)),
            sessions: FakeSessionRepository::new(Arc::clone(&transaction_datastore)),
            bestätigungen: FakeEmailBestätigungRepository::new(
                Arc::clone(&transaction_datastore),
            ),
            wiederherstellungen: FakePasswortWiederherstellungRepository::new(
                transaction_datastore.clone(),
            ),
            transaction_datastore,
        }
    }
}

#[async_trait]
impl UnitOfWorkImpl for FakeUnitOfWork {
    async fn commit(self: Box<Self>) -> RepositoryResult<()> {
        self.log.lock().unwrap().push(UoWEvent::Commit);
        self.backing_datastore
            .replace_with(self.transaction_datastore.as_ref());
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> RepositoryResult<()> {
        self.log.lock().unwrap().push(UoWEvent::Rollback);
        Ok(())
    }

    fn profiles(&self) -> &dyn UserProfileRepository {
        &self.profiles
    }

    fn games(&self) -> &dyn GameRepository {
        &self.games
    }

    fn boards(&self) -> &dyn BoardRepository {
        &self.boards
    }

    fn captures(&self) -> &dyn CaptureRepository {
        &self.captures
    }

    fn decks(&self) -> &dyn DeckRepository {
        &self.decks
    }

    fn benutzer(&self) -> &dyn BenutzerRepository {
        &self.benutzer
    }

    fn konten(&self) -> &dyn KontoRepository {
        &self.konten
    }

    fn sessions(&self) -> &dyn SessionRepository {
        &self.sessions
    }

    fn bestätigungen(&self) -> &dyn EmailBestätigungRepository {
        &self.bestätigungen
    }

    fn wiederherstellungen(&self) -> &dyn PasswortWiederherstellungRepository {
        &self.wiederherstellungen
    }
}

#[derive(Debug)]
pub enum UoWEvent {
    Begin,
    Commit,
    Rollback,
    Error(String),
}
