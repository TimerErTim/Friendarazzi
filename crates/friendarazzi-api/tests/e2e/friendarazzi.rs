use chrono::Duration;
use friendarazzi_core::domain::media::png_bytes_for_test;
use poem::http::StatusCode;
use serde_json::json;

use crate::{E2eHarness, neuer_benutzer};

#[test_log::test(tokio::test)]
async fn e2e_friendarazzi_undercover_bingo_and_galleries() {
    let harness = E2eHarness::new().await;
    let (t_alice, id_alice) = neuer_benutzer(&harness, "alice@example.at").await;
    let (t_bob, _id_bob) = neuer_benutzer(&harness, "bob@example.at").await;
    let (t_carol, _id_carol) = neuer_benutzer(&harness, "carol@example.at").await;

    // Starter decks are seeded and accessible
    let (status, decks) = harness.client.get_json_auth("/api/decks", &t_alice).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(decks.as_array().unwrap().len(), 4);

    // Create game lobby
    let (status, created) = harness
        .client
        .post_json_auth(
            "/api/games",
            json!({
                "title": "Kiez Tour",
                "gridSize": 3,
                "visibilityMode": "Undercover",
                "includeFreeSpace": true
            }),
            &t_alice,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let game_id = created["id"].as_str().unwrap().to_owned();
    let invite_code = created["inviteCode"].as_str().unwrap().to_owned();

    // Preview by invite code & join
    let (status, preview) = harness
        .client
        .get_json_auth(&format!("/api/games/by-code/{invite_code}"), &t_bob)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["title"], "Kiez Tour");

    for token in [&t_bob, &t_carol] {
        let (status, joined) = harness
            .client
            .post_json_auth(
                "/api/games/join",
                json!({ "inviteCode": invite_code }),
                token,
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{joined}");
    }

    // Start drafting
    let (status, drafting) = harness
        .client
        .post_json_auth(
            &format!("/api/games/{game_id}/start-drafting"),
            json!({}),
            &t_alice,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{drafting}");
    assert_eq!(drafting["status"], "Drafting");

    // Each player fills their quotas & locks in
    for token in [&t_alice, &t_bob, &t_carol] {
        let (status, detail) = harness
            .client
            .get_json_auth(&format!("/api/games/{game_id}"), token)
            .await;
        assert_eq!(status, StatusCode::OK);
        let quotas = detail["myQuotas"].as_array().unwrap();
        let mut prompts = Vec::new();
        for q in quotas {
            let target_id = q["targetUser"]["id"].as_str().unwrap();
            let count = q["requiredCount"].as_u64().unwrap();
            for i in 0..count {
                prompts.push(json!({
                    "targetUserId": target_id,
                    "text": format!("Macht Quatsch #{i}"),
                    "source": "Custom"
                }));
            }
        }
        let (status, _) = harness
            .client
            .put_json_auth(
                &format!("/api/games/{game_id}/prompts"),
                json!({ "prompts": prompts }),
                token,
            )
            .await;
        assert_eq!(status, StatusCode::OK);

        let (status, locked) = harness
            .client
            .post_json_auth(
                &format!("/api/games/{game_id}/prompts/lock"),
                json!({}),
                token,
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{locked}");
    }

    // Now Active! Read Alice's board
    let (status, active) = harness
        .client
        .get_json_auth(&format!("/api/games/{game_id}"), &t_alice)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(active["status"], "Active");
    let tiles = active["myBoard"]["tiles"].as_array().unwrap();
    assert_eq!(tiles.len(), 9);
    let tile_00 = tiles
        .iter()
        .find(|t| t["row"] == 0 && t["col"] == 0)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let tile_22 = tiles
        .iter()
        .find(|t| t["row"] == 2 && t["col"] == 2)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Alice captures tile_00
    let png = png_bytes_for_test(200, 150);
    let (status, cap1) = harness
        .client
        .post_multipart_file_auth(
            &format!("/api/games/{game_id}/tiles/{tile_00}/capture"),
            "file",
            "shot.png",
            "image/png",
            png.clone(),
            &t_alice,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{cap1}");
    assert_eq!(cap1["gameCompleted"], false);
    let cap1_key = cap1["capture"]["imageStorageKey"]
        .as_str()
        .unwrap()
        .to_owned();

    // Undercover mode: Bob cannot see Alice's capture in feed OR fetch its media
    let (status, bob_view) = harness
        .client
        .get_json_auth(&format!("/api/games/{game_id}"), &t_bob)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bob_view["feed"].as_array().unwrap().len(), 0);
    let (media_status, _, _) = harness
        .client
        .get_bytes_auth(&format!("/api/media/{cap1_key}"), &t_bob)
        .await;
    assert_eq!(media_status, StatusCode::FORBIDDEN);

    // Advance time & Alice hits Bingo on (2,2)!
    harness.vorspulen(Duration::minutes(12));
    let (status, cap2) = harness
        .client
        .post_multipart_file_auth(
            &format!("/api/games/{game_id}/tiles/{tile_22}/capture"),
            "file",
            "win.png",
            "image/png",
            png,
            &t_alice,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{cap2}");
    assert_eq!(cap2["gameCompleted"], true);

    // Completed! Bob can now view all boards, superlatives, and media
    let (status, finished) = harness
        .client
        .get_json_auth(&format!("/api/games/{game_id}"), &t_bob)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(finished["status"], "Completed");
    assert_eq!(finished["winner"]["id"], id_alice);
    assert_eq!(finished["winner"]["goldMedalCount"], 1);
    assert_eq!(finished["allBoards"].as_array().unwrap().len(), 3);
    assert_eq!(finished["feed"].as_array().unwrap().len(), 2);
    assert!(!finished["superlatives"].as_array().unwrap().is_empty());

    let (media_status, bytes, ct) = harness
        .client
        .get_bytes_auth(&format!("/api/media/{cap1_key}"), &t_bob)
        .await;
    assert_eq!(media_status, StatusCode::OK);
    assert_eq!(ct, "image/jpeg");
    assert!(!bytes.is_empty());

    // Alice's gallery shows her captured photos grouped by game
    let (status, gallery) = harness
        .client
        .get_json_auth(&format!("/api/users/{id_alice}/gallery"), &t_bob)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(gallery["user"]["goldMedalCount"], 1);
    assert_eq!(gallery["captured"].as_array().unwrap().len(), 1);
    assert_eq!(
        gallery["captured"][0]["photos"].as_array().unwrap().len(),
        2
    );
}

#[test_log::test(tokio::test)]
async fn e2e_blurred_teaser_redacts_target_and_serves_thumbnail() {
    let harness = E2eHarness::new().await;
    let (t_alice, _) = neuer_benutzer(&harness, "a2@example.at").await;
    let (t_bob, _) = neuer_benutzer(&harness, "b2@example.at").await;
    let (t_carol, _) = neuer_benutzer(&harness, "c2@example.at").await;

    let (_, created) = harness
        .client
        .post_json_auth(
            "/api/games",
            json!({
                "title": "Blurred Hunt",
                "gridSize": 3,
                "visibilityMode": "BlurredTeaser",
                "includeFreeSpace": true
            }),
            &t_alice,
        )
        .await;
    let game_id = created["id"].as_str().unwrap().to_owned();
    let invite_code = created["inviteCode"].as_str().unwrap().to_owned();

    for token in [&t_bob, &t_carol] {
        harness
            .client
            .post_json_auth(
                "/api/games/join",
                json!({ "inviteCode": invite_code }),
                token,
            )
            .await;
    }
    harness
        .client
        .post_json_auth(
            &format!("/api/games/{game_id}/start-drafting"),
            json!({}),
            &t_alice,
        )
        .await;

    for token in [&t_alice, &t_bob, &t_carol] {
        let (_, detail) = harness
            .client
            .get_json_auth(&format!("/api/games/{game_id}"), token)
            .await;
        let mut prompts = Vec::new();
        for q in detail["myQuotas"].as_array().unwrap() {
            for i in 0..q["requiredCount"].as_u64().unwrap() {
                prompts.push(json!({
                    "targetUserId": q["targetUser"]["id"].as_str().unwrap(),
                    "text": format!("Hint {i}"),
                    "source": "StarterDeck"
                }));
            }
        }
        harness
            .client
            .put_json_auth(
                &format!("/api/games/{game_id}/prompts"),
                json!({ "prompts": prompts }),
                token,
            )
            .await;
        harness
            .client
            .post_json_auth(
                &format!("/api/games/{game_id}/prompts/lock"),
                json!({}),
                token,
            )
            .await;
    }

    let (_, active) = harness
        .client
        .get_json_auth(&format!("/api/games/{game_id}"), &t_alice)
        .await;
    let tile_00 = active["myBoard"]["tiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["row"] == 0 && t["col"] == 0)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let (_, cap) = harness
        .client
        .post_multipart_file_auth(
            &format!("/api/games/{game_id}/tiles/{tile_00}/capture"),
            "file",
            "shot.png",
            "image/png",
            png_bytes_for_test(160, 160),
            &t_alice,
        )
        .await;
    let full_key = cap["capture"]["imageStorageKey"]
        .as_str()
        .unwrap()
        .to_owned();

    let (status, bob_view) = harness
        .client
        .get_json_auth(&format!("/api/games/{game_id}"), &t_bob)
        .await;
    assert_eq!(status, StatusCode::OK);
    let feed = bob_view["feed"].as_array().unwrap();
    assert_eq!(feed.len(), 1);
    assert_eq!(feed[0]["isBlurred"], true);
    assert!(feed[0]["target"].is_null());
    assert!(feed[0]["promptText"].is_null());
    assert!(feed[0]["imageStorageKey"].is_null());
    let thumb_key = feed[0]["thumbnailStorageKey"].as_str().unwrap();

    // Bob can fetch the blurred thumbnail, but NOT the full photo!
    let (thumb_status, _, _) = harness
        .client
        .get_bytes_auth(&format!("/api/media/{thumb_key}"), &t_bob)
        .await;
    assert_eq!(thumb_status, StatusCode::OK);

    let (full_status, _, _) = harness
        .client
        .get_bytes_auth(&format!("/api/media/{full_key}"), &t_bob)
        .await;
    assert_eq!(full_status, StatusCode::FORBIDDEN);
}

async fn e2e_enroll_and_start_game(
    harness: &E2eHarness,
    host_token: &str,
    guest_tokens: &[&str],
    title: &str,
    grid_size: u8,
    visibility_mode: &str,
    include_free_space: bool,
) -> (String, String) {
    let (status, created) = harness
        .client
        .post_json_auth(
            "/api/games",
            json!({
                "title": title,
                "gridSize": grid_size,
                "visibilityMode": visibility_mode,
                "includeFreeSpace": include_free_space
            }),
            host_token,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let game_id = created["id"].as_str().unwrap().to_owned();
    let invite_code = created["inviteCode"].as_str().unwrap().to_owned();

    for token in guest_tokens {
        let (st, joined) = harness
            .client
            .post_json_auth(
                "/api/games/join",
                json!({ "inviteCode": invite_code }),
                token,
            )
            .await;
        assert_eq!(st, StatusCode::OK, "{joined}");
    }

    let (st, drafting) = harness
        .client
        .post_json_auth(
            &format!("/api/games/{game_id}/start-drafting"),
            json!({}),
            host_token,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{drafting}");

    let mut all = vec![host_token];
    all.extend_from_slice(guest_tokens);
    for token in all {
        let (_, detail) = harness
            .client
            .get_json_auth(&format!("/api/games/{game_id}"), token)
            .await;
        let mut prompts = Vec::new();
        for q in detail["myQuotas"].as_array().unwrap() {
            let target_id = q["targetUser"]["id"].as_str().unwrap();
            let count = q["requiredCount"].as_u64().unwrap();
            for i in 0..count {
                prompts.push(json!({
                    "targetUserId": target_id,
                    "text": format!("Prompt {i}"),
                    "source": "Custom"
                }));
            }
        }
        let (st, _) = harness
            .client
            .put_json_auth(
                &format!("/api/games/{game_id}/prompts"),
                json!({ "prompts": prompts }),
                token,
            )
            .await;
        assert_eq!(st, StatusCode::OK);
        let (st, _) = harness
            .client
            .post_json_auth(
                &format!("/api/games/{game_id}/prompts/lock"),
                json!({}),
                token,
            )
            .await;
        assert_eq!(st, StatusCode::OK);
    }

    (game_id, invite_code)
}

#[test_log::test(tokio::test)]
async fn e2e_live_feed_mode_and_non_participant_access_control() {
    let harness = E2eHarness::new().await;
    let (t_alice, _) = neuer_benutzer(&harness, "lf-alice@example.at").await;
    let (t_bob, _) = neuer_benutzer(&harness, "lf-bob@example.at").await;
    let (t_carol, _) = neuer_benutzer(&harness, "lf-carol@example.at").await;
    let (t_outsider, id_outsider) = neuer_benutzer(&harness, "lf-outsider@example.at").await;

    // Anonymous access refused on protected Friendarazzi routes
    for path in ["/api/games", "/api/decks", "/api/media/nonexistent.jpg"] {
        let (st, _) = harness.client.get_json(path).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "path={path}");
    }
    let (st, _) = harness
        .client
        .get_json(&format!("/api/users/{id_outsider}/gallery"))
        .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    let (game_id, _) = e2e_enroll_and_start_game(
        &harness,
        &t_alice,
        &[&t_bob, &t_carol],
        "Live Hunt",
        3,
        "LiveFeed",
        true,
    )
    .await;

    let (_, active) = harness
        .client
        .get_json_auth(&format!("/api/games/{game_id}"), &t_alice)
        .await;
    let tile_00 = active["myBoard"]["tiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["row"] == 0 && t["col"] == 0)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let (st, cap) = harness
        .client
        .post_multipart_file_auth(
            &format!("/api/games/{game_id}/tiles/{tile_00}/capture"),
            "file",
            "live.png",
            "image/png",
            png_bytes_for_test(180, 120),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{cap}");
    let full_key = cap["capture"]["imageStorageKey"]
        .as_str()
        .unwrap()
        .to_owned();
    let thumb_key = cap["capture"]["thumbnailStorageKey"]
        .as_str()
        .unwrap()
        .to_owned();

    // LiveFeed: Bob immediately sees full unredacted feed item & fetches full image
    let (st, bob_view) = harness
        .client
        .get_json_auth(&format!("/api/games/{game_id}"), &t_bob)
        .await;
    assert_eq!(st, StatusCode::OK);
    let feed = bob_view["feed"].as_array().unwrap();
    assert_eq!(feed.len(), 1);
    assert_eq!(feed[0]["isBlurred"], false);
    assert!(!feed[0]["promptText"].is_null());
    assert!(!feed[0]["target"].is_null());
    assert_eq!(feed[0]["imageStorageKey"], full_key);

    let (media_st, bytes, ct) = harness
        .client
        .get_bytes_auth(&format!("/api/media/{full_key}"), &t_bob)
        .await;
    assert_eq!(media_st, StatusCode::OK);
    assert_eq!(ct, "image/jpeg");
    assert!(!bytes.is_empty());

    // Outsider cannot read game detail or either capture media key
    let (st, _) = harness
        .client
        .get_json_auth(&format!("/api/games/{game_id}"), &t_outsider)
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    for key in [&full_key, &thumb_key] {
        let (st, _, _) = harness
            .client
            .get_bytes_auth(&format!("/api/media/{key}"), &t_outsider)
            .await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }

    // Outsider's GET /api/games list does not leak games they did not join
    let (st, outsider_games) = harness
        .client
        .get_json_auth("/api/games", &t_outsider)
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(outsider_games.as_array().unwrap().len(), 0);

    let (st, bob_games) = harness.client.get_json_auth("/api/games", &t_bob).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(bob_games.as_array().unwrap().len(), 1);
}

#[test_log::test(tokio::test)]
async fn e2e_lobby_settings_patch_error_mappings_and_late_join_conflict() {
    let harness = E2eHarness::new().await;
    let (t_alice, _) = neuer_benutzer(&harness, "ls-alice@example.at").await;
    let (t_bob, _) = neuer_benutzer(&harness, "ls-bob@example.at").await;
    let (t_carol, _) = neuer_benutzer(&harness, "ls-carol@example.at").await;
    let (t_late, _) = neuer_benutzer(&harness, "ls-late@example.at").await;

    let (st, created) = harness
        .client
        .post_json_auth(
            "/api/games",
            json!({
                "title": "Starttitel",
                "gridSize": 3,
                "visibilityMode": "Undercover",
                "includeFreeSpace": true
            }),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    let game_id = created["id"].as_str().unwrap().to_owned();
    let invite_code = created["inviteCode"].as_str().unwrap().to_owned();

    // Start drafting with only 1 player -> BadRequest (TooFewPlayers)
    let (st, _) = harness
        .client
        .post_json_auth(
            &format!("/api/games/{game_id}/start-drafting"),
            json!({}),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // Join bob & carol
    for token in [&t_bob, &t_carol] {
        let (st, _) = harness
            .client
            .post_json_auth(
                "/api/games/join",
                json!({ "inviteCode": invite_code }),
                token,
            )
            .await;
        assert_eq!(st, StatusCode::OK);
    }

    // Non-host PUT settings -> Forbidden
    let (st, _) = harness
        .client
        .put_json_auth(
            &format!("/api/games/{game_id}/settings"),
            json!({
                "title": "Fremd",
                "gridSize": 4,
                "visibilityMode": "LiveFeed",
                "includeFreeSpace": false
            }),
            &t_bob,
        )
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Host PUT settings -> OK with updated values
    let (st, patched) = harness
        .client
        .put_json_auth(
            &format!("/api/games/{game_id}/settings"),
            json!({
                "title": "Aktualisiert 4x4",
                "gridSize": 4,
                "visibilityMode": "LiveFeed",
                "includeFreeSpace": false
            }),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{patched}");
    assert_eq!(patched["title"], "Aktualisiert 4x4");
    assert_eq!(patched["gridSize"], 4);
    assert_eq!(patched["visibilityMode"], "LiveFeed");
    assert_eq!(patched["includeFreeSpace"], false);

    // Host starts drafting
    let (st, _) = harness
        .client
        .post_json_auth(
            &format!("/api/games/{game_id}/start-drafting"),
            json!({}),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::OK);

    // Late join after Drafting starts -> Conflict
    let (st, _) = harness
        .client
        .post_json_auth(
            "/api/games/join",
            json!({ "inviteCode": invite_code }),
            &t_late,
        )
        .await;
    assert_eq!(st, StatusCode::CONFLICT);

    // Locking before meeting prompt quota -> BadRequest
    let (st, _) = harness
        .client
        .post_json_auth(
            &format!("/api/games/{game_id}/prompts/lock"),
            json!({}),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}

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
    assert_eq!(ct, "image/jpeg");
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
    let (game_id, _) = e2e_enroll_and_start_game(
        &harness,
        &t_alice,
        &[&t_bob, &t_carol],
        "Gallery Check",
        3,
        "LiveFeed",
        true,
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
