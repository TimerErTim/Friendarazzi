use std::sync::{Arc, OnceLock};

use chrono::{DateTime, Duration, Utc};
use friendarazzi_core::{
    App,
    adapters::{BlockingEventQueue, InMemorySigning, InMemoryThrottle, MultithreadedEventQueue},
    application::{AuthContext, IdentityProviderConfiguration, WeakApp},
    domain::{
        EmailAdresse, Game, GameId, GridSize, NonEmptyString, PromptSource,
        READY_GRACE_PERIOD_SECONDS, VisibilityMode,
        konto::{KlartextPasswort, SecretToken, SessionId},
        target_quotas_for_author,
    },
    ports::{ClientKey, Clock, EventQueue, RepositoryError},
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

#[test_log::test(pollster::test)]
async fn blocking_event_queue_schedules_cancels_and_runs_due_delayed_game_start() {
    let clock = Arc::new(FixedClock::new(test_time()));
    let mail = Arc::new(FakeMailSender::new());
    let queue_slot = Arc::new(OnceLock::new());
    let queue_slot_clone = Arc::clone(&queue_slot);

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
        .event_queue(move |weak: WeakApp| {
            let q = Arc::new(BlockingEventQueue::new(weak));
            let _ = queue_slot_clone.set(Arc::clone(&q));
            q as Arc<dyn EventQueue>
        })
        .build();
    let queue = queue_slot.get().unwrap();

    let (_alice, _bob, carol, game_id) =
        setup_drafting_ready_for_last_lock(&app, &mail, &clock).await;
    assert_eq!(queue.pending_len(), 0);

    // Last player locks -> schedules delayed start for +60s, game remains Drafting.
    let drafting = app
        .execute(carol, LockDraftPrompts { game_id })
        .await
        .unwrap();
    assert!(matches!(drafting, Game::Drafting(_)));
    assert_eq!(queue.pending_len(), 1);

    // Before 60s elapse, running due events does nothing.
    clock.advance_by(Duration::seconds(READY_GRACE_PERIOD_SECONDS - 1));
    assert_eq!(queue.run_due().await, 0);
    assert_eq!(queue.pending_len(), 1);
    assert!(matches!(load_game(&app, game_id).await, Game::Drafting(_)));

    // Unlock cancels the scheduled event.
    app.execute(carol, UnlockDraftPrompts { game_id })
        .await
        .unwrap();
    assert_eq!(queue.pending_len(), 0);
    clock.advance_by(Duration::seconds(10));
    assert_eq!(queue.run_due().await, 0);
    assert!(matches!(load_game(&app, game_id).await, Game::Drafting(_)));

    // Re-lock schedules a fresh 60s countdown.
    app.execute(carol, LockDraftPrompts { game_id })
        .await
        .unwrap();
    assert_eq!(queue.pending_len(), 1);

    // Once 60s elapse, run_due executes DelayedGameStart and activates the game.
    clock.advance_by(Duration::seconds(READY_GRACE_PERIOD_SECONDS));
    assert_eq!(queue.run_due().await, 1);
    assert_eq!(queue.pending_len(), 0);
    assert!(matches!(load_game(&app, game_id).await, Game::Active(_)));
}

#[test_log::test(pollster::test)]
async fn blocking_event_queue_executes_immediate_enqueue_inline() {
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
        .event_queue(BlockingEventQueue::adapter())
        .build();

    let (_alice, _bob, carol, game_id) =
        setup_drafting_ready_for_last_lock(&app, &mail, &clock).await;
    let Game::Drafting(d) = app
        .execute(carol, LockDraftPrompts { game_id })
        .await
        .unwrap()
    else {
        panic!("expected drafting");
    };
    let scheduled_for = d.auto_start_at().unwrap();

    clock.advance_by(Duration::seconds(READY_GRACE_PERIOD_SECONDS));
    app.event_queue
        .enqueue(Event::DelayedGameStart(DelayedGameStart {
            game_id,
            scheduled_for,
        }))
        .await
        .unwrap();

    assert!(matches!(load_game(&app, game_id).await, Game::Active(_)));
}

#[test_log::test(pollster::test)]
async fn default_multithreaded_event_queue_executes_due_events_asynchronously() {
    let clock = Arc::new(FixedClock::new(test_time()));
    let mail = Arc::new(FakeMailSender::new());
    let queue_slot = Arc::new(OnceLock::new());
    let queue_slot_clone = Arc::clone(&queue_slot);

    // Explicitly construct via MultithreadedEventQueue::new to verify non-blocking worker execution,
    // and also verify default App::builder() builds without `.event_queue(...)`.
    let default_app = App::builder()
        .uow_provider(Box::new(FakeUnitOfWorkProvider::empty()))
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .build();
    drop(default_app);

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
        .event_queue(move |weak: WeakApp| {
            let q = Arc::new(MultithreadedEventQueue::with_workers(weak, 2));
            let _ = queue_slot_clone.set(Arc::clone(&q));
            q as Arc<dyn EventQueue>
        })
        .build();
    let queue = queue_slot.get().unwrap();

    let (_alice, _bob, carol, game_id) =
        setup_drafting_ready_for_last_lock(&app, &mail, &clock).await;
    let Game::Drafting(d) = app
        .execute(carol, LockDraftPrompts { game_id })
        .await
        .unwrap()
    else {
        panic!("expected drafting");
    };
    let scheduled_for = d.auto_start_at().unwrap();

    // Advance FixedClock and wake workers; worker thread runs DelayedGameStart on App.
    clock.advance_by(Duration::seconds(READY_GRACE_PERIOD_SECONDS));
    queue.notify();
    queue.wait_due_idle();

    let game = load_game(&app, game_id).await;
    assert!(matches!(game, Game::Active(_)));

    // Immediate enqueue of an idempotent event also completes cleanly on workers.
    app.event_queue
        .enqueue(Event::DelayedGameStart(DelayedGameStart {
            game_id,
            scheduled_for,
        }))
        .await
        .unwrap();
    queue.wait_due_idle();
    assert_eq!(queue.pending_len(), 0);
}
