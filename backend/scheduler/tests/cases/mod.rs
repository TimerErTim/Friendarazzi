use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use friendarazzi_core::{
    App,
    adapters::{InMemorySigning, InMemoryThrottle},
    application::{AuthContext, IdentityProviderConfiguration},
    domain::{
        EmailAdresse, Game, GameId, GridSize, NonEmptyString, PromptSource, PromptText,
        READY_GRACE_PERIOD_SECONDS, VisibilityMode,
        konto::{KlartextPasswort, SecretToken, SessionId},
        target_quotas_for_author,
    },
    ports::{ClientKey, Clock, RepositoryError},
    service::{
        CreateGame, DraftPromptInput, EmailBestätigen, HostFillRemainingDrafts, JoinGame,
        LockDraftPrompts, Registrieren, SaveDraftPrompts, StartDrafting,
    },
    uow::UnitOfWorkProvider,
};
use friendarazzi_fakes::{FakeMailSender, FixedClock, FixedRng};
use friendarazzi_scheduler::{FriendarazziScheduler, FriendarazziSchedulerConfig, StateStore};
use uuid::Uuid;

pub fn test_time() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).expect("valid timestamp")
}

async fn register(app: &App, mail: &FakeMailSender, email: &str) -> AuthContext {
    let client_key = ClientKey::new("127.0.0.1".parse().unwrap(), email.to_owned());
    let session = app
        .execute(
            AuthContext::Anonymous,
            Registrieren {
                email: EmailAdresse::normalisiert(email).unwrap(),
                passwort: KlartextPasswort::neu("korrekt-pferd-batterie").unwrap(),
                display_name: None,
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
    app.execute(
        AuthContext::Anonymous,
        EmailBestätigen {
            token: SecretToken::try_from(token_str.as_str()).unwrap(),
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

pub async fn load_game(app: &App, game_id: GameId) -> Game {
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

#[allow(dead_code)]
pub async fn setup_active_game_with_deadline(
    uow_provider: Box<dyn UnitOfWorkProvider>,
    clock: Arc<FixedClock>,
    ends_at: DateTime<Utc>,
) -> (App, GameId) {
    let mail = Arc::new(FakeMailSender::new());
    let app = App::builder()
        .uow_provider(uow_provider)
        .signing(Arc::new(InMemorySigning::from_secret_bytes(&[9u8; 32])))
        .throttle(Arc::new(InMemoryThrottle::new()))
        .mail_sender(Arc::clone(&mail) as Arc<dyn friendarazzi_core::ports::MailSender>)
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .clock(clock.clone() as Arc<dyn Clock>)
        .rng(Arc::new(FixedRng::default()))
        .build();

    let alice = register(&app, &mail, "exp-a@example.com").await;
    let bob = register(&app, &mail, "exp-b@example.com").await;
    let carol = register(&app, &mail, "exp-c@example.com").await;

    let game = app
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Expire Sched").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
                draft_deadline_at: None,
                ends_at: Some(ends_at),
            },
        )
        .await
        .unwrap();
    for guest in [bob, carol] {
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
    app.execute(
        alice,
        HostFillRemainingDrafts {
            game_id: game.id(),
            start_immediately: true,
        },
    )
    .await
    .unwrap();

    (app, game.id())
}

pub async fn setup_locked_drafting_game(
    uow_provider: Box<dyn UnitOfWorkProvider>,
    clock: Arc<FixedClock>,
) -> (App, GameId) {
    let mail = Arc::new(FakeMailSender::new());
    let app = App::builder()
        .uow_provider(uow_provider)
        .signing(Arc::new(InMemorySigning::from_secret_bytes(&[7u8; 32])))
        .throttle(Arc::new(InMemoryThrottle::new()))
        .mail_sender(Arc::clone(&mail) as Arc<dyn friendarazzi_core::ports::MailSender>)
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .clock(clock.clone() as Arc<dyn Clock>)
        .rng(Arc::new(FixedRng::default()))
        .build();

    let alice = register(&app, &mail, "sched-a@example.com").await;
    let bob = register(&app, &mail, "sched-b@example.com").await;
    let carol = register(&app, &mail, "sched-c@example.com").await;

    let game = app
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Scheduler Case").unwrap(),
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
                    text: PromptText::new(format!("Prompt {next_num}")).unwrap(),
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
    (app, game.id())
}

/// Framework conformance test shared between `InMemoryStateStore` and `SQLiteStateStore`:
/// verifies scheduler startup, `trigger_now`, job execution, and graceful `shutdown`.
pub async fn scheduler_framework_runs_and_stops_cleanly<S>(
    uow_provider: Box<dyn UnitOfWorkProvider>,
    store: S,
) where
    S: StateStore + Send + Sync + 'static,
{
    let clock = Arc::new(FixedClock::new(test_time()));
    let (app, game_id) = setup_locked_drafting_game(uow_provider, Arc::clone(&clock)).await;
    assert!(matches!(load_game(&app, game_id).await, Game::Drafting(_)));

    clock.advance_by(Duration::seconds(READY_GRACE_PERIOD_SECONDS));

    let (handle, fut) = FriendarazziScheduler::new(app.clone())
        .state_store(store)
        .start_with_handle(FriendarazziSchedulerConfig::default());
    let worker = tokio::spawn(fut);

    // Wait briefly for the active job to register before issuing trigger_now.
    for _ in 0..50 {
        if handle.trigger_now().await.is_ok() {
            break;
        }
        tokio::time::sleep(StdDuration::from_millis(10)).await;
    }

    for _ in 0..100 {
        if matches!(load_game(&app, game_id).await, Game::Active(_)) {
            break;
        }
        tokio::time::sleep(StdDuration::from_millis(10)).await;
    }
    assert!(matches!(load_game(&app, game_id).await, Game::Active(_)));

    handle.shutdown();
    tokio::time::timeout(StdDuration::from_secs(2), worker)
        .await
        .expect("scheduler stopped promptly on shutdown")
        .unwrap()
        .unwrap();
}
