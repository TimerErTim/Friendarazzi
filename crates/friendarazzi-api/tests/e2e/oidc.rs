use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use base64::{
    Engine as _, engine::general_purpose::STANDARD, engine::general_purpose::URL_SAFE_NO_PAD,
};
use friendarazzi_core::domain::konto::OidcProviderId;
use friendarazzi_oidc::{
    AppleOidcConfig, GenericOidcConfig, GoogleOidcConfig, HttpOidcProviderClient,
    OidcProvidersConfig,
};
use poem::{
    EndpointExt, Route, Server, handler,
    http::StatusCode,
    listener::TcpAcceptor,
    web::{Data, Form, Json},
};
use ring::{
    rand::SystemRandom,
    signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair},
};
use serde_json::{Value, json};

use crate::{E2eHarness, PASSWORT};

#[derive(Clone)]
struct StubIdTokenClaim {
    sub: String,
    email: String,
    email_verified: bool,
    nonce: String,
    name: Option<String>,
    aud: String,
}

#[derive(Clone)]
struct StubOidcState {
    base_url: String,
    key_pair: Arc<EcdsaKeyPair>,
    kid: String,
    issued_by_code: Arc<Mutex<HashMap<String, StubIdTokenClaim>>>,
}

#[handler]
async fn discovery(state: Data<&StubOidcState>) -> Json<Value> {
    Json(json!({
        "issuer": state.base_url,
        "authorization_endpoint": format!("{}/authorize", state.base_url),
        "token_endpoint": format!("{}/token", state.base_url),
        "jwks_uri": format!("{}/jwks.json", state.base_url),
    }))
}

#[handler]
async fn jwks(state: Data<&StubOidcState>) -> Json<Value> {
    let pub_bytes = state.key_pair.public_key().as_ref();
    Json(json!({
        "keys": [{
            "kty": "EC",
            "kid": state.kid,
            "alg": "ES256",
            "crv": "P-256",
            "x": URL_SAFE_NO_PAD.encode(&pub_bytes[1..33]),
            "y": URL_SAFE_NO_PAD.encode(&pub_bytes[33..65]),
        }]
    }))
}

#[handler]
async fn token(
    state: Data<&StubOidcState>,
    Form(form): Form<HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let Some(code) = form.get("code") else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "missing_code" })),
        );
    };
    let Some(claim) = state.issued_by_code.lock().unwrap().remove(code) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid_grant" })),
        );
    };

    let header = json!({
        "alg": "ES256",
        "kid": state.kid,
        "typ": "JWT",
    });
    let payload = json!({
        "iss": state.base_url,
        "sub": claim.sub,
        "aud": claim.aud,
        "exp": 1_700_000_600i64,
        "iat": 1_700_000_000i64,
        "nonce": claim.nonce,
        "email": claim.email,
        "email_verified": claim.email_verified,
        "name": claim.name,
    });
    let h_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap());
    let p_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap());
    let signing_input = format!("{h_b64}.{p_b64}");
    let sig = state
        .key_pair
        .sign(&SystemRandom::new(), signing_input.as_bytes())
        .unwrap();
    let id_token = format!("{signing_input}.{}", URL_SAFE_NO_PAD.encode(sig.as_ref()));

    (
        StatusCode::OK,
        Json(json!({
            "access_token": "stub-access",
            "token_type": "Bearer",
            "id_token": id_token,
        })),
    )
}

async fn start_stub_idp() -> (StubOidcState, tokio::task::JoinHandle<()>) {
    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = std_listener.local_addr().unwrap();
    std_listener.set_nonblocking(true).unwrap();
    let acceptor = TcpAcceptor::from_std(std_listener).unwrap();

    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
    let key_pair =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng).unwrap();

    let state = StubOidcState {
        base_url: format!("http://{addr}"),
        key_pair: Arc::new(key_pair),
        kid: "stub-es256-1".to_owned(),
        issued_by_code: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Route::new()
        .at("/.well-known/openid-configuration", poem::get(discovery))
        .at("/jwks.json", poem::get(jwks))
        .at("/token", poem::post(token))
        .data(state.clone());

    let handle = tokio::spawn(async move {
        let _ = Server::new_with_acceptor(acceptor).run(app).await;
    });

    (state, handle)
}

