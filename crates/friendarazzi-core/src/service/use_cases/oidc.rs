//! OpenID Connect sign-in, provider linking, unlink protection, and code exchange.

use async_trait::async_trait;
use error_stack::{IntoReport, Report, ResultExt};

use crate::{
    application::ExecutionContext,
    domain::{
        EmailAdresse, NeuerBenutzer, NonEmptyString,
        konto::{
            ExternalIdentityId, NeueExternalIdentity, NeuerOidcAuthState, NeuerOidcExchangeCode,
            OidcExchangePurpose, OidcFlowMode, OidcProviderId, RefreshToken, SecretToken,
            pkce_s256_challenge,
        },
    },
    ports::{ClientKey, OidcClientError, RepositoryError, ThrottleAction},
    service::{SessionAusgestellt, UseCase},
};

use super::{claims_for_session, new_session};

#[derive(Debug, thiserror::Error)]
pub enum OidcUseCaseError {
    #[error("die Anfrage ist ungültig")]
    UngültigeAnfrage,
    #[error("für diese Aktion ist eine Anmeldung erforderlich")]
    NichtAngemeldet,
    #[error("dieser Anmeldeanbieter ist nicht konfiguriert")]
    AnbieterNichtKonfiguriert,
    #[error("zu viele Versuche; bitte später erneut versuchen")]
    Gedrosselt,
    #[error("OIDC-Code ist ungültig oder abgelaufen")]
    CodeUngültig,
    #[error("OIDC-Identität wurde nicht gefunden")]
    IdentitätNichtGefunden,
    #[error("die letzte verbleibende Anmeldemethode kann nicht entfernt werden")]
    LetzteAnmeldemethode,
    #[error("OIDC-Vorgang konnte nicht gestartet werden")]
    StartFehlgeschlagen,
    #[error("Datenbankzugriff fehlgeschlagen")]
    DatenbankFehler,
    #[error("Sitzung konnte nicht erstellt werden")]
    SessionFehler,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcCallbackOutcome {
    pub redirect_url: String,
}

/// Start an OIDC authorization flow (public login, authenticated link, or delete re-auth).
#[derive(Debug, Clone)]
pub struct OidcStartAuth {
    pub provider_id: OidcProviderId,
    pub mode: OidcFlowMode,
    pub return_to: Option<String>,
    pub client_key: ClientKey,
}

#[async_trait]
impl UseCase<String> for OidcStartAuth {
    type Error = Report<OidcUseCaseError>;

    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<String, Self::Error> {
        let now = ctx.clock().now();

        match self.mode {
            OidcFlowMode::Login => {}
            OidcFlowMode::Link { benutzer_id } | OidcFlowMode::DeleteReauth { benutzer_id } => {
                let caller = ctx
                    .auth()
                    .require_benutzer_id()
                    .change_context(OidcUseCaseError::NichtAngemeldet)?;
                if caller != benutzer_id {
                    return Err(Report::new(OidcUseCaseError::NichtAngemeldet));
                }
            }
        }

        let return_to = match self.return_to {
            Some(raw) => Some(validated_relative_path(&raw)?),
            None => None,
        };

        ctx.throttle()
            .check_and_increment(&self.client_key, ThrottleAction::OidcStart, now)
            .await
            .change_context(OidcUseCaseError::Gedrosselt)?;

        let configured = ctx
            .oidc_client()
            .configured_providers()
            .into_iter()
            .any(|p| p.id == self.provider_id);
        if !configured {
            return Err(Report::new(OidcUseCaseError::AnbieterNichtKonfiguriert));
        }

        let state_token =
            SecretToken::random().change_context(OidcUseCaseError::StartFehlgeschlagen)?;
        let nonce_token =
            SecretToken::random().change_context(OidcUseCaseError::StartFehlgeschlagen)?;
        let verifier_token =
            SecretToken::random().change_context(OidcUseCaseError::StartFehlgeschlagen)?;

        let state_raw = state_token.base64url();
        let nonce_raw = nonce_token.base64url();
        let verifier_raw = verifier_token.base64url();
        let code_challenge = pkce_s256_challenge(&verifier_raw);

        // Fetch/build provider URL before opening the SQLite write transaction so external
        // HTTP discovery never holds a database write lock.
        let auth_url = ctx
            .oidc_client()
            .build_authorization_url(&self.provider_id, &state_raw, &nonce_raw, &code_challenge)
            .await
            .map_err(|err| match err.current_context() {
                OidcClientError::ProviderNotConfigured => {
                    err.change_context(OidcUseCaseError::AnbieterNichtKonfiguriert)
                }
                _ => err.change_context(OidcUseCaseError::StartFehlgeschlagen),
            })?;

        let uow = ctx
            .enter()
            .await
            .change_context(OidcUseCaseError::DatenbankFehler)?;

        let save = async {
            uow.oidc_auth_states()
                .create(NeuerOidcAuthState::neu(
                    self.provider_id,
                    state_token.hash(),
                    nonce_raw.as_str(),
                    verifier_raw.as_str(),
                    self.mode,
                    return_to,
                    now,
                ))
                .await
                .map_err(IntoReport::into_report)
                .change_context(OidcUseCaseError::DatenbankFehler)?;
            Ok::<_, Report<OidcUseCaseError>>(())
        }
        .await;

        uow.finish(save, OidcUseCaseError::DatenbankFehler).await?;
        Ok(auth_url)
    }
}

/// Complete an OIDC callback (`GET` or Apple `form_post`) and issue a 120s browser exchange code.
#[derive(Debug, Clone)]
pub struct OidcCompleteCallback {
    pub provider_id: OidcProviderId,
    pub code: Option<String>,
    pub state: Option<String>,
    pub provider_error: Option<String>,
    pub display_name_hint: Option<String>,
}

#[async_trait]
impl UseCase<OidcCallbackOutcome> for OidcCompleteCallback {
    type Error = Report<OidcUseCaseError>;

    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<OidcCallbackOutcome, Self::Error> {
        let now = ctx.clock().now();
        let base_url = ctx
            .identity_provider_conf()
            .frontend_basis_url
            .as_deref()
            .unwrap_or("");

        let Some(state_raw) = self
            .state
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            let reason = if self.provider_error.is_some() {
                "provider_error"
            } else {
                "invalid_state"
            };
            return Ok(OidcCallbackOutcome {
                redirect_url: build_redirect(base_url, "/anmeldung/oidc", &[("error", reason)]),
            });
        };

