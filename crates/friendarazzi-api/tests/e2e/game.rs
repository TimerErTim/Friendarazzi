use chrono::Duration;
use poem::http::StatusCode;
use serde_json::json;

use crate::{E2eHarness, neuer_benutzer};

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
                "visibilityMode": "Undercover"
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
                "visibilityMode": "LiveFeed"
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
                "visibilityMode": "LiveFeed"
            }),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{patched}");
    assert_eq!(patched["title"], "Aktualisiert 4x4");
    assert_eq!(patched["gridSize"], 4);
    assert_eq!(patched["visibilityMode"], "LiveFeed");

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
async fn e2e_user_can_host_and_join_unlimited_parallel_games() {
    let harness = E2eHarness::new().await;
    let (t_alice, _) = neuer_benutzer(&harness, "multi-alice@example.at").await;
    let (t_bob, _) = neuer_benutzer(&harness, "multi-bob@example.at").await;
    let (t_carol, _) = neuer_benutzer(&harness, "multi-carol@example.at").await;

    // Alice hosts 2 parallel games and Bob/Carol each host 1 game that Alice joins.
    let mut alice_game_ids = Vec::new();
    for title in ["Alice Host 1", "Alice Host 2"] {
        let (st, created) = harness
            .client
            .post_json_auth(
                "/api/games",
                json!({
                    "title": title,
                    "gridSize": 3,
                    "visibilityMode": "Undercover"
                }),
                &t_alice,
            )
            .await;
        assert_eq!(st, StatusCode::OK, "{created}");
        alice_game_ids.push(created["id"].as_str().unwrap().to_owned());
    }

    for (host_token, title) in [(&t_bob, "Bob Host"), (&t_carol, "Carol Host")] {
        let (st, created) = harness
            .client
            .post_json_auth(
                "/api/games",
                json!({
                    "title": title,
                    "gridSize": 3,
                    "visibilityMode": "LiveFeed"
                }),
                host_token,
            )
            .await;
        assert_eq!(st, StatusCode::OK, "{created}");
        let code = created["inviteCode"].as_str().unwrap().to_owned();
        let gid = created["id"].as_str().unwrap().to_owned();

        let (st, joined) = harness
            .client
            .post_json_auth("/api/games/join", json!({ "inviteCode": code }), &t_alice)
            .await;
        assert_eq!(st, StatusCode::OK, "{joined}");
        assert_eq!(joined["id"].as_str().unwrap(), gid);
        alice_game_ids.push(gid);
    }

    let (st, list) = harness.client.get_json_auth("/api/games", &t_alice).await;
    assert_eq!(st, StatusCode::OK, "{list}");
    let items = list.as_array().unwrap();
    assert_eq!(items.len(), 4);
    for expected_id in &alice_game_ids {
        assert!(
            items
                .iter()
                .any(|g| g["id"].as_str() == Some(expected_id.as_str())),
            "missing parallel game {expected_id} in {list}"
        );
    }
}

#[test_log::test(tokio::test)]
async fn e2e_reversible_ready_grace_period_and_summary_progress() {
    let harness = E2eHarness::new().await;
    let (t_alice, _) = neuer_benutzer(&harness, "grace-alice@example.at").await;
    let (t_bob, _) = neuer_benutzer(&harness, "grace-bob@example.at").await;
    let (t_carol, _) = neuer_benutzer(&harness, "grace-carol@example.at").await;

    let (st, created) = harness
        .client
        .post_json_auth(
            "/api/games",
            json!({
                "title": "Grace Runde",
                "gridSize": 3,
                "visibilityMode": "Undercover"
            }),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    let game_id = created["id"].as_str().unwrap().to_owned();
    let invite_code = created["inviteCode"].as_str().unwrap().to_owned();

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

    let (st, _) = harness
        .client
        .post_json_auth(
            &format!("/api/games/{game_id}/start-drafting"),
            json!({}),
            &t_alice,
        )
        .await;
    assert_eq!(st, StatusCode::OK);

    // Fill prompts for all three players and lock Alice + Bob first.
    for token in [&t_alice, &t_bob, &t_carol] {
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
                    "text": format!("Grace Prompt {i}"),
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
    }

    for token in [&t_alice, &t_bob] {
        let (st, locked) = harness
            .client
            .post_json_auth(
                &format!("/api/games/{game_id}/prompts/lock"),
                json!({}),
                token,
            )
            .await;
        assert_eq!(st, StatusCode::OK);
        assert!(locked["autoStartAt"].is_null());
    }

    // Carol still needs action on /api/games summary before locking.
    let (st, carol_list) = harness.client.get_json_auth("/api/games", &t_carol).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(carol_list[0]["needsAction"], true);
    assert_eq!(carol_list[0]["myDraftFilled"], 9);
    assert_eq!(carol_list[0]["myDraftLocked"], false);
    assert_eq!(carol_list[0]["readyPlayerCount"], 2);

    // Carol locks -> shared 60s countdown begins; game stays Drafting.
    let (st, countdown) = harness
        .client
        .post_json_auth(
            &format!("/api/games/{game_id}/prompts/lock"),
            json!({}),
            &t_carol,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(countdown["status"], "Drafting");
    assert!(countdown["autoStartAt"].is_string());

    // After 59s, still Drafting.
    harness.vorspulen(Duration::seconds(59));
    let (st, almost) = harness
        .client
        .get_json_auth(&format!("/api/games/{game_id}"), &t_alice)
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(almost["status"], "Drafting");

    // Carol withdraws readiness during countdown -> cancels pending start, keeps prompts.
    let (st, unlocked) = harness
        .client
        .post_json_auth(
            &format!("/api/games/{game_id}/prompts/unlock"),
            json!({}),
            &t_carol,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{unlocked}");
    assert_eq!(unlocked["status"], "Drafting");
    assert!(unlocked["autoStartAt"].is_null());
    assert_eq!(unlocked["myDrafts"].as_array().unwrap().len(), 9);

    // Carol re-locks -> starts fresh 60s countdown.
    let (st, relocked) = harness
        .client
        .post_json_auth(
            &format!("/api/games/{game_id}/prompts/lock"),
            json!({}),
            &t_carol,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    assert!(relocked["autoStartAt"].is_string());

    harness.vorspulen(Duration::seconds(59));
    let (st, still_drafting) = harness
        .client
        .get_json_auth(&format!("/api/games/{game_id}"), &t_bob)
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(still_drafting["status"], "Drafting");

    // Full minute expires -> transitions to Active and unlock is now Conflict.
    harness.vorspulen(Duration::seconds(1));
    let (st, active) = harness
        .client
        .get_json_auth(&format!("/api/games/{game_id}"), &t_bob)
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(active["status"], "Active");
    assert_eq!(active["myBoard"]["tiles"].as_array().unwrap().len(), 9);

    let (st, _) = harness
        .client
        .post_json_auth(
            &format!("/api/games/{game_id}/prompts/unlock"),
            json!({}),
            &t_carol,
        )
        .await;
    assert_eq!(st, StatusCode::CONFLICT);
}
