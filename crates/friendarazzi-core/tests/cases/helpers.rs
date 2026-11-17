use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use friendarazzi_core::{
    App,
    adapters::{InMemorySigning, InMemoryThrottle},
    application::{AppBuilder, IdentityProviderConfiguration, SetUowProvider},
    domain::konto::AccessTokenClaims,
    ports::{Clock, Signing},
    service::SessionPrüfen,
};
use friendarazzi_fakes::{FakeMailSender, FixedClock};

pub fn test_signing_key() -> InMemorySigning {
    InMemorySigning::from_secret_bytes(&[7u8; 32])
}

pub fn fremder_schluessel() -> InMemorySigning {
    InMemorySigning::from_secret_bytes(&[9u8; 32])
}

const SIGNIERTE_NACHRICHT: &str = "eyJhbGciOiJFZERTQSJ9.eyJzdWIiOiJ0ZXN0In0";

pub fn pruefen_mit(signing: &dyn Signing, claims: AccessTokenClaims) -> SessionPrüfen {
    SessionPrüfen {
        claims,
        signed_message: SIGNIERTE_NACHRICHT.to_owned(),
        signature: signing.sign(SIGNIERTE_NACHRICHT.as_bytes()),
    }
}

pub fn pruefen(_harness: &Harness, claims: AccessTokenClaims) -> SessionPrüfen {
    pruefen_mit(&test_signing_key(), claims)
}

pub fn test_konfiguration() -> IdentityProviderConfiguration {
    IdentityProviderConfiguration::neu("friendarazzi.app", "friendarazzi-api")
}

pub const FRONTEND_BASIS_URL: &str = "https://friendarazzi.app";

pub struct Harness {
    app: App,
    clock: Arc<FixedClock>,
    mail: Arc<FakeMailSender>,
}

impl Harness {
    pub fn app(&self) -> &App {
        &self.app
    }
    pub fn mail(&self) -> &FakeMailSender {
        self.mail.as_ref()
    }

    pub fn jetzt(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    pub fn vorspulen(&self, dauer: Duration) {
        self.clock.advance_by(dauer);
    }

    pub fn letztes_token(&self) -> String {
        let mail = self.mail.latest();
        let text = mail.text(FRONTEND_BASIS_URL);
        let marker = "token=";
        let start = text
            .find(marker)
            .unwrap_or_else(|| panic!("mail enthält keinen token-link: {text}"))
            + marker.len();
        text[start..]
            .chars()
            .take_while(|zeichen| {
                zeichen.is_ascii_alphanumeric() || *zeichen == '-' || *zeichen == '_'
            })
            .collect()
    }
}

pub fn build(builder: AppBuilder<SetUowProvider>) -> Harness {
    let clock = Arc::new(FixedClock::new(jetzt()));
    let mail = Arc::new(FakeMailSender::new());
    let app = builder
        .signing(Arc::new(test_signing_key()))
        .throttle(Arc::new(InMemoryThrottle::new()))
        .mail_sender(mail.clone())
        .konto(test_konfiguration())
        .clock(clock.clone())
        .build();
    Harness { app, clock, mail }
}

pub fn jetzt() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).expect("static timestamp is valid")
}
