use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use chrono::{DateTime, Duration, Utc};
use error_stack::Report;
use friendarazzi_core::{
    App,
    adapters::{BlockingEventQueue, InMemorySigning, InMemoryThrottle},
    application::{AuthContext, IdentityProviderConfiguration},
    domain::{
        EmailAdresse, Game, GameId, GridSize, NonEmptyString, PromptSource,
        READY_GRACE_PERIOD_SECONDS, VisibilityMode,
        konto::{KlartextPasswort, SecretToken, SessionId},
        target_quotas_for_author,
    },
    ports::{ClientKey, Clock, EventQueue, EventQueueError, RepositoryError},
    service::{
        CreateGame, DelayedGameStart, DraftPromptInput, Event, JoinGame, LockDraftPrompts,
        RegistrierungAbschließen, RegistrierungStarten, SaveDraftPrompts, StartDrafting,
        UnlockDraftPrompts,
    },
};
use friendarazzi_fakes::{FakeMailSender, FakeUnitOfWorkProvider, FixedClock, FixedRng};
use uuid::Uuid;

fn test_time() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).expect("valid timestamp")
}

async fn register(app: &App, mail: &FakeMailSender, email: &str) -> AuthContext {
    let client_key = ClientKey::new("127.0.0.1".parse().unwrap(), email.to_owned());
    app.execute(
        AuthContext::Anonymous,
        RegistrierungStarten {
            email: EmailAdresse::normalisiert(email).unwrap(),
            client_key: client_key.clone(),
        },
    )
    .await
    .unwrap();
    let text = mail.latest().text("https://friendarazzi.app");
    let start = text.find("token=").unwrap() + "token=".len();
    let token_str: String = text[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let session = app
        .execute(
            AuthContext::Anonymous,
            RegistrierungAbschließen {
                token: SecretToken::try_from(token_str.as_str()).unwrap(),
                passwort: KlartextPasswort::neu("korrekt-pferd-batterie").unwrap(),
                client_key,
            },
        )
        .await
        .unwrap();
    AuthContext::Authenticated {
        benutzer_id: session.benutzer.id(),
        session_id: SessionId(Uuid::new_v4()),
    }
}

async fn setup_drafting_ready_for_last_lock(
    app: &App,
    mail: &FakeMailSender,
    clock: &FixedClock,
) -> (AuthContext, AuthContext, AuthContext, GameId) {
    let alice = register(app, mail, "alice@example.com").await;
    let bob = register(app, mail, "bob@example.com").await;
    let carol = register(app, mail, "carol@example.com").await;

    let game = app
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Delayed Start Test").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
            },
        )
        .await
        .unwrap();

    for guest in [bob, carol] {
        clock.advance_by(Duration::seconds(1));
        app.execute(
            guest,
            JoinGame {
                invite_code: game.invite_code().clone(),
            },
        )
        .await
        .unwrap();
    }

    app.execute(alice, StartDrafting { game_id: game.id() })
        .await
        .unwrap();

    let player_ids = vec![
        alice.require_benutzer_id().unwrap(),
        bob.require_benutzer_id().unwrap(),
        carol.require_benutzer_id().unwrap(),
    ];
    for (idx, auth) in [alice, bob, carol].into_iter().enumerate() {
        let uid = auth.require_benutzer_id().unwrap();
        let quotas =
            target_quotas_for_author(&player_ids, uid, game.playable_tile_count()).unwrap();
        let mut prompts = Vec::new();
        for (target, count) in quotas {
            for i in 0..count {
                prompts.push(DraftPromptInput {
                    target_user_id: target,
                    text: NonEmptyString::neu(format!("P{idx} {i}")).unwrap(),
                    source: PromptSource::Custom,
                });
            }
        }
        app.execute(
            auth,
            SaveDraftPrompts {
                game_id: game.id(),
                prompts,
            },
        )
        .await
        .unwrap();
    }

    app.execute(alice, LockDraftPrompts { game_id: game.id() })
        .await
        .unwrap();
    app.execute(bob, LockDraftPrompts { game_id: game.id() })
        .await
        .unwrap();

    (alice, bob, carol, game.id())
}

