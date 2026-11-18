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
