//! In-app notifications, per-user notification preferences, and persisted unsubscribe tokens.

use std::str::FromStr;

use chrono::{DateTime, Duration, Utc};
use error_stack::{Report, ResultExt};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    ResultReport,
    domain::{
        BenutzerId, CaptureId, GameId, NonEmptyString,
        konto::{SecretToken, SecretTokenHash},
    },
};

const CONSTRUCTING: &str = "while constructing benachrichtigung";

/// Unsubscribe tokens stay valid for 30 days after the notification email was created.
pub const UNSUBSCRIBE_TOKEN_LIFETIME: Duration = Duration::days(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BenachrichtigungId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BenachrichtigungArt {
    SpielGestartet,
    AufnahmeHochgeladen,
    SpielBeendet,
}

impl BenachrichtigungArt {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SpielGestartet => "game_started",
            Self::AufnahmeHochgeladen => "capture_posted",
            Self::SpielBeendet => "game_completed",
        }
    }

    pub fn dedupe_key(self, game_id: GameId, capture_id: Option<CaptureId>) -> String {
        match (self, capture_id) {
            (Self::AufnahmeHochgeladen, Some(cid)) => format!("capture_posted:{}", cid.0),
            (Self::SpielGestartet, _) => format!("game_started:{}", game_id.0),
            (Self::SpielBeendet, _) => format!("game_completed:{}", game_id.0),
            (Self::AufnahmeHochgeladen, None) => format!("capture_posted_game:{}", game_id.0),
        }
    }
}

impl FromStr for BenachrichtigungArt {
    type Err = BenachrichtigungFehler;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "game_started" => Ok(Self::SpielGestartet),
            "capture_posted" => Ok(Self::AufnahmeHochgeladen),
            "game_completed" => Ok(Self::SpielBeendet),
            _ => Err(BenachrichtigungFehler::UngültigeArt),
        }
    }
}

#[derive(Debug, Error)]
pub enum BenachrichtigungFehler {
    #[error("unbekannte Benachrichtigungsart")]
    UngültigeArt,
    #[error("der Deduplizierungsschlüssel darf nicht leer sein")]
    UngültigerDedupeSchlüssel,
    #[error("der Abmelde-Link ist ungültig oder abgelaufen")]
    UngültigesAbbestellToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeueBenachrichtigung {
    pub empfänger_id: BenutzerId,
    pub art: BenachrichtigungArt,
    pub game_id: GameId,
    pub capture_id: Option<CaptureId>,
    pub actor_id: Option<BenutzerId>,
    pub target_id: Option<BenutzerId>,
    pub dedupe_key: NonEmptyString,
    pub abbestell_token_hash: SecretTokenHash,
    pub erstellt_am: DateTime<Utc>,
}

impl NeueBenachrichtigung {
    pub fn spiel_gestartet(
        empfänger_id: BenutzerId,
        game_id: GameId,
        erstellt_am: DateTime<Utc>,
    ) -> (Self, String) {
        let dedupe_key =
            NonEmptyString::neu(BenachrichtigungArt::SpielGestartet.dedupe_key(game_id, None))
                .expect("static prefix makes dedupe key non-empty");
        let (raw_token, abbestell_token_hash) = AbbestellToken::neu_paar();
        (
            Self {
                empfänger_id,
                art: BenachrichtigungArt::SpielGestartet,
                game_id,
                capture_id: None,
                actor_id: None,
                target_id: None,
                dedupe_key,
                abbestell_token_hash,
                erstellt_am,
            },
            raw_token,
        )
    }

    pub fn aufnahme_hochgeladen(
        empfänger_id: BenutzerId,
        game_id: GameId,
        capture_id: CaptureId,
        catcher_id: BenutzerId,
        target_id: BenutzerId,
        erstellt_am: DateTime<Utc>,
    ) -> (Self, String) {
        let dedupe_key = NonEmptyString::neu(
            BenachrichtigungArt::AufnahmeHochgeladen.dedupe_key(game_id, Some(capture_id)),
        )
        .expect("static prefix makes dedupe key non-empty");
        let (raw_token, abbestell_token_hash) = AbbestellToken::neu_paar();
        (
            Self {
                empfänger_id,
                art: BenachrichtigungArt::AufnahmeHochgeladen,
                game_id,
                capture_id: Some(capture_id),
                actor_id: Some(catcher_id),
                target_id: Some(target_id),
                dedupe_key,
                abbestell_token_hash,
                erstellt_am,
            },
            raw_token,
        )
    }