async fn load_game(app: &App, game_id: GameId) -> Game {
    app.execute_fn::<_, _, RepositoryError>(AuthContext::Anonymous, async move |ctx| {
        let uow = ctx.enter().await?;
        let game = uow.games().find_by_id(game_id).await;
        let res = game.map(|g| g.into_data());
        uow.finish(res, RepositoryError::Unknown).await
    })
    .await
    .unwrap()
}

struct RecordingEventQueue {
    batches: std::sync::Mutex<Vec<Vec<Event>>>,
}

impl RecordingEventQueue {
    fn new() -> Self {
        Self {
            batches: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn total_events(&self) -> usize {
        self.batches.lock().unwrap().iter().map(Vec::len).sum()
    }
}

#[async_trait::async_trait]
impl EventQueue for RecordingEventQueue {
    async fn enqueue_submitted(&self, events: Vec<Event>) -> Result<(), Report<EventQueueError>> {
        self.batches.lock().unwrap().push(events);
        Ok(())
    }
}

#[test_log::test(pollster::test)]
async fn staged_events_are_only_dispatched_after_execution_succeeds() {
    let clock = Arc::new(FixedClock::new(test_time()));
    let recorder = Arc::new(RecordingEventQueue::new());
    let app = App::builder()
        .uow_provider(Box::new(FakeUnitOfWorkProvider::empty()))
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .clock(Arc::clone(&clock) as Arc<dyn Clock>)
        .event_queue(Arc::clone(&recorder))
        .build();

    let dummy = DelayedGameStart {
        game_id: GameId(Uuid::new_v4()),
        scheduled_for: clock.now(),
    };

    // Failing execution must discard staged events even if submitted before the error.
    let err_res = app
        .execute_fn::<_, (), RepositoryError>(AuthContext::Anonymous, {
            let ev = dummy.clone();
            async move |ctx| {
                let uow = ctx.enter().await?;
                ctx.event_queue().submit(ev);
                uow.rollback().await?;
                Err(Report::new(RepositoryError::Conflict))
            }
        })
        .await;
    assert!(err_res.is_err());
    assert_eq!(recorder.total_events(), 0);

    // Succeeding execution dispatches staged events after the UoW closes.
    app.execute_fn::<_, (), RepositoryError>(AuthContext::Anonymous, {
        let ev = dummy.clone();
        async move |ctx| {
            let uow = ctx.enter().await?;
            ctx.event_queue().submit(ev);
            uow.commit().await
        }
    })
    .await
    .unwrap();
    assert_eq!(recorder.total_events(), 1);
}

#[test_log::test(pollster::test)]
async fn blocking_event_queue_runs_after_uow_commit_without_deadlock() {
    let clock = Arc::new(FixedClock::new(test_time()));
    let mail = Arc::new(FakeMailSender::new());

    let app = App::builder()
        .uow_provider(Box::new(FakeUnitOfWorkProvider::empty()))
        .signing(Arc::new(InMemorySigning::from_secret_bytes(&[7u8; 32])))
        .throttle(Arc::new(InMemoryThrottle::new()))
        .mail_sender(Arc::clone(&mail) as Arc<dyn friendarazzi_core::ports::MailSender>)
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .clock(Arc::clone(&clock) as Arc<dyn Clock>)
        .rng(Arc::new(FixedRng::default()))
        .event_queue(BlockingEventQueue::from_app)
        .build();

    let (_alice, _bob, carol, game_id) =
        setup_drafting_ready_for_last_lock(&app, &mail, &clock).await;

    // `LockDraftPrompts` submits `DelayedGameStart` inside an open UoW for `now + 60s`.
    // Because staging defers dispatch until after the UoW commits, `BlockingEventQueue`
    // cannot deadlock on the outer UoW. Advance `FixedClock` from a background thread so
    // the inline `sleep_until` unblocks once `LockDraftPrompts` finishes its UoW.
    let advancer_clock = Arc::clone(&clock);
    let advancer = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(20));
        advancer_clock.advance_by(Duration::seconds(READY_GRACE_PERIOD_SECONDS));
    });
    let locked = app
        .execute(carol, LockDraftPrompts { game_id })
        .await
        .unwrap();
    advancer.join().unwrap();
    assert!(matches!(locked, Game::Drafting(_)));
    assert!(matches!(load_game(&app, game_id).await, Game::Active(_)));

    // Also verify direct submission inside an open UoW when the deadline is already due.
    let scheduled_for = clock.now();
    app.execute_fn::<_, (), RepositoryError>(AuthContext::Anonymous, async move |ctx| {
        let uow = ctx.enter().await?;
        ctx.event_queue().submit(DelayedGameStart {
            game_id,
            scheduled_for,
        });
        uow.commit().await
    })
    .await
    .unwrap();
}

