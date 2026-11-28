use friendarazzi_core::{
    application::AuthContext,
    domain::{
        EmailAdresse, NonEmptyString,
        konto::{
            KlartextPasswort, OidcFlowMode, OidcProviderDescriptor, OidcProviderId,
            OidcProviderKind, SecretToken, VerifiedOidcIdentity,
        },
    },
    ports::{ClientKey, OidcClientError, RepositoryError},
    service::{
        Anmeldung, BenutzerLöschen, OidcCompleteCallback, OidcExchangeCodeForSession,
        OidcStartAuth, OidcUseCaseError, PasswortErstmaligSetzen, PasswortErstmaligSetzenFehler,
        Registrieren, UnlinkExternalIdentity,
    },
};

use crate::base_app_builder;

use super::helpers::{FRONTEND_BASIS_URL, Harness, TEST_PASSWORT, build};

fn client_key(tag: &str) -> ClientKey {
    ClientKey::new("127.0.0.1".parse().unwrap(), tag.to_owned())
}

async fn harness_with_providers() -> Harness {
    let h = build(base_app_builder().await);
    h.oidc().set_providers(vec![
        OidcProviderDescriptor {
            id: OidcProviderId::parse("google").unwrap(),
            kind: OidcProviderKind::Google,
            display_name: "Google".to_owned(),
        },
        OidcProviderDescriptor {
            id: OidcProviderId::parse("apple").unwrap(),
            kind: OidcProviderKind::Apple,
            display_name: "Apple".to_owned(),
        },
    ]);
    h
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

#[test_log::test(pollster::test)]
async fn oidc_first_login_creates_verified_passwordless_user_and_exchanges_code() {
    let h = harness_with_providers().await;
    let google = OidcProviderId::parse("google").unwrap();

    let auth_url = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcStartAuth {
                provider_id: google.clone(),
                mode: OidcFlowMode::Login,
                return_to: Some("/games".to_owned()),
                client_key: client_key("oidc-1"),
            },
        )
        .await
        .unwrap();
    let state = query_param(&auth_url, "state").unwrap();

    h.oidc().stub_exchange(
        "auth-code-1",
        Ok(VerifiedOidcIdentity {
            provider_id: google.clone(),
            subject: "sub-google-1".to_owned(),
            email: EmailAdresse::normalisiert("mia@example.at").unwrap(),
            display_name: Some("Mia".to_owned()),
        }),
    );

    let callback = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcCompleteCallback {
                provider_id: google,
                code: Some("auth-code-1".to_owned()),
                state: Some(state),
                provider_error: None,
                display_name_hint: None,
            },
        )
        .await
        .unwrap();
    assert!(
        callback
            .redirect_url
            .starts_with(&format!("{FRONTEND_BASIS_URL}/anmeldung/oidc?"))
    );
    let exchange_code = query_param(&callback.redirect_url, "code").unwrap();
    assert_eq!(
        query_param(&callback.redirect_url, "next").as_deref(),
        Some("%2Fgames")
    );

    let session = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcExchangeCodeForSession {
                code: SecretToken::try_from(exchange_code.as_str()).unwrap(),
                client_key: client_key("oidc-1"),
            },
        )
        .await
        .unwrap();
    assert!(session.benutzer.ist_verifiziert());
    assert_eq!(session.benutzer.display_name().as_str(), "Mia");

    // Replaying the browser exchange code fails.
    let replay = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcExchangeCodeForSession {
                code: SecretToken::try_from(exchange_code.as_str()).unwrap(),
                client_key: client_key("oidc-1"),
            },
        )
        .await;
    assert!(matches!(
        replay.unwrap_err().current_context(),
        OidcUseCaseError::CodeUngültig
    ));

    // No password Konto row exists for a pure OIDC user.
    let konten_count = h
        .app()
        .execute_fn::<_, _, RepositoryError>(AuthContext::Anonymous, async move |ctx| {
            let uow = ctx.enter().await?;
            let count = uow
                .konten()
                .find_by_benutzer(session.benutzer.id())
                .await?
                .len();
            uow.rollback().await?;
            Ok(count)
        })
        .await
        .unwrap();
    assert_eq!(konten_count, 0);
}

