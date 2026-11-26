use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration};
use error_stack::Report;
use friendarazzi_core::{
    App, ResultReport,
    adapters::InMemoryThrottle,
    domain::{GridSize, NonEmptyString, VisibilityMode},
    ports::{Clock, Mail, MailError, MailSender},
    service::{CreateGame, JoinGame, SubmitCapture},
};
use friendarazzi_fakes::{FixedClock, FixedRng, png_bytes_for_test};
use friendarazzi_filesystemstore::FileSystemObjectStore;
use friendarazzi_sqlite::SQLiteInstance;
use poem::http::StatusCode;
use serde_json::json;

use crate::{
    E2eHarness, FRONTEND_BASIS_URL, enroll_and_start_game, neuer_benutzer, test_konfiguration,
    test_signing_key,
};

struct FailingMailSender;

#[async_trait]
impl MailSender for FailingMailSender {
    async fn send(&self, _mail: Mail) -> ResultReport<(), MailError> {
        Err(Report::new(MailError::Zustellung))
    }

    async fn prüfe_verbindung(&self) -> ResultReport<(), MailError> {
        Err(Report::new(MailError::Zustellung))
    }
}

#[test_log::test(tokio::test)]
async fn e2e_notifications_inbox_preferences_watch_unsubscribe_and_blurred_redaction() {
    let harness = E2eHarness::new().await;
    let (t_alice, _id_alice) = neuer_benutzer(&harness, "n-alice@example.at").await;
    let (t_bob, id_bob) = neuer_benutzer(&harness, "n-bob@example.at").await;
    let (t_carol, _id_carol) = neuer_benutzer(&harness, "n-carol@example.at").await;

    // Default preferences exist and default to true.
    let (st, prefs) = harness
        .client
        .get_json_auth("/api/notifications/preferences", &t_bob)
        .await;
    assert_eq!(st, StatusCode::OK, "{prefs}");
    assert_eq!(prefs["defaultWatchGames"], true);
    assert_eq!(prefs["emailGameStarted"], true);
    assert_eq!(prefs["emailCapturePosted"], true);
    assert_eq!(prefs["emailGameCompleted"], true);

    // Carol disables game-started email via preferences PUT (inbox still receives it).
    let (st, carol_prefs) = harness
        .client
        .put_json_auth(
            "/api/notifications/preferences",
            json!({ "emailGameStarted": false }),
            &t_carol,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{carol_prefs}");
    assert_eq!(carol_prefs["emailGameStarted"], false);
    assert_eq!(carol_prefs["emailCapturePosted"], true);

    // Start a LiveFeed game: Alice, Bob, Carol get GameStarted inbox entries; only Alice and Bob get GameStarted email.
    harness.mail.clear();
    let (game_id, _invite_code) = enroll_and_start_game(
        &harness,
        &t_alice,
        &[&t_bob, &t_carol],
        "Live Hunt",
        3,
        "LiveFeed",
    )
    .await;

    let started_mails = harness.mail.sent();
    assert_eq!(started_mails.len(), 2);
    assert!(
        started_mails
            .iter()
            .all(|m| m.empfänger.as_ref() != "n-carol@example.at")
    );

    let (st, carol_inbox) = harness
        .client
        .get_json_auth("/api/notifications", &t_carol)
        .await;
    assert_eq!(st, StatusCode::OK, "{carol_inbox}");
    assert_eq!(carol_inbox["unreadCount"], 1);
    assert_eq!(carol_inbox["notifications"][0]["kind"], "GameStarted");

    // Alice captures Bob and finishes a row -> Bob gets CapturePosted (with prompt + captureId) and GameCompleted!
    let (st, alice_detail) = harness
        .client
        .get_json_auth(&format!("/api/games/{game_id}"), &t_alice)
        .await;
    assert_eq!(st, StatusCode::OK);
    let tiles = alice_detail["myBoard"]["tiles"].as_array().unwrap();
    let bob_tile = tiles
        .iter()
        .find(|t| t["targetUser"]["id"] == id_bob)
        .unwrap();
    let bob_row = bob_tile["row"].as_u64().unwrap();
    let row_tiles: Vec<_> = tiles
        .iter()
        .filter(|t| t["row"].as_u64().unwrap() == bob_row)
        .collect();

    harness.mail.clear();
    let png = png_bytes_for_test(220, 220);
    let mut bob_capture_id = String::new();
    for tile in row_tiles {
        let tid = tile["id"].as_str().unwrap();
        let (st, cap_out) = harness
            .client
            .post_multipart_file_auth(
                &format!("/api/games/{game_id}/tiles/{tid}/capture"),
                "file",
                "proof.png",
                "image/png",
                png.clone(),
                &t_alice,
            )
            .await;
        assert_eq!(st, StatusCode::OK, "{cap_out}");
        if tid == bob_tile["id"].as_str().unwrap() {
            bob_capture_id = cap_out["capture"]["id"].as_str().unwrap().to_owned();
        }
    }

    // Bob's inbox shows unread count, capture details, and mark-read / mark-all-read updates it.
    let (st, bob_inbox) = harness
        .client
        .get_json_auth("/api/notifications", &t_bob)
        .await;
    assert_eq!(st, StatusCode::OK, "{bob_inbox}");
    let notifs = bob_inbox["notifications"].as_array().unwrap();
    assert!(bob_inbox["unreadCount"].as_u64().unwrap() >= 3);

    let cap_notif = notifs
        .iter()
        .find(|n| n["captureId"].as_str() == Some(bob_capture_id.as_str()))
        .expect("capture notification present");
    assert_eq!(cap_notif["kind"], "CapturePosted");
    assert_eq!(cap_notif["isBlurred"], false);
    assert!(cap_notif["promptText"].as_str().is_some());
    assert_eq!(cap_notif["target"]["id"], id_bob);

    // Carol cannot mark Bob's notification as read -> 403.
    let bob_notif_id = cap_notif["id"].as_str().unwrap();
    let (st, _) = harness
        .client
        .post_json_auth(
            &format!("/api/notifications/{bob_notif_id}/read"),
            json!({}),
            &t_carol,
        )
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Bob marks one read, then marks all read.
    let (st, _) = harness
        .client
        .post_json_auth(
            &format!("/api/notifications/{bob_notif_id}/read"),
            json!({}),
            &t_bob,
        )
        .await;
    assert_eq!(st, StatusCode::OK);

    let (st, bob_inbox_after_one) = harness
        .client
        .get_json_auth("/api/notifications", &t_bob)
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        bob_inbox_after_one["unreadCount"].as_u64().unwrap(),
        bob_inbox["unreadCount"].as_u64().unwrap() - 1
    );

    let (st, _) = harness
        .client
        .post_json_auth("/api/notifications/read-all", json!({}), &t_bob)
        .await;
    assert_eq!(st, StatusCode::OK);

    let (st, bob_inbox_after_all) = harness
        .client
        .get_json_auth("/api/notifications", &t_bob)
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(bob_inbox_after_all["unreadCount"], 0);

    // Logged-out unsubscribe using token from Bob's capture mail disables ONLY emailCapturePosted.
    let bob_cap_mail = harness
        .mail
        .sent()
        .into_iter()
        .find(|m| {
            m.empfänger.as_ref() == "n-bob@example.at"
                && m.text(FRONTEND_BASIS_URL).contains(&bob_capture_id)
        })
        .expect("Bob received capture mail");
    let unsub_url = bob_cap_mail
        .unsubscribe_url(FRONTEND_BASIS_URL)
        .expect("has unsubscribe url");
    let unsub_token = unsub_url.split("token=").nth(1).unwrap().to_owned();

    let (st, unsub_res) = harness
        .client
        .post_json(
            "/api/notifications/unsubscribe",
            json!({ "token": unsub_token }),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{unsub_res}");
    assert_eq!(unsub_res["kind"], "CapturePosted");

    let (st, bob_prefs_after) = harness
        .client
        .get_json_auth("/api/notifications/preferences", &t_bob)
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(bob_prefs_after["emailCapturePosted"], false);
    assert_eq!(bob_prefs_after["emailGameCompleted"], true);

    // BlurredTeaser mode does NOT expose target or promptText in inbox or mail while active.
    let (blur_game_id, _) = enroll_and_start_game(
        &harness,
        &t_alice,
        &[&t_bob, &t_carol],
        "Blurred Round",
        3,
        "BlurredTeaser",
    )
    .await;
    let (st, blur_detail) = harness
        .client
        .get_json_auth(&format!("/api/games/{blur_game_id}"), &t_alice)
        .await;
    assert_eq!(st, StatusCode::OK);
    let blur_bob_tile = blur_detail["myBoard"]["tiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["targetUser"]["id"] == id_bob)
        .unwrap();
    let blur_tid = blur_bob_tile["id"].as_str().unwrap();

    harness.mail.clear();
    let (st, blur_cap_out) = harness
        .client
        .post_multipart_file_auth(
            &format!("/api/games/{blur_game_id}/tiles/{blur_tid}/capture"),
            "file",
            "blur.png",
            "image/png",
            png,
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{blur_cap_out}");
    let blur_cap_id = blur_cap_out["capture"]["id"].as_str().unwrap();

    let (st, bob_blur_inbox) = harness
        .client
        .get_json_auth("/api/notifications", &t_bob)
        .await;
    assert_eq!(st, StatusCode::OK);
    let blur_notif = bob_blur_inbox["notifications"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["captureId"].as_str() == Some(blur_cap_id))
        .expect("blurred teaser notif present");
    assert_eq!(blur_notif["isBlurred"], true);
    assert!(blur_notif["target"].is_null());
    assert!(blur_notif["promptText"].is_null());

    // Per-game watch toggle via HTTP PUT /api/games/:id/watch.
    let (st, unwatched_detail) = harness
        .client
        .put_json_auth(
            &format!("/api/games/{blur_game_id}/watch"),
            json!({ "watching": false }),
            &t_bob,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{unwatched_detail}");
    assert_eq!(unwatched_detail["watching"], false);
}

#[test_log::test(tokio::test)]
async fn e2e_failing_mail_relay_does_not_fail_capture_or_game_completion() {
    let sqlite = SQLiteInstance::in_temp_dir().await.unwrap();
    sqlite.migrate_repos_to_latest().await.unwrap();
    let clock = Arc::new(FixedClock::new(
        DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
    ));
    let app = App::builder()
        .uow_provider(Box::new(sqlite))
        .object_store(Arc::new(FileSystemObjectStore::in_temp_dir().unwrap()))
        .signing(Arc::new(test_signing_key()))
        .throttle(Arc::new(InMemoryThrottle::new()))
        .mail_sender(Arc::new(FailingMailSender))
        .konto(test_konfiguration())
        .clock(Arc::clone(&clock) as Arc<dyn Clock>)
        .rng(Arc::new(FixedRng::default()))
        .build();

    // Create 3 users directly via uow? Better: register via AuthContext using SQLite directly, or
    // use `NeuerBenutzer` + `CreateGame`/`JoinGame`? Let's create users cleanly via `uow.benutzer().create`.
    let now = clock.now();
    let (a, b, c) = app
        .execute_fn(
            friendarazzi_core::application::AuthContext::Anonymous,
            async move |ctx| {
                let uow = ctx.enter().await.unwrap();
                let a = uow
                    .benutzer()
                    .create(friendarazzi_core::domain::NeuerBenutzer::registrierung(
                        friendarazzi_core::domain::EmailAdresse::normalisiert("fa@example.at")
                            .unwrap(),
                        now,
                    ))
                    .await
                    .unwrap()
                    .id();
                let b = uow
                    .benutzer()
                    .create(friendarazzi_core::domain::NeuerBenutzer::registrierung(
                        friendarazzi_core::domain::EmailAdresse::normalisiert("fb@example.at")
                            .unwrap(),
                        now,
                    ))
                    .await
                    .unwrap()
                    .id();
                let c = uow
                    .benutzer()
                    .create(friendarazzi_core::domain::NeuerBenutzer::registrierung(
                        friendarazzi_core::domain::EmailAdresse::normalisiert("fc@example.at")
                            .unwrap(),
                        now,
                    ))
                    .await
                    .unwrap()
                    .id();
                uow.finish(
                    Ok::<_, error_stack::Report<friendarazzi_core::ports::RepositoryError>>((
                        a, b, c,
                    )),
                    friendarazzi_core::ports::RepositoryError::OperationFailed,
                )
                .await
            },
        )
        .await
        .unwrap();

    let auth_a = friendarazzi_core::application::AuthContext::Authenticated {
        benutzer_id: a,
        session_id: friendarazzi_core::domain::konto::SessionId(uuid::Uuid::new_v4()),
    };
    let auth_b = friendarazzi_core::application::AuthContext::Authenticated {
        benutzer_id: b,
        session_id: friendarazzi_core::domain::konto::SessionId(uuid::Uuid::new_v4()),
    };
    let auth_c = friendarazzi_core::application::AuthContext::Authenticated {
        benutzer_id: c,
        session_id: friendarazzi_core::domain::konto::SessionId(uuid::Uuid::new_v4()),
    };

    let game = app
        .execute(
            auth_a,
            CreateGame {
                title: NonEmptyString::neu("FailMail").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
            },
        )
        .await
        .unwrap();
    app.execute(
        auth_b,
        JoinGame {
            invite_code: game.invite_code().clone(),
        },
    )
    .await
    .unwrap();
    app.execute(
        auth_c,
        JoinGame {
            invite_code: game.invite_code().clone(),
        },
    )
    .await
    .unwrap();
    app.execute(
        auth_a,
        friendarazzi_core::service::StartDrafting { game_id: game.id() },
    )
    .await
    .unwrap();

    let quotas_and_save = |auth, uid| {
        let app = &app;
        let gid = game.id();
        async move {
            let qs =
                friendarazzi_core::domain::target_quotas_for_author(&[a, b, c], uid, 9).unwrap();
            let mut prompts = Vec::new();
            for (target, count) in qs {
                for idx in 0..count {
                    prompts.push(friendarazzi_core::service::DraftPromptInput {
                        target_user_id: target,
                        text: NonEmptyString::neu(format!("Auftrag {idx}")).unwrap(),
                        source: friendarazzi_core::domain::PromptSource::Custom,
                    });
                }
            }
            app.execute(
                auth,
                friendarazzi_core::service::SaveDraftPrompts {
                    game_id: gid,
                    prompts,
                },
            )
            .await
            .unwrap();
            app.execute(
                auth,
                friendarazzi_core::service::LockDraftPrompts { game_id: gid },
            )
            .await
            .unwrap();
        }
    };
    quotas_and_save(auth_a, a).await;
    quotas_and_save(auth_b, b).await;
    quotas_and_save(auth_c, c).await;

    clock.advance_by(Duration::seconds(61));
    let start_out = app
        .execute(
            friendarazzi_core::application::AuthContext::Anonymous,
            friendarazzi_core::service::StartReadyGames,
        )
        .await
        .unwrap();
    assert_eq!(start_out.started_games, vec![game.id()]);

    let gid = game.id();
    let tiles = app
        .execute_fn(auth_a, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let board = uow.boards().find_by_game_and_owner(gid, a).await.unwrap();
            let res = board.into_data().tiles().to_vec();
            uow.finish(
                Ok::<_, error_stack::Report<friendarazzi_core::ports::RepositoryError>>(res),
                friendarazzi_core::ports::RepositoryError::OperationFailed,
            )
            .await
        })
        .await
        .unwrap();

    let png = png_bytes_for_test(160, 160);
    for t in tiles.iter().filter(|t| t.row() == 0) {
        app.execute(
            auth_a,
            SubmitCapture {
                game_id: gid,
                tile_id: t.id(),
                image_bytes: png.clone(),
            },
        )
        .await
        .expect("failing mail relay must never fail capture or game completion");
    }
}
