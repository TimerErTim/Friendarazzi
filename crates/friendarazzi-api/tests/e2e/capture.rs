use chrono::Duration;
use friendarazzi_core::domain::media::png_bytes_for_test;
use poem::http::StatusCode;
use serde_json::json;

use crate::{E2eHarness, enroll_and_start_game, neuer_benutzer};

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
                "visibilityMode": "Undercover"
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
    let tile_at = |r: i64, c: i64| {
        tiles
            .iter()
            .find(|t| t["row"] == r && t["col"] == c)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let tile_00 = tile_at(0, 0);
    let tile_11 = tile_at(1, 1);
    let tile_22 = tile_at(2, 2);

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
    assert!(cap1["winningLine"].is_null());
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

    // Advance time & Alice captures (1,1) and (2,2) to complete diagonal Bingo
    harness.vorspulen(Duration::minutes(6));
    let (status, cap2) = harness
        .client
        .post_multipart_file_auth(
            &format!("/api/games/{game_id}/tiles/{tile_11}/capture"),
            "file",
            "mid.png",
            "image/png",
            png.clone(),
            &t_alice,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{cap2}");
    assert!(cap2["winningLine"].is_null());

    harness.vorspulen(Duration::minutes(6));
    let (status, cap3) = harness
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
    assert_eq!(status, StatusCode::OK, "{cap3}");
    assert_eq!(cap3["winningLine"].as_array().unwrap().len(), 3);

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
    assert_eq!(finished["feed"].as_array().unwrap().len(), 3);
    assert!(!finished["superlatives"].as_array().unwrap().is_empty());

    let (media_status, bytes, ct) = harness
        .client
        .get_bytes_auth(&format!("/api/media/{cap1_key}"), &t_bob)
        .await;
    assert_eq!(media_status, StatusCode::OK);
    assert_eq!(ct, "image/webp");
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
        3
    );
}

#[test_log::test(tokio::test)]
async fn e2e_blurred_teaser_redacts_target_and_serves_thumbnail() {
    let harness = E2eHarness::new().await;
    let (t_alice, _) = neuer_benutzer(&harness, "a2@example.at").await;
    let (t_bob, _) = neuer_benutzer(&harness, "b2@example.at").await;
    let (t_carol, _) = neuer_benutzer(&harness, "c2@example.at").await;

    let (game_id, _) = enroll_and_start_game(
        &harness,
        &t_alice,
        &[&t_bob, &t_carol],
        "Blurred Hunt",
        3,
        "BlurredTeaser",
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

    // Bob can fetch the blurred thumbnail, but NOT the full photo
    let (thumb_status, _, thumb_ct) = harness
        .client
        .get_bytes_auth(&format!("/api/media/{thumb_key}"), &t_bob)
        .await;
    assert_eq!(thumb_status, StatusCode::OK);
    assert_eq!(thumb_ct, "image/webp");

    let (full_status, _, _) = harness
        .client
        .get_bytes_auth(&format!("/api/media/{full_key}"), &t_bob)
        .await;
    assert_eq!(full_status, StatusCode::FORBIDDEN);
}

#[test_log::test(tokio::test)]
async fn e2e_live_feed_mode_and_non_participant_access_control() {
    let harness = E2eHarness::new().await;
    let (t_alice, _) = neuer_benutzer(&harness, "lf-alice@example.at").await;
    let (t_bob, _) = neuer_benutzer(&harness, "lf-bob@example.at").await;
    let (t_carol, _) = neuer_benutzer(&harness, "lf-carol@example.at").await;
    let (t_outsider, id_outsider) = neuer_benutzer(&harness, "lf-outsider@example.at").await;

    // Anonymous access refused on protected Friendarazzi routes
    for path in ["/api/games", "/api/decks", "/api/media/nonexistent.webp"] {
        let (st, _) = harness.client.get_json(path).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "path={path}");
    }
    let (st, _) = harness
        .client
        .get_json(&format!("/api/users/{id_outsider}/gallery"))
        .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    let (game_id, _) = enroll_and_start_game(
        &harness,
        &t_alice,
        &[&t_bob, &t_carol],
        "Live Hunt",
        3,
        "LiveFeed",
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
    assert_eq!(ct, "image/webp");
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