#[test_log::test(pollster::test)]
async fn oidc_login_never_silently_links_existing_email() {
    let h = harness_with_providers().await;
    let ck = client_key("conflict-1");

    h.app()
        .execute(
            AuthContext::Anonymous,
            Registrieren {
                email: EmailAdresse::normalisiert("lena@example.at").unwrap(),
                passwort: KlartextPasswort::neu(TEST_PASSWORT).unwrap(),
                display_name: Some(NonEmptyString::neu("Lena").unwrap()),
                client_key: ck.clone(),
            },
        )
        .await
        .unwrap();

    let google = OidcProviderId::parse("google").unwrap();
    let auth_url = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcStartAuth {
                provider_id: google.clone(),
                mode: OidcFlowMode::Login,
                return_to: None,
                client_key: ck,
            },
        )
        .await
        .unwrap();
    let state = query_param(&auth_url, "state").unwrap();

    h.oidc().stub_exchange(
        "auth-code-conflict",
        Ok(VerifiedOidcIdentity {
            provider_id: google.clone(),
            subject: "sub-lena".to_owned(),
            email: EmailAdresse::normalisiert("lena@example.at").unwrap(),
            display_name: Some("Lena OIDC".to_owned()),
        }),
    );

    let callback = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcCompleteCallback {
                provider_id: google,
                code: Some("auth-code-conflict".to_owned()),
                state: Some(state),
                provider_error: None,
                display_name_hint: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        query_param(&callback.redirect_url, "error").as_deref(),
        Some("email_conflict")
    );
}

