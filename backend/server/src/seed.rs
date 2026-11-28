//! Idempotent development data seeder for the standalone server.

use std::{collections::HashMap, io::Cursor};

use chrono::{DateTime, Duration, Utc};
use error_stack::{IntoReport, Report, ResultExt};
use friendarazzi_core::{
    App, ResultReport,
    application::{
        AuthContext, ExecutionContext,
        uow::{UnitOfWork, Versioned},
    },
    domain::{
        Benutzer, BenutzerId, Board, Capture, EmailAdresse, Game, GridSize, InviteCode,
        NeueBenachrichtigung, NeuerBenutzer, NewGame, NonEmptyString, PromptSource,
        PromptSubmission, PromptSubmissionId, READY_GRACE_PERIOD_SECONDS, StarterDeck,
        VisibilityMode, builtin_starter_decks,
        konto::{KlartextPasswort, NeuesKonto, hash_password},
        target_quotas_for_author,
    },
    ports::RepositoryError,
};
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use thiserror::Error;
use uuid::Uuid;

pub const DEV_PASSWORD: &str = "friendarazzi";

const ANCHOR_USER_EMAIL: &str = "anna@friendarazzi.dev";

#[derive(Debug, Error)]
#[error("Database seeding failed")]
pub struct SeedError;

struct SeedUserSpec {
    email: &'static str,
    display_name: &'static str,
    palette: [[u8; 3]; 4],
}

const SEED_USERS: &[SeedUserSpec] = &[
    SeedUserSpec {
        email: ANCHOR_USER_EMAIL,
        display_name: "Anna",
        palette: [[255, 229, 0], [255, 77, 141], [0, 229, 255], [20, 20, 20]],
    },
    SeedUserSpec {
        email: "ben@friendarazzi.dev",
        display_name: "Ben",
        palette: [[0, 229, 255], [255, 229, 0], [20, 20, 20], [255, 77, 141]],
    },
    SeedUserSpec {
        email: "clara@friendarazzi.dev",
        display_name: "Clara",
        palette: [[255, 77, 141], [0, 229, 255], [255, 229, 0], [20, 20, 20]],
    },
    SeedUserSpec {
        email: "david@friendarazzi.dev",
        display_name: "David",
        palette: [[34, 197, 94], [255, 229, 0], [0, 229, 255], [255, 77, 141]],
    },
];

/// Populate the database and object store with deterministic sample data if not already seeded.
pub async fn seed(app: &App) -> Result<(), Report<SeedError>> {
    app.execute_fn(AuthContext::Anonymous, async move |ctx| {
        seed_with_context(ctx).await
    })
    .await
}

async fn seed_with_context(ctx: ExecutionContext<'_>) -> ResultReport<(), SeedError> {
    let uow = ctx.enter().await.change_context(SeedError)?;
    let anchor_email = EmailAdresse::normalisiert(ANCHOR_USER_EMAIL).change_context(SeedError)?;
    match uow.benutzer().find_by_email(&anchor_email).await {
        Ok(_) => {
            tracing::info!(
                email = anchor_email.as_ref(),
                "seed data already present; skipping"
            );
            return uow.finish(Ok(()), SeedError).await;
        }
        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {}
        Err(err) => {
            return uow
                .finish(Err(err.change_context(SeedError)), SeedError)
                .await;
        }
    }

    let mut uploaded_keys = Vec::new();
    let result = populate_seed_data(&ctx, &uow, &mut uploaded_keys).await;
    let committed = uow.finish(result, SeedError).await;

    if let Err(err) = committed {
        for key in &uploaded_keys {
            if let Err(cleanup_err) = ctx.object_store().ensure_deleted(key).await {
                tracing::warn!(%key, error = %cleanup_err, "failed to clean up seeded object after rollback");
            }
        }
        return Err(err);
    }

    tracing::info!(
        users = SEED_USERS.len(),
        password = DEV_PASSWORD,
        "seeded development users, games, boards, captures, and notifications"
    );
    Ok(())
}

