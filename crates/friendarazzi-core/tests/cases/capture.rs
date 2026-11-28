//! Shared capture submission, bingo evaluation, and award computation conformance tests.

use std::sync::{Arc, Mutex};

use chrono::Duration;
use friendarazzi_core::{
    ResultReport,
    application::AuthContext,
    domain::{
        BenachrichtigungArt, BoardCoordinates, CaptureId, Game, GridSize, NonEmptyString,
        VisibilityMode, compute_game_awards,
    },
    ports::{EventQueue, EventQueueError, RepositoryError, collect_object},
    service::{
        CaptureUseCaseError, CreateGame, Event, HostFillRemainingDrafts, JoinGame, RetractCapture,
        StartDrafting, SubmitCapture, UpdateProfile, handle_event,
    },
};
use friendarazzi_fakes::png_bytes_for_test;
use uuid::Uuid;

use super::helpers::{build, is_webp, load_tiles, register_user, setup_active_game, user_id_of};

#[test_log::test(pollster::test)]
async fn full_game_lifecycle_bingo_awards_gold_medal() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "alice@example.at").await;
    let bob = register_user(&harness, "bob@example.at").await;
    let carol = register_user(&harness, "carol@example.at").await;
    let alice_id = user_id_of(alice);

    let game = setup_active_game(
        &harness,
        alice,
        &[bob, carol],
        "Freitagabend",
        GridSize::Three,
        VisibilityMode::Undercover,
    )
    .await;

    let alice_tiles = load_tiles(&harness, alice, game.id()).await;
    assert_eq!(alice_tiles.len(), 9);
    for t in &alice_tiles {
        assert_ne!(t.target_user_id(), alice_id);
    }

    let png = png_bytes_for_test(320, 240);
    for coord in [(0, 0), (1, 1), (2, 2)] {
        let tid = alice_tiles
            .iter()
            .find(|t| t.row() == coord.0 && t.col() == coord.1)
            .unwrap()
            .id();
        harness.vorspulen(Duration::seconds(15));
        let out = harness
            .app()
            .execute(
                alice,
                SubmitCapture {
                    game_id: game.id(),
                    tile_id: tid,
                    image_bytes: png.clone(),
                },
            )
            .await
            .unwrap();
        assert_eq!(out.winning_line.is_some(), coord == (2, 2));
    }

    let updated = harness
        .app()
        .execute(
            alice,
            UpdateProfile {
                display_name: NonEmptyString::neu("Alice").unwrap(),
                equipped_title: Some(NonEmptyString::neu("Schnappschuss-Jäger").unwrap()),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.gold_medal_count(), 1);
    assert_eq!(updated.effective_title(), "Schnappschuss-Jäger");
}

