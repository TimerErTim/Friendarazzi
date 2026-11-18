use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use friendarazzi_core::{
    App,
    adapters::{InMemorySigning, InMemoryThrottle},
    application::{AppBuilder, AuthContext, IdentityProviderConfiguration, SetUowProvider},
    domain::{
        BenutzerId, BoardTile, EmailAdresse, Game, GameId, GridSize, NonEmptyString, PromptSource,
        VisibilityMode,
        konto::{AccessTokenClaims, KlartextPasswort, SecretToken, SessionId},
        target_quotas_for_author,
    },
    ports::{ClientKey, Clock, RepositoryError, Signing},
    service::{
        CreateGame, DraftPromptInput, JoinGame, LockDraftPrompts, RegistrierungAbschließen,
        RegistrierungStarten, SaveDraftPrompts, SessionPrüfen, StartDrafting,
    },
};
use friendarazzi_fakes::{FakeMailSender, FixedClock, FixedRng};
use uuid::Uuid;

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
        .rng(Arc::new(FixedRng::default()))
        .build();
    Harness { app, clock, mail }
}

pub fn jetzt() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).expect("static timestamp is valid")
}

pub const TEST_PASSWORT: &str = "korrekt-pferd-batterie";

pub async fn register_user(harness: &Harness, email: &str) -> AuthContext {
    let client_key = ClientKey::new("127.0.0.1".parse().unwrap(), email.to_owned());
    harness
        .app()
        .execute(
            AuthContext::Anonymous,
            RegistrierungStarten {
                email: EmailAdresse::normalisiert(email).unwrap(),
                client_key: client_key.clone(),
            },
        )
        .await
        .unwrap();
    let token = SecretToken::try_from(harness.letztes_token().as_str()).unwrap();
    let session = harness
        .app()
        .execute(
            AuthContext::Anonymous,
            RegistrierungAbschließen {
                token,
                passwort: KlartextPasswort::neu(TEST_PASSWORT).unwrap(),
                client_key,
            },
        )
        .await
        .unwrap();
    AuthContext::Authenticated {
        benutzer_id: session.benutzer.id(),
        session_id: SessionId(Uuid::new_v4()),
    }
}

pub fn user_id_of(auth: AuthContext) -> BenutzerId {
    auth.require_benutzer_id().unwrap()
}

pub fn balanced_prompts_for(
    player_ids: &[BenutzerId],
    author: BenutzerId,
    tiles_needed: usize,
    source: PromptSource,
    label_prefix: &str,
) -> Vec<DraftPromptInput> {
    let quotas = target_quotas_for_author(player_ids, author, tiles_needed).unwrap();
    let mut prompts = Vec::new();
    for (target, count) in quotas {
        for idx in 0..count {
            prompts.push(DraftPromptInput {
                target_user_id: target,
                text: NonEmptyString::neu(format!("{label_prefix} {idx}")).unwrap(),
                source,
            });
        }
    }
    prompts
}

pub async fn setup_active_game(
    harness: &Harness,
    host: AuthContext,
    guests: &[AuthContext],
    title: &str,
    grid_size: GridSize,
    visibility_mode: VisibilityMode,
) -> Game {
    let game = harness
        .app()
        .execute(
            host,
            CreateGame {
                title: NonEmptyString::neu(title).unwrap(),
                grid_size,
                visibility_mode,
            },
        )
        .await
        .unwrap();

    for &guest in guests {
        harness.vorspulen(Duration::seconds(1));
        harness
            .app()
            .execute(
                guest,
                JoinGame {
                    invite_code: game.invite_code().clone(),
                },
            )
            .await
            .unwrap();
    }

    let drafted = harness
        .app()
        .execute(host, StartDrafting { game_id: game.id() })
        .await
        .unwrap();

    let mut all_auths = Vec::with_capacity(1 + guests.len());
    all_auths.push(host);
    all_auths.extend_from_slice(guests);
    let player_ids: Vec<_> = all_auths.iter().copied().map(user_id_of).collect();
    let tiles_needed = drafted.playable_tile_count();

    let mut last_state = drafted;
    for (p_idx, &auth) in all_auths.iter().enumerate() {
        let uid = user_id_of(auth);
        let prompts = balanced_prompts_for(
            &player_ids,
            uid,
            tiles_needed,
            PromptSource::Custom,
            &format!("P{p_idx} Action"),
        );
        harness
            .app()
            .execute(
                auth,
                SaveDraftPrompts {
                    game_id: game.id(),
                    prompts,
                },
            )
            .await
            .unwrap();
        last_state = harness
            .app()
            .execute(auth, LockDraftPrompts { game_id: game.id() })
            .await
            .unwrap();
    }
    assert!(matches!(last_state, Game::Active(_)));
    last_state
}

pub async fn load_tiles(harness: &Harness, auth: AuthContext, game_id: GameId) -> Vec<BoardTile> {
    let owner_id = user_id_of(auth);
    harness
        .app()
        .execute_fn(auth, async move |ctx| {
            let uow = ctx.enter().await.unwrap();
            let board = uow
                .boards()
                .find_by_game_and_owner(game_id, owner_id)
                .await
                .unwrap();
            let tiles = board.into_data().tiles().to_vec();
            uow.finish(
                Ok::<_, error_stack::Report<RepositoryError>>(tiles),
                RepositoryError::Unknown,
            )
            .await
        })
        .await
        .unwrap()
}

pub fn is_webp(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP"
}