        let Ok(state_token) = SecretToken::try_from(state_raw) else {
            return Ok(OidcCallbackOutcome {
                redirect_url: build_redirect(
                    base_url,
                    "/anmeldung/oidc",
                    &[("error", "invalid_state")],
                ),
            });
        };

        // UoW 1: load and immediately consume the state row before contacting the IdP.
        let uow = ctx
            .enter()
            .await
            .change_context(OidcUseCaseError::DatenbankFehler)?;
        let consumed = async {
            let mut state_row = match uow
                .oidc_auth_states()
                .find_by_state_hash(&state_token.hash())
                .await
            {
                Ok(found) => found,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Ok::<_, Report<OidcUseCaseError>>(None);
                }
                Err(err) => return Err(err).change_context(OidcUseCaseError::DatenbankFehler),
            };
            if state_row.verbrauchen(now).is_err() {
                return Ok(None);
            }
            uow.oidc_auth_states()
                .update(&mut state_row)
                .await
                .map_err(IntoReport::into_report)
                .change_context(OidcUseCaseError::DatenbankFehler)?;
            Ok(Some(state_row.into_data()))
        }
        .await;

        let Some(auth_state) = uow
            .finish(consumed, OidcUseCaseError::DatenbankFehler)
            .await?
        else {
            return Ok(OidcCallbackOutcome {
                redirect_url: build_redirect(
                    base_url,
                    "/anmeldung/oidc",
                    &[("error", "invalid_state")],
                ),
            });
        };

        if auth_state.provider_id() != &self.provider_id {
            return Ok(OidcCallbackOutcome {
                redirect_url: mode_error_redirect(base_url, auth_state.mode(), "provider_mismatch"),
            });
        }

        if self
            .provider_error
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
        {
            return Ok(OidcCallbackOutcome {
                redirect_url: mode_error_redirect(base_url, auth_state.mode(), "cancelled"),
            });
        }

