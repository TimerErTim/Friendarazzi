//! Shared game lobby, drafting, and board assembly conformance tests.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::Duration;
use friendarazzi_core::{
    ResultReport,
    application::AuthContext,
    domain::{
        BenutzerId, BoardTileId, Game, GameId, GridSize, InviteCode, NonEmptyString, PromptSource,
        PromptSubmissionId, PromptText, READY_GRACE_PERIOD_SECONDS, VisibilityMode,
    },
    ports::{EventQueue, EventQueueError, RepositoryError},
    service::{
        AbandonGame, CaptureUseCaseError, CreateGame, DraftPromptInput, FinishExpiredGames,
        GameUseCaseError, HostFillRemainingDrafts, JoinGame, KickPlayer, LeaveGame,
        LockDraftPrompts, ProfileUseCaseError, SaveDraftPrompts, SetDraftDeadline, StartDrafting,
        StartReadyGames, SubmitCapture, TransferHost, UnlockDraftPrompts, UpdateGameSettings,
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
                draft_deadline_at: None,
                ends_at: None,
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
                draft_deadline_at: None,
                ends_at: None,
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
                draft_deadline_at: None,
                ends_at: None,
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
                draft_deadline_at: None,
                ends_at: None,
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
                draft_deadline_at: None,
                ends_at: None,
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
                draft_deadline_at: None,
                ends_at: None,
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
                draft_deadline_at: None,
                ends_at: None,
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
                    text: PromptText::new("Selbstporträt").unwrap(),
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
                    text: PromptText::new("Fremder").unwrap(),
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
                    text: PromptText::new("Zu wenige").unwrap(),
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
    assert!(d.draft_closes_at().is_some());

    // Before 60s pass, StartReadyGames leaves the game in Drafting
    harness.vorspulen(Duration::seconds(READY_GRACE_PERIOD_SECONDS - 1));
    let still_waiting = harness
        .app()
        .execute(AuthContext::Anonymous, StartReadyGames)
        .await
        .unwrap();
    assert!(still_waiting.started_games.is_empty());

    // Last player withdraws readiness during countdown -> cancels pending start and keeps prompts
    let unlocked = harness
        .app()
        .execute(alice, UnlockDraftPrompts { game_id: game.id() })
        .await
        .unwrap();
    let Game::Drafting(d_unlocked) = unlocked else {
        panic!("expected drafting after unlock");
    };
    assert!(d_unlocked.draft_closes_at().is_none());
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
        .execute(AuthContext::Anonymous, StartReadyGames)
        .await
        .unwrap();
    assert!(still_counting.started_games.is_empty());

    harness.vorspulen(Duration::seconds(1));
    let started = harness
        .app()
        .execute(AuthContext::Anonymous, StartReadyGames)
        .await
        .unwrap();
    assert_eq!(started.started_games, vec![game.id()]);

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

struct NoopEventQueue;

#[async_trait::async_trait]
impl EventQueue for NoopEventQueue {
    async fn enqueue(
        &self,
        _events: Vec<friendarazzi_core::service::Event>,
    ) -> ResultReport<(), EventQueueError> {
        Ok(())
    }
}

#[test_log::test(pollster::test)]
async fn start_ready_games_ignores_superseded_countdown_until_current_expires() {
    let harness = build_with_event_queue(
        super::super::base_app_builder().await,
        Arc::new(NoopEventQueue).into(),
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
                draft_deadline_at: None,
                ends_at: None,
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
                Ok::<_, error_stack::Report<RepositoryError>>(d.draft_closes_at().unwrap()),
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
    let second_start_at = relocked.draft_closes_at().unwrap();
    assert!(second_start_at > first_start_at);

    // Advancing to the superseded deadline is not enough yet.
    harness.vorspulen(Duration::seconds(READY_GRACE_PERIOD_SECONDS - 5));
    let mid = harness
        .app()
        .execute(AuthContext::Anonymous, StartReadyGames)
        .await
        .unwrap();
    assert!(mid.started_games.is_empty());
    assert_eq!(mid.next_draft_closes_at, Some(second_start_at));

    // Remaining seconds elapse -> StartReadyGames activates the game and deals boards.
    harness.vorspulen(Duration::seconds(5));
    let done = harness
        .app()
        .execute(AuthContext::Anonymous, StartReadyGames)
        .await
        .unwrap();
    assert_eq!(done.started_games, vec![game_id]);

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

#[test_log::test(pollster::test)]
async fn start_ready_games_starts_expired_drafts_and_reports_next_deadline() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "sched-alice@example.com").await;
    let bob = register_user(&harness, "sched-bob@example.com").await;
    let carol = register_user(&harness, "sched-carol@example.com").await;

    let setup_locked = |title: &str| {
        let harness = &harness;
        let title = title.to_owned();
        async move {
            let game = harness
                .app()
                .execute(
                    alice,
                    CreateGame {
                        title: NonEmptyString::neu(&title).unwrap(),
                        grid_size: GridSize::Three,
                        visibility_mode: VisibilityMode::LiveFeed,
                        draft_deadline_at: None,
                        ends_at: None,
                    },
                )
                .await
                .unwrap();
            let gid = game.id();
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
                .execute(alice, StartDrafting { game_id: gid })
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
                    &format!("{title}{idx}"),
                );
                harness
                    .app()
                    .execute(
                        auth,
                        SaveDraftPrompts {
                            game_id: gid,
                            prompts,
                        },
                    )
                    .await
                    .unwrap();
                harness
                    .app()
                    .execute(auth, LockDraftPrompts { game_id: gid })
                    .await
                    .unwrap();
            }
            let Game::Drafting(d) = harness
                .app()
                .execute_fn(alice, async move |ctx| {
                    let uow = ctx.enter().await.unwrap();
                    let loaded = uow.games().find_by_id(gid).await.unwrap();
                    uow.finish(
                        Ok::<_, error_stack::Report<RepositoryError>>(loaded.into_data()),
                        RepositoryError::Unknown,
                    )
                    .await
                })
                .await
                .unwrap()
            else {
                panic!("expected drafting game");
            };
            (gid, d.draft_closes_at().unwrap())
        }
    };

    let (game_a, deadline_a) = setup_locked("SchedA").await;
    harness.vorspulen(Duration::seconds(30));
    let (game_b, deadline_b) = setup_locked("SchedB").await;
    assert!(deadline_b > deadline_a);

    // At T+30s neither countdown has elapsed; earliest pending deadline is game A.
    let before = harness
        .app()
        .execute(AuthContext::Anonymous, StartReadyGames)
        .await
        .unwrap();
    assert!(before.started_games.is_empty());
    assert_eq!(before.next_draft_closes_at, Some(deadline_a));

    // At T+60s game A expires and starts; game B remains the next pending deadline.
    harness.vorspulen(Duration::seconds(READY_GRACE_PERIOD_SECONDS - 30));
    let mid = harness
        .app()
        .execute(AuthContext::Anonymous, StartReadyGames)
        .await
        .unwrap();
    assert_eq!(mid.started_games, vec![game_a]);
    assert_eq!(mid.next_draft_closes_at, Some(deadline_b));

    // At T+90s game B expires and starts; no countdowns remain.
    harness.vorspulen(Duration::seconds(30));
    let end = harness
        .app()
        .execute(AuthContext::Anonymous, StartReadyGames)
        .await
        .unwrap();
    assert_eq!(end.started_games, vec![game_b]);
    assert_eq!(end.next_draft_closes_at, None);
}