#[test_log::test(pollster::test)]
async fn unlink_last_method_refused_until_password_set_and_passwordless_delete_reauth_works() {
    let h = harness_with_providers().await;
    let apple = OidcProviderId::parse("apple").unwrap();
    let ck = client_key("apple-1");

    // 1. Create passwordless Apple account (including Apple private-relay email).
    let auth_url = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcStartAuth {
                provider_id: apple.clone(),
                mode: OidcFlowMode::Login,
                return_to: None,
                client_key: ck.clone(),
            },
        )
        .await
        .unwrap();
    let state = query_param(&auth_url, "state").unwrap();
    h.oidc().stub_exchange(
        "apple-code-1",
        Ok(VerifiedOidcIdentity {
            provider_id: apple.clone(),
            subject: "sub-apple-1".to_owned(),
            email: EmailAdresse::normalisiert("relay123@privaterelay.appleid.com").unwrap(),
            display_name: None,
        }),
    );
    let cb = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcCompleteCallback {
                provider_id: apple.clone(),
                code: Some("apple-code-1".to_owned()),
                state: Some(state),
                provider_error: None,
                display_name_hint: Some("Max".to_owned()),
            },
        )
        .await
        .unwrap();
    let code = query_param(&cb.redirect_url, "code").unwrap();
    let session = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcExchangeCodeForSession {
                code: SecretToken::try_from(code.as_str()).unwrap(),
                client_key: ck.clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(session.benutzer.display_name().as_str(), "Max");

    let caller = AuthContext::Authenticated {
        benutzer_id: session.benutzer.id(),
        session_id: session.session_id,
    };

    let benutzer_id = session.benutzer.id();
    let identities = h
        .app()
        .execute_fn::<_, _, RepositoryError>(caller, async move |ctx| {
            let uow = ctx.enter().await?;
            let list = uow
                .external_identities()
                .find_by_benutzer(benutzer_id)
                .await?;
            uow.rollback().await?;
            Ok(list)
        })
        .await
        .unwrap();
    let identity_id = identities[0].id();

    // 2. Unlinking the only login method must fail cleanly.
    let unlink_err = h
        .app()
        .execute(caller, UnlinkExternalIdentity { identity_id })
        .await;
    assert!(matches!(
        unlink_err.unwrap_err().current_context(),
        OidcUseCaseError::LetzteAnmeldemethode
    ));

    // 3. Re-auth deletion flow succeeds while passwordless.
    let reauth_url = h
        .app()
        .execute(
            caller,
            OidcStartAuth {
                provider_id: apple.clone(),
                mode: OidcFlowMode::DeleteReauth { benutzer_id },
                return_to: None,
                client_key: ck.clone(),
            },
        )
        .await
        .unwrap();
    let reauth_state = query_param(&reauth_url, "state").unwrap();
    h.oidc().stub_exchange(
        "apple-reauth-code",
        Ok(VerifiedOidcIdentity {
            provider_id: apple.clone(),
            subject: "sub-apple-1".to_owned(),
            email: EmailAdresse::normalisiert("relay123@privaterelay.appleid.com").unwrap(),
            display_name: None,
        }),
    );
    let reauth_cb = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcCompleteCallback {
                provider_id: apple,
                code: Some("apple-reauth-code".to_owned()),
                state: Some(reauth_state),
                provider_error: None,
                display_name_hint: None,
            },
        )
        .await
        .unwrap();
    let reauth_code = query_param(&reauth_cb.redirect_url, "delete_reauth_code").unwrap();

    // Verify setting initial password works on a second OIDC user, allowing unlink.
    let google = OidcProviderId::parse("google").unwrap();
    let u2_url = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcStartAuth {
                provider_id: google.clone(),
                mode: OidcFlowMode::Login,
                return_to: None,
                client_key: ck.clone(),
            },
        )
        .await
        .unwrap();
    h.oidc().stub_exchange(
        "g-code-2",
        Ok(VerifiedOidcIdentity {
            provider_id: google.clone(),
            subject: "sub-g2".to_owned(),
            email: EmailAdresse::normalisiert("nina@example.at").unwrap(),
            display_name: Some("Nina".to_owned()),
        }),
    );
    let u2_cb = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcCompleteCallback {
                provider_id: google,
                code: Some("g-code-2".to_owned()),
                state: query_param(&u2_url, "state"),
                provider_error: None,
                display_name_hint: None,
            },
        )
        .await
        .unwrap();
    let u2_session = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcExchangeCodeForSession {
                code: SecretToken::try_from(query_param(&u2_cb.redirect_url, "code").unwrap())
                    .unwrap(),
                client_key: ck.clone(),
            },
        )
        .await
        .unwrap();
    let u2_caller = AuthContext::Authenticated {
        benutzer_id: u2_session.benutzer.id(),
        session_id: u2_session.session_id,
    };
    h.app()
        .execute(
            u2_caller,
            PasswortErstmaligSetzen {
                neues_passwort: KlartextPasswort::neu(TEST_PASSWORT).unwrap(),
            },
        )
        .await
        .unwrap();
    // Calling PasswortErstmaligSetzen a second time fails with PasswortBereitsVorhanden.
    let dup_pw = h
        .app()
        .execute(
            u2_caller,
            PasswortErstmaligSetzen {
                neues_passwort: KlartextPasswort::neu(TEST_PASSWORT).unwrap(),
            },
        )
        .await;
    assert!(matches!(
        dup_pw.unwrap_err().current_context(),
        PasswortErstmaligSetzenFehler::PasswortBereitsVorhanden
    ));
    let u2_ids = h
        .app()
        .execute_fn::<_, _, RepositoryError>(u2_caller, async move |ctx| {
            let uow = ctx.enter().await?;
            let list = uow
                .external_identities()
                .find_by_benutzer(u2_session.benutzer.id())
                .await?;
            uow.rollback().await?;
            Ok(list)
        })
        .await
        .unwrap();
    h.app()
        .execute(
            u2_caller,
            UnlinkExternalIdentity {
                identity_id: u2_ids[0].id(),
            },
        )
        .await
        .unwrap();
    h.app()
        .execute(
            AuthContext::Anonymous,
            Anmeldung {
                email: EmailAdresse::normalisiert("nina@example.at").unwrap(),
                passwort: KlartextPasswort::neu(TEST_PASSWORT).unwrap(),
                client_key: ck.clone(),
            },
        )
        .await
        .unwrap();

    // Finally delete the passwordless Apple account using its reauth_code.
    h.app()
        .execute(
            caller,
            BenutzerLöschen {
                passwort: None,
                reauth_code: Some(SecretToken::try_from(reauth_code.as_str()).unwrap()),
                bestätigt: true,
            },
        )
        .await
        .unwrap();

    // Verify unverified email error maps gracefully cleanly without panics.
    let unverified_google = OidcProviderId::parse("google").unwrap();
    let u3_url = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcStartAuth {
                provider_id: unverified_google.clone(),
                mode: OidcFlowMode::Login,
                return_to: None,
                client_key: ck,
            },
        )
        .await
        .unwrap();
    h.oidc()
        .stub_exchange("unverified-code", Err(OidcClientError::EmailNotVerified));
    let u3_cb = h
        .app()
        .execute(
            AuthContext::Anonymous,
            OidcCompleteCallback {
                provider_id: unverified_google,
                code: Some("unverified-code".to_owned()),
                state: query_param(&u3_url, "state"),
                provider_error: None,
                display_name_hint: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        query_param(&u3_cb.redirect_url, "error").as_deref(),
        Some("email_not_verified")
    );
}
