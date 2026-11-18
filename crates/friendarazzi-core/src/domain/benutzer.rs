//! Benutzer aggregate: core user identity, profile, avatar processing, Gold Medals, and rank tiers.

use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    ResultReport,
    domain::{
        EmailAdresse, NonEmptyString,
        konto::KontoFehler,
        media::{MediaProcessingError, decode_image_within_limit, encode_webp, resize_square},
    },
};

const CONSTRUCTING: &str = "while constructing benutzer";
const AVATAR_EDGE: u32 = 400;
const MAX_AVATAR_BYTES: usize = 10 * 1024 * 1024;

/// Ordered rank tiers unlocked by cumulative Gold Medals.
pub const RANK_TIERS: &[(u32, &str)] = &[
    (0, "Neuling"),
    (1, "Schnappschuss-Jäger"),
    (3, "Paparazzo"),
    (5, "Paparazzi King"),
    (10, "Grandmaster"),
];

/// Identity of one Benutzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BenutzerId(pub Uuid);

/// Account-level authority of a Benutzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Rolle {
    /// Every registered person.
    #[default]
    Benutzer,
    /// Spans the product; provisioned out of band, never by an API path.
    Systemadministrator,
}

impl Rolle {
    /// Single stored spelling, shared by every adapter.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Benutzer => "Benutzer",
            Self::Systemadministrator => "Systemadministrator",
        }
    }
}

impl std::str::FromStr for Rolle {
    type Err = KontoFehler;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Benutzer" => Ok(Self::Benutzer),
            "Systemadministrator" => Ok(Self::Systemadministrator),
            _ => Err(KontoFehler::RolleUnbekannt),
        }
    }
}

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("Profilangabe darf nicht leer sein")]
    EmptyField,
    #[error("Dieser Titel wurde noch nicht freigeschaltet")]
    TitleLocked,
    #[error("Benutzername ist ungültig")]
    InvalidUsername,
    #[error("ungültige Avatar-Bilddatei")]
    InvalidAvatar,
}

/// Creation input: everything of a Benutzer except its identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeuerBenutzer {
    rolle: Rolle,
    email_address: EmailAdresse,
    username: NonEmptyString,
    display_name: NonEmptyString,
    created_at: DateTime<Utc>,
}

impl NeuerBenutzer {
    pub fn neu(
        rolle: Rolle,
        email_address: EmailAdresse,
        username: NonEmptyString,
        display_name: NonEmptyString,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            rolle,
            email_address,
            username,
            display_name,
            created_at,
        }
    }

    /// The only role a registration may produce (rule 9: no self-service elevation).
    pub fn registrierung(email_address: EmailAdresse, created_at: DateTime<Utc>) -> Self {
        let base = default_username_from_email(&email_address);
        Self::neu(
            Rolle::Benutzer,
            email_address,
            base.clone(),
            base,
            created_at,
        )
    }

    pub fn with_username(mut self, username: NonEmptyString) -> Self {
        self.username = username;
        self
    }

    pub fn rolle(&self) -> Rolle {
        self.rolle
    }

    pub fn email_address(&self) -> &EmailAdresse {
        &self.email_address
    }

    pub fn username(&self) -> &NonEmptyString {
        &self.username
    }

    pub fn display_name(&self) -> &NonEmptyString {
        &self.display_name
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

/// The logical identity and player profile the product knows: one person's account.
///
/// Created only by a completed registration (SPEC-konto rule 3), so there is no
/// unconfirmed state to represent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Benutzer {
    id: BenutzerId,
    rolle: Rolle,
    email_address: EmailAdresse,
    username: NonEmptyString,
    display_name: NonEmptyString,
    avatar_storage_key: Option<String>,
    gold_medal_count: u32,
    equipped_title: Option<String>,
    created_at: DateTime<Utc>,
}

impl Benutzer {
    /// Fresh Benutzer with a caller-provided identity.
    pub fn neu(id: BenutzerId, neu: NeuerBenutzer) -> Self {
        Self {
            id,
            rolle: neu.rolle,
            email_address: neu.email_address,
            username: neu.username,
            display_name: neu.display_name,
            avatar_storage_key: None,
            gold_medal_count: 0,
            equipped_title: None,
            created_at: neu.created_at,
        }
    }

