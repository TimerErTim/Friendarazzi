//! User profiles, Gold Medals, and rank title progression.

use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use thiserror::Error;

use crate::{
    ResultReport,
    domain::{EmailAdresse, NonEmptyString, konto::BenutzerId},
};

const CONSTRUCTING: &str = "while constructing user profile";

/// Ordered rank tiers unlocked by cumulative Gold Medals.
pub const RANK_TIERS: &[(u32, &str)] = &[
    (0, "Neuling"),
    (1, "Schnappschuss-Jäger"),
    (3, "Paparazzo"),
    (5, "Paparazzi King"),
    (10, "Grandmaster"),
];

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("Profilangabe darf nicht leer sein")]
    EmptyField,
    #[error("Dieser Titel wurde noch nicht freigeschaltet")]
    TitleLocked,
    #[error("Benutzername ist ungültig")]
    InvalidUsername,
}

/// Creation input for a new player's profile and medal counter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewUserProfile {
    pub user_id: BenutzerId,
    pub username: NonEmptyString,
    pub display_name: NonEmptyString,
    pub created_at: DateTime<Utc>,
}

impl NewUserProfile {
    pub fn from_email(
        user_id: BenutzerId,
        email: &EmailAdresse,
        created_at: DateTime<Utc>,
    ) -> Self {
        let base = default_username_from_email(email);
        Self {
            user_id,
            username: base.clone(),
            display_name: base,
            created_at,
        }
    }
}

/// Persistent player profile and Gold Medal prestige state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProfile {
    user_id: BenutzerId,
    username: NonEmptyString,
    display_name: NonEmptyString,
    avatar_storage_key: Option<String>,
    gold_medal_count: u32,
    equipped_title: Option<String>,
    created_at: DateTime<Utc>,
}

impl UserProfile {
    pub fn new(input: NewUserProfile) -> Self {
        Self {
            user_id: input.user_id,
            username: input.username,
            display_name: input.display_name,
            avatar_storage_key: None,
            gold_medal_count: 0,
            equipped_title: None,
            created_at: input.created_at,
        }
    }

    pub fn from_parts(
        user_id: BenutzerId,
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
            user_id,
            username,
            display_name,
            avatar_storage_key,
            gold_medal_count,
            equipped_title,
            created_at,
        })
    }

    pub fn user_id(&self) -> BenutzerId {
        self.user_id
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
        username: String,
        display_name: String,
        equipped_title: Option<String>,
    ) -> ResultReport<(), ProfileError> {
        let trimmed_user = username.trim();
        if trimmed_user.len() < 2
            || trimmed_user.len() > 32
            || !trimmed_user
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(Report::new(ProfileError::InvalidUsername));
        }
        let username =
            NonEmptyString::neu(trimmed_user).change_context(ProfileError::EmptyField)?;
        let display_name =
            NonEmptyString::neu(display_name).change_context(ProfileError::EmptyField)?;

        let normalized_title = match equipped_title.map(|t| t.trim().to_owned()) {
            Some(t) if !t.is_empty() => {
                let unlocked = self.unlocked_titles();
                if !unlocked.iter().any(|allowed| *allowed == t.as_str()) {
                    return Err(Report::new(ProfileError::TitleLocked));
                }
                Some(t)
            }
            _ => None,
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
    use uuid::Uuid;

    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[test_log::test]
    fn medal_progression_unlocks_titles_and_equips() {
        let email = EmailAdresse::normalisiert("tim.peko@example.at").unwrap();
        let mut profile = UserProfile::new(NewUserProfile::from_email(
            BenutzerId(Uuid::nil()),
            &email,
            now(),
        ));
        assert_eq!(profile.gold_medal_count(), 0);
        assert_eq!(profile.effective_title(), "Neuling");
        assert!(
            profile
                .update_details("tim".into(), "Tim".into(), Some("Paparazzi King".into()))
                .is_err()
        );

        for _ in 0..5 {
            profile.award_gold_medal();
        }
        assert_eq!(profile.gold_medal_count(), 5);
        assert_eq!(profile.effective_title(), "Paparazzi King");
        assert!(
            profile
                .update_details("tim".into(), "Tim P.".into(), Some("Paparazzo".into()))
                .is_ok()
        );
        assert_eq!(profile.effective_title(), "Paparazzo");
    }

    #[test_log::test]
    fn from_parts_roundtrip() {
        let email = EmailAdresse::normalisiert("anna@example.at").unwrap();
        let original = UserProfile::new(NewUserProfile::from_email(
            BenutzerId(Uuid::new_v4()),
            &email,
            now(),
        ));
        let loaded = UserProfile::from_parts(
            original.user_id(),
            original.username().as_str().to_owned(),
            original.display_name().as_str().to_owned(),
            original.avatar_storage_key().map(str::to_owned),
            original.gold_medal_count(),
            original.equipped_title().map(str::to_owned),
            original.created_at(),
        )
        .unwrap();
        assert_eq!(original, loaded);
    }
}