async fn populate_seed_data(
    ctx: &ExecutionContext<'_>,
    uow: &UnitOfWork<'_>,
    uploaded_keys: &mut Vec<String>,
) -> ResultReport<(), SeedError> {
    let now = ctx.clock().now();
    let mut users = create_seed_users(ctx, uow, now, uploaded_keys).await?;

    let mut decks = uow
        .decks()
        .find_all()
        .await
        .map_err(IntoReport::into_report)
        .change_context(SeedError)?;
    if decks.is_empty() {
        decks = builtin_starter_decks();
    }

    let night_out = find_deck(&decks, "night-out");
    let roommates = find_deck(&decks, "roommates");
    let festival = find_deck(&decks, "vacation-festival");

    let all_ids: Vec<BenutzerId> = users.iter().map(|u| u.id()).collect();
    let anna_id = all_ids[0];
    let ben_id = all_ids[1];
    let clara_id = all_ids[2];

    let completed_game = seed_completed_game(
        ctx,
        uow,
        &mut users,
        &all_ids,
        night_out,
        now,
        uploaded_keys,
    )
    .await?;

    let (active_game, ben_capture_id) =
        seed_active_game(ctx, uow, &all_ids, roommates, now, uploaded_keys).await?;

    seed_drafting_game(uow, &all_ids, festival, now).await?;
    seed_lobby_game(uow, &all_ids[..3], now).await?;

    seed_notifications(
        uow,
        anna_id,
        ben_id,
        clara_id,
        completed_game.id(),
        active_game.id(),
        ben_capture_id,
        now,
    )
    .await?;

    Ok(())
}

async fn create_seed_users(
    ctx: &ExecutionContext<'_>,
    uow: &UnitOfWork<'_>,
    now: DateTime<Utc>,
    uploaded_keys: &mut Vec<String>,
) -> ResultReport<Vec<Versioned<Benutzer>>, SeedError> {
    let password = KlartextPasswort::neu(DEV_PASSWORD).change_context(SeedError)?;
    let mut created = Vec::with_capacity(SEED_USERS.len());

    for (idx, spec) in SEED_USERS.iter().enumerate() {
        let registered_at = now - Duration::days(14 - idx as i64);
        let email = EmailAdresse::normalisiert(spec.email).change_context(SeedError)?;
        let display_name = NonEmptyString::neu(spec.display_name).change_context(SeedError)?;

        let mut user = uow
            .benutzer()
            .create(NeuerBenutzer::neu(email, display_name, registered_at))
            .await
            .map_err(IntoReport::into_report)
            .change_context(SeedError)?;

        let hashed = hash_password(&password).change_context(SeedError)?;
        uow.konten()
            .create(NeuesKonto::neu(user.id(), hashed, registered_at))
            .await
            .map_err(IntoReport::into_report)
            .change_context(SeedError)?;

        let avatar_png = render_quadrant_png(128, 128, spec.palette);
        let avatar_webp = Benutzer::process_avatar_photo(&avatar_png).change_context(SeedError)?;
        let avatar_key = format!("avatar_{}.webp", Uuid::new_v4());
        ctx.object_store()
            .put(&avatar_key, &avatar_webp)
            .await
            .change_context(SeedError)?;
        uploaded_keys.push(avatar_key.clone());

        user.set_avatar(Some(avatar_key));
        uow.benutzer()
            .update(&mut user)
            .await
            .map_err(IntoReport::into_report)
            .change_context(SeedError)?;

        created.push(user);
    }

    Ok(created)
}

