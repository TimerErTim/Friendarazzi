use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use error_stack::Report;
use friendarazzi_core::{
    ports::{
        BenachrichtigungRepository, BenutzerRepository, BoardRepository, CaptureRepository,
        DeckRepository, EmailBestätigungRepository, GameRepository, KontoRepository,
        PasswortWiederherstellungRepository, RepositoryResult, SessionRepository,
    },
    uow::{UnitOfWorkImpl, UnitOfWorkProvider},
};

use crate::repository::{
    FakeBenachrichtigungRepository, FakeBenutzerRepository, FakeBoardRepository,
    FakeCaptureRepository, FakeDatastore, FakeDeckRepository, FakeEmailBestätigungRepository,
    FakeGameRepository, FakeKontoRepository, FakePasswortWiederherstellungRepository,
    FakeSessionRepository,
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
    snapshotted_datastore: FakeDatastore,
    transaction_datastore: Arc<FakeDatastore>,
    games: FakeGameRepository,
    boards: FakeBoardRepository,
    captures: FakeCaptureRepository,
    decks: FakeDeckRepository,
    benutzer: FakeBenutzerRepository,
    konto: FakeKontoRepository,
    sessions: FakeSessionRepository,
    bestätigungen: FakeEmailBestätigungRepository,
    wiederherstellungen: FakePasswortWiederherstellungRepository,
    benachrichtigungen: FakeBenachrichtigungRepository,
}

impl FakeUnitOfWork {
    pub fn new(log: Arc<Mutex<Vec<UoWEvent>>>, datastore: Arc<FakeDatastore>) -> Self {
        let backing_datastore = datastore;
        let snapshotted_datastore = FakeDatastore::clone(&backing_datastore);
        let transaction_datastore = Arc::new(snapshotted_datastore.clone());
        Self {
            log,
            backing_datastore,
            games: FakeGameRepository::new(Arc::clone(&transaction_datastore)),
            boards: FakeBoardRepository::new(Arc::clone(&transaction_datastore)),
            captures: FakeCaptureRepository::new(Arc::clone(&transaction_datastore)),
            decks: FakeDeckRepository::new(Arc::clone(&transaction_datastore)),
            benutzer: FakeBenutzerRepository::new(Arc::clone(&transaction_datastore)),
            konto: FakeKontoRepository::new(Arc::clone(&transaction_datastore)),
            sessions: FakeSessionRepository::new(Arc::clone(&transaction_datastore)),
            bestätigungen: FakeEmailBestätigungRepository::new(
                Arc::clone(&transaction_datastore),
            ),
            wiederherstellungen: FakePasswortWiederherstellungRepository::new(
                transaction_datastore.clone(),
            ),
            benachrichtigungen: FakeBenachrichtigungRepository::new(transaction_datastore.clone()),
            snapshotted_datastore,
            transaction_datastore,
        }
    }
}

#[async_trait]
impl UnitOfWorkImpl for FakeUnitOfWork {
    async fn commit(mut self: Box<Self>) -> RepositoryResult<()> {
        self.log.lock().unwrap().push(UoWEvent::Commit);

        let new_snapshot = FakeDatastore::merge(
            &self.backing_datastore,
            &self.snapshotted_datastore,
            &self.transaction_datastore,
        )
        .map_err(Report::new)
        .inspect_err(|e| {
            self.log
                .lock()
                .unwrap()
                .push(UoWEvent::Error(format!("{e:?}")));
        })?;

        self.transaction_datastore.replace_with(&new_snapshot);
        self.snapshotted_datastore = new_snapshot;

        Ok(())
    }

    async fn rollback(self: Box<Self>) -> RepositoryResult<()> {
        self.log.lock().unwrap().push(UoWEvent::Rollback);
        Ok(())
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
        &self.konto
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

    fn benachrichtigungen(&self) -> &dyn BenachrichtigungRepository {
        &self.benachrichtigungen
    }
}

#[derive(Debug)]
pub enum UoWEvent {
    Begin,
    Commit,
    Rollback,
    Error(String),
}
