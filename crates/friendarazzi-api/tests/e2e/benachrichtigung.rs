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
    E2eHarness, FRONTEND_BASIS_URL, PASSWORT, enroll_and_start_game, mailpit, test_konfiguration,
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
    mailpit::leeren().await;
    let harness = E2eHarness::mit_smtp().await;
    let (t_alice, _id_alice, _) = register_via_mailpit(&harness, "n-alice@example.at").await;
    let (t_bob, id_bob, _) = register_via_mailpit(&harness, "n-bob@example.at").await;
    let (t_carol, id_carol, _) = register_via_mailpit(&harness, "n-carol@example.at").await;

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

    // Start a LiveFeed game: Alice, Bob, Carol get GameStarted inbox entries; only Alice and Bob get GameStarted email in Mailpit.
    mailpit::leeren().await;
    let (game_id, _invite_code) = enroll_and_start_game(
        &harness,
        &t_alice,
        &[&t_bob, &t_carol],
        "Live Hunt",
        3,
        "LiveFeed",
    )
    .await;

    assert!(
        !mailpit::nachrichten_an("n-alice@example.at")
            .await
            .is_empty()
    );
    assert!(!mailpit::nachrichten_an("n-bob@example.at").await.is_empty());
    assert!(
        mailpit::nachrichten_an("n-carol@example.at")
            .await
            .is_empty()
    );

    let (st, carol_inbox) = harness
        .client
        .get_json_auth("/api/notifications", &t_carol)
        .await;
    assert_eq!(st, StatusCode::OK, "{carol_inbox}");
    assert_eq!(carol_inbox["unreadCount"], 1);
    assert_eq!(carol_inbox["notifications"][0]["kind"], "GameStarted");

    // Alice captures Bob and finishes a row -> BOTH Bob and Carol get CapturePosted + GameCompleted in inbox and Mailpit.
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
    let bob_prompt = bob_tile["promptText"].as_str().unwrap().to_owned();
    let bob_row = bob_tile["row"].as_u64().unwrap();
    let row_tiles: Vec<_> = tiles
        .iter()
        .filter(|t| t["row"].as_u64().unwrap() == bob_row)
        .collect();

    mailpit::leeren().await;
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

    assert!(
        mailpit::nachrichten_an("n-alice@example.at")
            .await
            .is_empty()
    );
    let capture_link_suffix = format!("/games?id={game_id}&capture={bob_capture_id}");
    let capture_html_suffix = format!("/games?id={game_id}&amp;capture={bob_capture_id}");
    let mut bob_cap_mail_unsub = String::new();
    for email in ["n-bob@example.at", "n-carol@example.at"] {
        let msgs = mailpit::nachrichten_an(email).await;
        let cap_mail = msgs
            .iter()
            .find(|m| m.text.contains(&capture_link_suffix))
            .unwrap_or_else(|| panic!("{email} should receive CapturePosted email via Mailpit"));
        assert!(cap_mail.text.contains(&bob_prompt));
        assert!(cap_mail.html.contains(&bob_prompt));
        assert!(cap_mail.html.contains(&capture_html_suffix));
        assert!(!cap_mail.html.contains("eingebettet"));
        assert_html_has_website_brand_header_and_no_embedded_img(&cap_mail.html);

        let win_mail = msgs
            .iter()
            .find(|m| m.subject.contains("Bingo!"))
            .unwrap_or_else(|| panic!("{email} should receive GameCompleted email via Mailpit"));
        assert!(win_mail.subject.contains("Live Hunt"));
        assert!(win_mail.subject.contains("n-alice"));
        assert!(win_mail.text.contains(&format!("/games?id={game_id}")));
        assert!(win_mail.html.contains(&format!("/games?id={game_id}")));
        assert_html_has_website_brand_header_and_no_embedded_img(&win_mail.html);

        if email == "n-bob@example.at" {
            bob_cap_mail_unsub = cap_mail
                .header_first("List-Unsubscribe")
                .expect("List-Unsubscribe header")
                .trim_matches(|c| c == '<' || c == '>')
                .split("token=")
                .nth(1)
                .unwrap()
                .to_owned();
        }
    }

    // Bob's and Carol's inboxes both contain the CapturePosted notification.
    let (st, bob_inbox) = harness
        .client
        .get_json_auth("/api/notifications", &t_bob)
        .await;
    assert_eq!(st, StatusCode::OK, "{bob_inbox}");
    let notifs = bob_inbox["notifications"].as_array().unwrap();
    assert!(bob_inbox["unreadCount"].as_u64().unwrap() >= 3);

    let (st, carol_cap_inbox) = harness
        .client
        .get_json_auth("/api/notifications", &t_carol)
        .await;
    assert_eq!(st, StatusCode::OK, "{carol_cap_inbox}");
    let carol_cap = carol_cap_inbox["notifications"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["captureId"].as_str() == Some(bob_capture_id.as_str()))
        .expect("Carol inbox also has the capture notification");
    assert_eq!(carol_cap["kind"], "CapturePosted");
    assert_eq!(carol_cap["target"]["id"], id_bob);

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

    // Logged-out unsubscribe using token from Bob's Mailpit email disables ONLY emailCapturePosted.
    let (st, unsub_res) = harness
        .client
        .post_json(
            "/api/notifications/unsubscribe",
            json!({ "token": bob_cap_mail_unsub }),
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

    // BlurredTeaser mode does NOT expose target or promptText in inbox or Mailpit email while active.
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
    let alice_tiles = blur_detail["myBoard"]["tiles"].as_array().unwrap();
    let blur_bob_tile = alice_tiles
        .iter()
        .find(|t| t["targetUser"]["id"] == id_bob)
        .unwrap();
    let blur_tid = blur_bob_tile["id"].as_str().unwrap();
    let blur_bob_prompt = blur_bob_tile["promptText"].as_str().unwrap();
    let blur_carol_tile = alice_tiles
        .iter()
        .find(|t| t["targetUser"]["id"] == id_carol)
        .unwrap();

    mailpit::leeren().await;
    let (st, blur_cap_out) = harness
        .client
        .post_multipart_file_auth(
            &format!("/api/games/{blur_game_id}/tiles/{blur_tid}/capture"),
            "file",
            "blur.png",
            "image/png",
            png.clone(),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{blur_cap_out}");
    let blur_cap_id = blur_cap_out["capture"]["id"].as_str().unwrap();

    // Bob unsubscribed from CapturePosted emails, while watching Carol receives the redacted teaser email in Mailpit.
    assert!(mailpit::nachrichten_an("n-bob@example.at").await.is_empty());
    let carol_teaser = mailpit::letzte_nachricht_an("n-carol@example.at").await;
    assert!(carol_teaser.subject.contains("Teaser"));
    assert!(!carol_teaser.text.contains(blur_bob_prompt));
    assert!(!carol_teaser.html.contains(blur_bob_prompt));
    assert!(!carol_teaser.html.contains("n-bob"));
    assert!(!carol_teaser.html.contains("Datenschutz"));
    assert!(carol_teaser.html.contains("TEASER-FOTO"));
    assert_html_has_website_brand_header_and_no_embedded_img(&carol_teaser.html);

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

    // Carol un-watches this game via HTTP PUT /api/games/:id/watch; subsequent capture sends no inbox or email to Carol.
    let (st, unwatched_detail) = harness
        .client
        .put_json_auth(
            &format!("/api/games/{blur_game_id}/watch"),
            json!({ "watching": false }),
            &t_carol,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{unwatched_detail}");
    assert_eq!(unwatched_detail["watching"], false);

    mailpit::leeren().await;
    let blur_carol_tid = blur_carol_tile["id"].as_str().unwrap();
    let (st, second_blur_cap) = harness
        .client
        .post_multipart_file_auth(
            &format!("/api/games/{blur_game_id}/tiles/{blur_carol_tid}/capture"),
            "file",
            "blur2.png",
            "image/png",
            png,
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{second_blur_cap}");
    let second_blur_cap_id = second_blur_cap["capture"]["id"].as_str().unwrap();

    assert!(
        mailpit::nachrichten_an("n-carol@example.at")
            .await
            .is_empty()
    );
    let (st, carol_inbox_after_unwatch) = harness
        .client
        .get_json_auth("/api/notifications", &t_carol)
        .await;
    assert_eq!(st, StatusCode::OK);
    assert!(
        carol_inbox_after_unwatch["notifications"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| n["captureId"].as_str() != Some(second_blur_cap_id))
    );

    // Complete Blurred Round and verify Bob's BlurredTeaser inbox entry reveals target + promptText.
    if second_blur_cap["winningLine"].is_null() {
        let blur_bob_row = blur_bob_tile["row"].as_u64().unwrap();
        for tile in alice_tiles.iter().filter(|t| {
            let tid = t["id"].as_str().unwrap();
            t["row"].as_u64().unwrap() == blur_bob_row && tid != blur_tid && tid != blur_carol_tid
        }) {
            let tid = tile["id"].as_str().unwrap();
            let (st, out) = harness
                .client
                .post_multipart_file_auth(
                    &format!("/api/games/{blur_game_id}/tiles/{tid}/capture"),
                    "file",
                    "win.png",
                    "image/png",
                    png_bytes_for_test(220, 220),
                    &t_alice,
                )
                .await;
            assert_eq!(st, StatusCode::OK, "{out}");
            if !out["winningLine"].is_null() {
                break;
            }
        }
    }
    let (st, bob_revealed_inbox) = harness
        .client
        .get_json_auth("/api/notifications", &t_bob)
        .await;
    assert_eq!(st, StatusCode::OK);
    let revealed_blur_notif = bob_revealed_inbox["notifications"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["captureId"].as_str() == Some(blur_cap_id))
        .expect("blurred teaser notif present after completion");
    assert_eq!(revealed_blur_notif["isBlurred"], false);
    assert_eq!(revealed_blur_notif["target"]["id"], id_bob);
    assert_eq!(revealed_blur_notif["promptText"], blur_bob_prompt);

    // HTTP 400 / 401 / 404 contracts + defaultWatchGames = false via HTTP POST /api/games and /api/games/join.
    let (st, _) = harness.client.get_json("/api/notifications").await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = harness
        .client
        .post_json(
            "/api/notifications/unsubscribe",
            json!({ "token": "invalid-token" }),
        )
        .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, _) = harness
        .client
        .post_json_auth(
            &format!("/api/notifications/{}/read", uuid::Uuid::new_v4()),
            json!({}),
            &t_bob,
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    let (st, _) = harness
        .client
        .put_json_auth(
            "/api/notifications/preferences",
            json!({ "defaultWatchGames": false }),
            &t_bob,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    let (st, created_unwatched) = harness
        .client
        .post_json_auth(
            "/api/games",
            json!({
                "title": "Unwatched Default",
                "gridSize": 3,
                "visibilityMode": "LiveFeed"
            }),
            &t_bob,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{created_unwatched}");
    assert_eq!(created_unwatched["watching"], false);
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
    let started_draft = app
        .execute(
            auth_a,
            friendarazzi_core::service::StartDrafting { game_id: game.id() },
        )
        .await
        .unwrap();
    let player_ids = started_draft.sorted_player_ids();

    let quotas_and_save = |auth, uid| {
        let app = &app;
        let gid = game.id();
        let player_ids = &player_ids;
        async move {
            let qs =
                friendarazzi_core::domain::target_quotas_for_author(player_ids, uid, 9).unwrap();
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

fn assert_html_has_website_brand_header_and_no_embedded_img(html: &str) {
    assert!(!html.contains("<img"), "email HTML must not embed images");
    assert!(
        html.contains("box-shadow:2px 2px 0 #0d0d0d")
            && html.contains("<span style=\"color:#ffe600;\">F</span>")
            && html.contains("<span style=\"color:#ff3b9a;\">e</span>")
            && html.contains("<span style=\"color:#00e5ff;\">i</span>"),
        "expected 3x3 BrandMark and alternating Wordmark letters in HTML: {html}"
    );
    assert!(!html.contains(">Friend<"));
}

async fn register_via_mailpit(
    harness: &E2eHarness,
    email: &str,
) -> (String, String, mailpit::MailpitNachricht) {
    let (st, _) = harness
        .client
        .post_json("/api/registrierung/start", json!({ "eMailAdresse": email }))
        .await;
    assert_eq!(st, StatusCode::OK);

    let msg = mailpit::letzte_nachricht_an(email).await;
    let token = mailpit::token_aus(&msg.text);
    let (st, body, _) = harness
        .client
        .post_json_cookie(
            "/api/registrierung/abschluss",
            json!({ "token": token, "passwort": PASSWORT }),
            None,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{body}");

    let access = body["accessToken"].as_str().unwrap().to_owned();
    let (st, benutzer) = harness.client.get_json_auth("/api/benutzer", &access).await;
    assert_eq!(st, StatusCode::OK);
    let benutzer_id = benutzer["id"].as_str().unwrap().to_owned();
    (access, benutzer_id, msg)
}

#[test_log::test(tokio::test)]
async fn e2e_mailpit_delivers_html_and_text_emails_across_registration_and_notifications() {
    mailpit::leeren().await;
    let harness = E2eHarness::mit_smtp().await;

    // Registration mails arrive via SMTP with both text and website-branded HTML.
    let (t_alice, _id_alice, reg_alice) =
        register_via_mailpit(&harness, "mp-alice@example.at").await;
    assert_eq!(reg_alice.subject, "Bestätige deine E-Mail-Adresse");
    assert!(
        reg_alice
            .text
            .contains(&format!("{FRONTEND_BASIS_URL}/registrierung?token="))
    );
    assert!(
        reg_alice
            .html
            .contains(&format!("{FRONTEND_BASIS_URL}/registrierung?token="))
    );
    assert_html_has_website_brand_header_and_no_embedded_img(&reg_alice.html);
    assert!(
        reg_alice
            .header_first("Content-Type")
            .is_some_and(|v| v.contains("multipart/alternative"))
    );

    let (t_bob, id_bob, _) = register_via_mailpit(&harness, "mp-bob@example.at").await;
    let (t_carol, _id_carol, _) = register_via_mailpit(&harness, "mp-carol@example.at").await;

    // Password recovery also delivers multipart HTML + plain text via Mailpit.
    let (st, _) = harness
        .client
        .post_json(
            "/api/passwort/vergessen",
            json!({ "eMailAdresse": "mp-alice@example.at" }),
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    let reset_mail = mailpit::letzte_nachricht_an("mp-alice@example.at").await;
    assert_eq!(reset_mail.subject, "Setze dein Passwort zurück");
    assert!(
        reset_mail
            .text
            .contains(&format!("{FRONTEND_BASIS_URL}/passwort?token="))
    );
    assert!(
        reset_mail
            .html
            .contains(&format!("{FRONTEND_BASIS_URL}/passwort?token="))
    );
    assert_html_has_website_brand_header_and_no_embedded_img(&reset_mail.html);

    // Carol opts out of GameStarted email; Bob keeps defaults enabled.
    let (st, _) = harness
        .client
        .put_json_auth(
            "/api/notifications/preferences",
            json!({ "emailGameStarted": false }),
            &t_carol,
        )
        .await;
    assert_eq!(st, StatusCode::OK);

    // Clear Mailpit before game lifecycle notifications.
    mailpit::leeren().await;

    let (game_id, _) = enroll_and_start_game(
        &harness,
        &t_alice,
        &[&t_bob, &t_carol],
        "Mailpit Hunt",
        3,
        "LiveFeed",
    )
    .await;

    // GameStarted: Bob and Alice receive multipart HTML + text email; Carol does not.
    assert!(
        mailpit::nachrichten_an("mp-carol@example.at")
            .await
            .is_empty()
    );
    let bob_started = mailpit::letzte_nachricht_an("mp-bob@example.at").await;
    assert!(bob_started.subject.contains("Mailpit Hunt"));
    let game_link = format!("{FRONTEND_BASIS_URL}/games?id={game_id}");
    assert!(bob_started.text.contains(&game_link));
    assert!(bob_started.html.contains(&game_link));
    assert!(bob_started.html.contains("SPIELSTART"));
    assert_html_has_website_brand_header_and_no_embedded_img(&bob_started.html);
    assert!(
        bob_started
            .header_first("List-Unsubscribe")
            .is_some_and(|h| h.contains("/abbestellen?token="))
    );
    assert!(bob_started.header_first("List-Unsubscribe-Post").is_none());

    // Alice captures Bob and completes a row -> Bob gets CapturePosted (with deep link) and Carol gets GameCompleted.
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
    let bob_prompt = bob_tile["promptText"].as_str().unwrap().to_owned();
    let bob_row = bob_tile["row"].as_u64().unwrap();
    let row_tiles: Vec<_> = tiles
        .iter()
        .filter(|t| t["row"].as_u64().unwrap() == bob_row)
        .collect();

    let png = png_bytes_for_test(200, 200);
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

    let bob_msgs = mailpit::nachrichten_an("mp-bob@example.at").await;
    let capture_link_suffix = format!("/games?id={game_id}&capture={bob_capture_id}");
    let bob_capture_mail = bob_msgs
        .iter()
        .find(|m| m.text.contains(&capture_link_suffix))
        .expect("Bob received capture notification email via Mailpit");
    assert!(bob_capture_mail.subject.contains("mp-alice"));
    assert!(bob_capture_mail.subject.contains("mp-bob"));
    assert!(bob_capture_mail.subject.contains("Mailpit Hunt"));
    assert!(bob_capture_mail.text.contains("mp-alice"));
    assert!(bob_capture_mail.text.contains("mp-bob"));
    assert!(bob_capture_mail.text.contains(&bob_prompt));
    assert!(bob_capture_mail.html.contains(&bob_prompt));
    assert!(
        bob_capture_mail
            .html
            .contains(&format!("/games?id={game_id}&amp;capture={bob_capture_id}"))
    );
    assert_html_has_website_brand_header_and_no_embedded_img(&bob_capture_mail.html);

    let carol_msgs = mailpit::nachrichten_an("mp-carol@example.at").await;
    let carol_capture_mail = carol_msgs
        .iter()
        .find(|m| m.text.contains(&capture_link_suffix))
        .expect("Carol (non-target watching player) also receives CapturePosted email via Mailpit");
    assert!(carol_capture_mail.subject.contains("mp-alice"));
    assert!(carol_capture_mail.subject.contains("mp-bob"));
    assert!(carol_capture_mail.text.contains(&bob_prompt));
    assert!(carol_capture_mail.html.contains(&bob_prompt));
    assert!(!carol_capture_mail.html.contains("eingebettet"));
    assert_html_has_website_brand_header_and_no_embedded_img(&carol_capture_mail.html);
    assert!(
        mailpit::nachrichten_an("mp-alice@example.at")
            .await
            .iter()
            .all(|m| !m.text.contains("capture=")),
        "catcher Alice receives no capture notification emails"
    );

    let carol_win = carol_msgs
        .iter()
        .find(|m| m.subject.contains("Bingo!"))
        .expect("Carol received GameCompleted win email via Mailpit");
    assert!(carol_win.subject.contains("mp-alice"));
    assert!(carol_win.subject.contains("Mailpit Hunt"));
    assert!(carol_win.text.contains(&game_link));
    assert!(carol_win.html.contains(&game_link));
    assert!(carol_win.html.contains("BINGO-SIEG"));
    assert_html_has_website_brand_header_and_no_embedded_img(&carol_win.html);

    // One-click unsubscribe token extracted from List-Unsubscribe header in Mailpit works logged out.
    let unsub_header = bob_capture_mail
        .header_first("List-Unsubscribe")
        .expect("List-Unsubscribe header");
    let unsub_token = unsub_header
        .trim_matches(|c| c == '<' || c == '>')
        .split("token=")
        .nth(1)
        .unwrap()
        .to_owned();
    let (st, unsub_out) = harness
        .client
        .post_json(
            "/api/notifications/unsubscribe",
            json!({ "token": unsub_token }),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{unsub_out}");
    assert_eq!(unsub_out["kind"], "CapturePosted");

    // BlurredTeaser mode also delivers HTML + text via Mailpit without leaking target or prompt.
    let (blur_game_id, _) = enroll_and_start_game(
        &harness,
        &t_alice,
        &[&t_bob, &t_carol],
        "Mailpit Blur",
        3,
        "BlurredTeaser",
    )
    .await;
    let (st, blur_detail) = harness
        .client
        .get_json_auth(&format!("/api/games/{blur_game_id}"), &t_alice)
        .await;
    assert_eq!(st, StatusCode::OK);
    let blur_carol_tile = blur_detail["myBoard"]["tiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["targetUser"]["id"] == _id_carol)
        .unwrap();
    let blur_tid = blur_carol_tile["id"].as_str().unwrap();
    let blur_prompt = blur_carol_tile["promptText"].as_str().unwrap();

    mailpit::leeren().await;
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

    let carol_teaser = mailpit::letzte_nachricht_an("mp-carol@example.at").await;
    assert!(carol_teaser.subject.contains("Teaser"));
    assert!(!carol_teaser.text.contains(blur_prompt));
    assert!(!carol_teaser.html.contains(blur_prompt));
    assert!(!carol_teaser.html.contains("mp-carol"));
    assert!(carol_teaser.html.contains("TEASER-FOTO"));
    assert_html_has_website_brand_header_and_no_embedded_img(&carol_teaser.html);
}
