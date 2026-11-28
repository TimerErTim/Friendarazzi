use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use error_stack::Report;
use friendarazzi_core::{
    App,
    adapters::BlockingEventQueue,
    application::{
        AuthContext, IdentityProviderConfiguration,
        uow::{UnitOfWorkImpl, UnitOfWorkProvider, Versioned},
    },
    domain::{
        Benachrichtigung, BenachrichtigungArt, BenachrichtigungId, BenutzerId, CaptureId,
        EmailAdresse, GameId, GridSize, NeueBenachrichtigung, NeuerBenutzer, NonEmptyString,
        PromptSource, PromptText, READY_GRACE_PERIOD_SECONDS, VisibilityMode, konto::SessionId,
        target_quotas_for_author,
    },
    ports::{
        BenachrichtigungRepository, BenutzerRepository, BoardRepository, CaptureRepository, Clock,
        DeckRepository, EmailBestätigungRepository, GameRepository, KontoRepository,
        PasswortWiederherstellungRepository, RepositoryError, RepositoryResult, SessionRepository,
    },
    service::{
        CaptureUploaded, CreateGame, DraftPromptInput, Event, JoinGame, LockDraftPrompts,
        SaveDraftPrompts, StartDrafting, StartReadyGames, SubmitCapture, handle_event,
    },
};
use friendarazzi_fakes::{
    FakeBenachrichtigungRepository, FakeBenutzerRepository, FakeBoardRepository,
    FakeCaptureRepository, FakeDatastore, FakeDeckRepository, FakeEmailBestätigungRepository,
    FakeGameRepository, FakeKontoRepository, FakeMailSender,
    FakePasswortWiederherstellungRepository, FakeSessionRepository, FakeUnitOfWorkProvider,
    FixedClock, FixedRng, RecordingEventQueue, png_bytes_for_test,
};
use uuid::Uuid;

struct FaultyBenachrichtigungRepo {
    inner: FakeBenachrichtigungRepository,
    fail_for_user: Arc<Mutex<Option<BenutzerId>>>,
}

#[async_trait]
impl BenachrichtigungRepository for FaultyBenachrichtigungRepo {
    async fn create(
        &self,
        neu: NeueBenachrichtigung,
    ) -> RepositoryResult<Versioned<Benachrichtigung>> {
        if *self.fail_for_user.lock().unwrap() == Some(neu.empfänger_id) {
            return Err(Report::new(RepositoryError::OperationFailed));
        }
        self.inner.create(neu).await
    }

    async fn find_by_id(
        &self,
        id: BenachrichtigungId,
    ) -> RepositoryResult<Versioned<Benachrichtigung>> {
        self.inner.find_by_id(id).await
    }