    pub fn spiel_beendet(
        empfänger_id: BenutzerId,
        game_id: GameId,
        winner_id: BenutzerId,
        erstellt_am: DateTime<Utc>,
    ) -> (Self, String) {
        let dedupe_key =
            NonEmptyString::neu(BenachrichtigungArt::SpielBeendet.dedupe_key(game_id, None))
                .expect("static prefix makes dedupe key non-empty");
        let (raw_token, abbestell_token_hash) = AbbestellToken::neu_paar();
        (
            Self {
                empfänger_id,
                art: BenachrichtigungArt::SpielBeendet,
                game_id,
                capture_id: None,
                actor_id: Some(winner_id),
                target_id: None,
                dedupe_key,
                abbestell_token_hash,
                erstellt_am,
            },
            raw_token,
        )
    }
}

/// Persistent in-app notification for one recipient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Benachrichtigung {
    id: BenachrichtigungId,
    empfänger_id: BenutzerId,
    art: BenachrichtigungArt,
    game_id: GameId,
    capture_id: Option<CaptureId>,
    actor_id: Option<BenutzerId>,
    target_id: Option<BenutzerId>,
    dedupe_key: NonEmptyString,
    abbestell_token_hash: SecretTokenHash,
    erstellt_am: DateTime<Utc>,
    gelesen_am: Option<DateTime<Utc>>,
}

impl Benachrichtigung {
    pub fn neu(id: BenachrichtigungId, neu: NeueBenachrichtigung) -> Self {
        Self {
            id,
            empfänger_id: neu.empfänger_id,
            art: neu.art,
            game_id: neu.game_id,
            capture_id: neu.capture_id,
            actor_id: neu.actor_id,
            target_id: neu.target_id,
            dedupe_key: neu.dedupe_key,
            abbestell_token_hash: neu.abbestell_token_hash,
            erstellt_am: neu.erstellt_am,
            gelesen_am: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: BenachrichtigungId,
        empfänger_id: BenutzerId,
        art: &str,
        game_id: GameId,
        capture_id: Option<CaptureId>,
        actor_id: Option<BenutzerId>,
        target_id: Option<BenutzerId>,
        dedupe_key: String,
        abbestell_token_hash: String,
        erstellt_am: DateTime<Utc>,
        gelesen_am: Option<DateTime<Utc>>,
    ) -> ResultReport<Self, BenachrichtigungFehler> {
        let art = art.parse().map_err(Report::new).attach(CONSTRUCTING)?;
        let dedupe_key = NonEmptyString::neu(dedupe_key)
            .change_context(BenachrichtigungFehler::UngültigerDedupeSchlüssel)
            .attach(CONSTRUCTING)?;
        let abbestell_token_hash = SecretTokenHash::from_hex(abbestell_token_hash)
            .change_context(BenachrichtigungFehler::UngültigesAbbestellToken)
            .attach(CONSTRUCTING)?;
        Ok(Self {
            id,
            empfänger_id,
            art,
            game_id,
            capture_id,
            actor_id,
            target_id,
            dedupe_key,
            abbestell_token_hash,
            erstellt_am,
            gelesen_am,
        })
    }

    pub fn id(&self) -> BenachrichtigungId {
        self.id
    }

    pub fn empfänger_id(&self) -> BenutzerId {
        self.empfänger_id
    }

    pub fn art(&self) -> BenachrichtigungArt {
        self.art
    }

    pub fn game_id(&self) -> GameId {
        self.game_id
    }

    pub fn capture_id(&self) -> Option<CaptureId> {
        self.capture_id
    }

    pub fn actor_id(&self) -> Option<BenutzerId> {
        self.actor_id
    }

    pub fn target_id(&self) -> Option<BenutzerId> {
        self.target_id
    }

    pub fn dedupe_key(&self) -> &NonEmptyString {
        &self.dedupe_key
    }

    pub fn abbestell_token_hash(&self) -> &SecretTokenHash {
        &self.abbestell_token_hash
    }

    pub fn erstellt_am(&self) -> DateTime<Utc> {
        self.erstellt_am
    }

    pub fn gelesen_am(&self) -> Option<DateTime<Utc>> {
        self.gelesen_am
    }

    pub fn ist_ungelesen(&self) -> bool {
        self.gelesen_am.is_none()
    }

    pub fn abbestell_token_abgelaufen(&self, jetzt: DateTime<Utc>) -> bool {
        jetzt >= self.erstellt_am + UNSUBSCRIBE_TOKEN_LIFETIME
    }

    /// Mark the notification read if still unread; idempotent so first read time survives.
    pub fn als_gelesen_markieren(&mut self, jetzt: DateTime<Utc>) {
        if self.gelesen_am.is_none() {
            self.gelesen_am = Some(jetzt);
        }
    }
}

/// Timestamped opt-in/opt-out toggle for one preference dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreferenceToggle {
    aktiv: bool,
    aktualisiert_am: DateTime<Utc>,
}