async fn seed_completed_game(
    ctx: &ExecutionContext<'_>,
    uow: &UnitOfWork<'_>,
    users: &mut [Versioned<Benutzer>],
    player_ids: &[BenutzerId],
    deck: &StarterDeck,
    now: DateTime<Utc>,
    uploaded_keys: &mut Vec<String>,
) -> ResultReport<Game, SeedError> {
    let created_at = now - Duration::days(2);
    let started_at = created_at + Duration::minutes(15);
    let ended_at = started_at + Duration::minutes(95);

    let (mut active_game, mut boards) = create_started_game(
        ctx,
        uow,
        "Kiez Tour Freitag",
        "KIEZ01",
        GridSize::Three,
        VisibilityMode::Undercover,
        player_ids,
        deck,
        created_at,
        started_at,
    )
    .await?;

    let anna_id = player_ids[0];
    let ben_id = player_ids[1];
    let clara_id = player_ids[2];

    // Non-winning captures first so awards and feeds have multiple catchers.
    record_tile_capture(
        ctx,
        uow,
        board_mut(&mut boards, ben_id)?,
        0,
        started_at + Duration::minutes(12),
        [[0, 229, 255], [255, 229, 0], [255, 77, 141], [34, 197, 94]],
        uploaded_keys,
    )
    .await?;
    record_tile_capture(
        ctx,
        uow,
        board_mut(&mut boards, clara_id)?,
        4,
        started_at + Duration::minutes(28),
        [[255, 77, 141], [34, 197, 94], [255, 229, 0], [0, 229, 255]],
        uploaded_keys,
    )
    .await?;

    // Anna completes row 0: tile indices 0, 1, 2.
    record_tile_capture(
        ctx,
        uow,
        board_mut(&mut boards, anna_id)?,
        0,
        started_at + Duration::minutes(18),
        [[255, 229, 0], [20, 20, 20], [0, 229, 255], [255, 77, 141]],
        uploaded_keys,
    )
    .await?;
    record_tile_capture(
        ctx,
        uow,
        board_mut(&mut boards, anna_id)?,
        1,
        started_at + Duration::minutes(45),
        [[255, 77, 141], [255, 229, 0], [20, 20, 20], [0, 229, 255]],
        uploaded_keys,
    )
    .await?;
    let (_, winning_line) = record_tile_capture(
        ctx,
        uow,
        board_mut(&mut boards, anna_id)?,
        2,
        ended_at,
        [[34, 197, 94], [0, 229, 255], [255, 229, 0], [255, 77, 141]],
        uploaded_keys,
    )
    .await?;
    assert!(winning_line.is_some());

    let game_version = active_game.v();
    let Game::Active(active) = active_game.into_data() else {
        return Err(Report::new(SeedError).attach("expected active game before completion"));
    };
    let completed = active
        .complete(&boards[&anna_id], ended_at)
        .change_context(SeedError)?;
    active_game = Versioned::new(game_version, Game::Completed(completed));
    uow.games()
        .update(&mut active_game)
        .await
        .map_err(IntoReport::into_report)
        .change_context(SeedError)?;

    users[0].award_gold_medal();
    uow.benutzer()
        .update(&mut users[0])
        .await
        .map_err(IntoReport::into_report)
        .change_context(SeedError)?;

    Ok(active_game.into_data())
}

async fn seed_active_game(
    ctx: &ExecutionContext<'_>,
    uow: &UnitOfWork<'_>,
    player_ids: &[BenutzerId],
    deck: &StarterDeck,
    now: DateTime<Utc>,
    uploaded_keys: &mut Vec<String>,
) -> ResultReport<(Game, friendarazzi_core::domain::CaptureId), SeedError> {
    let created_at = now - Duration::hours(5);
    let started_at = now - Duration::hours(4);

    let (active_game, mut boards) = create_started_game(
        ctx,
        uow,
        "WG-Wochenende",
        "WG2026",
        GridSize::Three,
        VisibilityMode::BlurredTeaser,
        player_ids,
        deck,
        created_at,
        started_at,
    )
    .await?;

    let ben_id = player_ids[1];
    let clara_id = player_ids[2];

    let (ben_capture_id, _) = record_tile_capture(
        ctx,
        uow,
        board_mut(&mut boards, ben_id)?,
        0,
        now - Duration::minutes(45),
        [[255, 229, 0], [0, 229, 255], [255, 77, 141], [34, 197, 94]],
        uploaded_keys,
    )
    .await?;
    record_tile_capture(
        ctx,
        uow,
        board_mut(&mut boards, clara_id)?,
        4,
        now - Duration::minutes(20),
        [[0, 229, 255], [255, 77, 141], [34, 197, 94], [255, 229, 0]],
        uploaded_keys,
    )
    .await?;

    Ok((active_game.into_data(), ben_capture_id))
}

