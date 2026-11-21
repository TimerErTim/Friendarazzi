#![cfg(feature = "event-queue")]

use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use friendarazzi_core::{
    App,
    adapters::{InMemorySigning, InMemoryThrottle},
    application::{AuthContext, IdentityProviderConfiguration},
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
use friendarazzi_fakes::{FakeMailSender, FixedClock, FixedRng};
use friendarazzi_sqlite::SQLiteInstance;
use friendarazzi_sqlite::event_queue::{self, EventQueueConfig, SqliteEventQueue};
use tempdir::TempDir;
use tokio::sync::watch;
use uuid::Uuid;

fn test_time() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).expect("valid timestamp")
}

async fn build_app_on_db(
    sqlite: Arc<SQLiteInstance>,
    clock: Arc<FixedClock>,
    mail: Arc<FakeMailSender>,
    queue: Arc<SqliteEventQueue>,
) -> App {
    App::builder()
        .uow_provider(Box::new(sqlite))
        .signing(Arc::new(InMemorySigning::from_secret_bytes(&[7u8; 32])))
        .throttle(Arc::new(InMemoryThrottle::new()))
        .mail_sender(mail as Arc<dyn friendarazzi_core::ports::MailSender>)
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .clock(clock as Arc<dyn Clock>)
        .rng(Arc::new(FixedRng::default()))
        .event_queue(queue)
        .build()
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

#[test_log::test(tokio::test)]
async fn sqlite_event_queue_persists_across_restart_and_activates_delayed_game() {
    let dir = TempDir::new("friendarazzi-eq-test").unwrap();
    let db_path = dir.path().join("friendarazzi.db");

    let clock = Arc::new(FixedClock::new(test_time()));
    let mail = Arc::new(FakeMailSender::new());

    let sqlite = Arc::new(SQLiteInstance::local(&db_path).await.unwrap());
    sqlite.migrate_repos_to_latest().await.unwrap();
    let queue = Arc::new(
        SqliteEventQueue::new(Arc::clone(&sqlite)).with_clock(Arc::clone(&clock) as Arc<dyn Clock>),
    );
    queue.migrate_to_latest().await.unwrap();

    let app = build_app_on_db(
        Arc::clone(&sqlite),
        Arc::clone(&clock),
        Arc::clone(&mail),
        Arc::clone(&queue),
    )
    .await;

    let alice = register(&app, &mail, "alice@example.com").await;
    let bob = register(&app, &mail, "bob@example.com").await;
    let carol = register(&app, &mail, "carol@example.com").await;

    let game = app
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Persisted Countdown").unwrap(),
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
    let first = app
        .execute(carol, LockDraftPrompts { game_id: game.id() })
        .await
        .unwrap();
    let Game::Drafting(first) = first else {
        panic!("expected Drafting game");
    };
    let stale_deadline = first.auto_start_at().expect("countdown set");

    // Unlock + re-lock gives a new deadline; the first event becomes a stale no-op.
    app.execute(carol, UnlockDraftPrompts { game_id: game.id() })
        .await
        .unwrap();
    let second = app
        .execute(carol, LockDraftPrompts { game_id: game.id() })
        .await
        .unwrap();
    let Game::Drafting(second) = second else {
        panic!("expected Drafting game");
    };
    let current_deadline = second.auto_start_at().expect("countdown set");

    let game_id = game.id();
    app.execute_fn::<_, (), RepositoryError>(AuthContext::Anonymous, async move |ctx| {
        let uow = ctx.enter().await?;
        ctx.event_queue().submit(DelayedGameStart {
            game_id,
            scheduled_for: stale_deadline,
        });
        ctx.event_queue().submit(DelayedGameStart {
            game_id,
            scheduled_for: current_deadline,
        });
        uow.commit().await
    })
    .await
    .unwrap();
    assert_eq!(queue.pending_count().await.unwrap(), 2);

    // Simulate process restart on the same SQLite file.
    drop(app);
    drop(queue);
    drop(sqlite);

    let sqlite2 = Arc::new(SQLiteInstance::local(&db_path).await.unwrap());
    sqlite2.migrate_repos_to_latest().await.unwrap();
    let queue2 = Arc::new(
        SqliteEventQueue::new(Arc::clone(&sqlite2))
            .with_clock(Arc::clone(&clock) as Arc<dyn Clock>),
    );
    queue2.migrate_to_latest().await.unwrap();
    assert_eq!(queue2.pending_count().await.unwrap(), 2);

    let app2 = build_app_on_db(
        Arc::clone(&sqlite2),
        Arc::clone(&clock),
        Arc::clone(&mail),
        Arc::clone(&queue2),
    )
    .await;

    clock.advance_by(Duration::seconds(READY_GRACE_PERIOD_SECONDS));

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let worker = tokio::spawn(event_queue::run_with_config(
        Arc::clone(&queue2),
        app2.clone(),
        EventQueueConfig {
            idle_poll_interval: StdDuration::from_millis(20),
            ..EventQueueConfig::default()
        },
        async move {
            let _ = shutdown_rx.wait_for(|&stopped| stopped).await;
        },
    ));

    // Wait until both events are processed and deleted from _event_queue.
    for _ in 0..100 {
        let conn = sqlite2.create_connection().await.unwrap();
        let mut rows = conn
            .query("SELECT COUNT(*) FROM _event_queue", ())
            .await
            .unwrap();
        let total: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
        if total == 0 {
            break;
        }
        tokio::time::sleep(StdDuration::from_millis(10)).await;
    }

    let _ = shutdown_tx.send(true);
    let report = worker.await.unwrap().unwrap();
    assert_eq!(report.processed, 2);
    assert_eq!(report.failed, 0);
    assert_eq!(queue2.pending_count().await.unwrap(), 0);
    assert!(matches!(load_game(&app2, game.id()).await, Game::Active(_)));
}

#[test_log::test(tokio::test)]
async fn sqlite_event_queue_recovers_stale_processing_and_shuts_down_gracefully() {
    let sqlite = Arc::new(SQLiteInstance::in_temp_dir().await.unwrap());
    sqlite.migrate_repos_to_latest().await.unwrap();
    let clock = Arc::new(FixedClock::new(test_time()));
    let queue = Arc::new(
        SqliteEventQueue::new(Arc::clone(&sqlite)).with_clock(Arc::clone(&clock) as Arc<dyn Clock>),
    );
    queue.migrate_to_latest().await.unwrap();

    let app = build_app_on_db(
        Arc::clone(&sqlite),
        Arc::clone(&clock),
        Arc::new(FakeMailSender::new()),
        Arc::clone(&queue),
    )
    .await;

    // Enqueue a due event, then simulate a crashed worker that left it in 'processing'.
    let game_id = GameId(Uuid::new_v4());
    let due_at = clock.now();
    queue
        .enqueue_submitted(vec![Event::from(DelayedGameStart {
            game_id,
            scheduled_for: due_at,
        })])
        .await
        .unwrap();

    {
        let conn = sqlite.create_connection().await.unwrap();
        conn.execute("UPDATE _event_queue SET status = 'processing'", ())
            .await
            .unwrap();
    }

    // Also enqueue a future event that stays unclaimed until its scheduled_for deadline.
    let future_at = clock.now() + Duration::seconds(600);
    queue
        .enqueue_submitted(vec![Event::from(DelayedGameStart {
            game_id,
            scheduled_for: future_at,
        })])
        .await
        .unwrap();

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let worker = tokio::spawn(event_queue::run_with_config(
        Arc::clone(&queue),
        app,
        EventQueueConfig {
            idle_poll_interval: StdDuration::from_secs(60),
            shutdown_grace_period: StdDuration::from_millis(50),
            ..EventQueueConfig::default()
        },
        async move {
            let _ = shutdown_rx.wait_for(|&stopped| stopped).await;
        },
    ));

    // Wait until both the recovered row and second pending row are processed.
    for _ in 0..100 {
        let conn = sqlite.create_connection().await.unwrap();
        let mut rows = conn
            .query("SELECT COUNT(*) FROM _event_queue", ())
            .await
            .unwrap();
        let total: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
        if total == 0 {
            break;
        }
        tokio::time::sleep(StdDuration::from_millis(10)).await;
    }

    // Verify graceful shutdown wakes an idle worker promptly instead of waiting 60s.
    tokio::time::sleep(StdDuration::from_millis(20)).await;
    let _ = shutdown_tx.send(true);
    let report = tokio::time::timeout(StdDuration::from_secs(2), worker)
        .await
        .expect("worker stopped promptly on shutdown")
        .unwrap()
        .unwrap();

    assert_eq!(report.recovered, 1);
    assert_eq!(report.processed, 2);
    assert_eq!(report.failed, 0);
    assert_eq!(queue.pending_count().await.unwrap(), 0);
    assert_eq!(queue.failed_count().await.unwrap(), 0);
}
