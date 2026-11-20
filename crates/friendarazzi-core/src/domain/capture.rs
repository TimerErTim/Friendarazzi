//! Photo captures, capture media normalization, visibility predicates, and post-game awards.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use error_stack::ResultExt;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    ResultReport,
    domain::{
        BenutzerId, NonEmptyString,
        board::BoardTileId,
        game::{Game, GameId, VisibilityMode},
        media::{
            MediaProcessingError, decode_image_within_limit, encode_webp, pixelate_and_blur,
            resize_max_edge,
        },
    },
};

const CONSTRUCTING: &str = "while constructing capture";
const MAX_CAPTURE_EDGE: u32 = 1600;
const BLUR_SMALL_EDGE: u32 = 14;
const BLUR_OUTPUT_EDGE: u32 = 64;
const MAX_CAPTURE_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CaptureId(pub Uuid);

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("Kacheltext darf nicht leer sein")]
    EmptyPromptText,
    #[error("Speicherschlüssel darf nicht leer sein")]
    EmptyStorageKey,
}

/// Normalized full WebP photo and 64px blurred teaser WebP for a capture.
pub struct ProcessedCaptureMedia {
    pub full_webp: Vec<u8>,
    pub blurred_webp: Vec<u8>,
}

/// A candid photo proof captured for a tile during a game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    id: CaptureId,
    game_id: GameId,
    tile_id: BoardTileId,
    catcher_user_id: BenutzerId,
    target_user_id: BenutzerId,
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
    pub target_user_id: BenutzerId,
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
        target_user_id: BenutzerId,
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

    /// Normalize a capture photo to max 1600px WebP and generate a 64px pixelated+blurred teaser WebP.
    pub fn process_photo(bytes: &[u8]) -> Result<ProcessedCaptureMedia, MediaProcessingError> {
        let img = decode_image_within_limit(bytes, MAX_CAPTURE_BYTES)?;
        let full = resize_max_edge(&img, MAX_CAPTURE_EDGE);
        let full_webp = encode_webp(&full)?;

        let blurred = pixelate_and_blur(&img, BLUR_SMALL_EDGE, BLUR_OUTPUT_EDGE, 2.2);
        let blurred_webp = encode_webp(&blurred)?;

        Ok(ProcessedCaptureMedia {
            full_webp,
            blurred_webp,
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

    pub fn target_user_id(&self) -> BenutzerId {
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

/// Domain service predicate: whether `viewer` may see the unredacted photo and prompt for `capture`.
pub fn can_view_full_capture(game: &Game, capture: &Capture, viewer: BenutzerId) -> bool {
    if !game.has_player(viewer) {
        return false;
    }
    matches!(game, Game::Completed(_))
        || game.visibility_mode() == VisibilityMode::LiveFeed
        || capture.catcher_user_id() == viewer
}

/// Domain service predicate: whether `viewer` may see the blurred teaser thumbnail for `capture`.
pub fn can_view_blurred_capture(game: &Game, capture: &Capture, viewer: BenutzerId) -> bool {
    if !game.has_player(viewer) {
        return false;
    }
    can_view_full_capture(game, capture, viewer)
        || game.visibility_mode() == VisibilityMode::BlurredTeaser
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MostWantedAward {
    pub user_id: BenutzerId,
    pub metric_value: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostAward {
    pub user_id: BenutzerId,
    pub metric_value: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickestDrawAward {
    pub user_id: BenutzerId,
    pub metric_value: i64,
    pub capture_id: CaptureId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SniperAward {
    pub user_id: BenutzerId,
    pub metric_value: i64,
}

/// Explicit post-game awards calculated for a completed game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameAwards {
    pub most_wanted: MostWantedAward,
    pub ghost: GhostAward,
    pub quickest_draw: QuickestDrawAward,
    pub sniper: SniperAward,
}

/// Compute post-game awards from `players` and chronological `captures`.
pub fn compute_game_awards(
    players: &[BenutzerId],
    captures: &[Capture],
    started_at: DateTime<Utc>,
) -> Option<GameAwards> {
    if players.is_empty() || captures.is_empty() {
        return None;
    }

    let mut targeted_counts: HashMap<BenutzerId, i64> =
        players.iter().copied().map(|id| (id, 0)).collect();
    let mut by_catcher: HashMap<BenutzerId, Vec<DateTime<Utc>>> = HashMap::new();

    for cap in captures {
        *targeted_counts.entry(cap.target_user_id()).or_insert(0) += 1;
        by_catcher
            .entry(cap.catcher_user_id())
            .or_default()
            .push(cap.captured_at());
    }

    let most_wanted = targeted_counts
        .iter()
        .max_by(|(a_id, a_c), (b_id, b_c)| a_c.cmp(b_c).then_with(|| b_id.cmp(a_id)))
        .map(|(&user_id, &metric_value)| MostWantedAward {
            user_id,
            metric_value,
        })
        .unwrap();

    let ghost = targeted_counts
        .iter()
        .min_by(|(a_id, a_c), (b_id, b_c)| a_c.cmp(b_c).then_with(|| a_id.cmp(b_id)))
        .map(|(&user_id, &metric_value)| GhostAward {
            user_id,
            metric_value,
        })
        .unwrap();

    let quickest_draw = captures
        .iter()
        .min_by_key(|c| c.captured_at())
        .map(|fastest| QuickestDrawAward {
            user_id: fastest.catcher_user_id(),
            metric_value: (fastest.captured_at() - started_at).num_seconds().max(0),
            capture_id: fastest.id(),
        })
        .unwrap();

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
    let (sniper_user_id, sniper_metric_value) = best_sniper.unwrap();
    let sniper = SniperAward {
        user_id: sniper_user_id,
        metric_value: sniper_metric_value,
    };

    Some(GameAwards {
        most_wanted,
        ghost,
        quickest_draw,
        sniper,
    })
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;
    use crate::domain::media::tests::{
        QuadrantColor::{Blue as B, Green as G, Red as R, Yellow as Y},
        assert_no_exif_metadata, assert_quadrants, encode_with_exif, exif_tiff_chunk,
        png_bytes_for_test, quadrant_image_for_test,
    };

    fn mk_capture(catcher: BenutzerId, target: BenutzerId, at: DateTime<Utc>) -> Capture {
        Capture::new(
            CaptureId(Uuid::new_v4()),
            NewCapture {
                game_id: GameId(Uuid::nil()),
                tile_id: BoardTileId(Uuid::new_v4()),
                catcher_user_id: catcher,
                target_user_id: target,
                prompt_text: NonEmptyString::neu("Test").unwrap(),
                image_storage_key: "capture_a.webp".into(),
                thumbnail_storage_key: "thumb_a.webp".into(),
                captured_at: at,
            },
        )
    }

    #[test_log::test]
    fn processes_large_image_into_bounded_webp_and_64px_blur() {
        let raw = png_bytes_for_test(2400, 1200);
        let processed = Capture::process_photo(&raw).unwrap();

        let full = image::load_from_memory(&processed.full_webp).unwrap();
        assert_eq!(full.width(), 1600);
        assert_eq!(full.height(), 800);

        let teaser = image::load_from_memory(&processed.blurred_webp).unwrap();
        assert_eq!(teaser.width(), 64);
        assert_eq!(teaser.height(), 64);
    }

    #[test_log::test]
    fn process_photo_applies_exif_orientation_and_removes_exif() {
        let src = quadrant_image_for_test(2400, 1200);
        let raw = encode_with_exif(&src, image::ImageFormat::Jpeg, exif_tiff_chunk(6, false));
        let processed = Capture::process_photo(&raw).unwrap();

        assert_no_exif_metadata(&processed.full_webp);
        assert_no_exif_metadata(&processed.blurred_webp);

        let full = image::load_from_memory(&processed.full_webp).unwrap();
        assert_eq!((full.width(), full.height()), (800, 1600));
        assert_quadrants(&full, [B, R, Y, G]);

        let teaser = image::load_from_memory(&processed.blurred_webp).unwrap();
        assert_eq!((teaser.width(), teaser.height()), (64, 64));
    }

    #[test_log::test]
    fn awards_return_none_when_players_or_captures_are_empty() {
        let p1 = BenutzerId(Uuid::from_u128(1));
        let start = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

        assert_eq!(compute_game_awards(&[], &[], start), None);
        assert_eq!(compute_game_awards(&[p1], &[], start), None);
        assert_eq!(
            compute_game_awards(&[], &[mk_capture(p1, p1, start)], start),
            None
        );
    }

    #[test_log::test]
    fn awards_compute_most_wanted_ghost_quickest_and_sniper() {
        let p1 = BenutzerId(Uuid::from_u128(1));
        let p2 = BenutzerId(Uuid::from_u128(2));
        let p3 = BenutzerId(Uuid::from_u128(3));
        let start = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

        let captures = vec![
            mk_capture(p1, p2, start + Duration::seconds(45)),
            mk_capture(p1, p2, start + Duration::minutes(10)),
            mk_capture(p3, p1, start + Duration::minutes(120)),
        ];

        let awards = compute_game_awards(&[p1, p2, p3], &captures, start).unwrap();
        assert_eq!(
            awards.most_wanted,
            MostWantedAward {
                user_id: p2,
                metric_value: 2
            }
        );
        assert_eq!(
            awards.ghost,
            GhostAward {
                user_id: p3,
                metric_value: 0
            }
        );
        assert_eq!(awards.quickest_draw.user_id, p1);
        assert_eq!(awards.quickest_draw.metric_value, 45);
        assert_eq!(
            awards.sniper,
            SniperAward {
                user_id: p1,
                metric_value: 2
            }
        );
    }
}
