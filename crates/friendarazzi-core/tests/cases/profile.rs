//! Shared user profile, title unlock, and avatar storage conformance tests.

use friendarazzi_core::{
    domain::{GridSize, NonEmptyString, VisibilityMode, media::png_bytes_for_test},
    ports::{RepositoryError, collect_object},
    service::{ProfileUseCaseError, SubmitCapture, UpdateProfile, UploadAvatar},
};

use super::helpers::{build, is_webp, load_tiles, register_user, setup_active_game};

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
                username: NonEmptyString::neu("alice_p").unwrap(),
                display_name: NonEmptyString::neu("Alice").unwrap(),
                equipped_title: Some(NonEmptyString::neu("Schnappschuss-Jäger").unwrap()),
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
                username: NonEmptyString::neu("a").unwrap(),
                display_name: NonEmptyString::neu("Alice").unwrap(),
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
                username: NonEmptyString::neu("alice_unique").unwrap(),
                display_name: NonEmptyString::neu("Alice P").unwrap(),
                equipped_title: Some(NonEmptyString::neu("Neuling").unwrap()),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.username().as_str(), "alice_unique");
    assert_eq!(updated.effective_title(), "Neuling");

    // Keeping one's own username while editing display_name succeeds; None clears equipped title
    let updated2 = harness
        .app()
        .execute(
            alice,
            UpdateProfile {
                username: NonEmptyString::neu("alice_unique").unwrap(),
                display_name: NonEmptyString::neu("Alice Updated").unwrap(),
                equipped_title: None,
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
                username: NonEmptyString::neu("alice_unique").unwrap(),
                display_name: NonEmptyString::neu("Bob").unwrap(),
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

    // Upload avatar #1, then replace with avatar #2 and verify WebP format + old blob deletion
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

    let (old_missing, new_is_webp) = harness
        .app()
        .execute_fn(alice, async move |ctx| {
            let old_res = ctx.object_store().get(&key1).await.unwrap();
            let new_stream = ctx.object_store().get(&key2).await.unwrap().unwrap();
            let new_bytes = collect_object(new_stream).await.unwrap();
            Ok::<_, error_stack::Report<RepositoryError>>((old_res.is_none(), is_webp(&new_bytes)))
        })
        .await
        .unwrap();
    assert!(old_missing, "replaced avatar blob should be deleted");
    assert!(new_is_webp, "new avatar blob should be encoded as WebP");

    // Alice wins 3 games -> unlocks "Paparazzo" (requires 3 medals), while "Paparazzi King" (5) stays locked
    for n in 0..3 {
        let g = setup_active_game(
            &harness,
            alice,
            &[bob, carol],
            &format!("Win {n}"),
            GridSize::Three,
            VisibilityMode::Undercover,
        )
        .await;
        let tiles = load_tiles(&harness, alice, g.id()).await;
        for coord in [(0, 0), (1, 1), (2, 2)] {
            let tid = tiles
                .iter()
                .find(|t| t.row() == coord.0 && t.col() == coord.1)
                .unwrap()
                .id();
            harness
                .app()
                .execute(
                    alice,
                    SubmitCapture {
                        game_id: g.id(),
                        tile_id: tid,
                        image_bytes: png.clone(),
                    },
                )
                .await
                .unwrap();
        }
    }

    let err = harness
        .app()
        .execute(
            alice,
            UpdateProfile {
                username: NonEmptyString::neu("alice_unique").unwrap(),
                display_name: NonEmptyString::neu("Alice").unwrap(),
                equipped_title: Some(NonEmptyString::neu("Paparazzi King").unwrap()),
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
                username: NonEmptyString::neu("alice_unique").unwrap(),
                display_name: NonEmptyString::neu("Alice").unwrap(),
                equipped_title: Some(NonEmptyString::neu("Paparazzo").unwrap()),
            },
        )
        .await
        .unwrap();
    assert_eq!(paparazzi.gold_medal_count(), 3);
    assert_eq!(paparazzi.effective_title(), "Paparazzo");
}