    async fn find_by_empfänger(
        &self,
        empfänger_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Benachrichtigung>>> {
        self.inner.find_by_empfänger(empfänger_id).await
    }

    async fn find_by_abbestell_token(
        &self,
        token_hash: &friendarazzi_core::domain::konto::SecretTokenHash,
    ) -> RepositoryResult<Versioned<Benachrichtigung>> {
        self.inner.find_by_abbestell_token(token_hash).await
    }

    async fn update(
        &self,
        benachrichtigung: &mut Versioned<Benachrichtigung>,
    ) -> RepositoryResult<()> {
        self.inner.update(benachrichtigung).await
    }

    async fn delete(&self, benachrichtigung: Versioned<Benachrichtigung>) -> RepositoryResult<()> {
        self.inner.delete(benachrichtigung).await
    }

    async fn delete_by_empfänger(&self, empfänger_id: BenutzerId) -> RepositoryResult<()> {
        self.inner.delete_by_empfänger(empfänger_id).await
    }
}

struct FaultyUow {
    backing: Arc<FakeDatastore>,
    snapshotted: FakeDatastore,
    tx: Arc<FakeDatastore>,
    games: FakeGameRepository,
    boards: FakeBoardRepository,
    captures: FakeCaptureRepository,
    decks: FakeDeckRepository,
    benutzer: FakeBenutzerRepository,
    konten: FakeKontoRepository,
    sessions: FakeSessionRepository,
    bestätigungen: FakeEmailBestätigungRepository,
    wiederherstellungen: FakePasswortWiederherstellungRepository,
    benachrichtigungen: FaultyBenachrichtigungRepo,
}

#[async_trait]
impl UnitOfWorkImpl for FaultyUow {
    async fn commit(mut self: Box<Self>) -> RepositoryResult<()> {
        let new_snapshot = FakeDatastore::merge(&self.backing, &self.snapshotted, &self.tx)
            .map_err(Report::new)?;
        self.tx.replace_with(&new_snapshot);
        self.snapshotted = new_snapshot;
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> RepositoryResult<()> {
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
    fn benachrichtigungen(&self) -> &dyn BenachrichtigungRepository {
        &self.benachrichtigungen
    }
    fn external_identities(&self) -> &dyn friendarazzi_core::ports::ExternalIdentityRepository {
        unimplemented!()
    }
    fn oidc_auth_states(&self) -> &dyn friendarazzi_core::ports::OidcAuthStateRepository {
        unimplemented!()
    }
    fn oidc_exchange_codes(&self) -> &dyn friendarazzi_core::ports::OidcExchangeCodeRepository {
        unimplemented!()
    }
}

struct FaultyUowProvider {
    datastore: Arc<FakeDatastore>,
    fail_for_user: Arc<Mutex<Option<BenutzerId>>>,
}

#[async_trait]
impl UnitOfWorkProvider for FaultyUowProvider {
    async fn begin(&self) -> RepositoryResult<Box<dyn UnitOfWorkImpl>> {
        let snapshotted = FakeDatastore::clone(&self.datastore);
        let tx = Arc::new(snapshotted.clone());
        Ok(Box::new(FaultyUow {
            backing: Arc::clone(&self.datastore),
            snapshotted,
            games: FakeGameRepository::new(Arc::clone(&tx)),
            boards: FakeBoardRepository::new(Arc::clone(&tx)),
            captures: FakeCaptureRepository::new(Arc::clone(&tx)),
            decks: FakeDeckRepository::new(Arc::clone(&tx)),
            benutzer: FakeBenutzerRepository::new(Arc::clone(&tx)),
            konten: FakeKontoRepository::new(Arc::clone(&tx)),
            sessions: FakeSessionRepository::new(Arc::clone(&tx)),
            bestätigungen: FakeEmailBestätigungRepository::new(Arc::clone(&tx)),
            wiederherstellungen: FakePasswortWiederherstellungRepository::new(Arc::clone(&tx)),
            benachrichtigungen: FaultyBenachrichtigungRepo {
                inner: FakeBenachrichtigungRepository::new(Arc::clone(&tx)),
                fail_for_user: Arc::clone(&self.fail_for_user),
            },
            tx,
        }))
    }
}

fn sample_event() -> CaptureUploaded {
    CaptureUploaded {
        game_id: GameId(Uuid::new_v4()),
        capture_id: CaptureId(Uuid::new_v4()),
        uploader_id: BenutzerId(Uuid::new_v4()),
        target_id: BenutzerId(Uuid::new_v4()),
        occurred_at: Utc::now(),
    }
}

#[test_log::test(pollster::test)]
async fn staged_events_are_only_dispatched_after_execution_succeeds() {
    let recorder = Arc::new(RecordingEventQueue::new());
    let app = App::builder()
        .uow_provider(Box::new(FakeUnitOfWorkProvider::empty()))
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .event_queue(Arc::clone(&recorder))
        .build();

    let dummy = sample_event();

    let err_res = app
        .execute_fn::<_, (), RepositoryError>(AuthContext::Anonymous, {
            let ev = dummy.clone();
            async move |ctx| {
                let uow = ctx.enter().await?;
                ctx.events().submit(ev);
                uow.rollback().await?;
                Err(Report::new(RepositoryError::Conflict))
            }
        })
        .await;
    assert!(err_res.is_err());
    assert_eq!(recorder.total_events(), 0);

    app.execute_fn::<_, (), RepositoryError>(AuthContext::Anonymous, {
        let ev = dummy.clone();
        async move |ctx| {
            let uow = ctx.enter().await?;
            ctx.events().submit(ev);
            uow.commit().await
        }
    })
    .await
    .unwrap();
    assert_eq!(recorder.total_events(), 1);
}

#[test_log::test(pollster::test)]
async fn blocking_event_queue_runs_after_uow_commit_without_deadlock() {
    let app = App::builder()
        .uow_provider(Box::new(FakeUnitOfWorkProvider::empty()))
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .event_queue(BlockingEventQueue::from_app)
        .build();

    let ev = sample_event();
    app.execute_fn::<_, (), RepositoryError>(AuthContext::Anonymous, async move |ctx| {
        let uow = ctx.enter().await?;
        ctx.events().submit(ev);
        uow.commit().await
    })
    .await
    .unwrap();
}

#[test_log::test(pollster::test)]
async fn default_event_queue_executes_submitted_events() {
    let app = App::builder()
        .uow_provider(Box::new(FakeUnitOfWorkProvider::empty()))
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .build();

    let ev = sample_event();
    handle_event(&app, Event::from(ev.clone())).await.unwrap();

    app.execute_fn::<_, (), RepositoryError>(AuthContext::Anonymous, async move |ctx| {
        let uow = ctx.enter().await?;
        ctx.events().submit(ev);
        uow.commit().await
    })
    .await
    .unwrap();

    let drop_start = std::time::Instant::now();
    drop(app);
    assert!(drop_start.elapsed() < std::time::Duration::from_secs(1));
}

#[test_log::test(pollster::test)]
async fn per_user_uow_split_isolates_failures_across_all_game_events() {
    let fail_for_user = Arc::new(Mutex::new(None));
    let provider = FaultyUowProvider {
        datastore: Arc::new(FakeDatastore::new()),
        fail_for_user: Arc::clone(&fail_for_user),
    };
    let t0 = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
    let clock = Arc::new(FixedClock::new(t0));
    let mail = Arc::new(FakeMailSender::new());
    let recorder = Arc::new(RecordingEventQueue::new());

    let app = App::builder()
        .uow_provider(Box::new(provider))
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .clock(clock.clone() as Arc<dyn Clock>)
        .rng(Arc::new(FixedRng::default()))
        .mail_sender(mail.clone())
        .event_queue(Arc::clone(&recorder))
        .build();

    let (alice_id, bob_id, carol_id) = app
        .execute_fn::<_, _, RepositoryError>(AuthContext::Anonymous, async move |ctx| {
            let uow = ctx.enter().await?;
            let mut ids = Vec::new();
            for email in ["alice@eq.at", "bob@eq.at", "carol@eq.at"] {
                let b = uow
                    .benutzer()
                    .create(NeuerBenutzer::registrierung(
                        EmailAdresse::normalisiert(email).unwrap(),
                        t0,
                    ))
                    .await?;
                ids.push(b.id());
            }
            uow.commit().await?;
            Ok((ids[0], ids[1], ids[2]))
        })
        .await
        .unwrap();

    let auth = |uid| AuthContext::Authenticated {
        benutzer_id: uid,
        session_id: SessionId(Uuid::new_v4()),
    };
    let alice = auth(alice_id);
    let bob = auth(bob_id);
    let carol = auth(carol_id);

    let created = app
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Split Test").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
                draft_deadline_at: None,
                ends_at: None,
            },
        )
        .await
        .unwrap();
    for guest in [bob, carol] {
        clock.advance_by(Duration::seconds(1));
        app.execute(
            guest,
            JoinGame {
                invite_code: created.invite_code().clone(),
            },
        )
        .await
        .unwrap();
    }
    let drafted = app
        .execute(
            alice,
            StartDrafting {
                game_id: created.id(),
            },
        )
        .await
        .unwrap();

