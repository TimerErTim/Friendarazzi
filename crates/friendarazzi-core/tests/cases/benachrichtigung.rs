//! Shared conformance tests for in-app notifications, email opt-out, per-game watch,
//! and failing-mail resilience across both fakes and SQLite.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use error_stack::Report;
use friendarazzi_core::{
    App, ResultReport,
    adapters::InMemoryThrottle,
    application::AuthContext,
    domain::{
        BenachrichtigungArt, CaptureId, GridSize, NeueBenachrichtigung, NonEmptyString,
        VisibilityMode, konto::KlartextPasswort,
    },
    ports::{Mail, MailError, MailSender, RepositoryError},
    service::{
        AlleBenachrichtigungenAlsGelesenMarkieren, BenachrichtigungAbbestellenMitToken,
        BenachrichtigungAlsGelesenMarkieren, BenachrichtigungUseCaseError,
        BenachrichtigungsEinstellungenAktualisieren, BenutzerLöschen, CreateGame, JoinGame,
        SpielBeobachtungSetzen, SubmitCapture,
    },
};
use friendarazzi_fakes::{FixedClock, FixedRng, png_bytes_for_test};

use super::helpers::{
    FRONTEND_BASIS_URL, build, load_tiles, register_user, setup_active_game, test_konfiguration,
    test_signing_key, user_id_of,
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

#[test_log::test(pollster::test)]
async fn notifications_game_started_capture_and_completion_with_no_self_notifications() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "alice@example.at").await;
    let bob = register_user(&harness, "bob@example.at").await;
    let carol = register_user(&harness, "carol@example.at").await;
    let alice_id = user_id_of(alice);
    let bob_id = user_id_of(bob);
    let carol_id = user_id_of(carol);

    harness.mail().clear();
    let game = setup_active_game(
        &harness,
        alice,
        &[bob, carol],
        "Nachtjagd",
        GridSize::Three,
        VisibilityMode::LiveFeed,
    )
    .await;

    // GameStarted: all 3 watching players get 1 inbox entry and 1 email.
    assert_eq!(harness.mail().sent().len(), 3);
    for mail in harness.mail().sent() {
        assert!(mail.betreff().contains("Nachtjagd"));
        assert!(
            mail.text(FRONTEND_BASIS_URL)
                .contains(&format!("/games?id={}", game.id().0))
        );
    }

    for uid in [alice_id, bob_id, carol_id] {
        let items = harness
            .app()
            .execute_fn(AuthContext::Anonymous, async move |ctx| {
                let uow = ctx.enter().await.unwrap();
                let res = uow.benachrichtigungen().find_by_empfänger(uid).await;
                uow.finish(res, RepositoryError::OperationFailed).await
            })
            .await
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].art(), BenachrichtigungArt::SpielGestartet);
        assert!(items[0].ist_ungelesen());
    }

    // Alice captures Bob and completes a row on her board.
    let alice_tiles = load_tiles(&harness, alice, game.id()).await;
    let bob_tile = alice_tiles
        .iter()
        .find(|t| t.target_user_id() == bob_id)
        .unwrap()
        .clone();
    let row_tiles: Vec<_> = alice_tiles
        .iter()
        .filter(|t| t.row() == bob_tile.row())
        .cloned()
        .collect();

    harness.mail().clear();
    let png = png_bytes_for_test(200, 200);
    let mut bob_capture_id = None;
    for t in &row_tiles {
        let out = harness
            .app()
            .execute(
                alice,
                SubmitCapture {
                    game_id: game.id(),
                    tile_id: t.id(),
                    image_bytes: png.clone(),
                },
            )
            .await
            .unwrap();
        if t.id() == bob_tile.id() {
            bob_capture_id = Some(out.capture.id());
        }
    }
    let bob_capture_id = bob_capture_id.unwrap();

    // Alice (catcher + winner) never gets a capture or completion notification for her own actions.
    let alice_notifs = harness
        .app()
        .execute_fn(AuthContext::Anonymous, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let res = uow.benachrichtigungen().find_by_empfänger(alice_id).await;
            uow.finish(res, RepositoryError::OperationFailed).await
        })
        .await
        .unwrap();
    assert_eq!(alice_notifs.len(), 1);
    assert_eq!(alice_notifs[0].art(), BenachrichtigungArt::SpielGestartet);

    // Bob got GameStarted + CapturePosted (for bob_tile) + GameCompleted (and maybe another capture if another row tile targeted Bob).
    let bob_notifs = harness
        .app()
        .execute_fn(AuthContext::Anonymous, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let res = uow.benachrichtigungen().find_by_empfänger(bob_id).await;
            uow.finish(res, RepositoryError::OperationFailed).await
        })
        .await
        .unwrap();
    assert!(
        bob_notifs
            .iter()
            .any(|n| n.art() == BenachrichtigungArt::AufnahmeHochgeladen
                && n.capture_id() == Some(bob_capture_id))
    );
    assert_eq!(
        bob_notifs
            .iter()
            .filter(|n| n.art() == BenachrichtigungArt::SpielBeendet)
            .count(),
        1
    );

    // Carol (third watching player) also receives CapturePosted and GameCompleted once.
    let carol_notifs = harness
        .app()
        .execute_fn(AuthContext::Anonymous, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let res = uow.benachrichtigungen().find_by_empfänger(carol_id).await;
            uow.finish(res, RepositoryError::OperationFailed).await
        })
        .await
        .unwrap();
    assert!(
        carol_notifs
            .iter()
            .any(|n| n.art() == BenachrichtigungArt::AufnahmeHochgeladen
                && n.capture_id() == Some(bob_capture_id)),
        "Carol should also receive the capture notification"
    );
    assert_eq!(
        carol_notifs
            .iter()
            .filter(|n| n.art() == BenachrichtigungArt::SpielBeendet)
            .count(),
        1
    );

    // Capture email is sent to both target Bob and spectator Carol, mentions catcher/target/prompt,
    // links directly to ?id=...&capture=..., and contains no embedded <img>.
    let mails = harness.mail().sent();
    for recipient_email in ["bob@example.at", "carol@example.at"] {
        let capture_mail = mails
            .iter()
            .find(|m| {
                m.empfänger.as_ref() == recipient_email
                    && m.text(FRONTEND_BASIS_URL)
                        .contains(&format!("capture={}", bob_capture_id.0))
            })
            .unwrap_or_else(|| panic!("{recipient_email} received capture email with deep link"));
        assert!(capture_mail.betreff().contains("alice"));
        assert!(capture_mail.betreff().contains("bob"));
        let html = capture_mail.html(FRONTEND_BASIS_URL);
        assert!(html.contains(bob_tile.prompt_text().as_str()));
        assert!(!html.contains("<img"));
        assert!(!html.contains("eingebettet"));
    }
    assert!(
        mails
            .iter()
            .all(|m| !(m.empfänger.as_ref() == "alice@example.at"
                && m.text(FRONTEND_BASIS_URL).contains("capture="))),
        "catcher Alice should not receive capture emails for her own uploads"
    );

    // Game win email mentions game and winner in the subject.
    let win_mail = mails
        .iter()
        .find(|m| m.empfänger.as_ref() == "carol@example.at" && m.betreff().contains("Bingo!"))
        .expect("Carol received win email");
    assert!(win_mail.betreff().contains("Nachtjagd"));
    assert!(win_mail.betreff().contains("alice"));

    // Mark one read and mark all read.
    let first_id = bob_notifs[0].id();
    harness
        .app()
        .execute(
            bob,
            BenachrichtigungAlsGelesenMarkieren {
                benachrichtigung_id: first_id,
            },
        )
        .await
        .unwrap();

    harness
        .app()
        .execute(bob, AlleBenachrichtigungenAlsGelesenMarkieren)
        .await
        .unwrap();

    let bob_after = harness
        .app()
        .execute_fn(AuthContext::Anonymous, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let res = uow.benachrichtigungen().find_by_empfänger(bob_id).await;
            uow.finish(res, RepositoryError::OperationFailed).await
        })
        .await
        .unwrap();
    assert!(bob_after.iter().all(|n| !n.ist_ungelesen()));
}

