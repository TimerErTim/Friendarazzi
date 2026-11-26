//! In-app notifications, per-user notification preferences, and signed unsubscribe tokens.

use std::str::FromStr;

use chrono::{DateTime, Duration, Utc};
use error_stack::{Report, ResultExt};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    ResultReport,
    domain::{BenutzerId, CaptureId, GameId, NonEmptyString, konto::Signature},
    ports::Signing,
};

const CONSTRUCTING: &str = "while constructing benachrichtigung";
const UNSUBSCRIBE_DOMAIN_PREFIX: &str = "friendarazzi:unsubscribe:v1";

/// Unsubscribe tokens stay valid for 30 days after the email was sent.
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
    pub erstellt_am: DateTime<Utc>,
}

impl NeueBenachrichtigung {
    pub fn spiel_gestartet(
        empfänger_id: BenutzerId,
        game_id: GameId,
        erstellt_am: DateTime<Utc>,
    ) -> Self {
        let dedupe_key =
            NonEmptyString::neu(BenachrichtigungArt::SpielGestartet.dedupe_key(game_id, None))
                .expect("static prefix makes dedupe key non-empty");
        Self {
            empfänger_id,
            art: BenachrichtigungArt::SpielGestartet,
            game_id,
            capture_id: None,
            actor_id: None,
            target_id: None,
            dedupe_key,
            erstellt_am,
        }
    }

    pub fn aufnahme_hochgeladen(
        empfänger_id: BenutzerId,
        game_id: GameId,
        capture_id: CaptureId,
        catcher_id: BenutzerId,
        target_id: BenutzerId,
        erstellt_am: DateTime<Utc>,
    ) -> Self {
        let dedupe_key = NonEmptyString::neu(
            BenachrichtigungArt::AufnahmeHochgeladen.dedupe_key(game_id, Some(capture_id)),
        )
        .expect("static prefix makes dedupe key non-empty");
        Self {
            empfänger_id,
            art: BenachrichtigungArt::AufnahmeHochgeladen,
            game_id,
            capture_id: Some(capture_id),
            actor_id: Some(catcher_id),
            target_id: Some(target_id),
            dedupe_key,
            erstellt_am,
        }
    }

