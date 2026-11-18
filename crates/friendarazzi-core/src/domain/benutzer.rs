//! Benutzer aggregate: core user identity, profile, avatar processing, Gold Medals, and rank tiers.

use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    ResultReport,
    domain::{
        EmailAdresse, NonEmptyString,
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

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("Profilangabe darf nicht leer sein")]
    EmptyField,
    #[error("Dieser Titel wurde noch nicht freigeschaltet")]
    TitleLocked,
    #[error("ungültige Avatar-Bilddatei")]
    InvalidAvatar,
}

/// Creation input: everything of a Benutzer except its identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeuerBenutzer {
    email_address: EmailAdresse,
    display_name: NonEmptyString,
    registered_at: DateTime<Utc>,
}

impl NeuerBenutzer {
    pub fn neu(
        email_address: EmailAdresse,
        display_name: NonEmptyString,
        registered_at: DateTime<Utc>,
    ) -> Self {
        Self {
            email_address,
            display_name,
            registered_at,
        }
    }

    pub fn registrierung(email_address: EmailAdresse, registered_at: DateTime<Utc>) -> Self {
        let display_name = default_display_name_from_email(&email_address);
        Self::neu(email_address, display_name, registered_at)
    }

    pub fn email_address(&self) -> &EmailAdresse {
        &self.email_address
    }

    pub fn display_name(&self) -> &NonEmptyString {
        &self.display_name
    }

    pub fn registered_at(&self) -> DateTime<Utc> {
        self.registered_at
    }
}

/// The logical identity and player profile the product knows: one person's account.
///
/// Created only by a completed registration (SPEC-konto rule 3), so there is no
/// unconfirmed state to represent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Benutzer {
    id: BenutzerId,
    email_address: EmailAdresse,
    display_name: NonEmptyString,
    avatar_storage_key: Option<String>,
    gold_medal_count: u32,
    equipped_title: Option<String>,
    registered_at: DateTime<Utc>,
}

impl Benutzer {
    /// Fresh Benutzer with a caller-provided identity.
    pub fn neu(id: BenutzerId, neu: NeuerBenutzer) -> Self {
        Self {
            id,
            email_address: neu.email_address,
            display_name: neu.display_name,
            avatar_storage_key: None,
            gold_medal_count: 0,
            equipped_title: None,
            registered_at: neu.registered_at,
        }
    }

    /// Rebuild a persisted Benutzer.
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: BenutzerId,
        email_address: EmailAdresse,
        display_name: String,
        avatar_storage_key: Option<String>,
        gold_medal_count: u32,
        equipped_title: Option<String>,
        registered_at: DateTime<Utc>,
    ) -> ResultReport<Self, ProfileError> {
        let display_name = NonEmptyString::neu(display_name)
            .change_context(ProfileError::EmptyField)
            .attach(CONSTRUCTING)?;
        Ok(Self {
            id,
            email_address,
            display_name,
            avatar_storage_key,
            gold_medal_count,
            equipped_title,
            registered_at,
        })
    }

    pub fn id(&self) -> BenutzerId {
        self.id
    }

    pub fn email_address(&self) -> &EmailAdresse {
        &self.email_address
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

    pub fn registered_at(&self) -> DateTime<Utc> {
        self.registered_at
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
        display_name: NonEmptyString,
        equipped_title: Option<NonEmptyString>,
    ) -> ResultReport<(), ProfileError> {
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

fn default_display_name_from_email(email: &EmailAdresse) -> NonEmptyString {
    let local = email.as_ref().split('@').next().unwrap_or("Spieler");
    NonEmptyString::neu(local)
        .unwrap_or_else(|_| NonEmptyString::neu("Spieler").expect("non-empty"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::media::tests::png_bytes_for_test;

    fn jetzt() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn email(raw: &str) -> EmailAdresse {
        EmailAdresse::normalisiert(raw).unwrap()
    }

    #[test_log::test]
    fn registrierung_setzt_anzeigename_und_registrierungszeitpunkt() {
        let benutzer = Benutzer::neu(
            BenutzerId(Uuid::nil()),
            NeuerBenutzer::registrierung(email("anna@example.at"), jetzt()),
        );
        assert_eq!(benutzer.display_name().as_str(), "anna");
        assert_eq!(benutzer.email_address().as_ref(), "anna@example.at");
        assert_eq!(benutzer.registered_at(), jetzt());
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
            email("anna@example.at"),
            NonEmptyString::neu("Anna").unwrap(),
            jetzt(),
        );
        let original = Benutzer::neu(BenutzerId(Uuid::nil()), neu);
        let geladen = Benutzer::from_parts(
            original.id(),
            original.email_address().clone(),
            original.display_name().as_str().to_owned(),
            original.avatar_storage_key().map(str::to_owned),
            original.gold_medal_count(),
            original.equipped_title().map(str::to_owned),
            original.registered_at(),
        )
        .unwrap();
        assert_eq!(original, geladen);
    }
}
