//! Shared game lobby, drafting, and board assembly conformance tests.

use std::collections::{HashMap, HashSet};

use chrono::Duration;
use friendarazzi_core::{
    adapters::BlockingEventQueue,
    application::AuthContext,
    domain::{
        BenutzerId, BoardTileId, Game, GameId, GridSize, InviteCode, NonEmptyString, PromptSource,
        PromptSubmissionId, READY_GRACE_PERIOD_SECONDS, VisibilityMode,
    },
    ports::RepositoryError,
    service::{
        CaptureUseCaseError, CreateGame, DelayedGameStart, DraftPromptInput, Event,
        FinalizeReadyGame, GameUseCaseError, JoinGame, LockDraftPrompts, ProfileUseCaseError,
        SaveDraftPrompts, StartDrafting, SubmitCapture, UnlockDraftPrompts, UpdateGameSettings,
        UpdateProfile, UploadAvatar,
    },
};
use friendarazzi_fakes::png_bytes_for_test;
use uuid::Uuid;

use super::helpers::{
    balanced_prompts_for, build, build_with_event_queue, load_tiles, register_user,
    setup_active_game, user_id_of,
};

#[test_log::test(pollster::test)]
async fn anonymous_users_cannot_run_game_capture_or_profile_use_cases() {
    let harness = build(super::super::base_app_builder().await);
    let dummy_game = GameId(Uuid::new_v4());
    let dummy_tile = BoardTileId(Uuid::new_v4());

    let err = harness
        .app()
        .execute(
            AuthContext::Anonymous,
            CreateGame {
                title: NonEmptyString::neu("Geheim").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::Undercover,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        GameUseCaseError::Unauthorized
    ));

    let err = harness
        .app()
        .execute(
            AuthContext::Anonymous,
            JoinGame {
                invite_code: InviteCode::new("ABCDEF").unwrap(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        GameUseCaseError::Unauthorized
    ));

    let err = harness
        .app()
        .execute(
            AuthContext::Anonymous,
            UpdateGameSettings {
                game_id: dummy_game,
                title: NonEmptyString::neu("Neu").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        GameUseCaseError::Unauthorized
    ));

    let err = harness
        .app()
        .execute(
            AuthContext::Anonymous,
            StartDrafting {
                game_id: dummy_game,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        GameUseCaseError::Unauthorized
    ));

    let err = harness
        .app()
        .execute(
            AuthContext::Anonymous,
            SaveDraftPrompts {
                game_id: dummy_game,
                prompts: Vec::new(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        GameUseCaseError::Unauthorized
    ));

    let err = harness
        .app()
        .execute(
            AuthContext::Anonymous,
            LockDraftPrompts {
                game_id: dummy_game,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        GameUseCaseError::Unauthorized
    ));

    let err = harness
        .app()
        .execute(
            AuthContext::Anonymous,
            SubmitCapture {
                game_id: dummy_game,
                tile_id: dummy_tile,
                image_bytes: png_bytes_for_test(16, 16),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        CaptureUseCaseError::Unauthorized
    ));

    let err = harness
        .app()
        .execute(
            AuthContext::Anonymous,
            UpdateProfile {
                display_name: NonEmptyString::neu("Anon").unwrap(),
                equipped_title: None,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        ProfileUseCaseError::Unauthorized
    ));

    let err = harness
        .app()
        .execute(
            AuthContext::Anonymous,
            UploadAvatar {
                image_bytes: png_bytes_for_test(32, 32),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        ProfileUseCaseError::Unauthorized
    ));
}

#[test_log::test(pollster::test)]
async fn lobby_settings_authorization_and_join_rules() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "alice@example.at").await;
    let bob = register_user(&harness, "bob@example.at").await;
    let carol = register_user(&harness, "carol@example.at").await;
    let late = register_user(&harness, "late@example.at").await;

    let game = harness
        .app()
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Lobby").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::Undercover,
            },
        )
        .await
        .unwrap();

    // Unknown invite code -> NotFound
    let missing_join = harness
        .app()
        .execute(
            bob,
            JoinGame {
                invite_code: InviteCode::new("ZZZZZZ").unwrap(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        missing_join.current_context(),
        GameUseCaseError::NotFound
    ));

    // StartDrafting with < 3 players -> TooFewPlayers
    harness
        .app()
        .execute(
            bob,
            JoinGame {
                invite_code: game.invite_code().clone(),
            },
        )
        .await
        .unwrap();
    let too_few = harness
        .app()
        .execute(alice, StartDrafting { game_id: game.id() })
        .await
        .unwrap_err();
    assert!(matches!(
        too_few.current_context(),
        GameUseCaseError::TooFewPlayers
    ));

    // Re-joining as an already enrolled member is idempotent
    harness
        .app()
        .execute(
            bob,
            JoinGame {
                invite_code: game.invite_code().clone(),
            },
        )
        .await
        .unwrap();
    let still_too_few = harness
        .app()
        .execute(alice, StartDrafting { game_id: game.id() })
        .await
        .unwrap_err();
    assert!(matches!(
        still_too_few.current_context(),
        GameUseCaseError::TooFewPlayers
    ));

    // Non-host cannot update settings or start drafting
    let err = harness
        .app()
        .execute(
            bob,
            UpdateGameSettings {
                game_id: game.id(),
                title: NonEmptyString::neu("Hacked").unwrap(),
                grid_size: GridSize::Four,
                visibility_mode: VisibilityMode::LiveFeed,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err.current_context(), GameUseCaseError::Forbidden));

    harness
        .app()
        .execute(
            carol,
            JoinGame {
                invite_code: game.invite_code().clone(),
            },
        )
        .await
        .unwrap();

    let err = harness
        .app()
        .execute(bob, StartDrafting { game_id: game.id() })
        .await
        .unwrap_err();
    assert!(matches!(err.current_context(), GameUseCaseError::Forbidden));

    // Host can update settings while in Lobby
    let updated = harness
        .app()
        .execute(
            alice,
            UpdateGameSettings {
                game_id: game.id(),
                title: NonEmptyString::neu("Wochenende 4x4").unwrap(),
                grid_size: GridSize::Four,
                visibility_mode: VisibilityMode::LiveFeed,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.title().as_str(), "Wochenende 4x4");
    assert_eq!(updated.grid_size(), GridSize::Four);
    assert_eq!(updated.visibility_mode(), VisibilityMode::LiveFeed);

    // Host starts drafting
    let drafted = harness
        .app()
        .execute(alice, StartDrafting { game_id: game.id() })
        .await
        .unwrap();
    assert!(matches!(drafted, Game::Drafting(_)));

    // Once drafting starts: settings update, second StartDrafting, and new player join -> Conflict
    let err = harness
        .app()
        .execute(
            alice,
            UpdateGameSettings {
                game_id: game.id(),
                title: NonEmptyString::neu("Zu spät").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::Undercover,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err.current_context(), GameUseCaseError::Conflict));

    let err = harness
        .app()
        .execute(alice, StartDrafting { game_id: game.id() })
        .await
        .unwrap_err();
    assert!(matches!(err.current_context(), GameUseCaseError::Conflict));

    let err = harness
        .app()
        .execute(
            late,
            JoinGame {
                invite_code: game.invite_code().clone(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err.current_context(), GameUseCaseError::Conflict));

    // Existing player calling JoinGame in Drafting remains idempotent
    let rejoined = harness
        .app()
        .execute(
            bob,
            JoinGame {
                invite_code: game.invite_code().clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(rejoined.id(), game.id());
}

#[test_log::test(pollster::test)]
async fn drafting_validation_quotas_replacement_and_unlock_behavior() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "alice@example.at").await;
    let bob = register_user(&harness, "bob@example.at").await;
    let carol = register_user(&harness, "carol@example.at").await;
    let outsider = register_user(&harness, "outsider@example.at").await;

    let alice_id = user_id_of(alice);
    let bob_id = user_id_of(bob);
    let outsider_id = user_id_of(outsider);

    let game = harness
        .app()
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Drafting Checks").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::BlurredTeaser,
            },
        )
        .await
        .unwrap();
    for g in [bob, carol] {
        harness
            .app()
            .execute(
                g,
                JoinGame {
                    invite_code: game.invite_code().clone(),
                },
            )
            .await
            .unwrap();
    }

    // Prompts cannot be saved or locked while still in Lobby
    let err = harness
        .app()
        .execute(
            alice,
            SaveDraftPrompts {
                game_id: game.id(),
                prompts: Vec::new(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err.current_context(), GameUseCaseError::Conflict));

    let err = harness
        .app()
        .execute(alice, LockDraftPrompts { game_id: game.id() })
        .await
        .unwrap_err();
    assert!(matches!(err.current_context(), GameUseCaseError::Conflict));

    let drafted = harness
        .app()
        .execute(alice, StartDrafting { game_id: game.id() })
        .await
        .unwrap();
    let tiles_needed = drafted.playable_tile_count();
    let player_ids = drafted.sorted_player_ids();

    // Non-participant gets Forbidden
    let err = harness
        .app()
        .execute(
            outsider,
            SaveDraftPrompts {
                game_id: game.id(),
                prompts: Vec::new(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err.current_context(), GameUseCaseError::Forbidden));
    let err = harness
        .app()
        .execute(outsider, LockDraftPrompts { game_id: game.id() })
        .await
        .unwrap_err();
    assert!(matches!(err.current_context(), GameUseCaseError::Forbidden));

    // Self-target prompt -> BadRequest
    let err = harness
        .app()
        .execute(
            alice,
            SaveDraftPrompts {
                game_id: game.id(),
                prompts: vec![DraftPromptInput {
                    target_user_id: alice_id,
                    text: NonEmptyString::neu("Selbstporträt").unwrap(),
                    source: PromptSource::Custom,
                }],
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        GameUseCaseError::BadRequest
    ));

    // Non-game target user -> BadRequest
    let err = harness
        .app()
        .execute(
            alice,
            SaveDraftPrompts {
                game_id: game.id(),
                prompts: vec![DraftPromptInput {
                    target_user_id: outsider_id,
                    text: NonEmptyString::neu("Fremder").unwrap(),
                    source: PromptSource::Custom,
                }],
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        GameUseCaseError::BadRequest
    ));

    // Locking with missing/unbalanced prompts -> BadRequest
    harness
        .app()
        .execute(
            alice,
            SaveDraftPrompts {
                game_id: game.id(),
                prompts: vec![DraftPromptInput {
                    target_user_id: bob_id,
                    text: NonEmptyString::neu("Zu wenige").unwrap(),
                    source: PromptSource::Custom,
                }],
            },
        )
        .await
        .unwrap();
    let err = harness
        .app()
        .execute(alice, LockDraftPrompts { game_id: game.id() })
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        GameUseCaseError::BadRequest
    ));

    // Alice saves valid prompts and locks in
    let alice_prompts = balanced_prompts_for(
        &player_ids,
        alice_id,
        tiles_needed,
        PromptSource::StarterDeck,
        "Alice Deck",
    );
    harness
        .app()
        .execute(
            alice,
            SaveDraftPrompts {
                game_id: game.id(),
                prompts: alice_prompts,
            },
        )
        .await
        .unwrap();
    let still_drafting = harness
        .app()
        .execute(alice, LockDraftPrompts { game_id: game.id() })
        .await
        .unwrap();
    assert!(matches!(still_drafting, Game::Drafting(_)));

    // Alice edits her prompts before Bob & Carol finish -> replaces prompts AND resets draft_locked
    let alice_edited = balanced_prompts_for(
        &player_ids,
        alice_id,
        tiles_needed,
        PromptSource::Custom,
        "Alice Edited",
    );
    harness
        .app()
        .execute(
            alice,
            SaveDraftPrompts {
                game_id: game.id(),
                prompts: alice_edited,
            },
        )
        .await
        .unwrap();

    // Game stays Drafting because Alice edited & unlocked
    for (idx, auth) in [bob, carol].into_iter().enumerate() {
        let uid = user_id_of(auth);
        let prompts = balanced_prompts_for(
            &player_ids,
            uid,
            tiles_needed,
            PromptSource::Custom,
            &format!("Guest{idx}"),
        );
        harness
            .app()
            .execute(
                auth,
                SaveDraftPrompts {
                    game_id: game.id(),
                    prompts,
                },
            )
            .await
            .unwrap();
        let state = harness
            .app()
            .execute(auth, LockDraftPrompts { game_id: game.id() })
            .await
            .unwrap();
        assert!(matches!(state, Game::Drafting(_)));
    }

    // Re-locking Alice completes all locks and starts the 60s countdown rather than activating immediately
    let countdown_game = harness
        .app()
        .execute(alice, LockDraftPrompts { game_id: game.id() })
        .await
        .unwrap();
    let Game::Drafting(d) = countdown_game else {
        panic!("expected drafting with countdown after all players lock");
    };
    assert!(d.auto_start_at().is_some());

    // Before 60s pass, finalizing leaves the game in Drafting
    harness.vorspulen(Duration::seconds(READY_GRACE_PERIOD_SECONDS - 1));
    let still_waiting = harness
        .app()
        .execute(alice, FinalizeReadyGame { game_id: game.id() })
        .await
        .unwrap();
    assert!(matches!(still_waiting, Game::Drafting(_)));

    // Last player withdraws readiness during countdown -> cancels pending start and keeps prompts
    let unlocked = harness
        .app()
        .execute(alice, UnlockDraftPrompts { game_id: game.id() })
        .await
        .unwrap();
    let Game::Drafting(d_unlocked) = unlocked else {
        panic!("expected drafting after unlock");
    };
    assert!(d_unlocked.auto_start_at().is_none());
    assert!(!d_unlocked.is_draft_locked(alice_id));
    assert_eq!(d_unlocked.prompts_by_author(alice_id).len(), tiles_needed);

    // Re-locking starts a fresh full 60s countdown
    let relocked = harness
        .app()
        .execute(alice, LockDraftPrompts { game_id: game.id() })
        .await
        .unwrap();
    assert!(matches!(relocked, Game::Drafting(_)));

    harness.vorspulen(Duration::seconds(READY_GRACE_PERIOD_SECONDS - 1));
    let still_counting = harness
        .app()
        .execute(bob, FinalizeReadyGame { game_id: game.id() })
        .await
        .unwrap();
    assert!(matches!(still_counting, Game::Drafting(_)));

    harness.vorspulen(Duration::seconds(1));
    let active = harness
        .app()
        .execute(bob, FinalizeReadyGame { game_id: game.id() })
        .await
        .unwrap();
    assert!(matches!(active, Game::Active(_)));

    // Once Active, readiness cannot be withdrawn
    let err = harness
        .app()
        .execute(alice, UnlockDraftPrompts { game_id: game.id() })
        .await
        .unwrap_err();
    assert!(matches!(err.current_context(), GameUseCaseError::Conflict));

    let all_prompts = harness
        .app()
        .execute_fn(alice, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let loaded = uow.games().find_by_id(game.id()).await.unwrap();
            let Game::Active(active) = loaded.into_data() else {
                panic!("expected active game");
            };
            let list = active.prompts().to_vec();
            uow.finish(
                Ok::<_, error_stack::Report<RepositoryError>>(list),
                RepositoryError::Unknown,
            )
            .await
        })
        .await
        .unwrap();
    assert_eq!(all_prompts.len(), 3 * tiles_needed);
    assert!(
        all_prompts
            .iter()
            .filter(|p| p.author_user_id == alice_id)
            .all(|p| p.text.as_str().starts_with("Alice Edited")),
        "old prompt submissions should be completely replaced"
    );
}

