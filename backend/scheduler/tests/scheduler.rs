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
    ports::{ClientKey, Clock, RepositoryError},
    service::{
        CreateGame, DraftPromptInput, JoinGame, LockDraftPrompts, RegistrierungAbschließen,
        RegistrierungStarten, SaveDraftPrompts, StartDrafting,
    },
};
use friendarazzi_fakes::{FakeMailSender, FixedClock, FixedRng};
use friendarazzi_scheduler::{FriendarazziScheduler, FriendarazziSchedulerConfig};
use friendarazzi_sqlite::SQLiteInstance;
use friendarazzi_sqlite::job_store::SQLiteStateStore;
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
        session_id: SessionId(Uuid::new_v4()),
        benutzer_id: session.benutzer.id(),
    }
}

async fn load_game(app: &App, game_id: GameId) -> Game {
    app.execute_fn(AuthContext::Anonymous, async move |ctx| {
        let uow = ctx.enter().await?;
        let loaded = uow.games().find_by_id(game_id).await?;
        uow.finish(
            Ok::<_, error_stack::Report<RepositoryError>>(loaded.into_data()),
            RepositoryError::Unknown,
        )
        .await
    })
    .await
    .unwrap()
}

#[test_log::test(tokio::test)]
async fn friendarazzi_scheduler_activates_ready_game_and_shuts_down_gracefully() {
    let sqlite = Arc::new(SQLiteInstance::in_temp_dir().await.unwrap());
    sqlite.migrate_repos_to_latest().await.unwrap();
    let mut store = SQLiteStateStore::new(Arc::clone(&sqlite));
    store.migrate_to_latest().await.unwrap();

    let clock = Arc::new(FixedClock::new(test_time()));
    let mail = Arc::new(FakeMailSender::new());

    let app = App::builder()
        .uow_provider(Box::new(Arc::clone(&sqlite)))
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

    let alice = register(&app, &mail, "sched-run-a@example.com").await;
    let bob = register(&app, &mail, "sched-run-b@example.com").await;
    let carol = register(&app, &mail, "sched-run-c@example.com").await;

    let game = app
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Scheduler Game").unwrap(),
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
    let drafted = app
        .execute(alice, StartDrafting { game_id: game.id() })
        .await
        .unwrap();
    let player_ids: Vec<_> = [alice, bob, carol]
        .into_iter()
        .map(|auth| match auth {
            AuthContext::Authenticated { benutzer_id, .. } => benutzer_id,
            AuthContext::Anonymous => unreachable!(),
        })
        .collect();
    let tiles_needed = drafted.playable_tile_count();
    let mut next_num = 1;
    for auth in [alice, bob, carol] {
        let self_id = match auth {
            AuthContext::Authenticated { benutzer_id, .. } => benutzer_id,
            AuthContext::Anonymous => unreachable!(),
        };
        let mut prompts = Vec::new();
        for (target_user_id, count) in
            target_quotas_for_author(&player_ids, self_id, tiles_needed).unwrap()
        {
            for _ in 0..count {
                prompts.push(DraftPromptInput {
                    target_user_id,
                    text: NonEmptyString::neu(format!("Sched prompt {next_num}")).unwrap(),
                    source: PromptSource::Custom,
                });
                next_num += 1;
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
        app.execute(auth, LockDraftPrompts { game_id: game.id() })
            .await
            .unwrap();
    }

    assert!(matches!(
        load_game(&app, game.id()).await,
        Game::Drafting(_)
    ));
    clock.advance_by(Duration::seconds(READY_GRACE_PERIOD_SECONDS));

    let (handle, fut) = FriendarazziScheduler::new(app.clone())
        .state_store(store)
        .start_with_handle(FriendarazziSchedulerConfig {
            grace_period: StdDuration::from_millis(25),
            ..FriendarazziSchedulerConfig::default()
        });
    let worker = tokio::spawn(fut);

    for _ in 0..100 {
        if matches!(load_game(&app, game.id()).await, Game::Active(_)) {
            break;
        }
        tokio::time::sleep(StdDuration::from_millis(10)).await;
    }
    assert!(matches!(load_game(&app, game.id()).await, Game::Active(_)));

    handle.shutdown();
    tokio::time::timeout(StdDuration::from_secs(2), worker)
        .await
        .expect("scheduler stopped promptly on shutdown")
        .unwrap()
        .unwrap();
}

#[test_log::test(tokio::test)]
async fn friendarazzi_scheduler_sleeps_inside_tick_when_deadline_is_within_grace_period() {
    let sqlite = Arc::new(SQLiteInstance::in_temp_dir().await.unwrap());
    sqlite.migrate_repos_to_latest().await.unwrap();
    let mut store = SQLiteStateStore::new(Arc::clone(&sqlite));
    store.migrate_to_latest().await.unwrap();

    let clock = Arc::new(FixedClock::new(test_time()));
    let mail = Arc::new(FakeMailSender::new());

    let app = App::builder()
        .uow_provider(Box::new(Arc::clone(&sqlite)))
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

    let alice = register(&app, &mail, "sched-sleep-a@example.com").await;
    let bob = register(&app, &mail, "sched-sleep-b@example.com").await;
    let carol = register(&app, &mail, "sched-sleep-c@example.com").await;

    let game = app
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Sleep Window Game").unwrap(),
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
    let drafted = app
        .execute(alice, StartDrafting { game_id: game.id() })
        .await
        .unwrap();
    let player_ids: Vec<_> = [alice, bob, carol]
        .into_iter()
        .map(|auth| match auth {
            AuthContext::Authenticated { benutzer_id, .. } => benutzer_id,
            AuthContext::Anonymous => unreachable!(),
        })
        .collect();
    let tiles_needed = drafted.playable_tile_count();
    let mut next_num = 1;
    for auth in [alice, bob, carol] {
        let self_id = match auth {
            AuthContext::Authenticated { benutzer_id, .. } => benutzer_id,
            AuthContext::Anonymous => unreachable!(),
        };
        let mut prompts = Vec::new();
        for (target_user_id, count) in
            target_quotas_for_author(&player_ids, self_id, tiles_needed).unwrap()
        {
            for _ in 0..count {
                prompts.push(DraftPromptInput {
                    target_user_id,
                    text: NonEmptyString::neu(format!("Sleep prompt {next_num}")).unwrap(),
                    source: PromptSource::Custom,
                });
                next_num += 1;
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
        app.execute(auth, LockDraftPrompts { game_id: game.id() })
            .await
            .unwrap();
    }

    // Leave 60ms before expiry inside a 200ms interval tick so the task sleeps in-place.
    clock.advance_by(Duration::seconds(READY_GRACE_PERIOD_SECONDS) - Duration::milliseconds(60));
    let advancer_clock = Arc::clone(&clock);
    tokio::spawn(async move {
        tokio::time::sleep(StdDuration::from_millis(220)).await;
        advancer_clock.advance_by(Duration::milliseconds(60));
    });

    let (handle, fut) = FriendarazziScheduler::new(app.clone())
        .state_store(store)
        .start_with_handle(FriendarazziSchedulerConfig {
            grace_period: StdDuration::from_millis(200),
            ..FriendarazziSchedulerConfig::default()
        });
    let worker = tokio::spawn(fut);

    for _ in 0..100 {
        if matches!(load_game(&app, game.id()).await, Game::Active(_)) {
            break;
        }
        tokio::time::sleep(StdDuration::from_millis(20)).await;
    }
    assert!(matches!(load_game(&app, game.id()).await, Game::Active(_)));

    handle.shutdown();
    tokio::time::timeout(StdDuration::from_secs(2), worker)
        .await
        .expect("scheduler stopped promptly on shutdown")
        .unwrap()
        .unwrap();
}
