use friendarazzi_core::domain::media::png_bytes_for_test;
use poem::http::StatusCode;
use serde_json::json;

use crate::{E2eHarness, enroll_and_start_game, neuer_benutzer};

#[test_log::test(tokio::test)]
async fn e2e_profile_update_avatar_upload_and_caught_vs_captured_gallery() {
    let harness = E2eHarness::new().await;
    let (t_alice, id_alice) = neuer_benutzer(&harness, "pg-alice@example.at").await;
    let (t_bob, id_bob) = neuer_benutzer(&harness, "pg-bob@example.at").await;
    let (t_carol, _) = neuer_benutzer(&harness, "pg-carol@example.at").await;

    // Locked title -> BadRequest
    let (st, _) = harness
        .client
        .put_json_auth(
            "/api/profil",
            json!({
                "username": "alice_pg",
                "displayName": "Alice PG",
                "equippedTitle": "Schnappschuss-Jäger"
            }),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // Valid profile update
    let (st, prof) = harness
        .client
        .put_json_auth(
            "/api/profil",
            json!({
                "username": "alice_pg",
                "displayName": "Alice PG",
                "equippedTitle": "Neuling"
            }),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{prof}");
    assert_eq!(prof["username"], "alice_pg");
    assert_eq!(prof["displayName"], "Alice PG");

    // Duplicate username by Bob -> Conflict
    let (st, _) = harness
        .client
        .put_json_auth(
            "/api/profil",
            json!({
                "username": "alice_pg",
                "displayName": "Bob",
                "equippedTitle": null
            }),
            &t_bob,
        )
        .await;
    assert_eq!(st, StatusCode::CONFLICT);

    // Avatar upload #1 -> accessible via /api/media/{key}
    let (st, av1) = harness
        .client
        .post_multipart_file_auth(
            "/api/profil/avatar",
            "file",
            "av1.png",
            "image/png",
            png_bytes_for_test(96, 96),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{av1}");
    let key1 = av1["avatarStorageKey"].as_str().unwrap().to_owned();
    let (media_st, bytes, ct) = harness
        .client
        .get_bytes_auth(&format!("/api/media/{key1}"), &t_bob)
        .await;
    assert_eq!(media_st, StatusCode::OK);
    assert_eq!(ct, "image/webp");
    assert!(!bytes.is_empty());

    // Avatar upload #2 -> replaces avatar and deletes old key
    let (st, av2) = harness
        .client
        .post_multipart_file_auth(
            "/api/profil/avatar",
            "file",
            "av2.png",
            "image/png",
            png_bytes_for_test(112, 112),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{av2}");
    let key2 = av2["avatarStorageKey"].as_str().unwrap().to_owned();
    assert_ne!(key1, key2);
    let (old_st, _, _) = harness
        .client
        .get_bytes_auth(&format!("/api/media/{key1}"), &t_bob)
        .await;
    assert_eq!(old_st, StatusCode::NOT_FOUND);
    let (new_st, _, _) = harness
        .client
        .get_bytes_auth(&format!("/api/media/{key2}"), &t_bob)
        .await;
    assert_eq!(new_st, StatusCode::OK);

    // Play a LiveFeed game where Alice captures Bob so Bob's `caught` tab receives it
    let (game_id, _) = enroll_and_start_game(
        &harness,
        &t_alice,
        &[&t_bob, &t_carol],
        "Gallery Check",
        3,
        "LiveFeed",
    )
    .await;

    let (_, alice_detail) = harness
        .client
        .get_json_auth(&format!("/api/games/{game_id}"), &t_alice)
        .await;
    let bob_target_tile = alice_detail["myBoard"]["tiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["targetUser"]["id"].as_str() == Some(id_bob.as_str()))
        .expect("Alice's board should contain at least one tile targeting Bob");
    let tile_id = bob_target_tile["id"].as_str().unwrap().to_owned();

    let (st, cap) = harness
        .client
        .post_multipart_file_auth(
            &format!("/api/games/{game_id}/tiles/{tile_id}/capture"),
            "file",
            "caught_bob.png",
            "image/png",
            png_bytes_for_test(140, 140),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{cap}");

    // Bob's gallery: `caught` has 1 group with Alice's photo of Bob; Alice's `captured` has it
    let (st, bob_gallery) = harness
        .client
        .get_json_auth(&format!("/api/users/{id_bob}/gallery"), &t_bob)
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(bob_gallery["caught"].as_array().unwrap().len(), 1);
    assert_eq!(
        bob_gallery["caught"][0]["photos"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        bob_gallery["caught"][0]["photos"][0]["catcher"]["id"],
        id_alice
    );

    let (st, alice_gallery) = harness
        .client
        .get_json_auth(&format!("/api/users/{id_alice}/gallery"), &t_bob)
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(alice_gallery["captured"].as_array().unwrap().len(), 1);
    assert_eq!(
        alice_gallery["captured"][0]["photos"][0]["target"]["id"],
        id_bob
    );
}