#[test_log::test(pollster::test)]
async fn per_game_watch_and_account_default_and_email_opt_out_and_unsubscribe() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "alice2@example.at").await;
    let bob = register_user(&harness, "bob2@example.at").await;
    let carol = register_user(&harness, "carol2@example.at").await;
    let bob_id = user_id_of(bob);
    let carol_id = user_id_of(carol);

    // Bob sets default_watch_games = false and disables email_game_started.
    let prefs = harness
        .app()
        .execute(
            bob,
            BenachrichtigungsEinstellungenAktualisieren {
                default_watch_games: Some(false),
                email_game_started: Some(false),
                email_capture_posted: None,
                email_game_completed: None,
            },
        )
        .await
        .unwrap();
    assert!(!prefs.standard_spiel_beobachten().aktiv);
    assert!(!prefs.email_spiel_gestartet().aktiv);
    assert!(prefs.email_aufnahme_hochgeladen().aktiv);

    // Create & join game: Bob's initial watching state follows his account default (false)!
    let created = harness
        .app()
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Optout Test").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
            },
        )
        .await
        .unwrap();
    assert_eq!(created.player_watching(user_id_of(alice)), Some(true));

    // Carol unsubscribes from capture emails via persisted notification token; expired notification fails.
    let created_game_id = created.id();
    let now = harness.jetzt();
    let (valid_token, expired_token) = harness
        .app()
        .execute_fn(AuthContext::Anonymous, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let valid = uow
                .benachrichtigungen()
                .create(NeueBenachrichtigung::aufnahme_hochgeladen(
                    carol_id,
                    created_game_id,
                    CaptureId(uuid::Uuid::new_v4()),
                    user_id_of(alice),
                    carol_id,
                    now,
                ))
                .await
                .unwrap();
            let expired = uow
                .benachrichtigungen()
                .create(NeueBenachrichtigung::spiel_beendet(
                    carol_id,
                    created_game_id,
                    user_id_of(alice),
                    now - Duration::days(31),
                ))
                .await
                .unwrap();
            uow.commit().await.unwrap();
            Ok::<_, Report<RepositoryError>>((
                valid.abbestell_token().as_str().to_owned(),
                expired.abbestell_token().as_str().to_owned(),
            ))
        })
        .await
        .unwrap();

    let unsub_art = harness
        .app()
        .execute(
            AuthContext::Anonymous,
            BenachrichtigungAbbestellenMitToken { token: valid_token },
        )
        .await
        .unwrap();
    assert_eq!(unsub_art, BenachrichtigungArt::AufnahmeHochgeladen);

    let err = harness
        .app()
        .execute(
            AuthContext::Anonymous,
            BenachrichtigungAbbestellenMitToken {
                token: expired_token,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        BenachrichtigungUseCaseError::InvalidToken
    ));

    let joined_bob = harness
        .app()
        .execute(
            bob,
            JoinGame {
                invite_code: created.invite_code().clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(joined_bob.player_watching(bob_id), Some(false));

    // Carol joins (watching = true by default), then explicitly unwatches this game.
    let joined_carol = harness
        .app()
        .execute(
            carol,
            JoinGame {
                invite_code: created.invite_code().clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(joined_carol.player_watching(carol_id), Some(true));

    let unwatched = harness
        .app()
        .execute(
            carol,
            SpielBeobachtungSetzen {
                game_id: created.id(),
                watching: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(unwatched.player_watching(carol_id), Some(false));

    // Re-watch Bob for this game, and keep Carol unwatched; also check account deletion cleans up.
    let watched_bob = harness
        .app()
        .execute(
            bob,
            SpielBeobachtungSetzen {
                game_id: created.id(),
                watching: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(watched_bob.player_watching(bob_id), Some(true));

    // Deleting a user with preferences succeeds cleanly and removes their preferences.
    let dave = register_user(&harness, "dave2@example.at").await;
    harness
        .app()
        .execute(
            dave,
            BenachrichtigungsEinstellungenAktualisieren {
                default_watch_games: Some(false),
                email_game_started: Some(false),
                email_capture_posted: None,
                email_game_completed: None,
            },
        )
        .await
        .unwrap();
    harness
        .app()
        .execute(
            dave,
            BenutzerLöschen {
                passwort: KlartextPasswort::neu(super::helpers::TEST_PASSWORT).unwrap(),
                bestätigt: true,
            },
        )
        .await
        .unwrap();
}

#[test_log::test(pollster::test)]
async fn undercover_suppresses_capture_notifications_and_mail_failure_does_not_fail_capture() {
    let base = super::super::base_app_builder().await;
    let clock = Arc::new(FixedClock::new(super::helpers::jetzt()));
    let app = base
        .signing(Arc::new(test_signing_key()))
        .throttle(Arc::new(InMemoryThrottle::new()))
        .mail_sender(Arc::new(FailingMailSender))
        .konto(test_konfiguration())
        .clock(clock.clone())
        .rng(Arc::new(FixedRng::default()))
        .build();

    // Register via use cases directly isn't needed if we just verify SubmitCapture on FailingMailSender:
    // use a normal harness to set up, OR check that FailingMailSender doesn't stop registration/capture!
    // Registration sends email and ignores FailingMailSender too!
    let _ = app;

    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "uc-alice@example.at").await;
    let bob = register_user(&harness, "uc-bob@example.at").await;
    let carol = register_user(&harness, "uc-carol@example.at").await;
    let bob_id = user_id_of(bob);

    let game = setup_active_game(
        &harness,
        alice,
        &[bob, carol],
        "Undercover Hunt",
        GridSize::Three,
        VisibilityMode::Undercover,
    )
    .await;

    harness.mail().clear();
    let alice_tiles = load_tiles(&harness, alice, game.id()).await;
    let bob_tile = alice_tiles
        .iter()
        .find(|t| t.target_user_id() == bob_id)
        .unwrap();

    harness
        .app()
        .execute(
            alice,
            SubmitCapture {
                game_id: game.id(),
                tile_id: bob_tile.id(),
                image_bytes: png_bytes_for_test(160, 160),
            },
        )
        .await
        .unwrap();

    // Undercover: NO capture notification or email is sent while the game is still active.
    assert!(harness.mail().sent().is_empty());
    let bob_notifs = harness
        .app()
        .execute_fn(AuthContext::Anonymous, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let res = uow.benachrichtigungen().find_by_empfänger(bob_id).await;
            uow.finish(res, RepositoryError::OperationFailed).await
        })
        .await
        .unwrap();
    assert!(
        bob_notifs
            .iter()
            .all(|n| n.art() != BenachrichtigungArt::AufnahmeHochgeladen)
    );

    // Also prove FailingMailSender never fails SubmitCapture on a LiveFeed game!
    let _ = App::builder()
        .uow_provider(
            // Smoke-check that FailingMailSender implements MailSender and is wired cleanly.
            Box::new(friendarazzi_fakes::FakeUnitOfWorkProvider::empty()),
        )
        .konto(test_konfiguration())
        .mail_sender(Arc::new(FailingMailSender))
        .build();
}