impl PreferenceToggle {
    pub fn neu(aktiv: bool, aktualisiert_am: DateTime<Utc>) -> Self {
        Self {
            aktiv,
            aktualisiert_am,
        }
    }

    pub fn aktiv(&self) -> bool {
        self.aktiv
    }

    pub fn aktualisiert_am(&self) -> DateTime<Utc> {
        self.aktualisiert_am
    }

    pub fn setze(&mut self, aktiv: bool, jetzt: DateTime<Utc>) {
        self.aktiv = aktiv;
        self.aktualisiert_am = jetzt;
    }
}

/// Per-user notification preferences: default game watch state + per-event email opt-out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenachrichtigungsEinstellungen {
    standard_spiel_beobachten: PreferenceToggle,
    email_spiel_gestartet: PreferenceToggle,
    email_aufnahme_hochgeladen: PreferenceToggle,
    email_spiel_beendet: PreferenceToggle,
}

impl BenachrichtigungsEinstellungen {
    pub fn from_parts(
        standard_spiel_beobachten: PreferenceToggle,
        email_spiel_gestartet: PreferenceToggle,
        email_aufnahme_hochgeladen: PreferenceToggle,
        email_spiel_beendet: PreferenceToggle,
    ) -> Self {
        Self {
            standard_spiel_beobachten,
            email_spiel_gestartet,
            email_aufnahme_hochgeladen,
            email_spiel_beendet,
        }
    }

    pub fn standard(jetzt: DateTime<Utc>) -> Self {
        let toggle = PreferenceToggle::neu(true, jetzt);
        Self::from_parts(toggle, toggle, toggle, toggle)
    }

    pub fn standard_spiel_beobachten(&self) -> PreferenceToggle {
        self.standard_spiel_beobachten
    }

    pub fn email_spiel_gestartet(&self) -> PreferenceToggle {
        self.email_spiel_gestartet
    }

    pub fn email_aufnahme_hochgeladen(&self) -> PreferenceToggle {
        self.email_aufnahme_hochgeladen
    }

    pub fn email_spiel_beendet(&self) -> PreferenceToggle {
        self.email_spiel_beendet
    }

    pub fn email_erlaubt(&self, art: BenachrichtigungArt) -> bool {
        match art {
            BenachrichtigungArt::SpielGestartet => self.email_spiel_gestartet.aktiv,
            BenachrichtigungArt::AufnahmeHochgeladen => self.email_aufnahme_hochgeladen.aktiv,
            BenachrichtigungArt::SpielBeendet => self.email_spiel_beendet.aktiv,
        }
    }

    pub fn setze_standard_spiel_beobachten(&mut self, aktiv: bool, jetzt: DateTime<Utc>) {
        self.standard_spiel_beobachten.setze(aktiv, jetzt);
    }

    pub fn setze_email(&mut self, art: BenachrichtigungArt, aktiv: bool, jetzt: DateTime<Utc>) {
        match art {
            BenachrichtigungArt::SpielGestartet => self.email_spiel_gestartet.setze(aktiv, jetzt),
            BenachrichtigungArt::AufnahmeHochgeladen => {
                self.email_aufnahme_hochgeladen.setze(aktiv, jetzt)
            }
            BenachrichtigungArt::SpielBeendet => self.email_spiel_beendet.setze(aktiv, jetzt),
        }
    }
}

/// CSPRNG unsubscribe token generator/hasher paired with a persisted [`SecretTokenHash`].
pub struct AbbestellToken;

impl AbbestellToken {
    pub fn neu_paar() -> (String, SecretTokenHash) {
        let secret = SecretToken::random().expect("CSPRNG available");
        let raw = secret.base64url().to_string();
        let hash = secret.hash();
        (raw, hash)
    }