    /// Rebuild a persisted Benutzer.
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: BenutzerId,
        rolle: Rolle,
        email_address: EmailAdresse,
        username: String,
        display_name: String,
        avatar_storage_key: Option<String>,
        gold_medal_count: u32,
        equipped_title: Option<String>,
        created_at: DateTime<Utc>,
    ) -> ResultReport<Self, ProfileError> {
        let username = NonEmptyString::neu(username)
            .change_context(ProfileError::EmptyField)
            .attach(CONSTRUCTING)?;
        let display_name = NonEmptyString::neu(display_name)
            .change_context(ProfileError::EmptyField)
            .attach(CONSTRUCTING)?;
        Ok(Self {
            id,
            rolle,
            email_address,
            username,
            display_name,
            avatar_storage_key,
            gold_medal_count,
            equipped_title,
            created_at,
        })
    }

    pub fn id(&self) -> BenutzerId {
        self.id
    }

    pub fn rolle(&self) -> Rolle {
        self.rolle
    }

    pub fn email_address(&self) -> &EmailAdresse {
        &self.email_address
    }

    pub fn username(&self) -> &NonEmptyString {
        &self.username
    }

    pub fn display_name(&self) -> &NonEmptyString {
        &self.display_name
    }

    pub fn avatar_storage_key(&self) -> Option<&str> {
        self.avatar_storage_key.as_deref()
    }

    pub fn gold_medal_count(&self) -> u32 {
        self.gold_medal_count
    }

    pub fn equipped_title(&self) -> Option<&str> {
        self.equipped_title.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn ist_systemadministrator(&self) -> bool {
        self.rolle == Rolle::Systemadministrator
    }

    /// Grant the account-level authority. The only caller is the startup bootstrap
    /// (rule 9): no API path reaches this, and the resulting row is written back with a
    /// plain `update` so optimistic concurrency applies like any other mutation.
    pub fn befördere_zu_systemadministrator(&mut self) {
        self.rolle = Rolle::Systemadministrator;
    }

    /// All rank titles unlocked by this user's Gold Medal count.
    pub fn unlocked_titles(&self) -> Vec<&'static str> {
        RANK_TIERS
            .iter()
            .filter(|(min, _)| self.gold_medal_count >= *min)
            .map(|(_, title)| *title)
            .collect()
    }

    /// Highest unlocked title, or the user's chosen equipped title if unlocked.
    pub fn effective_title(&self) -> &'static str {
        let unlocked = self.unlocked_titles();
        if let Some(equipped) = &self.equipped_title
            && let Some(found) = unlocked.iter().find(|t| **t == equipped.as_str())
        {
            return found;
        }
        unlocked.last().copied().unwrap_or("Neuling")
    }

    pub fn update_details(
        &mut self,
        username: NonEmptyString,
        display_name: NonEmptyString,
        equipped_title: Option<NonEmptyString>,
    ) -> ResultReport<(), ProfileError> {
        let trimmed_user = username.as_str();
        if trimmed_user.len() < 2
            || trimmed_user.len() > 32
            || !trimmed_user
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(Report::new(ProfileError::InvalidUsername));
        }

        let normalized_title = match equipped_title {
            Some(title) => {
                let unlocked = self.unlocked_titles();
                if !unlocked.iter().any(|allowed| *allowed == title.as_str()) {
                    return Err(Report::new(ProfileError::TitleLocked));
                }
                Some(title.into_string())
            }
            None => None,
        };

        self.username = username;
        self.display_name = display_name;
        self.equipped_title = normalized_title;
        Ok(())
    }

    pub fn set_avatar(&mut self, storage_key: Option<String>) {
        self.avatar_storage_key = storage_key;
    }

    pub fn award_gold_medal(&mut self) {
        self.gold_medal_count = self.gold_medal_count.saturating_add(1);
    }

    /// Normalize a user avatar image to a square 400x400 WebP.
    pub fn process_avatar_photo(bytes: &[u8]) -> Result<Vec<u8>, MediaProcessingError> {
        let img = decode_image_within_limit(bytes, MAX_AVATAR_BYTES)?;
        let square = resize_square(&img, AVATAR_EDGE);
        encode_webp(&square)
    }
}

