//! Use cases, grouped by journey.

mod benachrichtigung;
mod capture;
mod game;
mod login;
mod password;
mod profile;
mod registration;
mod session;
mod user;

pub use benachrichtigung::*;
pub use capture::*;
pub use game::*;
pub use login::*;
pub use password::*;
pub use profile::*;
pub use registration::*;
pub use session::*;
pub use user::*;

use chrono::{DateTime, Utc};

use crate::{
    application::ExecutionContext,
    domain::{
        Benutzer, BenutzerId,
        konto::{AccessTokenClaims, NeueSession, RefreshToken, SessionId, TokenId},
    },
};

#[derive(Debug, Clone)]
pub struct SessionAusgestellt {
    pub benutzer: Benutzer,
    pub session_id: SessionId,
    pub refresh_token: RefreshToken,
    pub claims: AccessTokenClaims,
}

pub(crate) fn new_session(
    ctx: &ExecutionContext<'_>,
    benutzer_id: BenutzerId,
    refresh_token: &RefreshToken,
    jetzt: DateTime<Utc>,
) -> NeueSession {
    NeueSession::neu(
        benutzer_id,
        refresh_token.hash(),
        ctx.signing().public_key_hex(),
        jetzt,
    )
}

pub(crate) fn claims_for_session(
    ctx: &ExecutionContext<'_>,
    benutzer_id: BenutzerId,
    session_id: SessionId,
    jetzt: DateTime<Utc>,
) -> AccessTokenClaims {
    let konfiguration = ctx.identity_provider_conf();
    AccessTokenClaims::neu(
        benutzer_id,
        session_id,
        &konfiguration.issuer,
        &konfiguration.audience,
        jetzt,
        TokenId(uuid::Uuid::new_v4()),
    )
}