#[test_log::test(pollster::test)]
async fn draft_deadline_and_host_fill_remaining_workflows() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "dd-alice@example.at").await;
    let bob = register_user(&harness, "dd-bob@example.at").await;
    let carol = register_user(&harness, "dd-carol@example.at").await;
    let alice_id = user_id_of(alice);

    // Past draft_deadline_at on CreateGame -> BadRequest
    let past = harness.jetzt() - Duration::minutes(5);
    let err = harness
        .app()
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Past Deadline").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
                draft_deadline_at: Some(past),
                ends_at: None,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        GameUseCaseError::BadRequest
    ));

    // Create game A for host fill-remaining test
    let game_a = harness
        .app()
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Host Fill Test").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
                draft_deadline_at: None,
                ends_at: None,
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
                    invite_code: game_a.invite_code().clone(),
                },
            )
            .await
            .unwrap();
    }
    let drafted_a = harness
        .app()
        .execute(
            alice,
            StartDrafting {
                game_id: game_a.id(),
            },
        )
        .await
        .unwrap();
    let tiles_needed = drafted_a.playable_tile_count();
    let player_ids = drafted_a.sorted_player_ids();

    // Non-host cannot set draft deadline or fill remaining
    let err = harness
        .app()
        .execute(
            bob,
            SetDraftDeadline {
                game_id: game_a.id(),
                draft_deadline_at: Some(harness.jetzt() + Duration::minutes(15)),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err.current_context(), GameUseCaseError::Forbidden));

    let err = harness
        .app()
        .execute(
            bob,
            HostFillRemainingDrafts {
                game_id: game_a.id(),
                start_immediately: false,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err.current_context(), GameUseCaseError::Forbidden));

    // Host sets invalid past deadline -> BadRequest; future deadline -> succeeds
    let err = harness
        .app()
        .execute(
            alice,
            SetDraftDeadline {
                game_id: game_a.id(),
                draft_deadline_at: Some(harness.jetzt() - Duration::seconds(1)),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        GameUseCaseError::BadRequest
    ));

    let future_deadline = harness.jetzt() + Duration::minutes(30);
    let updated = harness
        .app()
        .execute(
            alice,
            SetDraftDeadline {
                game_id: game_a.id(),
                draft_deadline_at: Some(future_deadline),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.draft_deadline_at(), Some(future_deadline));

    // Alice saves custom prompts and locks; Bob & Carol stay empty/unlocked.
    let alice_prompts = balanced_prompts_for(
        &player_ids,
        alice_id,
        tiles_needed,
        PromptSource::Custom,
        "AliceCustom",
    );
    harness
        .app()
        .execute(
            alice,
            SaveDraftPrompts {
                game_id: game_a.id(),
                prompts: alice_prompts,
            },
        )
        .await
        .unwrap();
    harness
        .app()
        .execute(
            alice,
            LockDraftPrompts {
                game_id: game_a.id(),
            },
        )
        .await
        .unwrap();

    // Host fills remaining drafts without immediate start -> all locked & 60s countdown starts.
    let filled = harness
        .app()
        .execute(
            alice,
            HostFillRemainingDrafts {
                game_id: game_a.id(),
                start_immediately: false,
            },
        )
        .await
        .unwrap();
    let Game::Drafting(d) = filled else {
        panic!("expected Drafting with countdown");
    };
    assert!(d.draft_closes_at().is_some());

    // Now start immediately -> transitions to Active with 9 tiles per board.
    let started = harness
        .app()
        .execute(
            alice,
            HostFillRemainingDrafts {
                game_id: game_a.id(),
                start_immediately: true,
            },
        )
        .await
        .unwrap();
    assert!(matches!(started, Game::Active(_)));
    let alice_tiles = load_tiles(&harness, alice, game_a.id()).await;
    assert_eq!(alice_tiles.len(), 9);

    // Game B tests automatic autofill + start when draft_deadline_at expires.
    let deadline_b = harness.jetzt() + Duration::minutes(10);
    let game_b = harness
        .app()
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Deadline Auto Start").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
                draft_deadline_at: Some(deadline_b),
                ends_at: None,
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
                    invite_code: game_b.invite_code().clone(),
                },
            )
            .await
            .unwrap();
    }
    harness
        .app()
        .execute(
            alice,
            StartDrafting {
                game_id: game_b.id(),
            },
        )
        .await
        .unwrap();

    let before = harness
        .app()
        .execute(AuthContext::Anonymous, StartReadyGames)
        .await
        .unwrap();
    assert!(before.started_games.is_empty());
    assert_eq!(before.next_draft_closes_at, Some(deadline_b));

    harness.vorspulen(Duration::minutes(10));
    let after = harness
        .app()
        .execute(AuthContext::Anonymous, StartReadyGames)
        .await
        .unwrap();
    assert_eq!(after.started_games, vec![game_b.id()]);
    assert_eq!(after.next_draft_closes_at, None);
    let bob_tiles = load_tiles(&harness, bob, game_b.id()).await;
    assert_eq!(bob_tiles.len(), 9);
}