fn apple_dummy_pem() -> String {
    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
    let body = STANDARD.encode(pkcs8.as_ref());
    format!("-----BEGIN PRIVATE KEY-----\n{body}\n-----END PRIVATE KEY-----\n")
}

fn query_param(url: &str, name: &str) -> Option<String> {
    let (_, query) = url.split_once('?')?;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=')?;
        if k == name {
            return Some(v.to_owned());
        }
    }
    None
}

fn harness_oidc_client(issuer: &str) -> Arc<HttpOidcProviderClient> {
    Arc::new(HttpOidcProviderClient::new(OidcProvidersConfig {
        callback_base_url: "https://api.friendarazzi.app/api".to_owned(),
        google: Some(GoogleOidcConfig {
            client_id: "google-client-1".to_owned(),
            client_secret: "google-secret-1".to_owned(),
            issuer_url: issuer.to_owned(),
        }),
        apple: Some(AppleOidcConfig {
            client_id: "apple-client-1".to_owned(),
            team_id: "TEAMID".to_owned(),
            key_id: "KEYID".to_owned(),
            private_key_pem: apple_dummy_pem(),
            issuer_url: issuer.to_owned(),
        }),
        generic: vec![GenericOidcConfig {
            id: OidcProviderId::parse("authentik").unwrap(),
            display_name: "Authentik".to_owned(),
            issuer_url: issuer.to_owned(),
            client_id: "generic-client-1".to_owned(),
            client_secret: "generic-secret-1".to_owned(),
            scopes: None,
        }],
    }))
}