    let player_ids = [alice_id, bob_id, carol_id];
    let tiles_needed = drafted.playable_tile_count();
    for (idx, uid) in player_ids.iter().copied().enumerate() {
        let quotas = target_quotas_for_author(&player_ids, uid, tiles_needed).unwrap();
        let prompts = quotas
            .into_iter()
            .flat_map(|(target, count)| {
                (0..count).map(move |i| DraftPromptInput {
                    target_user_id: target,
                    text: PromptText::new(format!("P{idx}-{i}")).unwrap(),
                    source: PromptSource::Custom,
                })
            })
            .collect();
        app.execute(
            auth(uid),
            SaveDraftPrompts {
                game_id: created.id(),
                prompts,
            },
        )
        .await
        .unwrap();
        app.execute(
            auth(uid),
            LockDraftPrompts {
                game_id: created.id(),
            },
        )
        .await
        .unwrap();
    }

    clock.advance_by(Duration::seconds(READY_GRACE_PERIOD_SECONDS));
    recorder.clear();
    app.execute(AuthContext::Anonymous, StartReadyGames)
        .await
        .unwrap();
    let started_ev = recorder
        .batches()
        .into_iter()
        .flatten()
        .find(|e| matches!(e, Event::GameStarted(_)))
        .expect("GameStarted emitted");

    let count_notifs = |uid: BenutzerId, art: BenachrichtigungArt| {
        let app = app.clone();
        async move {
            app.execute_fn::<_, _, RepositoryError>(AuthContext::Anonymous, async move |ctx| {
                let uow = ctx.enter().await?;
                let list = uow.benachrichtigungen().find_by_empfänger(uid).await?;
                uow.rollback().await?;
                Ok(list.into_iter().filter(|n| n.art() == art).count())
            })
            .await
            .unwrap()
        }
    };