#[test_log::test(pollster::test)]
async fn ends_at_deadline_picks_most_filled_tiebreaks_and_draws_on_zero_captures() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "dl-alice@example.at").await;
    let bob = register_user(&harness, "dl-bob@example.at").await;
    let carol = register_user(&harness, "dl-carol@example.at").await;
    let alice_id = user_id_of(alice);
    let bob_id = user_id_of(bob);
    let carol_id = user_id_of(carol);
    let ends_at = harness.jetzt() + Duration::hours(1);

    // Game 1: tie-breaker on most filled (1 tile each, Carol captured earlier -> Carol wins + gets Gold Medal).
    let g1 = harness
        .app()
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Deadline Tiebreak").unwrap(),
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
                    invite_code: g1.invite_code().clone(),
                },
            )
            .await
            .unwrap();
    }
    harness
        .app()
        .execute(alice, StartDrafting { game_id: g1.id() })
        .await
        .unwrap();
    harness
        .app()
        .execute(
            alice,
            HostFillRemainingDrafts {
                game_id: g1.id(),
                start_immediately: true,
            },
        )
        .await
        .unwrap();

    let png = png_bytes_for_test(16, 16);
    let carol_tile = load_tiles(&harness, carol, g1.id()).await[0].id();
    let bob_tile = load_tiles(&harness, bob, g1.id()).await[0].id();
    harness
        .app()
        .execute(
            carol,
            SubmitCapture {
                game_id: g1.id(),
                tile_id: carol_tile,
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap();
    harness.vorspulen(Duration::minutes(2));
    harness
        .app()
        .execute(
            bob,
            SubmitCapture {
                game_id: g1.id(),
                tile_id: bob_tile,
                image_bytes: png,
            },
        )
        .await
        .unwrap();

    // Game 2: 0 captures by deadline -> Completed with CompletionReason::Deadline & winner_user_id == None (draw).
    let ends_at_g2 = harness.jetzt() + Duration::minutes(30);
    let g2 = harness
        .app()
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Deadline Draw").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
                draft_deadline_at: None,
                ends_at: Some(ends_at_g2),
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
                    invite_code: g2.invite_code().clone(),
                },
            )
            .await
            .unwrap();
    }
    harness
        .app()
        .execute(alice, StartDrafting { game_id: g2.id() })
        .await
        .unwrap();
    harness
        .app()
        .execute(
            alice,
            HostFillRemainingDrafts {
                game_id: g2.id(),
                start_immediately: true,
            },
        )
        .await
        .unwrap();

    harness.vorspulen(Duration::hours(2));
    let outcome = harness
        .app()
        .execute(AuthContext::Anonymous, FinishExpiredGames)
        .await
        .unwrap();
    assert_eq!(outcome.finished_games.len(), 2);

    let (loaded_g1, loaded_g2, alice_u, bob_u, carol_u) = harness
        .app()
        .execute_fn(AuthContext::Anonymous, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let g1 = uow.games().find_by_id(g1.id()).await.unwrap().into_data();
            let g2 = uow.games().find_by_id(g2.id()).await.unwrap().into_data();
            let au = uow
                .benutzer()
                .find_by_id(alice_id)
                .await
                .unwrap()
                .into_data();
            let bu = uow.benutzer().find_by_id(bob_id).await.unwrap().into_data();
            let cu = uow
                .benutzer()
                .find_by_id(carol_id)
                .await
                .unwrap()
                .into_data();
            uow.finish(
                Ok::<_, error_stack::Report<RepositoryError>>((g1, g2, au, bu, cu)),
                RepositoryError::Unknown,
            )
            .await
        })
        .await
        .unwrap();

    let Game::Completed(c1) = loaded_g1 else {
        panic!("expected g1 completed");
    };
    assert_eq!(c1.winner_user_id(), Some(carol_id));
    assert_eq!(
        c1.completion_reason(),
        friendarazzi_core::domain::CompletionReason::Deadline
    );
    assert_eq!(carol_u.gold_medal_count(), 1);
    assert_eq!(bob_u.gold_medal_count(), 0);
    assert_eq!(alice_u.gold_medal_count(), 0);

    let Game::Completed(c2) = loaded_g2 else {
        panic!("expected g2 completed");
    };
    assert_eq!(c2.winner_user_id(), None);
    assert_eq!(
        c2.completion_reason(),
        friendarazzi_core::domain::CompletionReason::Deadline
    );
}