/// Derive a clean initial handle from an email address.
pub fn default_username_from_email(email: &EmailAdresse) -> NonEmptyString {
    let local = email.as_ref().split('@').next().unwrap_or("player");
    let cleaned: String = local
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(24)
        .collect();
    NonEmptyString::neu(if cleaned.len() >= 2 {
        cleaned
    } else {
        "player".to_owned()
    })
    .expect("non-empty fallback")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::media::png_bytes_for_test;

    fn jetzt() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn email(raw: &str) -> EmailAdresse {
        EmailAdresse::normalisiert(raw).unwrap()
    }

    #[test_log::test]
    fn registrierung_ist_immer_ein_benutzer() {
        let benutzer = Benutzer::neu(
            BenutzerId(Uuid::nil()),
            NeuerBenutzer::registrierung(email("anna@example.at"), jetzt()),
        );
        assert_eq!(benutzer.rolle(), Rolle::Benutzer);
        assert!(!benutzer.ist_systemadministrator());
        assert_eq!(benutzer.username().as_str(), "anna");
        assert_eq!(benutzer.email_address().as_ref(), "anna@example.at");
    }

    #[test_log::test]
    fn rolle_roundtrip() {
        for rolle in [Rolle::Benutzer, Rolle::Systemadministrator] {
            assert_eq!(rolle.as_str().parse::<Rolle>().unwrap(), rolle);
        }
    }

    #[test_log::test]
    fn unbekannte_rolle_wird_abgelehnt() {
        let ergebnis = "Administrator".parse::<Rolle>();
        assert!(matches!(ergebnis, Err(KontoFehler::RolleUnbekannt)));
    }

    #[test_log::test]
    fn befoerderung_ist_eine_domain_mutation() {
        let mut benutzer = Benutzer::neu(
            BenutzerId(Uuid::nil()),
            NeuerBenutzer::registrierung(email("tim@example.at"), jetzt()),
        );
        benutzer.befördere_zu_systemadministrator();
        assert!(benutzer.ist_systemadministrator());
        assert_eq!(benutzer.created_at(), jetzt());
    }

    #[test_log::test]
    fn medal_progression_unlocks_titles_and_equips() {
        let mut benutzer = Benutzer::neu(
            BenutzerId(Uuid::nil()),
            NeuerBenutzer::registrierung(email("tim.peko@example.at"), jetzt()),
        );
        assert_eq!(benutzer.gold_medal_count(), 0);
        assert_eq!(benutzer.effective_title(), "Neuling");
        assert!(
            benutzer
                .update_details(
                    NonEmptyString::neu("tim").unwrap(),
                    NonEmptyString::neu("Tim").unwrap(),
                    Some(NonEmptyString::neu("Paparazzi King").unwrap()),
                )
                .is_err()
        );

        for _ in 0..5 {
            benutzer.award_gold_medal();
        }
        assert_eq!(benutzer.gold_medal_count(), 5);
        assert_eq!(benutzer.effective_title(), "Paparazzi King");
        assert!(
            benutzer
                .update_details(
                    NonEmptyString::neu("tim").unwrap(),
                    NonEmptyString::neu("Tim P.").unwrap(),
                    Some(NonEmptyString::neu("Paparazzo").unwrap()),
                )
                .is_ok()
        );
        assert_eq!(benutzer.effective_title(), "Paparazzo");
    }

    #[test_log::test]
    fn avatar_processing_produces_400px_square_webp() {
        let raw = png_bytes_for_test(900, 600);
        let webp = Benutzer::process_avatar_photo(&raw).unwrap();
        let decoded = image::load_from_memory(&webp).unwrap();
        assert_eq!(decoded.width(), 400);
        assert_eq!(decoded.height(), 400);
    }

    #[test_log::test]
    fn from_parts_roundtrip() {
        let neu = NeuerBenutzer::neu(
            Rolle::Systemadministrator,
            email("anna@example.at"),
            NonEmptyString::neu("anna_handle").unwrap(),
            NonEmptyString::neu("Anna").unwrap(),
            jetzt(),
        );
        let original = Benutzer::neu(BenutzerId(Uuid::nil()), neu);
        let geladen = Benutzer::from_parts(
            original.id(),
            original.rolle(),
            original.email_address().clone(),
            original.username().as_str().to_owned(),
            original.display_name().as_str().to_owned(),
            original.avatar_storage_key().map(str::to_owned),
            original.gold_medal_count(),
            original.equipped_title().map(str::to_owned),
            original.created_at(),
        )
        .unwrap();
        assert_eq!(original, geladen);
        assert!(geladen.ist_systemadministrator());
    }
}