#[test_log::test(pollster::test)]
async fn capture_validation_duplicates_cross_player_and_post_completion() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "alice@example.at").await;
    let bob = register_user(&harness, "bob@example.at").await;
    let carol = register_user(&harness, "carol@example.at").await;
    let outsider = register_user(&harness, "outsider@example.at").await;

    let game = setup_active_game(
        &harness,
        alice,
        &[bob, carol],
        "Capture Checks",
        GridSize::Three,
        VisibilityMode::LiveFeed,
    )
    .await;

    let alice_tiles = load_tiles(&harness, alice, game.id()).await;
    let bob_tiles = load_tiles(&harness, bob, game.id()).await;

    let tile_at = |tiles: &[friendarazzi_core::domain::BoardTile], r: u8, c: u8| {
        tiles
            .iter()
            .find(|t| t.row() == r && t.col() == c)
            .unwrap()
            .id()
    };
    let alice_00 = tile_at(&alice_tiles, 0, 0);
    let alice_11 = tile_at(&alice_tiles, 1, 1);
    let alice_22 = tile_at(&alice_tiles, 2, 2);
    let bob_00 = tile_at(&bob_tiles, 0, 0);
    let bob_01 = tile_at(&bob_tiles, 0, 1);

    // Corrupt image bytes -> InvalidImage
    let err = harness
        .app()
        .execute(
            alice,
            SubmitCapture {
                game_id: game.id(),
                tile_id: alice_00,
                image_bytes: b"not-a-valid-image".to_vec(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        CaptureUseCaseError::InvalidImage
    ));

    // Outsider without board -> Forbidden
    let png = png_bytes_for_test(120, 90);
    let err = harness
        .app()
        .execute(
            outsider,
            SubmitCapture {
                game_id: game.id(),
                tile_id: alice_00,
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        CaptureUseCaseError::Forbidden
    ));

    // Bob trying to capture Alice's tile -> NotFound on Bob's board
    let err = harness
        .app()
        .execute(
            bob,
            SubmitCapture {
                game_id: game.id(),
                tile_id: alice_00,
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        CaptureUseCaseError::NotFound
    ));

    // Valid capture stores BOTH full and blurred WebP in ObjectStore
    let outcome = harness
        .app()
        .execute(
            alice,
            SubmitCapture {
                game_id: game.id(),
                tile_id: alice_00,
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap();
    assert!(outcome.winning_line.is_none());

    let full_key = outcome.capture.image_storage_key().to_owned();
    let thumb_key = outcome.capture.thumbnail_storage_key().to_owned();
    let (full_bytes, thumb_bytes) = harness
        .app()
        .execute_fn(alice, async move |ctx| {
            let full_stream = ctx.object_store().get(&full_key).await.unwrap().unwrap();
            let thumb_stream = ctx.object_store().get(&thumb_key).await.unwrap().unwrap();
            let full = collect_object(full_stream).await.unwrap();
            let thumb = collect_object(thumb_stream).await.unwrap();
            Ok::<_, error_stack::Report<RepositoryError>>((full, thumb))
        })
        .await
        .unwrap();
    assert!(is_webp(&full_bytes));
    assert!(is_webp(&thumb_bytes));

    // Duplicate capture on same tile -> Conflict
    let err = harness
        .app()
        .execute(
            alice,
            SubmitCapture {
                game_id: game.id(),
                tile_id: alice_00,
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        CaptureUseCaseError::Conflict
    ));

    // Bob makes one valid capture before Alice finishes
    let bob_out = harness
        .app()
        .execute(
            bob,
            SubmitCapture {
                game_id: game.id(),
                tile_id: bob_00,
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap();
    assert!(bob_out.winning_line.is_none());

    // Alice finishes diagonal Bingo -> game Completed
    let mid_out = harness
        .app()
        .execute(
            alice,
            SubmitCapture {
                game_id: game.id(),
                tile_id: alice_11,
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap();
    assert!(mid_out.winning_line.is_none());

    let win_out = harness
        .app()
        .execute(
            alice,
            SubmitCapture {
                game_id: game.id(),
                tile_id: alice_22,
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap();
    assert!(win_out.winning_line.is_some());

    // Further captures after game completion -> Conflict and does not award Bob a medal
    let err = harness
        .app()
        .execute(
            bob,
            SubmitCapture {
                game_id: game.id(),
                tile_id: bob_01,
                image_bytes: png,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        CaptureUseCaseError::Conflict
    ));

    let bob_id = user_id_of(bob);
    let bob_medals = harness
        .app()
        .execute_fn(bob, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let count = uow
                .benutzer()
                .find_by_id(bob_id)
                .await
                .unwrap()
                .into_data()
                .gold_medal_count();
            uow.finish(
                Ok::<_, error_stack::Report<RepositoryError>>(count),
                RepositoryError::Unknown,
            )
            .await
        })
        .await
        .unwrap();
    assert_eq!(bob_medals, 0);
}

#[test_log::test(pollster::test)]
async fn capture_bingo_detection_covers_row_col_and_antidiagonal_for_non_host_winner() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "alice@example.at").await;
    let bob = register_user(&harness, "bob@example.at").await;
    let carol = register_user(&harness, "carol@example.at").await;
    let bob_id = user_id_of(bob);
    let carol_id = user_id_of(carol);
    let png = png_bytes_for_test(64, 64);

    let coords = |pts: &[(u8, u8)]| -> Vec<BoardCoordinates> {
        pts.iter()
            .map(|&(r, c)| BoardCoordinates::new(r, c, GridSize::Three).unwrap())
            .collect()
    };

    // Game 1: Bob wins via top row (0,0), (0,1), (0,2)
    let g_row = setup_active_game(
        &harness,
        alice,
        &[bob, carol],
        "Row Win",
        GridSize::Three,
        VisibilityMode::Undercover,
    )
    .await;
    let bob_tiles = load_tiles(&harness, bob, g_row.id()).await;
    for col in 0..3 {
        let tid = bob_tiles
            .iter()
            .find(|t| t.row() == 0 && t.col() == col)
            .unwrap()
            .id();
        let out = harness
            .app()
            .execute(
                bob,
                SubmitCapture {
                    game_id: g_row.id(),
                    tile_id: tid,
                    image_bytes: png.clone(),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            out.winning_line,
            (col == 2).then(|| coords(&[(0, 0), (0, 1), (0, 2)]))
        );
    }

    // Game 2: Carol wins via right column (0,2), (1,2), (2,2)
    let g_col = setup_active_game(
        &harness,
        alice,
        &[bob, carol],
        "Col Win",
        GridSize::Three,
        VisibilityMode::Undercover,
    )
    .await;
    let carol_tiles = load_tiles(&harness, carol, g_col.id()).await;
    for row in 0..3 {
        let tid = carol_tiles
            .iter()
            .find(|t| t.row() == row && t.col() == 2)
            .unwrap()
            .id();
        let out = harness
            .app()
            .execute(
                carol,
                SubmitCapture {
                    game_id: g_col.id(),
                    tile_id: tid,
                    image_bytes: png.clone(),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            out.winning_line,
            (row == 2).then(|| coords(&[(0, 2), (1, 2), (2, 2)]))
        );
    }

    // Game 3: Bob wins via anti-diagonal (0,2), (1,1), (2,0)
    let g_anti = setup_active_game(
        &harness,
        alice,
        &[bob, carol],
        "Anti-Diagonal Win",
        GridSize::Three,
        VisibilityMode::Undercover,
    )
    .await;
    let bob_tiles3 = load_tiles(&harness, bob, g_anti.id()).await;
    for (idx, coord) in [(0, 2), (1, 1), (2, 0)].into_iter().enumerate() {
        let tid = bob_tiles3
            .iter()
            .find(|t| t.row() == coord.0 && t.col() == coord.1)
            .unwrap()
            .id();
        let out = harness
            .app()
            .execute(
                bob,
                SubmitCapture {
                    game_id: g_anti.id(),
                    tile_id: tid,
                    image_bytes: png.clone(),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            out.winning_line,
            (idx == 2).then(|| coords(&[(0, 2), (1, 1), (2, 0)]))
        );
    }

    let (bob_medals, carol_medals) = harness
        .app()
        .execute_fn(alice, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let bm = uow
                .benutzer()
                .find_by_id(bob_id)
                .await
                .unwrap()
                .into_data()
                .gold_medal_count();
            let cm = uow
                .benutzer()
                .find_by_id(carol_id)
                .await
                .unwrap()
                .gold_medal_count();
            uow.finish(
                Ok::<_, error_stack::Report<RepositoryError>>((bm, cm)),
                RepositoryError::Unknown,
            )
            .await
        })
        .await
        .unwrap();
    assert_eq!(bob_medals, 2);
    assert_eq!(carol_medals, 1);
}

#[test_log::test(pollster::test)]
async fn superlatives_computed_from_persisted_captures() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "alice@example.at").await;
    let bob = register_user(&harness, "bob@example.at").await;
    let carol = register_user(&harness, "carol@example.at").await;
    let dave = register_user(&harness, "dave@example.at").await;
    let player_ids = vec![
        user_id_of(alice),
        user_id_of(bob),
        user_id_of(carol),
        user_id_of(dave),
    ];

    let game = setup_active_game(
        &harness,
        alice,
        &[bob, carol, dave],
        "Superlative Match",
        GridSize::Four,
        VisibilityMode::LiveFeed,
    )
    .await;

    let alice_tiles = load_tiles(&harness, alice, game.id()).await;
    let bob_tiles = load_tiles(&harness, bob, game.id()).await;
    let png = png_bytes_for_test(64, 64);

    // Quickest Draw: Alice captures first at +45s
    harness.vorspulen(Duration::seconds(45));
    let first_tile = alice_tiles
        .iter()
        .find(|t| t.row() == 0 && t.col() == 0)
        .unwrap();
    harness
        .app()
        .execute(
            alice,
            SubmitCapture {
                game_id: game.id(),
                tile_id: first_tile.id(),
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap();

    // Bob completes 4 captures in row 0 -> Bob wins Sniper + Bingo
    harness.vorspulen(Duration::hours(2));
    for col in 0..4 {
        harness.vorspulen(Duration::minutes(5));
        let tid = bob_tiles
            .iter()
            .find(|t| t.row() == 0 && t.col() == col)
            .unwrap()
            .id();
        harness
            .app()
            .execute(
                bob,
                SubmitCapture {
                    game_id: game.id(),
                    tile_id: tid,
                    image_bytes: png.clone(),
                },
            )
            .await
            .unwrap();
    }

    let (loaded_game, captures) = harness
        .app()
        .execute_fn(alice, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let g = uow.games().find_by_id(game.id()).await.unwrap().into_data();
            let caps: Vec<_> = uow
                .captures()
                .find_by_game(game.id())
                .await
                .unwrap()
                .into_iter()
                .map(|c| c.into_data())
                .collect();
            uow.finish(
                Ok::<_, error_stack::Report<RepositoryError>>((g, caps)),
                RepositoryError::Unknown,
            )
            .await
        })
        .await
        .unwrap();

    let Game::Completed(completed) = loaded_game else {
        panic!("expected completed game");
    };
    assert_eq!(completed.winner_user_id(), Some(user_id_of(bob)));
    assert_eq!(captures.len(), 5);

    let awards = compute_game_awards(&player_ids, &captures, completed.started_at()).unwrap();
    assert_eq!(awards.quickest_draw.user_id, user_id_of(alice));
    assert_eq!(awards.quickest_draw.metric_value, 45);
    assert_eq!(awards.sniper.user_id, user_id_of(bob));
    assert_eq!(awards.sniper.metric_value, 4);
}

#[test_log::test(pollster::test)]
async fn retract_capture_clears_tile_deletes_media_and_guards_completed_or_foreign() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "ret-alice@example.at").await;
    let bob = register_user(&harness, "ret-bob@example.at").await;
    let carol = register_user(&harness, "ret-carol@example.at").await;

    let game = setup_active_game(
        &harness,
        alice,
        &[bob, carol],
        "Retract Hunt",
        GridSize::Three,
        VisibilityMode::LiveFeed,
    )
    .await;
    let alice_tiles = load_tiles(&harness, alice, game.id()).await;
    let target_tile = alice_tiles[0].id();
    let png = png_bytes_for_test(16, 16);

    let submitted = harness
        .app()
        .execute(
            alice,
            SubmitCapture {
                game_id: game.id(),
                tile_id: target_tile,
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap();
    let cap_id = submitted.capture.id();
    let full_key = submitted.capture.image_storage_key().to_owned();

    // Another player cannot retract Alice's capture -> Forbidden
    let err = harness
        .app()
        .execute(
            bob,
            RetractCapture {
                game_id: game.id(),
                capture_id: cap_id,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        CaptureUseCaseError::Forbidden
    ));

    // Unknown capture -> NotFound
    let err = harness
        .app()
        .execute(
            alice,
            RetractCapture {
                game_id: game.id(),
                capture_id: CaptureId(Uuid::new_v4()),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        CaptureUseCaseError::NotFound
    ));

    // Owner retracts own capture -> tile cleared & media removed; tile can be captured again.
    harness
        .app()
        .execute(
            alice,
            RetractCapture {
                game_id: game.id(),
                capture_id: cap_id,
            },
        )
        .await
        .unwrap();

    let updated_tiles = load_tiles(&harness, alice, game.id()).await;
    let cleared = updated_tiles
        .iter()
        .find(|t| t.id() == target_tile)
        .unwrap();
    assert!(cleared.capture_id().is_none());
    let stored = harness
        .app()
        .execute_fn(alice, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let cap_missing = uow.captures().find_by_id(cap_id).await.is_err();
            let obj = ctx.object_store().get(&full_key).await.unwrap();
            uow.finish(
                Ok::<_, error_stack::Report<RepositoryError>>((cap_missing, obj.is_none())),
                RepositoryError::Unknown,
            )
            .await
        })
        .await
        .unwrap();
    assert!(stored.0, "capture row must be deleted");
    assert!(stored.1, "media object must be deleted");

    // Complete a Bingo row for Alice; once game Completed, retract is rejected with Conflict.
    let mut kept_capture_id = None;
    for col in 0..3 {
        let tid = updated_tiles
            .iter()
            .find(|t| t.row() == 0 && t.col() == col)
            .unwrap()
            .id();
        let out = harness
            .app()
            .execute(
                alice,
                SubmitCapture {
                    game_id: game.id(),
                    tile_id: tid,
                    image_bytes: png.clone(),
                },
            )
            .await
            .unwrap();
        if col == 0 {
            kept_capture_id = Some(out.capture.id());
        }
    }

    let err = harness
        .app()
        .execute(
            alice,
            RetractCapture {
                game_id: game.id(),
                capture_id: kept_capture_id.unwrap(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        CaptureUseCaseError::Conflict
    ));
}

#[test_log::test(pollster::test)]
async fn submit_capture_emits_near_bingo_notification_and_rejects_after_ends_at() {
    #[derive(Default)]
    struct RecordingQueue(Mutex<Vec<Event>>);
    #[async_trait::async_trait]
    impl EventQueue for RecordingQueue {
        async fn enqueue(&self, events: Vec<Event>) -> ResultReport<(), EventQueueError> {
            self.0.lock().unwrap().extend(events);
            Ok(())
        }
    }

    let queue = Arc::new(RecordingQueue::default());
    let harness = super::helpers::build_with_event_queue(
        super::super::base_app_builder().await,
        (Arc::clone(&queue) as Arc<dyn EventQueue>).into(),
    );
    let alice = register_user(&harness, "nb-alice@example.at").await;
    let bob = register_user(&harness, "nb-bob@example.at").await;
    let carol = register_user(&harness, "nb-carol@example.at").await;

    let ends_at = harness.jetzt() + Duration::minutes(30);
    let game = harness
        .app()
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Near Bingo & Cutoff").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
                draft_deadline_at: None,
                ends_at: Some(ends_at),
            },
        )
        .await
        .unwrap();
    for guest in [bob, carol] {
        harness
            .app()
            .execute(
                guest,
                JoinGame {
                    invite_code: game.invite_code().clone(),
                },
            )
            .await
            .unwrap();
    }
    harness
        .app()
        .execute(alice, StartDrafting { game_id: game.id() })
        .await
        .unwrap();
    harness
        .app()
        .execute(
            alice,
            HostFillRemainingDrafts {
                game_id: game.id(),
                start_immediately: true,
            },
        )
        .await
        .unwrap();

    let png = png_bytes_for_test(16, 16);
    let alice_tiles = load_tiles(&harness, alice, game.id()).await;
    // Filling (0,0) and (0,1) puts Alice 1 away from Bingo on row 0 -> NearBingoReached emitted!
    for col in 0..2 {
        let tid = alice_tiles
            .iter()
            .find(|t| t.row() == 0 && t.col() == col)
            .unwrap()
            .id();
        harness
            .app()
            .execute(
                alice,
                SubmitCapture {
                    game_id: game.id(),
                    tile_id: tid,
                    image_bytes: png.clone(),
                },
            )
            .await
            .unwrap();
    }

    let drained = std::mem::take(&mut *queue.0.lock().unwrap());
    assert!(
        drained
            .iter()
            .any(|ev| matches!(ev, Event::NearBingoReached(_))),
        "expected NearBingoReached event"
    );
    for ev in drained {
        handle_event(&harness.app(), ev).await.unwrap();
    }

    // Bob received a FastBingo in-app notification for Alice.
    let bob_id = user_id_of(bob);
    let notifs = harness
        .app()
        .execute_fn(AuthContext::Anonymous, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let list = uow
                .benachrichtigungen()
                .find_by_empfänger(bob_id)
                .await
                .unwrap();
            uow.finish(
                Ok::<_, error_stack::Report<RepositoryError>>(list),
                RepositoryError::Unknown,
            )
            .await
        })
        .await
        .unwrap();
    assert!(
        notifs
            .iter()
            .any(|n| n.art() == BenachrichtigungArt::FastBingo)
    );

    // Advance past ends_at: capture upload must be rejected with Conflict even before scheduler completes the game.
    harness.vorspulen(Duration::minutes(31));
    let third_tid = alice_tiles
        .iter()
        .find(|t| t.row() == 0 && t.col() == 2)
        .unwrap()
        .id();
    let err = harness
        .app()
        .execute(
            alice,
            SubmitCapture {
                game_id: game.id(),
                tile_id: third_tid,
                image_bytes: png,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        CaptureUseCaseError::Conflict
    ));
}