fn board_mut(
    boards: &mut HashMap<BenutzerId, Versioned<Board>>,
    owner_id: BenutzerId,
) -> ResultReport<&mut Versioned<Board>, SeedError> {
    boards
        .get_mut(&owner_id)
        .ok_or_else(|| Report::new(SeedError).attach("missing seeded board for player"))
}

async fn seed_drafting_game(
    uow: &UnitOfWork<'_>,
    player_ids: &[BenutzerId],
    deck: &StarterDeck,
    now: DateTime<Utc>,
) -> ResultReport<(), SeedError> {
    let created_at = now - Duration::hours(1);
    let game = create_lobby_with_players(
        uow,
        "Festival-Vorfreude",
        "FEST26",
        GridSize::Three,
        VisibilityMode::LiveFeed,
        player_ids,
        created_at,
    )
    .await?;

    let version = game.v();
    let Game::Lobby(lobby) = game.into_data() else {
        return Err(Report::new(SeedError));
    };
    let mut drafting = lobby.start_drafting().change_context(SeedError)?;
    let ordered_players = drafting.sorted_player_ids();
    let total_tiles = drafting.playable_tile_count();

    // Lock everyone except Anna so Anna immediately sees an open drafting prompt list.
    for (idx, &author) in ordered_players.iter().enumerate().skip(1) {
        let submissions = build_deck_submissions(&ordered_players, author, total_tiles, deck, idx)?;
        drafting
            .save_author_prompts(author, submissions, created_at + Duration::minutes(10))
            .change_context(SeedError)?;
        drafting
            .lock_author_prompts(author, created_at + Duration::minutes(12))
            .change_context(SeedError)?;
    }

    let mut updated = Versioned::new(version, Game::Drafting(drafting));
    uow.games()
        .update(&mut updated)
        .await
        .map_err(IntoReport::into_report)
        .change_context(SeedError)?;

    Ok(())
}

