use std::collections::{HashMap, HashSet};

use chrono::Duration;
use friendarazzi_core::{
    application::AuthContext,
    domain::{
        BoardTile, BoardTileId, EmailAdresse, Game, GameId, GameStatus, GridSize, InviteCode,
        NonEmptyString, PromptSource, PromptSubmissionId, SuperlativeKind, VisibilityMode,
        compute_superlatives,
        konto::{BenutzerId, KlartextPasswort, SecretToken, SessionId},
        media::png_bytes_for_test,
        target_quotas_for_author,
    },
    ports::{ClientKey, RepositoryError, collect_object},
    service::{
        CaptureUseCaseError, CreateGame, DraftPromptInput, GameUseCaseError, JoinGame,
        LockDraftPrompts, ProfileUseCaseError, RegistrierungAbschließen, RegistrierungStarten,
        SaveDraftPrompts, StartDrafting, SubmitCapture, UpdateGameSettings, UpdateProfile,
        UploadAvatar,
    },
};
use uuid::Uuid;

use super::helpers::{Harness, build};

const PASSWORT: &str = "korrekt-pferd-batterie";

async fn register_user(harness: &Harness, email: &str) -> AuthContext {
    let client_key = ClientKey::new("127.0.0.1".parse().unwrap(), email.to_owned());
    harness
        .app()
        .execute(
            AuthContext::Anonymous,
            RegistrierungStarten {
                email: EmailAdresse::normalisiert(email).unwrap(),
                client_key: client_key.clone(),
            },
        )
        .await
        .unwrap();
    let token = SecretToken::try_from(harness.letztes_token().as_str()).unwrap();
    let session = harness
        .app()
        .execute(
            AuthContext::Anonymous,
            RegistrierungAbschließen {
                token,
                passwort: KlartextPasswort::neu(PASSWORT).unwrap(),
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

fn user_id_of(auth: AuthContext) -> BenutzerId {
    auth.require_benutzer_id().unwrap()
}

fn balanced_prompts_for(
    player_ids: &[BenutzerId],
    author: BenutzerId,
    tiles_needed: usize,
    source: PromptSource,
    label_prefix: &str,
) -> Vec<DraftPromptInput> {
    let quotas = target_quotas_for_author(player_ids, author, tiles_needed).unwrap();
    let mut prompts = Vec::new();
    for (target, count) in quotas {
        for idx in 0..count {
            prompts.push(DraftPromptInput {
                target_user_id: target,
                text: NonEmptyString::neu(format!("{label_prefix} {idx}")).unwrap(),
                source,
            });
        }
    }
    prompts
}

async fn setup_active_game(
    harness: &Harness,
    host: AuthContext,
    guests: &[AuthContext],
    title: &str,
    grid_size: GridSize,
    visibility_mode: VisibilityMode,
    include_free_space: bool,
) -> Game {
    let game = harness
        .app()
        .execute(
            host,
            CreateGame {
                title: NonEmptyString::neu(title).unwrap(),
                grid_size,
                visibility_mode,
                include_free_space,
            },
        )
        .await
        .unwrap();

    for &guest in guests {
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
        .execute(host, StartDrafting { game_id: game.id() })
        .await
        .unwrap();

    let mut all_auths = Vec::with_capacity(1 + guests.len());
    all_auths.push(host);
    all_auths.extend_from_slice(guests);
    let player_ids: Vec<_> = all_auths.iter().copied().map(user_id_of).collect();
    let tiles_needed = drafted.playable_tile_count();

    let mut last_state = drafted;
    for (p_idx, &auth) in all_auths.iter().enumerate() {
        let uid = user_id_of(auth);
        let prompts = balanced_prompts_for(
            &player_ids,
            uid,
            tiles_needed,
            PromptSource::Custom,
            &format!("P{p_idx} Action"),
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
        last_state = harness
            .app()
            .execute(auth, LockDraftPrompts { game_id: game.id() })
            .await
            .unwrap();
    }
    assert_eq!(last_state.status(), GameStatus::Active);
    last_state
}

async fn load_tiles(harness: &Harness, auth: AuthContext, game_id: GameId) -> Vec<BoardTile> {
    let owner_id = user_id_of(auth);
    harness
        .app()
        .execute_fn(auth, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let board = uow
                .boards()
                .find_board_by_game_and_owner(game_id, owner_id)
                .await
                .unwrap();
            let tiles = uow.boards().find_tiles_by_board(board.id()).await.unwrap();
            uow.finish(
                Ok::<_, error_stack::Report<RepositoryError>>(tiles),
                RepositoryError::Unknown,
            )
            .await
        })
        .await
        .unwrap()
}

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
        true,
    )
    .await;

    let alice_tiles = load_tiles(&harness, alice, game.id()).await;
    assert_eq!(alice_tiles.len(), 9);
    for t in &alice_tiles {
        if !t.is_free_space() {
            assert_ne!(t.target_user_id(), Some(alice_id));
        }
    }

    let tile_00 = alice_tiles
        .iter()
        .find(|t| t.row() == 0 && t.col() == 0)
        .unwrap()
        .id();
    let tile_22 = alice_tiles
        .iter()
        .find(|t| t.row() == 2 && t.col() == 2)
        .unwrap()
        .id();

    let png = png_bytes_for_test(320, 240);
    let out1 = harness
        .app()
        .execute(
            alice,
            SubmitCapture {
                game_id: game.id(),
                tile_id: tile_00,
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap();
    assert!(!out1.game_completed);

    harness.vorspulen(Duration::seconds(30));
    let out2 = harness
        .app()
        .execute(
            alice,
            SubmitCapture {
                game_id: game.id(),
                tile_id: tile_22,
                image_bytes: png,
            },
        )
        .await
        .unwrap();
    assert!(out2.game_completed);
    assert!(out2.winning_line.is_some());

    let updated = harness
        .app()
        .execute(
            alice,
            UpdateProfile {
                username: "alice_winner".into(),
                display_name: "Alice".into(),
                equipped_title: Some("Schnappschuss-Jäger".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.gold_medal_count(), 1);
    assert_eq!(updated.effective_title(), "Schnappschuss-Jäger");
}

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
                include_free_space: true,
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
                include_free_space: false,
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
                username: "anon".into(),
                display_name: "Anon".into(),
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
                include_free_space: true,
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

    // Re-joining as an already enrolled member is idempotent (still 2 players, so still TooFewPlayers)
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
                include_free_space: false,
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
                include_free_space: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.title().as_str(), "Wochenende 4x4");
    assert_eq!(updated.grid_size(), GridSize::Four);
    assert_eq!(updated.visibility_mode(), VisibilityMode::LiveFeed);
    assert!(!updated.include_free_space());

    // Host starts drafting
    let drafted = harness
        .app()
        .execute(alice, StartDrafting { game_id: game.id() })
        .await
        .unwrap();
    assert_eq!(drafted.status(), GameStatus::Drafting);

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
                include_free_space: true,
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
    let carol_id = user_id_of(carol);
    let outsider_id = user_id_of(outsider);
    let player_ids = vec![alice_id, bob_id, carol_id];

    let game = harness
        .app()
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Drafting Checks").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::BlurredTeaser,
                include_free_space: true,
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
    assert_eq!(still_drafting.status(), GameStatus::Drafting);

    // Alice edits her prompts before Bob & Carol finish -> replaces prompts AND resets draft_locked to false
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

    // Even when Bob and Carol lock in now, game MUST stay Drafting because Alice edited & unlocked
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
        assert_eq!(state.status(), GameStatus::Drafting);
    }

    // Re-locking Alice completes all locks and activates the game with Alice's edited prompts only
    let active = harness
        .app()
        .execute(alice, LockDraftPrompts { game_id: game.id() })
        .await
        .unwrap();
    assert_eq!(active.status(), GameStatus::Active);

    let all_prompts = harness
        .app()
        .execute_fn(alice, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let list = uow.games().find_prompts_by_game(game.id()).await.unwrap();
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

    // 4x4 with include_free_space: true -> even grid must still have 16 playable tiles (no free space!)
    let game_4x4 = setup_active_game(
        &harness,
        p1,
        &[p2, p3, p4],
        "4x4 Runde",
        GridSize::Four,
        VisibilityMode::LiveFeed,
        true,
    )
    .await;
    assert_eq!(game_4x4.playable_tile_count(), 16);

    let g4_id = game_4x4.id();
    let prompts_4x4: HashMap<PromptSubmissionId, BenutzerId> = harness
        .app()
        .execute_fn(p1, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let map: HashMap<_, _> = uow
                .games()
                .find_prompts_by_game(g4_id)
                .await
                .unwrap()
                .into_iter()
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

    let mut placed_4x4 = HashSet::new();
    for auth in players {
        let owner = user_id_of(auth);
        let tiles = load_tiles(&harness, auth, g4_id).await;
        assert_eq!(tiles.len(), 16);
        for t in tiles {
            assert!(!t.is_free_space(), "4x4 grid cannot have a free space");
            assert_ne!(t.target_user_id(), Some(owner));
            let prompt_id = t.prompt_submission_id().unwrap();
            assert_ne!(prompts_4x4[&prompt_id], owner);
            assert!(placed_4x4.insert(prompt_id), "prompt placed twice");
        }
    }
    assert_eq!(placed_4x4.len(), prompts_4x4.len());

    // 5x5 with include_free_space: true -> 24 playable tiles + 1 free space at center (2,2)
    let game_5x5 = setup_active_game(
        &harness,
        p1,
        &[p2, p3, p4],
        "5x5 Großrunde",
        GridSize::Five,
        VisibilityMode::Undercover,
        true,
    )
    .await;
    assert_eq!(game_5x5.playable_tile_count(), 24);

    let g5_id = game_5x5.id();
    let prompts_5x5: HashMap<PromptSubmissionId, BenutzerId> = harness
        .app()
        .execute_fn(p1, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let map: HashMap<_, _> = uow
                .games()
                .find_prompts_by_game(g5_id)
                .await
                .unwrap()
                .into_iter()
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

    for auth in players {
        let owner = user_id_of(auth);
        let tiles = load_tiles(&harness, auth, g5_id).await;
        assert_eq!(tiles.len(), 25);
        for t in tiles {
            if t.row() == 2 && t.col() == 2 {
                assert!(t.is_free_space());
                assert!(t.prompt_submission_id().is_none());
                assert!(t.target_user_id().is_none());
            } else {
                assert!(!t.is_free_space());
                assert_ne!(t.target_user_id(), Some(owner));
                let prompt_id = t.prompt_submission_id().unwrap();
                assert_ne!(prompts_5x5[&prompt_id], owner);
            }
        }
    }
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
        true,
    )
    .await;

    let alice_tiles = load_tiles(&harness, alice, game.id()).await;
    let bob_tiles = load_tiles(&harness, bob, game.id()).await;

    let alice_00 = alice_tiles
        .iter()
        .find(|t| t.row() == 0 && t.col() == 0)
        .unwrap()
        .id();
    let alice_22 = alice_tiles
        .iter()
        .find(|t| t.row() == 2 && t.col() == 2)
        .unwrap()
        .id();
    let bob_00 = bob_tiles
        .iter()
        .find(|t| t.row() == 0 && t.col() == 0)
        .unwrap()
        .id();
    let bob_01 = bob_tiles
        .iter()
        .find(|t| t.row() == 0 && t.col() == 1)
        .unwrap()
        .id();

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

    // Valid capture stores BOTH full_jpeg and blurred_jpeg in ObjectStore
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
    assert!(!outcome.game_completed);

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
    assert!(full_bytes.starts_with(&[0xFF, 0xD8, 0xFF]));
    assert!(thumb_bytes.starts_with(&[0xFF, 0xD8, 0xFF]));

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
    assert!(!bob_out.game_completed);

    // Alice finishes Bingo -> game Completed
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
    assert!(win_out.game_completed);

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
            let prof = uow.profiles().find_by_user_id(bob_id).await.unwrap();
            let count = prof.gold_medal_count();
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

    // Game 1: Bob (non-host) wins via top row (0,0), (0,1), (0,2)
    let g_row = setup_active_game(
        &harness,
        alice,
        &[bob, carol],
        "Row Win",
        GridSize::Three,
        VisibilityMode::Undercover,
        true,
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
        assert_eq!(out.game_completed, col == 2);
        if col == 2 {
            assert_eq!(out.winning_line, Some(vec![(0, 0), (0, 1), (0, 2)]));
        }
    }

    // Game 2: Carol (non-host) wins via right column (0,2), (1,2), (2,2)
    let g_col = setup_active_game(
        &harness,
        alice,
        &[bob, carol],
        "Col Win",
        GridSize::Three,
        VisibilityMode::Undercover,
        true,
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
        assert_eq!(out.game_completed, row == 2);
        if row == 2 {
            assert_eq!(out.winning_line, Some(vec![(0, 2), (1, 2), (2, 2)]));
        }
    }

    // Game 3: Bob wins again via anti-diagonal (0,2) + center Free Space (1,1) + (2,0)
    let g_anti = setup_active_game(
        &harness,
        alice,
        &[bob, carol],
        "Anti-Diagonal Win",
        GridSize::Three,
        VisibilityMode::Undercover,
        true,
    )
    .await;
    let bob_tiles3 = load_tiles(&harness, bob, g_anti.id()).await;
    let t_02 = bob_tiles3
        .iter()
        .find(|t| t.row() == 0 && t.col() == 2)
        .unwrap()
        .id();
    let t_20 = bob_tiles3
        .iter()
        .find(|t| t.row() == 2 && t.col() == 0)
        .unwrap()
        .id();
    let o1 = harness
        .app()
        .execute(
            bob,
            SubmitCapture {
                game_id: g_anti.id(),
                tile_id: t_02,
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap();
    assert!(!o1.game_completed);
    let o2 = harness
        .app()
        .execute(
            bob,
            SubmitCapture {
                game_id: g_anti.id(),
                tile_id: t_20,
                image_bytes: png,
            },
        )
        .await
        .unwrap();
    assert!(o2.game_completed);
    assert_eq!(o2.winning_line, Some(vec![(0, 2), (1, 1), (2, 0)]));

    let (bob_medals, carol_medals) = harness
        .app()
        .execute_fn(alice, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let bm = uow
                .profiles()
                .find_by_user_id(bob_id)
                .await
                .unwrap()
                .gold_medal_count();
            let cm = uow
                .profiles()
                .find_by_user_id(carol_id)
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
        false,
    )
    .await;

    let alice_tiles = load_tiles(&harness, alice, game.id()).await;
    let bob_tiles = load_tiles(&harness, bob, game.id()).await;
    let png = png_bytes_for_test(64, 64);

    // Quickest Draw: Alice captures first at +45s
    harness.vorspulen(Duration::seconds(45));
    let first_tile = alice_tiles
        .iter()
        .find(|t| !t.is_free_space() && t.row() == 0 && t.col() == 0)
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

    // Jump 2 hours forward, then Bob completes 4 captures in row 0 -> Bob wins Sniper + Bingo
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

    assert_eq!(loaded_game.status(), GameStatus::Completed);
    assert_eq!(loaded_game.winner_user_id(), Some(user_id_of(bob)));
    assert_eq!(captures.len(), 5);

    let sups = compute_superlatives(&player_ids, &captures, loaded_game.started_at());
    let quickest = sups
        .iter()
        .find(|s| s.kind == SuperlativeKind::QuickestDraw)
        .unwrap();
    assert_eq!(quickest.user_id, user_id_of(alice));
    assert_eq!(quickest.metric_value, 45);

    let sniper = sups
        .iter()
        .find(|s| s.kind == SuperlativeKind::Sniper)
        .unwrap();
    assert_eq!(sniper.user_id, user_id_of(bob));
    assert_eq!(sniper.metric_value, 4);

    assert!(sups.iter().any(|s| s.kind == SuperlativeKind::MostWanted));
    assert!(sups.iter().any(|s| s.kind == SuperlativeKind::Ghost));
}

#[test_log::test(pollster::test)]
async fn profile_update_username_uniqueness_title_unlocks_and_avatar_cleanup() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "alice@example.at").await;
    let bob = register_user(&harness, "bob@example.at").await;
    let carol = register_user(&harness, "carol@example.at").await;
    let png = png_bytes_for_test(96, 96);

    // Initially Alice has 0 medals: equipping locked title fails
    let err = harness
        .app()
        .execute(
            alice,
            UpdateProfile {
                username: "alice_p".into(),
                display_name: "Alice".into(),
                equipped_title: Some("Schnappschuss-Jäger".into()),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        ProfileUseCaseError::TitleLocked
    ));

    // Invalid username (< 2 chars) -> BadRequest
    let err = harness
        .app()
        .execute(
            alice,
            UpdateProfile {
                username: "a".into(),
                display_name: "Alice".into(),
                equipped_title: None,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        ProfileUseCaseError::BadRequest
    ));

    // Alice claims username "alice_unique"
    let updated = harness
        .app()
        .execute(
            alice,
            UpdateProfile {
                username: "alice_unique".into(),
                display_name: "Alice P".into(),
                equipped_title: Some("Neuling".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.username().as_str(), "alice_unique");
    assert_eq!(updated.effective_title(), "Neuling");

    // Keeping one's own username while editing display_name succeeds
    let updated2 = harness
        .app()
        .execute(
            alice,
            UpdateProfile {
                username: "alice_unique".into(),
                display_name: "Alice Updated".into(),
                equipped_title: Some(String::new()),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated2.display_name().as_str(), "Alice Updated");
    assert_eq!(updated2.equipped_title(), None);

    // Bob cannot steal Alice's username -> UsernameTaken
    let err = harness
        .app()
        .execute(
            bob,
            UpdateProfile {
                username: "alice_unique".into(),
                display_name: "Bob".into(),
                equipped_title: None,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        ProfileUseCaseError::UsernameTaken
    ));

    // Invalid avatar upload -> BadRequest
    let err = harness
        .app()
        .execute(
            alice,
            UploadAvatar {
                image_bytes: b"bad-image".to_vec(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        ProfileUseCaseError::BadRequest
    ));

    // Upload avatar #1, then replace with avatar #2 and verify old avatar blob was deleted
    let prof1 = harness
        .app()
        .execute(
            alice,
            UploadAvatar {
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap();
    let key1 = prof1.avatar_storage_key().unwrap().to_owned();

    let prof2 = harness
        .app()
        .execute(
            alice,
            UploadAvatar {
                image_bytes: png.clone(),
            },
        )
        .await
        .unwrap();
    let key2 = prof2.avatar_storage_key().unwrap().to_owned();
    assert_ne!(key1, key2);

    let (old_missing, new_present) = harness
        .app()
        .execute_fn(alice, async move |ctx| {
            let old_res = ctx.object_store().get(&key1).await.unwrap();
            let new_stream = ctx.object_store().get(&key2).await.unwrap().unwrap();
            let new_bytes = collect_object(new_stream).await.unwrap();
            Ok::<_, error_stack::Report<RepositoryError>>((
                old_res.is_none(),
                !new_bytes.is_empty(),
            ))
        })
        .await
        .unwrap();
    assert!(old_missing, "replaced avatar blob should be deleted");
    assert!(new_present, "new avatar blob should exist in ObjectStore");

    // Alice wins 3 games -> unlocks "Paparazzo" (requires 3 medals), while "Paparazzi King" (5) stays locked
    for n in 0..3 {
        let g = setup_active_game(
            &harness,
            alice,
            &[bob, carol],
            &format!("Win {n}"),
            GridSize::Three,
            VisibilityMode::Undercover,
            true,
        )
        .await;
        let tiles = load_tiles(&harness, alice, g.id()).await;
        let t00 = tiles
            .iter()
            .find(|t| t.row() == 0 && t.col() == 0)
            .unwrap()
            .id();
        let t22 = tiles
            .iter()
            .find(|t| t.row() == 2 && t.col() == 2)
            .unwrap()
            .id();
        harness
            .app()
            .execute(
                alice,
                SubmitCapture {
                    game_id: g.id(),
                    tile_id: t00,
                    image_bytes: png.clone(),
                },
            )
            .await
            .unwrap();
        harness
            .app()
            .execute(
                alice,
                SubmitCapture {
                    game_id: g.id(),
                    tile_id: t22,
                    image_bytes: png.clone(),
                },
            )
            .await
            .unwrap();
    }

    let err = harness
        .app()
        .execute(
            alice,
            UpdateProfile {
                username: "alice_unique".into(),
                display_name: "Alice".into(),
                equipped_title: Some("Paparazzi King".into()),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err.current_context(),
        ProfileUseCaseError::TitleLocked
    ));

    let paparazzi = harness
        .app()
        .execute(
            alice,
            UpdateProfile {
                username: "alice_unique".into(),
                display_name: "Alice".into(),
                equipped_title: Some("Paparazzo".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(paparazzi.gold_medal_count(), 3);
    assert_eq!(paparazzi.effective_title(), "Paparazzo");
}