#[test_log::test(pollster::test)]
async fn board_assembly_4x4_and_5x5_enforce_author_and_target_constraints() {
    let harness = build(super::super::base_app_builder().await);
    let p1 = register_user(&harness, "p1@example.at").await;
    let p2 = register_user(&harness, "p2@example.at").await;
    let p3 = register_user(&harness, "p3@example.at").await;
    let p4 = register_user(&harness, "p4@example.at").await;
    let players = [p1, p2, p3, p4];

    for (grid_size, title, expected_count) in [
        (GridSize::Four, "4x4 Runde", 16),
        (GridSize::Five, "5x5 Großrunde", 25),
    ] {
        let game = setup_active_game(
            &harness,
            p1,
            &[p2, p3, p4],
            title,
            grid_size,
            VisibilityMode::LiveFeed,
        )
        .await;
        assert_eq!(game.playable_tile_count(), expected_count);

        let game_id = game.id();
        let prompts: HashMap<PromptSubmissionId, BenutzerId> = harness
            .app()
            .execute_fn(p1, async move |ctx| {
                let uow = ctx.enter().await.unwrap();
                let loaded = uow.games().find_by_id(game_id).await.unwrap();
                let Game::Active(active) = loaded.into_data() else {
                    panic!("expected active game");
                };
                let map: HashMap<_, _> = active
                    .prompts()
                    .iter()
                    .map(|p| (p.id, p.author_user_id))
                    .collect();
                uow.finish(
                    Ok::<_, error_stack::Report<RepositoryError>>(map),
                    RepositoryError::Unknown,
                )
                .await
            })
            .await
            .unwrap();

        let mut placed = HashSet::new();
        for auth in players {
            let owner = user_id_of(auth);
            let tiles = load_tiles(&harness, auth, game_id).await;
            assert_eq!(tiles.len(), expected_count);
            for t in tiles {
                assert_ne!(t.target_user_id(), owner);
                let prompt_id = t.prompt_submission_id();
                assert_ne!(prompts[&prompt_id], owner);
                assert!(placed.insert(prompt_id), "prompt placed twice");
            }
        }
        assert_eq!(placed.len(), prompts.len());
    }
}