async fn seed_lobby_game(
    uow: &UnitOfWork<'_>,
    player_ids: &[BenutzerId],
    now: DateTime<Utc>,
) -> ResultReport<(), SeedError> {
    create_lobby_with_players(
        uow,
        "Sommerurlaub-Planung",
        "URLAUB",
        GridSize::Four,
        VisibilityMode::Undercover,
        player_ids,
        now - Duration::minutes(20),
    )
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn seed_notifications(
    uow: &UnitOfWork<'_>,
    anna_id: BenutzerId,
    ben_id: BenutzerId,
    clara_id: BenutzerId,
    completed_game_id: friendarazzi_core::domain::GameId,
    active_game_id: friendarazzi_core::domain::GameId,
    ben_capture_id: friendarazzi_core::domain::CaptureId,
    now: DateTime<Utc>,
) -> ResultReport<(), SeedError> {
    let (completed_notif, _) = NeueBenachrichtigung::spiel_beendet(
        anna_id,
        completed_game_id,
        anna_id,
        now - Duration::days(2) + Duration::minutes(110),
    );
    let mut created_completed = uow
        .benachrichtigungen()
        .create(completed_notif)
        .await
        .map_err(IntoReport::into_report)
        .change_context(SeedError)?;
    created_completed.als_gelesen_markieren(now - Duration::days(1));
    uow.benachrichtigungen()
        .update(&mut created_completed)
        .await
        .map_err(IntoReport::into_report)
        .change_context(SeedError)?;

    let (started_notif, _) =
        NeueBenachrichtigung::spiel_gestartet(anna_id, active_game_id, now - Duration::hours(4));
    uow.benachrichtigungen()
        .create(started_notif)
        .await
        .map_err(IntoReport::into_report)
        .change_context(SeedError)?;

    let (capture_notif, _) = NeueBenachrichtigung::aufnahme_hochgeladen(
        anna_id,
        active_game_id,
        ben_capture_id,
        ben_id,
        clara_id,
        now - Duration::minutes(45),
    );
    uow.benachrichtigungen()
        .create(capture_notif)
        .await
        .map_err(IntoReport::into_report)
        .change_context(SeedError)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn create_started_game(
    ctx: &ExecutionContext<'_>,
    uow: &UnitOfWork<'_>,
    title: &str,
    invite_code: &str,
    grid_size: GridSize,
    visibility_mode: VisibilityMode,
    player_ids: &[BenutzerId],
    deck: &StarterDeck,
    created_at: DateTime<Utc>,
    started_at: DateTime<Utc>,
) -> ResultReport<(Versioned<Game>, HashMap<BenutzerId, Versioned<Board>>), SeedError> {
    let game = create_lobby_with_players(
        uow,
        title,
        invite_code,
        grid_size,
        visibility_mode,
        player_ids,
        created_at,
    )
    .await?;

    let version = game.v();
    let Game::Lobby(lobby) = game.into_data() else {
        return Err(Report::new(SeedError));
    };
    let mut drafting = lobby.start_drafting().change_context(SeedError)?;
    let ordered_players = drafting.sorted_player_ids();
    let total_tiles = drafting.playable_tile_count();
    let lock_time = started_at - Duration::seconds(READY_GRACE_PERIOD_SECONDS);

    for (idx, &author) in ordered_players.iter().enumerate() {
        let submissions = build_deck_submissions(&ordered_players, author, total_tiles, deck, idx)?;
        drafting
            .save_author_prompts(author, submissions, lock_time - Duration::minutes(5))
            .change_context(SeedError)?;
        drafting
            .lock_author_prompts(author, lock_time)
            .change_context(SeedError)?;
    }

    // Persist `Drafting` first so `prompt_submission` rows exist before `board_tile` references them.
    let mut persisted_drafting = Versioned::new(version, Game::Drafting(drafting));
    uow.games()
        .update(&mut persisted_drafting)
        .await
        .map_err(IntoReport::into_report)
        .change_context(SeedError)?;

    let drafting_version = persisted_drafting.v();
    let Game::Drafting(drafting) = persisted_drafting.into_data() else {
        return Err(Report::new(SeedError));
    };
    let outcome = {
        let mut rng = ctx.rng();
        drafting
            .finalize_ready_start(started_at, &mut *rng)
            .change_context(SeedError)?
    };
    let friendarazzi_core::domain::DraftStartOutcome::Started {
        game: active,
        boards,
    } = outcome
    else {
        return Err(Report::new(SeedError).attach("seeded drafting game did not transition"));
    };

    let mut board_map = HashMap::with_capacity(boards.len());
    for new_board in boards {
        let created = uow
            .boards()
            .create(new_board)
            .await
            .map_err(IntoReport::into_report)
            .change_context(SeedError)?;
        board_map.insert(created.owner_user_id(), created);
    }

    let mut updated_game = Versioned::new(drafting_version, Game::Active(active));
    uow.games()
        .update(&mut updated_game)
        .await
        .map_err(IntoReport::into_report)
        .change_context(SeedError)?;

    Ok((updated_game, board_map))
}

async fn create_lobby_with_players(
    uow: &UnitOfWork<'_>,
    title: &str,
    invite_code: &str,
    grid_size: GridSize,
    visibility_mode: VisibilityMode,
    player_ids: &[BenutzerId],
    created_at: DateTime<Utc>,
) -> ResultReport<Versioned<Game>, SeedError> {
    let (&host_id, guests) = player_ids
        .split_first()
        .ok_or_else(|| Report::new(SeedError).attach("seed game requires at least one player"))?;
    let mut game = uow
        .games()
        .create(NewGame {
            invite_code: InviteCode::new(invite_code).change_context(SeedError)?,
            host_user_id: host_id,
            host_watching: true,
            title: NonEmptyString::neu(title).change_context(SeedError)?,
            grid_size,
            visibility_mode,
            created_at,
        })
        .await
        .map_err(IntoReport::into_report)
        .change_context(SeedError)?;

    let version = game.v();
    let Game::Lobby(mut lobby) = game.into_data() else {
        return Err(Report::new(SeedError));
    };
    for (idx, &guest_id) in guests.iter().enumerate() {
        lobby.join_player(
            guest_id,
            created_at + Duration::seconds((idx + 1) as i64),
            true,
        );
    }
    game = Versioned::new(version, Game::Lobby(lobby));
    uow.games()
        .update(&mut game)
        .await
        .map_err(IntoReport::into_report)
        .change_context(SeedError)?;

    Ok(game)
}

fn build_deck_submissions(
    player_ids: &[BenutzerId],
    author: BenutzerId,
    total_tiles: usize,
    deck: &StarterDeck,
    seed_offset: usize,
) -> ResultReport<Vec<PromptSubmission>, SeedError> {
    let quotas =
        target_quotas_for_author(player_ids, author, total_tiles).change_context(SeedError)?;
    let mut prompt_cursor = seed_offset * 5;
    let mut submissions = Vec::with_capacity(total_tiles);

    for (target_user_id, count) in quotas {
        for _ in 0..count {
            let raw_prompt = &deck.prompts[prompt_cursor % deck.prompts.len()];
            prompt_cursor += 1;
            let text = NonEmptyString::neu(raw_prompt.clone()).change_context(SeedError)?;
            submissions.push(
                PromptSubmission::new(
                    PromptSubmissionId(Uuid::new_v4()),
                    author,
                    target_user_id,
                    text,
                    PromptSource::StarterDeck,
                )
                .change_context(SeedError)?,
            );
        }
    }

    Ok(submissions)
}

async fn record_tile_capture(
    ctx: &ExecutionContext<'_>,
    uow: &UnitOfWork<'_>,
    board: &mut Versioned<Board>,
    tile_index: usize,
    captured_at: DateTime<Utc>,
    palette: [[u8; 3]; 4],
    uploaded_keys: &mut Vec<String>,
) -> ResultReport<
    (
        friendarazzi_core::domain::CaptureId,
        Option<Vec<friendarazzi_core::domain::BoardCoordinates>>,
    ),
    SeedError,
> {
    let tile = board.tiles()[tile_index].clone();
    let png = render_quadrant_png(320, 240, palette);
    let processed = Capture::process_photo(&png).change_context(SeedError)?;
    let capture_uuid = Uuid::new_v4();
    let full_key = format!("capture_{capture_uuid}.webp");
    let thumb_key = format!("thumb_{capture_uuid}.webp");

    ctx.object_store()
        .put(&full_key, &processed.full_webp)
        .await
        .change_context(SeedError)?;
    uploaded_keys.push(full_key.clone());

    ctx.object_store()
        .put(&thumb_key, &processed.blurred_webp)
        .await
        .change_context(SeedError)?;
    uploaded_keys.push(thumb_key.clone());

    let capture = uow
        .captures()
        .create(friendarazzi_core::domain::NewCapture {
            game_id: board.game_id(),
            tile_id: tile.id(),
            catcher_user_id: board.owner_user_id(),
            target_user_id: tile.target_user_id(),
            prompt_text: tile.prompt_text().clone(),
            image_storage_key: full_key,
            thumbnail_storage_key: thumb_key,
            captured_at,
        })
        .await
        .map_err(IntoReport::into_report)
        .change_context(SeedError)?;

    let capture_id = capture.id();
    let winning_line = board
        .record_capture(tile.id(), capture_id)
        .change_context(SeedError)?;
    uow.boards()
        .update(board)
        .await
        .map_err(IntoReport::into_report)
        .change_context(SeedError)?;

    Ok((capture_id, winning_line))
}

fn find_deck<'a>(decks: &'a [StarterDeck], slug: &str) -> &'a StarterDeck {
    decks.iter().find(|d| d.slug == slug).unwrap_or(&decks[0])
}