#[test_log::test(tokio::test)]
async fn oidc_end_to_end_flows_with_real_http_client_and_stub_idp() {
    let (idp, server_handle) = start_stub_idp().await;
    let oidc_client = harness_oidc_client(&idp.base_url);
    let h = E2eHarness::with_oidc(oidc_client).await;

    // 1. Discovery of configured providers.
    let (status, providers_body) = h.client.get_json("/api/auth/providers").await;
    assert_eq!(status, StatusCode::OK);
    let providers = providers_body["providers"].as_array().unwrap();
    assert_eq!(providers.len(), 3);

    // 2. Google GET callback -> 302 with single-use code (no tokens in URL) -> session + verified email.
    let start_resp = h
        .client
        .client()
        .get("/api/auth/oidc/google/start?next=%2Fgames")
        .send()
        .await;
    assert_eq!(start_resp.0.status(), StatusCode::FOUND);
    let auth_url = start_resp
        .0
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let state = query_param(&auth_url, "state").unwrap();
    let nonce = query_param(&auth_url, "nonce").unwrap();

    idp.issued_by_code.lock().unwrap().insert(
        "g-code-1".to_owned(),
        StubIdTokenClaim {
            sub: "google-sub-1".to_owned(),
            email: "lena.oidc@example.at".to_owned(),
            email_verified: true,
            nonce,
            name: Some("Lena OIDC".to_owned()),
            aud: "google-client-1".to_owned(),
        },
    );

    let cb_resp = h
        .client
        .client()
        .get(format!(
            "/api/auth/oidc/google/callback?code=g-code-1&state={state}"
        ))
        .send()
        .await;
    assert_eq!(cb_resp.0.status(), StatusCode::FOUND);
    let redirect = cb_resp
        .0
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert!(!redirect.contains("accessToken"));
    assert!(!redirect.contains("refreshToken"));
    let exchange_code = query_param(&redirect, "code").unwrap();

    let (st, session_body, cookie) = h
        .client
        .post_json_cookie(
            "/api/auth/oidc/exchange",
            json!({ "code": exchange_code }),
            None,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{session_body}");
    assert!(cookie.is_some());
    let lena_token = session_body["accessToken"].as_str().unwrap().to_owned();

    let (st, me) = h.client.get_json_auth("/api/benutzer", &lena_token).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(me["emailVerified"], true);
    assert_eq!(me["displayName"], "Lena OIDC");

    // 3. No silent email linking: existing password account conflicts gracefully cleanly cleanly.
    let (reg_st, _, _) = h
        .client
        .post_json_cookie(
            "/api/registrierung",
            json!({ "eMailAdresse": "existing@example.at", "passwort": PASSWORT }),
            None,
        )
        .await;
    assert_eq!(reg_st, StatusCode::OK);
    let conflict_start = h
        .client
        .client()
        .get("/api/auth/oidc/google/start")
        .send()
        .await;
    let conflict_loc = conflict_start
        .0
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let conflict_state = query_param(&conflict_loc, "state").unwrap();
    let conflict_nonce = query_param(&conflict_loc, "nonce").unwrap();
    idp.issued_by_code.lock().unwrap().insert(
        "g-conflict".to_owned(),
        StubIdTokenClaim {
            sub: "google-sub-conflict".to_owned(),
            email: "existing@example.at".to_owned(),
            email_verified: true,
            nonce: conflict_nonce,
            name: None,
            aud: "google-client-1".to_owned(),
        },
    );
    let conflict_cb = h
        .client
        .client()
        .get(format!(
            "/api/auth/oidc/google/callback?code=g-conflict&state={conflict_state}"
        ))
        .send()
        .await;
    assert_eq!(conflict_cb.0.status(), StatusCode::FOUND);
    assert_eq!(
        query_param(
            conflict_cb
                .0
                .headers()
                .get("Location")
                .unwrap()
                .to_str()
                .unwrap(),
            "error"
        )
        .as_deref(),
        Some("email_conflict")
    );

    // 4. Apple form_post callback with private-relay email + user display name JSON.
    let apple_start = h
        .client
        .client()
        .get("/api/auth/oidc/apple/start")
        .send()
        .await;
    let apple_loc = apple_start
        .0
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert!(apple_loc.contains("response_mode=form_post"));
    let apple_state = query_param(&apple_loc, "state").unwrap();
    let apple_nonce = query_param(&apple_loc, "nonce").unwrap();
    idp.issued_by_code.lock().unwrap().insert(
        "apple-code-1".to_owned(),
        StubIdTokenClaim {
            sub: "apple-sub-1".to_owned(),
            email: "xyz99@privaterelay.appleid.com".to_owned(),
            email_verified: true,
            nonce: apple_nonce,
            name: None,
            aud: "apple-client-1".to_owned(),
        },
    );
    let encoded_user =
        r#"%7B%22name%22%3A%7B%22firstName%22%3A%22Max%22%2C%22lastName%22%3A%22Muster%22%7D%7D"#;
    let apple_user = format!("code=apple-code-1&state={apple_state}&user={encoded_user}");
    let apple_cb = h
        .client
        .client()
        .post("/api/auth/oidc/apple/callback")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(apple_user)
        .send()
        .await;
    assert_eq!(apple_cb.0.status(), StatusCode::FOUND);
    let apple_code = query_param(
        apple_cb
            .0
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
        "code",
    )
    .unwrap();
    let (st, apple_sess, _) = h
        .client
        .post_json_cookie(
            "/api/auth/oidc/exchange",
            json!({ "code": apple_code }),
            None,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    let apple_token = apple_sess["accessToken"].as_str().unwrap().to_owned();
    let (_, apple_me) = h.client.get_json_auth("/api/benutzer", &apple_token).await;
    assert_eq!(apple_me["displayName"], "Max Muster");

    // 5. Authenticated login methods, last-method unlink guard, set initial password, and unlink.
    let (st, methods) = h
        .client
        .get_json_auth("/api/konto/anmeldemethoden", &lena_token)
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(methods["hasPassword"], false);
    let google_identity_id = methods["externalIdentities"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Unlink fails with 409 when only login method.
    let del_last = h
        .client
        .client()
        .delete(format!("/api/konto/anmeldemethoden/{google_identity_id}"))
        .header("Authorization", format!("Bearer {lena_token}"))
        .send()
        .await;
    assert_eq!(del_last.0.status(), StatusCode::CONFLICT);

    // Link Authentik provider while signed in as Lena.
    let (st, link_start) = h
        .client
        .post_json_auth(
            "/api/konto/anmeldemethoden/authentik/start",
            json!({ "mode": "link" }),
            &lena_token,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{link_start}");
    let link_url = link_start["authorizationUrl"].as_str().unwrap();
    let link_state = query_param(link_url, "state").unwrap();
    let link_nonce = query_param(link_url, "nonce").unwrap();
    idp.issued_by_code.lock().unwrap().insert(
        "link-code-1".to_owned(),
        StubIdTokenClaim {
            sub: "authentik-sub-lena".to_owned(),
            email: "lena.alt@example.at".to_owned(),
            email_verified: true,
            nonce: link_nonce,
            name: None,
            aud: "generic-client-1".to_owned(),
        },
    );
    let link_cb = h
        .client
        .client()
        .get(format!(
            "/api/auth/oidc/authentik/callback?code=link-code-1&state={link_state}"
        ))
        .send()
        .await;
    assert_eq!(link_cb.0.status(), StatusCode::FOUND);
    assert_eq!(
        query_param(
            link_cb
                .0
                .headers()
                .get("Location")
                .unwrap()
                .to_str()
                .unwrap(),
            "oidc_linked"
        )
        .as_deref(),
        Some("authentik")
    );

    // Set initial password and verify password login works.
    let (st, _) = h
        .client
        .post_json_auth(
            "/api/konto/passwort",
            json!({ "passwort": PASSWORT }),
            &lena_token,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    let (st, pw_login, _) = h
        .client
        .post_json_cookie(
            "/api/anmeldung",
            json!({ "eMailAdresse": "lena.oidc@example.at", "passwort": PASSWORT }),
            None,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{pw_login}");

    // Now unlinking Google succeeds.
    let del_ok = h
        .client
        .client()
        .delete(format!("/api/konto/anmeldemethoden/{google_identity_id}"))
        .header("Authorization", format!("Bearer {lena_token}"))
        .send()
        .await;
    assert_eq!(del_ok.0.status(), StatusCode::OK);

    // 6. Passwordless Apple user deletes account via deleteReauth OIDC ceremony.
    let (st, reauth_start) = h
        .client
        .post_json_auth(
            "/api/konto/anmeldemethoden/apple/start",
            json!({ "mode": "deleteReauth" }),
            &apple_token,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    let reauth_url = reauth_start["authorizationUrl"].as_str().unwrap();
    let reauth_state = query_param(reauth_url, "state").unwrap();
    let reauth_nonce = query_param(reauth_url, "nonce").unwrap();
    idp.issued_by_code.lock().unwrap().insert(
        "apple-reauth".to_owned(),
        StubIdTokenClaim {
            sub: "apple-sub-1".to_owned(),
            email: "xyz99@privaterelay.appleid.com".to_owned(),
            email_verified: true,
            nonce: reauth_nonce,
            name: None,
            aud: "apple-client-1".to_owned(),
        },
    );
    let reauth_cb = h
        .client
        .client()
        .post("/api/auth/oidc/apple/callback")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("code=apple-reauth&state={reauth_state}"))
        .send()
        .await;
    assert_eq!(reauth_cb.0.status(), StatusCode::FOUND);
    let reauth_code = query_param(
        reauth_cb
            .0
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
        "delete_reauth_code",
    )
    .unwrap();

    let (st, _) = h
        .client
        .post_json_auth(
            "/api/benutzer/loeschung",
            json!({ "reauthCode": reauth_code, "bestaetigt": true }),
            &apple_token,
        )
        .await;
    assert_eq!(st, StatusCode::OK);

    server_handle.abort();
}