#[test_log::test(pollster::test)]
async fn delayed_game_start_event_ignores_stale_countdown_and_activates_current_one() {
    let harness = build_with_event_queue(
        super::super::base_app_builder().await,
        BlockingEventQueue::adapter(),
    );
    let alice = register_user(&harness, "alice-ev@example.at").await;
    let bob = register_user(&harness, "bob-ev@example.at").await;
    let carol = register_user(&harness, "carol-ev@example.at").await;

    let game = harness
        .app()
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Event Start Test").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
            },
        )
        .await
        .unwrap();
    let game_id = game.id();

    for guest in [bob, carol] {
        harness.vorspulen(Duration::seconds(1));
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

    let drafted = harness
        .app()
        .execute(alice, StartDrafting { game_id })
        .await
        .unwrap();
    let player_ids = vec![user_id_of(alice), user_id_of(bob), user_id_of(carol)];
    let tiles_needed = drafted.playable_tile_count();
    for (idx, auth) in [alice, bob, carol].into_iter().enumerate() {
        let prompts = balanced_prompts_for(
            &player_ids,
            user_id_of(auth),
            tiles_needed,
            PromptSource::Custom,
            &format!("Ev{idx}"),
        );
        harness
            .app()
            .execute(auth, SaveDraftPrompts { game_id, prompts })
            .await
            .unwrap();
        harness
            .app()
            .execute(auth, LockDraftPrompts { game_id })
            .await
            .unwrap();
    }

    let first_start_at = harness
        .app()
        .execute_fn(alice, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let loaded = uow.games().find_by_id(game_id).await.unwrap();
            let Game::Drafting(d) = loaded.into_data() else {
                panic!("expected drafting");
            };
            uow.finish(
                Ok::<_, error_stack::Report<RepositoryError>>(d.auto_start_at().unwrap()),
                RepositoryError::Unknown,
            )
            .await
        })
        .await
        .unwrap();

    // Unlock and re-lock to supersede the first countdown timestamp.
    harness.vorspulen(Duration::seconds(5));
    harness
        .app()
        .execute(carol, UnlockDraftPrompts { game_id })
        .await
        .unwrap();
    let Game::Drafting(relocked) = harness
        .app()
        .execute(carol, LockDraftPrompts { game_id })
        .await
        .unwrap()
    else {
        panic!("expected drafting");
    };
    let second_start_at = relocked.auto_start_at().unwrap();
    assert!(second_start_at > first_start_at);

    // Advance past both timestamps; stale event must be a no-op because scheduled_for differs.
    harness.vorspulen(Duration::seconds(READY_GRACE_PERIOD_SECONDS));
    harness
        .app()
        .execute(
            AuthContext::Anonymous,
            Event::DelayedGameStart(DelayedGameStart {
                game_id,
                scheduled_for: first_start_at,
            }),
        )
        .await
        .unwrap();

    let still_drafting = harness
        .app()
        .execute_fn(alice, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let loaded = uow.games().find_by_id(game_id).await.unwrap();
            uow.finish(
                Ok::<_, error_stack::Report<RepositoryError>>(loaded.into_data()),
                RepositoryError::Unknown,
            )
            .await
        })
        .await
        .unwrap();
    assert!(matches!(still_drafting, Game::Drafting(_)));

    // Current event activates the game and deals boards.
    harness
        .app()
        .execute(
            AuthContext::Anonymous,
            Event::DelayedGameStart(DelayedGameStart {
                game_id,
                scheduled_for: second_start_at,
            }),
        )
        .await
        .unwrap();

    let active = harness
        .app()
        .execute_fn(alice, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let loaded = uow.games().find_by_id(game_id).await.unwrap();
            uow.finish(
                Ok::<_, error_stack::Report<RepositoryError>>(loaded.into_data()),
                RepositoryError::Unknown,
            )
            .await
        })
        .await
        .unwrap();
    assert!(matches!(active, Game::Active(_)));
    for auth in [alice, bob, carol] {
        assert_eq!(
            load_tiles(&harness, auth, game_id).await.len(),
            tiles_needed
        );
    }
}