#[test_log::test(pollster::test)]
async fn roster_leave_kick_revert_transfer_abandon_and_late_join() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "rl-alice@example.at").await;
    let bob = register_user(&harness, "rl-bob@example.at").await;
    let carol = register_user(&harness, "rl-carol@example.at").await;
    let dave = register_user(&harness, "rl-dave@example.at").await;
    let eve = register_user(&harness, "rl-eve@example.at").await;

    let alice_id = user_id_of(alice);
    let bob_id = user_id_of(bob);
    let carol_id = user_id_of(carol);
    let dave_id = user_id_of(dave);
    let eve_id = user_id_of(eve);

    let game = harness
        .app()
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Roster Lifecycle").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
                draft_deadline_at: None,
                ends_at: None,
            },
        )
        .await
        .unwrap();
    let gid = game.id();
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

    // Enter Drafting with 3 players, add Dave? No, Dave can't join in Drafting yet, so first host leaves:
    let after_draft = harness
        .app()
        .execute(alice, StartDrafting { game_id: gid })
        .await
        .unwrap();
    assert!(matches!(after_draft, Game::Drafting(_)));

    // Alice saves a prompt before leaving so we verify her drafts are cleaned up too.
    let prompts = balanced_prompts_for(
        &[alice_id, bob_id, carol_id],
        alice_id,
        after_draft.playable_tile_count(),
        PromptSource::Custom,
        "ByeAlice",
    );
    harness
        .app()
        .execute(
            alice,
            SaveDraftPrompts {
                game_id: gid,
                prompts,
            },
        )
        .await
        .unwrap();

    // Host Alice leaves during Drafting -> auto-transfers host to Bob (earliest) AND reverts to Lobby (<3 players)!
    let after_leave = harness
        .app()
        .execute(alice, LeaveGame { game_id: gid })
        .await
        .unwrap();
    assert!(matches!(after_leave, Game::Lobby(_)));
    assert_eq!(after_leave.host_user_id(), bob_id);
    assert!(!after_leave.has_player(alice_id));

    // Dave joins and Eve joins -> 4 players in Lobby, Bob starts Drafting again.
    for guest in [dave, eve] {
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
    harness
        .app()
        .execute(bob, StartDrafting { game_id: gid })
        .await
        .unwrap();

    // Non-host cannot kick or transfer host.
    let err = harness
        .app()
        .execute(
            carol,
            KickPlayer {
                game_id: gid,
                target_user_id: dave_id,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err.current_context(), GameUseCaseError::Forbidden));

    let err = harness
        .app()
        .execute(
            carol,
            TransferHost {
                game_id: gid,
                new_host_user_id: dave_id,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err.current_context(), GameUseCaseError::Forbidden));

    // Bob transfers host to Carol, Carol kicks Dave (remains Drafting since 3 left: Bob, Carol, Eve).
    let transferred = harness
        .app()
        .execute(
            bob,
            TransferHost {
                game_id: gid,
                new_host_user_id: carol_id,
            },
        )
        .await
        .unwrap();
    assert_eq!(transferred.host_user_id(), carol_id);

    let after_kick = harness
        .app()
        .execute(
            carol,
            KickPlayer {
                game_id: gid,
                target_user_id: dave_id,
            },
        )
        .await
        .unwrap();
    assert!(matches!(after_kick, Game::Drafting(_)));
    assert!(!after_kick.has_player(dave_id));

    // Start active play cleanly with Bob, Carol, Eve.
    let active = harness
        .app()
        .execute(
            carol,
            HostFillRemainingDrafts {
                game_id: gid,
                start_immediately: true,
            },
        )
        .await
        .unwrap();
    assert!(matches!(active, Game::Active(_)));

    // Late join: Dave joins while game is Active and gets a 9-tile starter-deck board!
    let late_joined = harness
        .app()
        .execute(
            dave,
            JoinGame {
                invite_code: game.invite_code().clone(),
            },
        )
        .await
        .unwrap();
    assert!(late_joined.has_player(dave_id));
    assert_eq!(load_tiles(&harness, dave, gid).await.len(), 9);
    let _ = eve_id;

    // Host Carol abandons the game -> Completed with CompletionReason::Abandoned, winner == None, 0 medals.
    let abandoned = harness
        .app()
        .execute(carol, AbandonGame { game_id: gid })
        .await
        .unwrap();
    let Game::Completed(comp) = abandoned else {
        panic!("expected completed after abandon");
    };
    assert_eq!(comp.winner_user_id(), None);
    assert_eq!(
        comp.completion_reason(),
        friendarazzi_core::domain::CompletionReason::Abandoned
    );
}
