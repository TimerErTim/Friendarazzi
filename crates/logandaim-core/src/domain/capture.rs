//! Photo captures and post-game superlatives.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use error_stack::ResultExt;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    ResultReport,
    domain::{NonEmptyString, board::BoardTileId, game::GameId, konto::BenutzerId},
};

const CONSTRUCTING: &str = "while constructing capture";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CaptureId(pub Uuid);

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("Kacheltext darf nicht leer sein")]
    EmptyPromptText,
    #[error("Speicherschlüssel darf nicht leer sein")]
    EmptyStorageKey,
}

/// A candid photo proof captured for a tile during a game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    id: CaptureId,
    game_id: GameId,
    tile_id: BoardTileId,
    catcher_user_id: BenutzerId,
    target_user_id: Option<BenutzerId>,
    prompt_text: NonEmptyString,
    image_storage_key: String,
    thumbnail_storage_key: String,
    captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCapture {
    pub game_id: GameId,
    pub tile_id: BoardTileId,
    pub catcher_user_id: BenutzerId,
    pub target_user_id: Option<BenutzerId>,
    pub prompt_text: NonEmptyString,
    pub image_storage_key: String,
    pub thumbnail_storage_key: String,
    pub captured_at: DateTime<Utc>,
}

impl Capture {
    pub fn new(id: CaptureId, input: NewCapture) -> Self {
        Self {
            id,
            game_id: input.game_id,
            tile_id: input.tile_id,
            catcher_user_id: input.catcher_user_id,
            target_user_id: input.target_user_id,
            prompt_text: input.prompt_text,
            image_storage_key: input.image_storage_key,
            thumbnail_storage_key: input.thumbnail_storage_key,
            captured_at: input.captured_at,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: CaptureId,
        game_id: GameId,
        tile_id: BoardTileId,
        catcher_user_id: BenutzerId,
        target_user_id: Option<BenutzerId>,
        prompt_text: String,
        image_storage_key: String,
        thumbnail_storage_key: String,
        captured_at: DateTime<Utc>,
    ) -> ResultReport<Self, CaptureError> {
        let prompt_text = NonEmptyString::neu(prompt_text)
            .change_context(CaptureError::EmptyPromptText)
            .attach(CONSTRUCTING)?;
        if image_storage_key.trim().is_empty() || thumbnail_storage_key.trim().is_empty() {
            return Err(
                error_stack::Report::new(CaptureError::EmptyStorageKey).attach(CONSTRUCTING)
            );
        }
        Ok(Self {
            id,
            game_id,
            tile_id,
            catcher_user_id,
            target_user_id,
            prompt_text,
            image_storage_key,
            thumbnail_storage_key,
            captured_at,
        })
    }

    pub fn id(&self) -> CaptureId {
        self.id
    }

    pub fn game_id(&self) -> GameId {
        self.game_id
    }

    pub fn tile_id(&self) -> BoardTileId {
        self.tile_id
    }

    pub fn catcher_user_id(&self) -> BenutzerId {
        self.catcher_user_id
    }

    pub fn target_user_id(&self) -> Option<BenutzerId> {
        self.target_user_id
    }

    pub fn prompt_text(&self) -> &NonEmptyString {
        &self.prompt_text
    }

    pub fn image_storage_key(&self) -> &str {
        &self.image_storage_key
    }

    pub fn thumbnail_storage_key(&self) -> &str {
        &self.thumbnail_storage_key
    }

    pub fn captured_at(&self) -> DateTime<Utc> {
        self.captured_at
    }
}

/// Automated post-game award derived from a completed game's captures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Superlative {
    pub kind: SuperlativeKind,
    pub user_id: BenutzerId,
    pub metric_value: i64,
    pub capture_id: Option<CaptureId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuperlativeKind {
    MostWanted,
    Ghost,
    QuickestDraw,
    Sniper,
}

/// Compute post-game superlatives from `players` and chronological `captures`.
pub fn compute_superlatives(
    players: &[BenutzerId],
    captures: &[Capture],
    started_at: Option<DateTime<Utc>>,
) -> Vec<Superlative> {
    if players.is_empty() || captures.is_empty() {
        return Vec::new();
    }

    let mut targeted_counts: HashMap<BenutzerId, i64> =
        players.iter().copied().map(|id| (id, 0)).collect();
    let mut by_catcher: HashMap<BenutzerId, Vec<DateTime<Utc>>> = HashMap::new();

    for cap in captures {
        if let Some(target) = cap.target_user_id() {
            *targeted_counts.entry(target).or_insert(0) += 1;
        }
        by_catcher
            .entry(cap.catcher_user_id())
            .or_default()
            .push(cap.captured_at());
    }

    let mut out = Vec::new();

    // Most Wanted (most caught)
    if let Some((&user_id, &count)) = targeted_counts
        .iter()
        .max_by(|(a_id, a_c), (b_id, b_c)| a_c.cmp(b_c).then_with(|| b_id.cmp(a_id)))
        && count > 0
    {
        out.push(Superlative {
            kind: SuperlativeKind::MostWanted,
            user_id,
            metric_value: count,
            capture_id: None,
        });
    }

    // Ghost (least caught)
    if let Some((&user_id, &count)) = targeted_counts
        .iter()
        .min_by(|(a_id, a_c), (b_id, b_c)| a_c.cmp(b_c).then_with(|| a_id.cmp(b_id)))
    {
        out.push(Superlative {
            kind: SuperlativeKind::Ghost,
            user_id,
            metric_value: count,
            capture_id: None,
        });
    }

    // Quickest Draw (fastest capture after game start)
    if let Some(fastest) = captures.iter().min_by_key(|c| c.captured_at()) {
        let elapsed = started_at
            .map(|s| (fastest.captured_at() - s).num_seconds().max(0))
            .unwrap_or(0);
        out.push(Superlative {
            kind: SuperlativeKind::QuickestDraw,
            user_id: fastest.catcher_user_id(),
            metric_value: elapsed,
            capture_id: Some(fastest.id()),
        });
    }

    // Sniper (most captures within any rolling 60-minute window)
    let mut best_sniper: Option<(BenutzerId, i64)> = None;
    for (catcher, mut times) in by_catcher {
        times.sort();
        let mut left = 0usize;
        let mut max_window = 0i64;
        for right in 0..times.len() {
            while times[right]
                .signed_duration_since(times[left])
                .num_seconds()
                > 3600
            {
                left += 1;
            }
            max_window = max_window.max((right - left + 1) as i64);
        }
        match best_sniper {
            Some((_, best_count)) if best_count >= max_window => {}
            _ => best_sniper = Some((catcher, max_window)),
        }
    }
    if let Some((user_id, count)) = best_sniper {
        out.push(Superlative {
            kind: SuperlativeKind::Sniper,
            user_id,
            metric_value: count,
            capture_id: None,
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    fn mk_capture(catcher: BenutzerId, target: Option<BenutzerId>, at: DateTime<Utc>) -> Capture {
        Capture::new(
            CaptureId(Uuid::new_v4()),
            NewCapture {
                game_id: GameId(Uuid::nil()),
                tile_id: BoardTileId(Uuid::new_v4()),
                catcher_user_id: catcher,
                target_user_id: target,
                prompt_text: NonEmptyString::neu("Test").unwrap(),
                image_storage_key: "capture_a.jpg".into(),
                thumbnail_storage_key: "thumb_a.jpg".into(),
                captured_at: at,
            },
        )
    }

    #[test_log::test]
    fn superlatives_compute_most_wanted_ghost_quickest_and_sniper() {
        let p1 = BenutzerId(Uuid::from_u128(1));
        let p2 = BenutzerId(Uuid::from_u128(2));
        let p3 = BenutzerId(Uuid::from_u128(3));
        let start = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

        let captures = vec![
            mk_capture(p1, Some(p2), start + Duration::seconds(45)),
            mk_capture(p1, Some(p2), start + Duration::minutes(10)),
            mk_capture(p3, Some(p1), start + Duration::minutes(120)),
        ];

        let sups = compute_superlatives(&[p1, p2, p3], &captures, Some(start));
        assert!(sups.iter().any(|s| s.kind == SuperlativeKind::MostWanted
            && s.user_id == p2
            && s.metric_value == 2));
        assert!(
            sups.iter().any(|s| s.kind == SuperlativeKind::Ghost
                && s.user_id == p3
                && s.metric_value == 0)
        );
        assert!(sups.iter().any(|s| s.kind == SuperlativeKind::QuickestDraw
            && s.user_id == p1
            && s.metric_value == 45));
        assert!(
            sups.iter().any(|s| s.kind == SuperlativeKind::Sniper
                && s.user_id == p1
                && s.metric_value == 2)
        );
    }
}