    pub fn spiel_beendet(
        empfänger_id: BenutzerId,
        game_id: GameId,
        winner_id: BenutzerId,
        erstellt_am: DateTime<Utc>,
    ) -> Self {
        let dedupe_key =
            NonEmptyString::neu(BenachrichtigungArt::SpielBeendet.dedupe_key(game_id, None))
                .expect("static prefix makes dedupe key non-empty");
        Self {
            empfänger_id,
            art: BenachrichtigungArt::SpielBeendet,
            game_id,
            capture_id: None,
            actor_id: Some(winner_id),
            target_id: None,
            dedupe_key,
            erstellt_am,
        }
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
        erstellt_am: DateTime<Utc>,
        gelesen_am: Option<DateTime<Utc>>,
    ) -> ResultReport<Self, BenachrichtigungFehler> {
        let art = art.parse().map_err(Report::new).attach(CONSTRUCTING)?;
        let dedupe_key = NonEmptyString::neu(dedupe_key)
            .change_context(BenachrichtigungFehler::UngültigerDedupeSchlüssel)
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

    pub fn erstellt_am(&self) -> DateTime<Utc> {
        self.erstellt_am
    }

    pub fn gelesen_am(&self) -> Option<DateTime<Utc>> {
        self.gelesen_am
    }

    pub fn ist_ungelesen(&self) -> bool {
        self.gelesen_am.is_none()
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
    pub aktiv: bool,
    pub aktualisiert_am: DateTime<Utc>,
}

impl PreferenceToggle {
    pub fn neu(aktiv: bool, aktualisiert_am: DateTime<Utc>) -> Self {
        Self {
            aktiv,
            aktualisiert_am,
        }
    }

    pub fn setze(&mut self, aktiv: bool, jetzt: DateTime<Utc>) {
        self.aktiv = aktiv;
        self.aktualisiert_am = jetzt;
    }
}

/// Per-user notification preferences: default game watch state + per-event email opt-out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenachrichtigungsEinstellungen {
    pub standard_spiel_beobachten: PreferenceToggle,
    pub email_spiel_gestartet: PreferenceToggle,
    pub email_aufnahme_hochgeladen: PreferenceToggle,
    pub email_spiel_beendet: PreferenceToggle,
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

/// Signed, expiring one-click email unsubscribe token scoped to one user and one event kind.
pub struct AbbestellToken;

impl AbbestellToken {
    pub fn erzeugen(
        benutzer_id: BenutzerId,
        art: BenachrichtigungArt,
        jetzt: DateTime<Utc>,
        signing: &dyn Signing,
    ) -> String {
        let exp = (jetzt + UNSUBSCRIBE_TOKEN_LIFETIME).timestamp();
        let payload = format!(
            "{UNSUBSCRIBE_DOMAIN_PREFIX}:{}:{}:{exp}",
            benutzer_id.0,
            art.as_str()
        );
        let signature = signing.sign(payload.as_bytes());
        format!(
            "{}.{}",
            hex::encode(payload.as_bytes()),
            hex::encode(signature.as_bytes())
        )
    }

    pub fn prüfen(
        raw: &str,
        signing: &dyn Signing,
        jetzt: DateTime<Utc>,
    ) -> ResultReport<(BenutzerId, BenachrichtigungArt), BenachrichtigungFehler> {
        let (payload_hex, sig_hex) = raw
            .trim()
            .split_once('.')
            .ok_or_else(|| Report::new(BenachrichtigungFehler::UngültigesAbbestellToken))?;

        let sig_bytes = hex::decode(sig_hex)
            .map_err(|_| Report::new(BenachrichtigungFehler::UngültigesAbbestellToken))?;
        let signature = Signature::from_bytes(&sig_bytes)
            .map_err(|_| Report::new(BenachrichtigungFehler::UngültigesAbbestellToken))?;
        let payload_bytes = hex::decode(payload_hex)
            .map_err(|_| Report::new(BenachrichtigungFehler::UngültigesAbbestellToken))?;

        signing
            .verify(&payload_bytes, &signature)
            .change_context(BenachrichtigungFehler::UngültigesAbbestellToken)?;

        let payload = std::str::from_utf8(&payload_bytes)
            .map_err(|_| Report::new(BenachrichtigungFehler::UngültigesAbbestellToken))?;

        let parts: Vec<&str> = payload.split(':').collect();
        if parts.len() != 6
            || format!("{}:{}:{}", parts[0], parts[1], parts[2]) != UNSUBSCRIBE_DOMAIN_PREFIX
        {
            return Err(Report::new(
                BenachrichtigungFehler::UngültigesAbbestellToken,
            ));
        }

        let user_uuid = Uuid::parse_str(parts[3])
            .map_err(|_| Report::new(BenachrichtigungFehler::UngültigesAbbestellToken))?;
        let art: BenachrichtigungArt = parts[4]
            .parse()
            .map_err(|_| Report::new(BenachrichtigungFehler::UngültigesAbbestellToken))?;
        let exp_ts: i64 = parts[5]
            .parse()
            .map_err(|_| Report::new(BenachrichtigungFehler::UngültigesAbbestellToken))?;

        if jetzt.timestamp() >= exp_ts {
            return Err(Report::new(
                BenachrichtigungFehler::UngültigesAbbestellToken,
            ));
        }

        Ok((BenutzerId(user_uuid), art))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::InMemorySigning;

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

        let mut notif = Benachrichtigung::neu(
            id,
            NeueBenachrichtigung::aufnahme_hochgeladen(
                user_id,
                game_id,
                capture_id,
                catcher_id,
                user_id,
                jetzt(),
            ),
        );
        assert!(notif.ist_ungelesen());

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
            notif.erstellt_am(),
            notif.gelesen_am(),
        )
        .unwrap();
        assert_eq!(notif, reloaded);
    }

    #[test_log::test]
    fn abbestell_token_roundtrip_and_rejects_tamper_expiry_wrong_key() {
        let signing = InMemorySigning::from_secret_bytes(&[7u8; 32]);
        let other = InMemorySigning::from_secret_bytes(&[9u8; 32]);
        let uid = BenutzerId(Uuid::new_v4());

        let token = AbbestellToken::erzeugen(
            uid,
            BenachrichtigungArt::AufnahmeHochgeladen,
            jetzt(),
            &signing,
        );

        let (parsed_uid, parsed_art) =
            AbbestellToken::prüfen(&token, &signing, jetzt() + Duration::days(1)).unwrap();
        assert_eq!(parsed_uid, uid);
        assert_eq!(parsed_art, BenachrichtigungArt::AufnahmeHochgeladen);

        // Wrong key fails
        assert!(matches!(
            AbbestellToken::prüfen(&token, &other, jetzt())
                .unwrap_err()
                .current_context(),
            BenachrichtigungFehler::UngültigesAbbestellToken
        ));

        // Expired fails
        assert!(matches!(
            AbbestellToken::prüfen(&token, &signing, jetzt() + UNSUBSCRIBE_TOKEN_LIFETIME)
                .unwrap_err()
                .current_context(),
            BenachrichtigungFehler::UngültigesAbbestellToken
        ));

        // Tampered payload fails
        let mut bytes = token.into_bytes();
        bytes[0] = if bytes[0] == b'a' { b'b' } else { b'a' };
        let tampered = String::from_utf8(bytes).unwrap();
        assert!(matches!(
            AbbestellToken::prüfen(&tampered, &signing, jetzt())
                .unwrap_err()
                .current_context(),
            BenachrichtigungFehler::UngültigesAbbestellToken
        ));
    }

    #[test_log::test]
    fn einstellungen_update_tracks_only_changed_toggle() {
        let mut prefs = BenachrichtigungsEinstellungen::standard(jetzt());
        assert!(prefs.standard_spiel_beobachten().aktiv);
        assert!(prefs.email_erlaubt(BenachrichtigungArt::SpielGestartet));
        assert!(prefs.email_erlaubt(BenachrichtigungArt::AufnahmeHochgeladen));
        assert!(prefs.email_erlaubt(BenachrichtigungArt::SpielBeendet));

        let later = jetzt() + Duration::hours(2);
        prefs.setze_email(BenachrichtigungArt::AufnahmeHochgeladen, false, later);
        assert!(!prefs.email_erlaubt(BenachrichtigungArt::AufnahmeHochgeladen));
        assert_eq!(prefs.email_aufnahme_hochgeladen().aktualisiert_am, later);
        assert_eq!(prefs.email_spiel_beendet().aktualisiert_am, jetzt());
    }
}