    // 1) GameStarted: Bob's UoW fails, yet Alice & Carol receive both inbox notifications and emails.
    mail.clear();
    *fail_for_user.lock().unwrap() = Some(bob_id);
    assert!(handle_event(&app, started_ev.clone()).await.is_err());
    assert_eq!(
        count_notifs(alice_id, BenachrichtigungArt::SpielGestartet).await,
        1
    );
    assert_eq!(
        count_notifs(carol_id, BenachrichtigungArt::SpielGestartet).await,
        1
    );
    assert_eq!(
        count_notifs(bob_id, BenachrichtigungArt::SpielGestartet).await,
        0
    );
    assert_eq!(mail.sent().len(), 2);

    // Retry once Bob's failure resolves: Bob recovers without duplicating Alice or Carol.
    *fail_for_user.lock().unwrap() = None;
    handle_event(&app, started_ev).await.unwrap();
    assert_eq!(
        count_notifs(bob_id, BenachrichtigungArt::SpielGestartet).await,
        1
    );
    assert_eq!(mail.sent().len(), 3);

    // 2) CaptureUploaded + 3) GameFinished via completing one row on Alice's board.
    let game_id = created.id();
    let tiles = app
        .execute_fn::<_, _, RepositoryError>(alice, async move |ctx| {
            let uow = ctx.enter().await?;
            let board = uow
                .boards()
                .find_by_game_and_owner(game_id, alice_id)
                .await?
                .into_data();
            uow.rollback().await?;
            Ok(board.tiles().to_vec())
        })
        .await
        .unwrap();
    let row_tiles: Vec<_> = tiles.iter().filter(|t| t.row() == 0).collect();

    recorder.clear();
    let png = png_bytes_for_test(160, 160);
    for t in row_tiles {
        app.execute(
            alice,
            SubmitCapture {
                game_id: created.id(),
                tile_id: t.id(),
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap();
    }
    let mut emitted: Vec<_> = recorder.batches().into_iter().flatten().collect();
    let finished_ev = emitted
        .iter()
        .find(|e| matches!(e, Event::GameFinished(_)))
        .cloned()
        .expect("GameFinished emitted");
    let capture_ev = emitted
        .drain(..)
        .find(|e| matches!(e, Event::CaptureUploaded(_)))
        .expect("CaptureUploaded emitted");

    // CaptureUploaded: Bob fails, Carol still gets inbox entry + email; retry recovers Bob without duplicate for Carol.
    mail.clear();
    *fail_for_user.lock().unwrap() = Some(bob_id);
    assert!(handle_event(&app, capture_ev.clone()).await.is_err());
    assert_eq!(
        count_notifs(carol_id, BenachrichtigungArt::AufnahmeHochgeladen).await,
        1
    );
    assert_eq!(
        count_notifs(bob_id, BenachrichtigungArt::AufnahmeHochgeladen).await,
        0
    );
    assert_eq!(mail.sent().len(), 1);
    assert_eq!(mail.sent()[0].empfänger.as_ref(), "carol@eq.at");

    *fail_for_user.lock().unwrap() = None;
    handle_event(&app, capture_ev).await.unwrap();
    assert_eq!(
        count_notifs(bob_id, BenachrichtigungArt::AufnahmeHochgeladen).await,
        1
    );
    assert_eq!(mail.sent().len(), 2);

    // GameFinished: Bob fails, Carol still gets inbox entry + email; retry recovers Bob without duplicate for Carol.
    mail.clear();
    *fail_for_user.lock().unwrap() = Some(bob_id);
    assert!(handle_event(&app, finished_ev.clone()).await.is_err());
    assert_eq!(
        count_notifs(carol_id, BenachrichtigungArt::SpielBeendet).await,
        1
    );
    assert_eq!(
        count_notifs(bob_id, BenachrichtigungArt::SpielBeendet).await,
        0
    );
    assert_eq!(mail.sent().len(), 1);
    assert_eq!(mail.sent()[0].empfänger.as_ref(), "carol@eq.at");

    *fail_for_user.lock().unwrap() = None;
    handle_event(&app, finished_ev).await.unwrap();
    assert_eq!(
        count_notifs(bob_id, BenachrichtigungArt::SpielBeendet).await,
        1
    );
    assert_eq!(mail.sent().len(), 2);
}
