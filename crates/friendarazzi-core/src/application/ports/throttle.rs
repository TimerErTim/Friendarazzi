use std::net::IpAddr;

use chrono::{DateTime, Duration, Utc};

use crate::application::ResultReport;

/// What a client is trying to do, so each action has its own budget.
///
/// Variants keep the German use-case nouns they name (`Anmeldung`, `RegistrierungStart`,
/// …): those are domain language. The type itself is application code and is English.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThrottleAction {
    /// Failed logins: 20 per 15 minutes (SPEC-konto rule 5, AC-15).
    Anmeldung,
    /// Single-step registration: 5 per hour.
    Registrierung,
    /// Confirmation link verification: 10 per hour.
    EmailBestätigung,
    /// Confirmation email resend: 3 per hour.
    EmailBestätigungErneut,
    /// Password recovery request: 5 per hour (rule 5, AC-16).
    Wiederherstellung,
    /// Session renewal: 5 per hour (rule 5, AC-16).
    Erneuerung,
    /// OIDC authorization initiation: 20 per 15 minutes.
    OidcStart,
    /// OIDC one-time code exchange: 20 per 15 minutes.
    OidcExchange,
}

impl ThrottleAction {
    /// Failed attempts allowed inside one window.
    pub fn threshold(self) -> u32 {
        match self {
            Self::Anmeldung | Self::OidcStart | Self::OidcExchange => 20,
            Self::EmailBestätigung => 10,
            Self::EmailBestätigungErneut => 3,
            Self::Registrierung | Self::Wiederherstellung | Self::Erneuerung => 5,
        }
    }

    /// Length of the window the threshold applies to.
    pub fn window(self) -> Duration {
        match self {
            Self::Anmeldung | Self::OidcStart | Self::OidcExchange => Duration::minutes(15),
            Self::Registrierung
            | Self::EmailBestätigung
            | Self::EmailBestätigungErneut
            | Self::Wiederherstellung
            | Self::Erneuerung => Duration::hours(1),
        }
    }

    /// Stable key of one action's counter, for a log line and a store key.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anmeldung => "anmeldung",
            Self::Registrierung => "registrierung",
            Self::EmailBestätigung => "email-bestaetigung",
            Self::EmailBestätigungErneut => "email-bestaetigung-erneut",
            Self::Wiederherstellung => "wiederherstellung",
            Self::Erneuerung => "erneuerung",
            Self::OidcStart => "oidc-start",
            Self::OidcExchange => "oidc-exchange",
        }
    }
}

/// Who is being throttled: one client, identified by its IP and a client fingerprint.
///
/// Never an E-Mail-Adresse. A per-address limit would let anyone lock an owner out by
/// guessing their address (SPEC-konto rule 5, Q-16).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClientKey {
    pub ip: IpAddr,
    pub fingerprint: String,
}

impl ClientKey {
    pub fn new(ip: IpAddr, fingerprint: impl Into<String>) -> Self {
        Self {
            ip,
            fingerprint: fingerprint.into(),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ThrottleError {
    #[error("zu viele Versuche; bitte später erneut versuchen")]
    TooManyAttempts,
    #[error("die Drossel ist nicht erreichbar")]
    Unavailable,
}

/// Client-scoped rate limiting. An infrastructure port, not domain behaviour: the
/// thresholds above are the product rule, the store is an adapter choice.
#[async_trait::async_trait]
pub trait Throttle: Send + Sync {
    /// Check whether this client has exhausted an action's failure budget.
    async fn check(
        &self,
        client: &ClientKey,
        action: ThrottleAction,
        now: DateTime<Utc>,
    ) -> ResultReport<(), ThrottleError>;

    /// Count an invalid attempt toward the action's failure budget.
    async fn increment(
        &self,
        client: &ClientKey,
        action: ThrottleAction,
        now: DateTime<Utc>,
    ) -> ResultReport<(), ThrottleError>;

    /// Count an attempt and refuse it once the budget is exhausted.
    async fn check_and_increment(
        &self,
        client: &ClientKey,
        action: ThrottleAction,
        now: DateTime<Utc>,
    ) -> ResultReport<(), ThrottleError>;

    /// Forget one action's counter for this client after a successful login (rule 5).
    async fn reset(
        &self,
        client: &ClientKey,
        action: ThrottleAction,
    ) -> ResultReport<(), ThrottleError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test]
    fn schwellen_entsprechen_der_spezifikation() {
        assert_eq!(ThrottleAction::Anmeldung.threshold(), 20);
        assert_eq!(ThrottleAction::Anmeldung.window(), Duration::minutes(15));
        assert_eq!(ThrottleAction::EmailBestätigung.threshold(), 10);
        assert_eq!(ThrottleAction::EmailBestätigungErneut.threshold(), 3);
        for aktion in [
            ThrottleAction::Registrierung,
            ThrottleAction::Wiederherstellung,
            ThrottleAction::Erneuerung,
        ] {
            assert_eq!(aktion.threshold(), 5);
            assert_eq!(aktion.window(), Duration::hours(1));
        }
    }
}
