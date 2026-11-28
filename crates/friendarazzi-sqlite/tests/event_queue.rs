#![cfg(feature = "event-queue")]

use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Utc};
use friendarazzi_core::{
    App,
    adapters::InMemorySigning,
    application::{AuthContext, IdentityProviderConfiguration},
    domain::{BenachrichtigungArt, BenutzerId, CaptureId, GameId, NeueBenachrichtigung},
    ports::{Clock, EventQueue, RepositoryError},
    service::{
        BenachrichtigungAbbestellenMitToken, CaptureUploaded, Event, GameFinished, GameStarted,
    },
};
use friendarazzi_fakes::FixedClock;
use friendarazzi_sqlite::SQLiteInstance;
use friendarazzi_sqlite::event_queue::{self, EventQueueConfig, SqliteEventQueue};
use tempdir::TempDir;
use tokio::sync::watch;
use uuid::Uuid;

fn test_time() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).expect("valid timestamp")
}

fn build_app_on_db(
    sqlite: Arc<SQLiteInstance>,
    clock: Arc<FixedClock>,
    queue: Arc<SqliteEventQueue>,
) -> App {
    App::builder()
        .uow_provider(Box::new(sqlite))
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .clock(clock as Arc<dyn Clock>)
        .event_queue(queue)
        .build()
}

fn sample_event() -> CaptureUploaded {
    CaptureUploaded {
        game_id: GameId(Uuid::new_v4()),
        capture_id: CaptureId(Uuid::new_v4()),
        uploader_id: BenutzerId(Uuid::new_v4()),
        target_id: BenutzerId(Uuid::new_v4()),
        occurred_at: test_time(),
    }
}

#[test_log::test(tokio::test)]
async fn sqlite_event_queue_persists_across_restart_and_processes_events() {
    let dir = TempDir::new("friendarazzi-eq-test").unwrap();
    let db_path = dir.path().join("friendarazzi.db");

    let clock = Arc::new(FixedClock::new(test_time()));
    let sqlite = Arc::new(SQLiteInstance::local(&db_path).await.unwrap());
    sqlite.migrate_repos_to_latest().await.unwrap();
    let mut queue = SqliteEventQueue::new(Arc::clone(&sqlite));
    queue.migrate_to_latest().await.unwrap();
    let queue = Arc::new(queue);

    let app = build_app_on_db(Arc::clone(&sqlite), Arc::clone(&clock), Arc::clone(&queue));

    let ev1 = GameStarted {
        game_id: GameId(Uuid::new_v4()),
        started_at: test_time(),
    };
    let ev2 = sample_event();
    let ev3 = GameFinished {
        game_id: GameId(Uuid::new_v4()),
        winner_id: BenutzerId(Uuid::new_v4()),
        finished_at: test_time(),
    };
    app.execute_fn::<_, (), RepositoryError>(AuthContext::Anonymous, async move |ctx| {
        let uow = ctx.enter().await?;
        ctx.events().submit(ev1);
        ctx.events().submit(ev2);
        ctx.events().submit(ev3);
        uow.commit().await
    })
    .await
    .unwrap();
    assert_eq!(queue.pending_count().await.unwrap(), 3);

    // Simulate process restart on the same SQLite file.
    drop(app);
    drop(queue);
    drop(sqlite);

    let sqlite2 = Arc::new(SQLiteInstance::local(&db_path).await.unwrap());
    sqlite2.migrate_repos_to_latest().await.unwrap();
    let mut queue2 = SqliteEventQueue::new(Arc::clone(&sqlite2));
    queue2.migrate_to_latest().await.unwrap();
    let queue2 = Arc::new(queue2);
    assert_eq!(queue2.pending_count().await.unwrap(), 3);

    let app2 = build_app_on_db(
        Arc::clone(&sqlite2),
        Arc::clone(&clock),
        Arc::clone(&queue2),
    );

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let worker = tokio::spawn(event_queue::run_with_config(
        Arc::clone(&queue2),
        app2,
        EventQueueConfig {
            idle_poll_interval: StdDuration::from_millis(20),
            ..EventQueueConfig::default()
        },
        async move {
            let _ = shutdown_rx.wait_for(|&stopped| stopped).await;
        },
    ));

    for _ in 0..100 {
        if queue2.pending_count().await.unwrap() == 0 {
            break;
        }
        tokio::time::sleep(StdDuration::from_millis(10)).await;
    }

    let _ = shutdown_tx.send(true);
    let report = worker.await.unwrap().unwrap();
    assert_eq!(report.processed, 3);
    assert_eq!(report.failed, 0);
    assert_eq!(queue2.pending_count().await.unwrap(), 0);
}