        let Some(auth_code) = self
            .code
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            return Ok(OidcCallbackOutcome {
                redirect_url: mode_error_redirect(base_url, auth_state.mode(), "missing_code"),
            });
        };

        let verified = match ctx
            .oidc_client()
            .exchange_and_verify(
                &self.provider_id,
                auth_code,
                auth_state.pkce_verifier(),
                auth_state.nonce(),
                now,
            )
            .await
        {
            Ok(identity) => identity,
            Err(err) => {
                let code = match err.current_context() {
                    OidcClientError::EmailNotVerified => "email_not_verified",
                    _ => "exchange_failed",
                };
                return Ok(OidcCallbackOutcome {
                    redirect_url: mode_error_redirect(base_url, auth_state.mode(), code),
                });
            }
        };

        let uow = ctx
            .enter()
            .await
            .change_context(OidcUseCaseError::DatenbankFehler)?;
        let finish_flow = async {
            match auth_state.mode() {
                OidcFlowMode::Login => {
                    let existing_identity = uow
                        .external_identities()
                        .find_by_provider_subject(&verified.provider_id, &verified.subject)
                        .await;
                    let benutzer_id = match existing_identity {
                        Ok(identity) => identity.benutzer_id(),
                        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                            // Never silently link an OIDC login to an existing account by email.
                            match uow.benutzer().find_by_email(&verified.email).await {
                                Ok(_) => {
                                    return Ok::<_, Report<OidcUseCaseError>>(build_redirect(
                                        base_url,
                                        "/anmeldung/oidc",
                                        &[
                                            ("error", "email_conflict"),
                                            ("provider", verified.provider_id.as_str()),
                                        ],
                                    ));
                                }
                                Err(err)
                                    if matches!(
                                        err.current_context(),
                                        RepositoryError::NotFound
                                    ) => {}
                                Err(err) => {
                                    return Err(err)
                                        .change_context(OidcUseCaseError::DatenbankFehler);
                                }
                            }

                            let display_name = choose_display_name(
                                verified.display_name.as_deref(),
                                self.display_name_hint.as_deref(),
                                &verified.email,
                            );
                            let benutzer = uow
                                .benutzer()
                                .create(NeuerBenutzer::neu(
                                    verified.email.clone(),
                                    display_name,
                                    now,
                                ))
                                .await
                                .map_err(IntoReport::into_report)
                                .change_context(OidcUseCaseError::DatenbankFehler)?;
                            uow.external_identities()
                                .create(
                                    NeueExternalIdentity::neu(
                                        benutzer.id(),
                                        verified.provider_id.clone(),
                                        verified.subject.clone(),
                                        Some(verified.email.clone()),
                                        now,
                                    )
                                    .change_context(OidcUseCaseError::StartFehlgeschlagen)?,
                                )
                                .await
                                .map_err(IntoReport::into_report)
                                .change_context(OidcUseCaseError::DatenbankFehler)?;
                            benutzer.id()
                        }
                        Err(err) => {
                            return Err(err).change_context(OidcUseCaseError::DatenbankFehler);
                        }
                    };

                    let exchange_token = SecretToken::random()
                        .change_context(OidcUseCaseError::StartFehlgeschlagen)?;
                    let exchange_raw = exchange_token.base64url();
                    uow.oidc_exchange_codes()
                        .create(NeuerOidcExchangeCode::neu(
                            exchange_token.hash(),
                            benutzer_id,
                            OidcExchangePurpose::Session,
                            now,
                        ))
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(OidcUseCaseError::DatenbankFehler)?;

                    let mut params = vec![("code", exchange_raw.as_str())];
                    if let Some(next_path) = auth_state.return_to() {
                        params.push(("next", next_path));
                    }
                    Ok(build_redirect(base_url, "/anmeldung/oidc", &params))
                }
                OidcFlowMode::Link { benutzer_id } => {
                    match uow
                        .external_identities()
                        .find_by_provider_subject(&verified.provider_id, &verified.subject)
                        .await
                    {
                        Ok(existing) if existing.benutzer_id() != benutzer_id => {
                            Ok(build_redirect(
                                base_url,
                                "/optionen",
                                &[("oidc_error", "already_linked")],
                            ))
                        }
                        Ok(_) => Ok(build_redirect(
                            base_url,
                            "/optionen",
                            &[("oidc_linked", verified.provider_id.as_str())],
                        )),
                        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                            uow.external_identities()
                                .create(
                                    NeueExternalIdentity::neu(
                                        benutzer_id,
                                        verified.provider_id.clone(),
                                        verified.subject,
                                        Some(verified.email),
                                        now,
                                    )
                                    .change_context(OidcUseCaseError::StartFehlgeschlagen)?,
                                )
                                .await
                                .map_err(IntoReport::into_report)
                                .change_context(OidcUseCaseError::DatenbankFehler)?;
                            Ok(build_redirect(
                                base_url,
                                "/optionen",
                                &[("oidc_linked", verified.provider_id.as_str())],
                            ))
                        }
                        Err(err) => Err(err).change_context(OidcUseCaseError::DatenbankFehler),
                    }
                }
                OidcFlowMode::DeleteReauth { benutzer_id } => {
                    match uow
                        .external_identities()
                        .find_by_provider_subject(&verified.provider_id, &verified.subject)
                        .await
                    {
                        Ok(existing) if existing.benutzer_id() == benutzer_id => {
                            let exchange_token = SecretToken::random()
                                .change_context(OidcUseCaseError::StartFehlgeschlagen)?;
                            let exchange_raw = exchange_token.base64url();
                            uow.oidc_exchange_codes()
                                .create(NeuerOidcExchangeCode::neu(
                                    exchange_token.hash(),
                                    benutzer_id,
                                    OidcExchangePurpose::DeleteReauth,
                                    now,
                                ))
                                .await
                                .map_err(IntoReport::into_report)
                                .change_context(OidcUseCaseError::DatenbankFehler)?;
                            Ok(build_redirect(
                                base_url,
                                "/optionen",
                                &[("delete_reauth_code", exchange_raw.as_str())],
                            ))
                        }
                        Ok(_) => Ok(build_redirect(
                            base_url,
                            "/optionen",
                            &[("oidc_error", "reauth_mismatch")],
                        )),
                        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                            Ok(build_redirect(
                                base_url,
                                "/optionen",
                                &[("oidc_error", "reauth_mismatch")],
                            ))
                        }
                        Err(err) => Err(err).change_context(OidcUseCaseError::DatenbankFehler),
                    }
                }
            }
        }
        .await;

        let redirect_url = uow
            .finish(finish_flow, OidcUseCaseError::DatenbankFehler)
            .await?;
        Ok(OidcCallbackOutcome { redirect_url })
    }
}