    pub fn hash_für_suche(raw: &str) -> ResultReport<SecretTokenHash, BenachrichtigungFehler> {
        SecretToken::try_from(raw.trim())
            .map(|token| token.hash())
            .map_err(|_| Report::new(BenachrichtigungFehler::UngültigesAbbestellToken))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jetzt() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[test_log::test]
    fn benachrichtigung_from_parts_roundtrip_and_mark_read() {
        let id = BenachrichtigungId(Uuid::new_v4());
        let user_id = BenutzerId(Uuid::new_v4());
        let game_id = GameId(Uuid::new_v4());
        let capture_id = CaptureId(Uuid::new_v4());
        let catcher_id = BenutzerId(Uuid::new_v4());

        let (neu, raw_token) = NeueBenachrichtigung::aufnahme_hochgeladen(
            user_id,
            game_id,
            capture_id,
            catcher_id,
            user_id,
            jetzt(),
        );
        let mut notif = Benachrichtigung::neu(id, neu);
        assert_eq!(
            &AbbestellToken::hash_für_suche(&raw_token).unwrap(),
            notif.abbestell_token_hash()
        );
        assert!(notif.ist_ungelesen());
        assert!(!notif.abbestell_token_abgelaufen(jetzt() + Duration::days(29)));
        assert!(notif.abbestell_token_abgelaufen(jetzt() + UNSUBSCRIBE_TOKEN_LIFETIME));

        let read_time = jetzt() + Duration::minutes(5);
        notif.als_gelesen_markieren(read_time);
        assert!(!notif.ist_ungelesen());
        assert_eq!(notif.gelesen_am(), Some(read_time));

        // Second mark-read keeps original timestamp.
        notif.als_gelesen_markieren(read_time + Duration::minutes(10));
        assert_eq!(notif.gelesen_am(), Some(read_time));

        let reloaded = Benachrichtigung::from_parts(
            notif.id(),
            notif.empfänger_id(),
            notif.art().as_str(),
            notif.game_id(),
            notif.capture_id(),
            notif.actor_id(),
            notif.target_id(),
            notif.dedupe_key().as_str().to_owned(),
            notif.abbestell_token_hash().as_str().to_owned(),
            notif.erstellt_am(),
            notif.gelesen_am(),
        )
        .unwrap();
        assert_eq!(notif, reloaded);
    }

    #[test_log::test]
    fn abbestell_token_is_unique_hashed_and_rejects_malformed() {
        let (first_raw, first_hash) = AbbestellToken::neu_paar();
        let (second_raw, second_hash) = AbbestellToken::neu_paar();
        assert_ne!(first_raw, second_raw);
        assert_ne!(first_hash, second_hash);
        assert_eq!(
            AbbestellToken::hash_für_suche(&first_raw).unwrap(),
            first_hash
        );

        assert!(matches!(
            AbbestellToken::hash_für_suche("not-a-valid-token")
                .unwrap_err()
                .current_context(),
            BenachrichtigungFehler::UngültigesAbbestellToken
        ));

        let err = Benachrichtigung::from_parts(
            BenachrichtigungId(Uuid::new_v4()),
            BenutzerId(Uuid::new_v4()),
            BenachrichtigungArt::SpielGestartet.as_str(),
            GameId(Uuid::new_v4()),
            None,
            None,
            None,
            "dedupe".to_owned(),
            "not-hex".to_owned(),
            jetzt(),
            None,
        )
        .unwrap_err();
        assert!(matches!(
            err.current_context(),
            BenachrichtigungFehler::UngültigesAbbestellToken
        ));
    }

    #[test_log::test]
    fn einstellungen_update_tracks_only_changed_toggle() {
        let mut prefs = BenachrichtigungsEinstellungen::standard(jetzt());
        assert!(prefs.standard_spiel_beobachten().aktiv());
        assert!(prefs.email_erlaubt(BenachrichtigungArt::SpielGestartet));
        assert!(prefs.email_erlaubt(BenachrichtigungArt::AufnahmeHochgeladen));
        assert!(prefs.email_erlaubt(BenachrichtigungArt::SpielBeendet));

        let later = jetzt() + Duration::hours(2);
        prefs.setze_email(BenachrichtigungArt::AufnahmeHochgeladen, false, later);
        assert!(!prefs.email_erlaubt(BenachrichtigungArt::AufnahmeHochgeladen));
        assert_eq!(prefs.email_aufnahme_hochgeladen().aktualisiert_am(), later);
        assert_eq!(prefs.email_spiel_beendet().aktualisiert_am(), jetzt());
    }
}