#[test_log::test(tokio::test)]
async fn sqlite_event_queue_recovers_stale_processing_and_shuts_down_gracefully() {
    let sqlite = Arc::new(SQLiteInstance::in_temp_dir().await.unwrap());
    sqlite.migrate_repos_to_latest().await.unwrap();
    let clock = Arc::new(FixedClock::new(test_time()));
    let mut queue = SqliteEventQueue::new(Arc::clone(&sqlite));
    queue.migrate_to_latest().await.unwrap();
    let queue = Arc::new(queue);

    let app = build_app_on_db(Arc::clone(&sqlite), Arc::clone(&clock), Arc::clone(&queue));

    queue
        .enqueue(vec![Event::from(sample_event())])
        .await
        .unwrap();

    {
        let conn = sqlite.create_connection().await.unwrap();
        conn.execute("UPDATE _event_queue SET status = 'processing'", ())
            .await
            .unwrap();
    }

    queue
        .enqueue(vec![Event::from(sample_event())])
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

#[test_log::test(tokio::test)]
async fn unsubscribe_token_survives_process_restart_with_fresh_random_signing_key() {
    let dir = TempDir::new("friendarazzi-unsub-restart-test").unwrap();
    let db_path = dir.path().join("friendarazzi.db");
    let clock = Arc::new(FixedClock::new(test_time()));

    let sqlite1 = Arc::new(SQLiteInstance::local(&db_path).await.unwrap());
    sqlite1.migrate_repos_to_latest().await.unwrap();
    let mut queue1 = SqliteEventQueue::new(Arc::clone(&sqlite1));
    queue1.migrate_to_latest().await.unwrap();
    let app1 = App::builder()
        .uow_provider(Box::new(Arc::clone(&sqlite1)))
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .signing(Arc::new(InMemorySigning::random()))
        .clock(clock.clone() as Arc<dyn Clock>)
        .event_queue(Arc::new(queue1))
        .build();

    let (user_id, token) = app1
        .execute_fn::<_, _, RepositoryError>(AuthContext::Anonymous, async move |ctx| {
            let uow = ctx.enter().await?;
            let user = uow
                .benutzer()
                .create(friendarazzi_core::domain::NeuerBenutzer::registrierung(
                    friendarazzi_core::domain::EmailAdresse::normalisiert("restart@example.at")
                        .unwrap(),
                    test_time(),
                ))
                .await?;
            let game = uow
                .games()
                .create(friendarazzi_core::domain::NewGame {
                    invite_code: friendarazzi_core::domain::InviteCode::new("RESTART1").unwrap(),
                    host_user_id: user.id(),
                    host_watching: true,
                    title: friendarazzi_core::domain::NonEmptyString::neu("Restart Test").unwrap(),
                    grid_size: friendarazzi_core::domain::GridSize::Three,
                    visibility_mode: friendarazzi_core::domain::VisibilityMode::LiveFeed,
                    draft_deadline_at: None,
                    created_at: test_time(),
                })
                .await?;
            let (neu, raw_token) =
                NeueBenachrichtigung::spiel_gestartet(user.id(), game.id(), test_time());
            uow.benachrichtigungen().create(neu).await?;
            uow.commit().await?;
            Ok((user.id(), raw_token))
        })
        .await
        .unwrap();

    // Simulate process restart with no signingKeyPath configured (fresh throwaway random key).
    drop(app1);
    drop(sqlite1);

    let sqlite2 = Arc::new(SQLiteInstance::local(&db_path).await.unwrap());
    sqlite2.migrate_repos_to_latest().await.unwrap();
    let mut queue2 = SqliteEventQueue::new(Arc::clone(&sqlite2));
    queue2.migrate_to_latest().await.unwrap();
    let app2 = App::builder()
        .uow_provider(Box::new(Arc::clone(&sqlite2)))
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .signing(Arc::new(InMemorySigning::random()))
        .clock(clock as Arc<dyn Clock>)
        .event_queue(Arc::new(queue2))
        .build();

    let kind = app2
        .execute(
            AuthContext::Anonymous,
            BenachrichtigungAbbestellenMitToken { token },
        )
        .await
        .unwrap();
    assert_eq!(kind, BenachrichtigungArt::SpielGestartet);

    let still_allowed = app2
        .execute_fn::<_, _, RepositoryError>(AuthContext::Anonymous, async move |ctx| {
            let uow = ctx.enter().await?;
            let user = uow.benutzer().find_by_id(user_id).await?;
            uow.rollback().await?;
            Ok(user.allows_email(BenachrichtigungArt::SpielGestartet))
        })
        .await
        .unwrap();
    assert!(!still_allowed);
}