/// Exchange a 120s single-use OIDC browser code for an active `Session`.
#[derive(Debug, Clone)]
pub struct OidcExchangeCodeForSession {
    pub code: SecretToken,
    pub client_key: ClientKey,
}

#[async_trait]
impl UseCase<SessionAusgestellt> for OidcExchangeCodeForSession {
    type Error = Report<OidcUseCaseError>;

    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<SessionAusgestellt, Self::Error> {
        let now = ctx.clock().now();

        ctx.throttle()
            .check_and_increment(&self.client_key, ThrottleAction::OidcExchange, now)
            .await
            .change_context(OidcUseCaseError::Gedrosselt)?;

        let refresh_token =
            RefreshToken::random().change_context(OidcUseCaseError::SessionFehler)?;

        let uow = ctx
            .enter()
            .await
            .change_context(OidcUseCaseError::DatenbankFehler)?;

        let outcome = async {
            let mut exchange = match uow
                .oidc_exchange_codes()
                .find_by_code_hash(&self.code.hash())
                .await
            {
                Ok(found) => found,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(OidcUseCaseError::CodeUngültig));
                }
                Err(err) => return Err(err).change_context(OidcUseCaseError::DatenbankFehler),
            };

            exchange
                .einlösen(OidcExchangePurpose::Session, now)
                .change_context(OidcUseCaseError::CodeUngültig)?;
            uow.oidc_exchange_codes()
                .update(&mut exchange)
                .await
                .map_err(IntoReport::into_report)
                .change_context(OidcUseCaseError::DatenbankFehler)?;

            let benutzer = uow
                .benutzer()
                .find_by_id(exchange.benutzer_id())
                .await
                .map_err(IntoReport::into_report)
                .change_context(OidcUseCaseError::DatenbankFehler)?;

            let session = uow
                .sessions()
                .create(new_session(&ctx, benutzer.id(), &refresh_token, now))
                .await
                .map_err(IntoReport::into_report)
                .change_context(OidcUseCaseError::SessionFehler)?;

            let claims = claims_for_session(&ctx, benutzer.id(), session.id(), now);
            Ok::<_, Report<OidcUseCaseError>>(SessionAusgestellt {
                benutzer: benutzer.into_data(),
                session_id: session.id(),
                refresh_token,
                claims,
            })
        }
        .await;

        uow.finish(outcome, OidcUseCaseError::DatenbankFehler).await
    }
}