#[test_log::test(pollster::test)]
async fn default_multithreaded_event_queue_waits_for_deadline_and_activates_game() {
    let clock = Arc::new(FixedClock::new(test_time()));
    let mail = Arc::new(FakeMailSender::new());

    // Default App::builder() uses MultithreadedEventQueue.
    let app = App::builder()
        .uow_provider(Box::new(FakeUnitOfWorkProvider::empty()))
        .signing(Arc::new(InMemorySigning::from_secret_bytes(&[7u8; 32])))
        .throttle(Arc::new(InMemoryThrottle::new()))
        .mail_sender(Arc::clone(&mail) as Arc<dyn friendarazzi_core::ports::MailSender>)
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .clock(Arc::clone(&clock) as Arc<dyn Clock>)
        .rng(Arc::new(FixedRng::default()))
        .build();

    let (_alice, _bob, carol, game_id) =
        setup_drafting_ready_for_last_lock(&app, &mail, &clock).await;

    // Lock -> countdown starts, game stays Drafting while clock < 60s.
    let drafting = app
        .execute(carol, LockDraftPrompts { game_id })
        .await
        .unwrap();
    assert!(matches!(drafting, Game::Drafting(_)));

    clock.advance_by(Duration::seconds(READY_GRACE_PERIOD_SECONDS - 1));
    std::thread::sleep(std::time::Duration::from_millis(20));
    assert!(matches!(load_game(&app, game_id).await, Game::Drafting(_)));

    // Unlock and re-lock -> stale event sees mismatch when woken; new event sleeps for +60s.
    app.execute(carol, UnlockDraftPrompts { game_id })
        .await
        .unwrap();
    app.execute(carol, LockDraftPrompts { game_id })
        .await
        .unwrap();

    clock.advance_by(Duration::seconds(READY_GRACE_PERIOD_SECONDS - 1));
    std::thread::sleep(std::time::Duration::from_millis(20));
    assert!(matches!(load_game(&app, game_id).await, Game::Drafting(_)));

    // Final second elapses -> background worker activates game automatically.
    clock.advance_by(Duration::seconds(1));
    for _ in 0..100 {
        if matches!(load_game(&app, game_id).await, Game::Active(_)) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(matches!(load_game(&app, game_id).await, Game::Active(_)));
}

#[test_log::test(pollster::test)]
async fn dropping_app_shuts_down_sleeping_multithreaded_workers_promptly() {
    let clock = Arc::new(FixedClock::new(test_time()));
    let mail = Arc::new(FakeMailSender::new());
    let app = App::builder()
        .uow_provider(Box::new(FakeUnitOfWorkProvider::empty()))
        .signing(Arc::new(InMemorySigning::from_secret_bytes(&[7u8; 32])))
        .throttle(Arc::new(InMemoryThrottle::new()))
        .mail_sender(Arc::clone(&mail) as Arc<dyn friendarazzi_core::ports::MailSender>)
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .clock(Arc::clone(&clock) as Arc<dyn Clock>)
        .rng(Arc::new(FixedRng::default()))
        .build();

    let (_alice, _bob, carol, game_id) =
        setup_drafting_ready_for_last_lock(&app, &mail, &clock).await;
    app.execute(carol, LockDraftPrompts { game_id })
        .await
        .unwrap();

    // Even with a worker sleeping on a 60s FixedClock deadline, dropping App returns immediately.
    let drop_start = std::time::Instant::now();
    drop(app);
    assert!(drop_start.elapsed() < std::time::Duration::from_secs(1));
    let _ = AtomicUsize::new(0).load(Ordering::Relaxed);
}
