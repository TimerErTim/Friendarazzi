use chrono::Duration;
use logandaim_core::{
    application::AuthContext,
    domain::{
        EmailAdresse, GameStatus, GridSize, NonEmptyString, PromptSource, VisibilityMode,
        konto::{BenutzerId, KlartextPasswort, SecretToken, SessionId},
        media::png_bytes_for_test,
        target_quotas_for_author,
    },
    ports::ClientKey,
    service::{
        CreateGame, DraftPromptInput, GameUseCaseError, JoinGame, LockDraftPrompts,
        RegistrierungAbschließen, RegistrierungStarten, SaveDraftPrompts, StartDrafting,
        SubmitCapture, UpdateProfile,
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

#[test_log::test(pollster::test)]
async fn full_game_lifecycle_bingo_awards_gold_medal() {
    let harness = build(super::super::base_app_builder().await);
    let alice = register_user(&harness, "alice@example.at").await;
    let bob = register_user(&harness, "bob@example.at").await;
    let carol = register_user(&harness, "carol@example.at").await;

    let alice_id = user_id_of(alice);
    let bob_id = user_id_of(bob);
    let carol_id = user_id_of(carol);

    // 1. Create Lobby (3x3 with free space -> 8 playable tiles per player)
    let game = harness
        .app()
        .execute(
            alice,
            CreateGame {
                title: NonEmptyString::neu("Freitagabend").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::Undercover,
                include_free_space: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(game.status(), GameStatus::Lobby);

    // Cannot start drafting with < 3 players
    let too_few = harness
        .app()
        .execute(alice, StartDrafting { game_id: game.id() })
        .await
        .unwrap_err();
    assert!(matches!(
        too_few.current_context(),
        GameUseCaseError::TooFewPlayers
    ));

    // Bob and Carol join
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

    // Host starts drafting
    let drafted = harness
        .app()
        .execute(alice, StartDrafting { game_id: game.id() })
        .await
        .unwrap();
    assert_eq!(drafted.status(), GameStatus::Drafting);

    let player_ids = vec![alice_id, bob_id, carol_id];
    let tiles_needed = drafted.playable_tile_count();

    for auth in [alice, bob, carol] {
        let uid = user_id_of(auth);
        let quotas = target_quotas_for_author(&player_ids, uid, tiles_needed).unwrap();
        let mut prompts = Vec::new();
        for (target, count) in quotas {
            for idx in 0..count {
                prompts.push(DraftPromptInput {
                    target_user_id: target,
                    text: NonEmptyString::neu(format!("Action {idx}")).unwrap(),
                    source: PromptSource::Custom,
                });
            }
        }
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
        if auth == carol {
            assert_eq!(state.status(), GameStatus::Active);
        } else {
            assert_eq!(state.status(), GameStatus::Drafting);
        }
    }

    // Verify Alice's board satisfies author != owner && target != owner
    let alice_tiles = harness
        .app()
        .execute_fn(alice, async |ctx| {
            let uow = ctx.enter().await.unwrap();
            let board = uow
                .boards()
                .find_board_by_game_and_owner(game.id(), alice_id)
                .await
                .unwrap();
            let tiles = uow.boards().find_tiles_by_board(board.id()).await.unwrap();
            uow.finish(
                Ok::<_, error_stack::Report<logandaim_core::ports::RepositoryError>>(tiles),
                logandaim_core::ports::RepositoryError::Unknown,
            )
            .await
        })
        .await
        .unwrap();

    assert_eq!(alice_tiles.len(), 9);
    for t in &alice_tiles {
        if !t.is_free_space() {
            assert_ne!(t.target_user_id(), Some(alice_id));
        }
    }

    // Capture (0,0) and (2,2) -> with (1,1) free space, Alice hits main diagonal Bingo!
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

    // Alice now has 1 Gold Medal and unlocked "Schnappschuss-Jäger"
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