/// Remove a linked external identity from the caller's `Benutzer`.
///
/// Refuses with [`OidcUseCaseError::LetzteAnmeldemethode`] when removing the final login method.
#[derive(Debug, Clone, Copy)]
pub struct UnlinkExternalIdentity {
    pub identity_id: ExternalIdentityId,
}

#[async_trait]
impl UseCase<()> for UnlinkExternalIdentity {
    type Error = Report<OidcUseCaseError>;

    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        let caller = ctx
            .auth()
            .require_benutzer_id()
            .change_context(OidcUseCaseError::NichtAngemeldet)?;

        let uow = ctx
            .enter()
            .await
            .change_context(OidcUseCaseError::DatenbankFehler)?;

        let outcome = async {
            let identity = match uow.external_identities().find_by_id(self.identity_id).await {
                Ok(found) => found,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(OidcUseCaseError::IdentitätNichtGefunden));
                }
                Err(err) => return Err(err).change_context(OidcUseCaseError::DatenbankFehler),
            };
            if identity.benutzer_id() != caller {
                return Err(Report::new(OidcUseCaseError::IdentitätNichtGefunden));
            }

            let konten = uow
                .konten()
                .find_by_benutzer(caller)
                .await
                .map_err(IntoReport::into_report)
                .change_context(OidcUseCaseError::DatenbankFehler)?;
            let identities = uow
                .external_identities()
                .find_by_benutzer(caller)
                .await
                .map_err(IntoReport::into_report)
                .change_context(OidcUseCaseError::DatenbankFehler)?;

            if konten.len() + identities.len() <= 1 {
                return Err(Report::new(OidcUseCaseError::LetzteAnmeldemethode));
            }

            uow.external_identities()
                .delete(identity)
                .await
                .map_err(IntoReport::into_report)
                .change_context(OidcUseCaseError::DatenbankFehler)?;

            Ok::<_, Report<OidcUseCaseError>>(())
        }
        .await;

        uow.finish(outcome, OidcUseCaseError::DatenbankFehler).await
    }
}

fn validated_relative_path(raw: &str) -> Result<String, Report<OidcUseCaseError>> {
    let trimmed = raw.trim();
    if !trimmed.starts_with('/') || trimmed.starts_with("//") || trimmed.contains('\\') {
        return Err(Report::new(OidcUseCaseError::UngültigeAnfrage));
    }
    Ok(trimmed.to_owned())
}

fn mode_error_redirect(base_url: &str, mode: OidcFlowMode, error: &str) -> String {
    match mode {
        OidcFlowMode::Login => build_redirect(base_url, "/anmeldung/oidc", &[("error", error)]),
        OidcFlowMode::Link { .. } | OidcFlowMode::DeleteReauth { .. } => {
            build_redirect(base_url, "/optionen", &[("oidc_error", error)])
        }
    }
}

fn build_redirect(base_url: &str, path: &str, params: &[(&str, &str)]) -> String {
    let mut url = String::from(base_url.trim_end_matches('/'));
    url.push_str(path);
    if !params.is_empty() {
        url.push('?');
        for (idx, (key, value)) in params.iter().enumerate() {
            if idx > 0 {
                url.push('&');
            }
            push_percent_encoded(&mut url, key);
            url.push('=');
            push_percent_encoded(&mut url, value);
        }
    }
    url
}

fn push_percent_encoded(out: &mut String, raw: &str) {
    for byte in raw.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push_str(&format!("{byte:02X}"));
        }
    }
}

fn choose_display_name(
    provider_name: Option<&str>,
    hint_name: Option<&str>,
    email: &EmailAdresse,
) -> NonEmptyString {
    if let Some(candidate) = provider_name.and_then(|raw| NonEmptyString::neu(raw).ok()) {
        return candidate;
    }
    if let Some(candidate) = hint_name.and_then(|raw| NonEmptyString::neu(raw).ok()) {
        return candidate;
    }
    let local = email
        .as_ref()
        .split_once('@')
        .map(|(prefix, _)| prefix)
        .unwrap_or("Spieler");
    NonEmptyString::neu(local).unwrap_or_else(|_| NonEmptyString::neu("Spieler").unwrap())
}