fn render_quadrant_png(width: u32, height: u32, palette: [[u8; 3]; 4]) -> Vec<u8> {
    let mut rgb = RgbImage::new(width, height);
    for (x, y, px) in rgb.enumerate_pixels_mut() {
        let idx = match (x < width / 2, y < height / 2) {
            (true, true) => 0,
            (false, true) => 1,
            (true, false) => 2,
            (false, false) => 3,
        };
        *px = Rgb(palette[idx]);
    }
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(rgb)
        .write_to(&mut out, ImageFormat::Png)
        .expect("PNG encoding of in-memory RGB buffer cannot fail");
    out.into_inner()
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::sync::Arc;

    use friendarazzi_core::{
        adapters::{InMemorySigning, InMemoryThrottle, NullMailSender},
        application::IdentityProviderConfiguration,
        ports::{ClientKey, ObjectStore},
        service::Anmeldung,
    };
    use friendarazzi_filesystemstore::FileSystemObjectStore;
    use friendarazzi_sqlite::SQLiteInstance;
    use tempdir::TempDir;

    use super::*;

    async fn build_test_app(dir: &TempDir) -> (App, Arc<FileSystemObjectStore>) {
        let db_path = dir.path().join("seed-test.db");
        let sqlite = Arc::new(SQLiteInstance::local(&db_path).await.unwrap());
        sqlite.migrate_repos_to_latest().await.unwrap();

        let object_store =
            Arc::new(FileSystemObjectStore::new(dir.path().join("storage")).unwrap());
        let app = App::builder()
            .uow_provider(Box::new(sqlite))
            .object_store(object_store.clone())
            .signing(Arc::new(InMemorySigning::random()))
            .throttle(Arc::new(InMemoryThrottle::new()))
            .mail_sender(Arc::new(NullMailSender))
            .konto(IdentityProviderConfiguration::neu(
                "friendarazzi.app",
                "friendarazzi-api",
            ))
            .build();
        (app, object_store)
    }

    #[test_log::test(tokio::test)]
    async fn seed_populates_users_games_media_and_notifications_idempotently() {
        let dir = TempDir::new("friendarazzi-seed").unwrap();
        let (app, object_store) = build_test_app(&dir).await;

        seed(&app).await.expect("initial seed succeeds");
        // Running a second time must be a safe no-op.
        seed(&app).await.expect("repeated seed is idempotent");

        let session = app
            .execute(
                AuthContext::Anonymous,
                Anmeldung {
                    email: EmailAdresse::normalisiert(ANCHOR_USER_EMAIL).unwrap(),
                    passwort: KlartextPasswort::neu(DEV_PASSWORD).unwrap(),
                    client_key: ClientKey::new("127.0.0.1".parse::<IpAddr>().unwrap(), "seed-test"),
                },
            )
            .await
            .expect("seeded user can sign in");

        assert_eq!(session.benutzer.gold_medal_count(), 1);
        assert!(session.benutzer.avatar_storage_key().is_some());

        let anna_id = session.benutzer.id();
        let avatar_key = session.benutzer.avatar_storage_key().unwrap().to_owned();
        let avatar_stream = object_store
            .get(&avatar_key)
            .await
            .expect("object store lookup succeeds");
        assert!(avatar_stream.is_some(), "avatar exists in object store");

        let (games, notifications, capture_keys) = app
            .execute_fn(AuthContext::Anonymous, async move |ctx| {
                let uow = ctx.enter().await.change_context(SeedError)?;
                let games = uow
                    .games()
                    .find_by_participating_user(anna_id)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(SeedError)?;
                let notifications = uow
                    .benachrichtigungen()
                    .find_by_empfänger(anna_id)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(SeedError)?;
                let captures = uow
                    .captures()
                    .find_by_catcher_user(anna_id)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(SeedError)?;
                let keys: Vec<(String, String)> = captures
                    .into_iter()
                    .map(|c| {
                        (
                            c.image_storage_key().to_owned(),
                            c.thumbnail_storage_key().to_owned(),
                        )
                    })
                    .collect();
                uow.finish(Ok((games, notifications, keys)), SeedError)
                    .await
            })
            .await
            .unwrap();

        assert_eq!(games.len(), 4);
        assert!(games.iter().any(|g| matches!(&**g, Game::Completed(_))));
        assert!(games.iter().any(|g| matches!(&**g, Game::Active(_))));
        assert!(games.iter().any(|g| matches!(&**g, Game::Drafting(_))));
        assert!(games.iter().any(|g| matches!(&**g, Game::Lobby(_))));

        assert_eq!(notifications.len(), 3);
        assert_eq!(capture_keys.len(), 3);
        for (full_key, thumb_key) in capture_keys {
            assert!(object_store.get(&full_key).await.unwrap().is_some());
            assert!(object_store.get(&thumb_key).await.unwrap().is_some());
        }
    }
}
