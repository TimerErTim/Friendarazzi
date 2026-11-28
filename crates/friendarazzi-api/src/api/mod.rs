//! Typed facade between HTTP and use cases.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, ResultExt};
use friendarazzi_core::{
    App, ResultReport,
    application::AuthContext,
    domain::{
        self, BenachrichtigungArt, BenachrichtigungId, BenachrichtigungsEinstellungen, BenutzerId,
        BoardTileId, Capture, CaptureId, Game, GameAwards, GameId, InviteCode, MIN_PLAYERS,
        can_view_blurred_capture, can_view_full_capture, compute_game_awards,
        target_quotas_for_author,
    },
    ports::{ClientKey, ObjectStream, RepositoryError, ThrottleAction},
    service::{
        AbandonGame, Abmeldung, AbmeldungFehler, AlleBenachrichtigungenAlsGelesenMarkieren,
        Anmeldung, AnmeldungFehler, BenachrichtigungAbbestellenMitToken,
        BenachrichtigungAlsGelesenMarkieren, BenachrichtigungUseCaseError,
        BenachrichtigungsEinstellungenAktualisieren, BenutzerLöschen, BenutzerLöschenFehler,
        CaptureUseCaseError, EmailBestätigen, EmailBestätigenFehler, EmailBestätigungErneutSenden,
        EmailBestätigungSendenFehler, Erneuerung, ErneuerungFehler, GameUseCaseError,
        HostFillRemainingDrafts, KickPlayer, LeaveGame, LockDraftPrompts, OidcCompleteCallback,
        OidcStartAuth, OidcUseCaseError, PasswortErstmaligSetzenFehler, PasswortVergessen,
        PasswortVergessenFehler, PasswortWiederherstellen, PasswortWiederherstellenFehler,
        ProfileUseCaseError, Registrieren, RegistrierenFehler, RetractCapture, SessionAusgestellt,
        SetDraftDeadline, StartDrafting, SubmitCapture, UnlinkExternalIdentity, UnlockDraftPrompts,
        UploadAvatar,
    },
};
use tracing::warn;

pub mod errors;
pub mod requests;

use self::errors::{GameReadError, MediaReadError, NotificationReadError, ProfileReadError};
use crate::schema::{
    AuthenticatedBenutzer, BoardTileView, BoardView, BuildVersion, CaptureFeedItem, CaptureOutcome,
    DraftTargetQuota, GalleryGameGroup, GameDetail, GameInvitePreview, GameSummary, GridCoordinate,
    LinkedExternalIdentity, LoginMethodsResponse, NotificationPreferences, NotificationView,
    NotificationsInbox, OidcAuthorizationRedirect, OidcProviderItem, OidcProvidersResponse,
    PlayerBadge, PromptSubmissionView, StarterDeck, SuperlativeKind, SuperlativeView,
    UnsubscribeOutcome, UserGallery, UserProfile,
};

#[derive(Clone)]
pub struct FriendarazziAppApi {
    app: Arc<App>,
}

impl FriendarazziAppApi {
    pub fn new(app: App) -> Self {
        Self::from_arc(Arc::new(app))
    }

    pub fn from_arc(app: Arc<App>) -> Self {
        Self { app }
    }

    pub fn inner_app(&self) -> Arc<App> {
        self.app.clone()
    }

    /// Compile-time build identity of the service.
    pub fn build_version() -> BuildVersion {
        BuildVersion {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            build_time: env!("FRIENDARAZZI_BUILD_TIME").to_owned(),
            commit: option_env!("FRIENDARAZZI_BUILD_COMMIT")
                .filter(|s| !s.trim().is_empty())
                .map(str::to_owned),
        }
    }

    // ---------------------------------------------------------------- Decks

    pub async fn decks_list(
        &self,
        auth: AuthContext,
    ) -> ResultReport<Vec<StarterDeck>, GameReadError> {
        self.app
            .execute_fn(auth, async |ctx| {
                let _user_id = ctx
                    .auth()
                    .require_benutzer_id()
                    .change_context(GameReadError::Unauthorized)?;
                let uow = ctx
                    .enter()
                    .await
                    .change_context(GameReadError::Persistence)?;
                let res = async {
                    let decks = uow
                        .decks()
                        .find_all()
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(GameReadError::Persistence)?;
                    Ok::<_, Report<GameReadError>>(
                        decks
                            .into_iter()
                            .map(|d| StarterDeck {
                                id: d.id.0,
                                slug: d.slug,
                                name: d.name,
                                description: d.description,
                                prompts: d.prompts,
                            })
                            .collect(),
                    )
                }
                .await;
                uow.finish(res, GameReadError::Persistence).await
            })
            .await
    }

    // ---------------------------------------------------------------- Games

    pub async fn games_list(
        &self,
        auth: AuthContext,
    ) -> ResultReport<Vec<GameSummary>, GameReadError> {
        self.app
            .execute_fn(auth, async |ctx| {
                let user_id = ctx
                    .auth()
                    .require_benutzer_id()
                    .change_context(GameReadError::Unauthorized)?;
                let now = ctx.clock().now();
                let uow = ctx
                    .enter()
                    .await
                    .change_context(GameReadError::Persistence)?;
                let res = async {
                    let games = uow
                        .games()
                        .find_by_participating_user(user_id)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(GameReadError::Persistence)?;

                    let mut summaries = Vec::with_capacity(games.len());
                    for game in games {
                        let game = game.into_data();
                        let winner = match &game {
                            Game::Completed(c) => {
                                if let Some(winner_id) = c.winner_user_id() {
                                    uow.benutzer().find_by_id(winner_id).await.ok().map(|b| {
                                        badge_from_benutzer(
                                            &b.into_data(),
                                            false,
                                            game.host_user_id() == winner_id,
                                        )
                                    })
                                } else {
                                    None
                                }
                            }
                            Game::Lobby(_) | Game::Drafting(_) | Game::Active(_) => None,
                        };
                        let my_captured_tiles = match &game {
                            Game::Active(_) | Game::Completed(_) => uow
                                .boards()
                                .find_by_game_and_owner(game.id(), user_id)
                                .await
                                .ok()
                                .map(|b| {
                                    b.into_data()
                                        .tiles()
                                        .iter()
                                        .filter(|t| t.capture_id().is_some())
                                        .count() as u32
                                }),
                            Game::Lobby(_) | Game::Drafting(_) => None,
                        };
                        summaries.push(game_to_summary(
                            &game,
                            Some(user_id),
                            my_captured_tiles,
                            winner,
                            now,
                        ));
                    }
                    Ok::<_, Report<GameReadError>>(summaries)
                }
                .await;
                uow.finish(res, GameReadError::Persistence).await
            })
            .await
    }

    pub async fn game_by_invite_code(
        &self,
        code: String,
    ) -> ResultReport<GameInvitePreview, GameReadError> {
        let invite_code = InviteCode::new(code).change_context(GameReadError::BadRequest)?;
        self.app
            .execute_fn(AuthContext::Anonymous, async |ctx| {
                let uow = ctx
                    .enter()
                    .await
                    .change_context(GameReadError::Persistence)?;
                let res = async {
                    let game = match uow.games().find_by_invite_code(&invite_code).await {
                        Ok(g) => g.into_data(),
                        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                            return Err(Report::new(GameReadError::NotFound));
                        }
                        Err(err) => return Err(err).change_context(GameReadError::Persistence),
                    };
                    Ok::<_, Report<GameReadError>>(GameInvitePreview {
                        id: game.id().0,
                        invite_code: game.invite_code().as_str().to_owned(),
                        title: game.title().as_str().to_owned(),
                        grid_size: game.grid_size() as u8,
                        visibility_mode: game.visibility_mode().into(),
                        status: (&game).into(),
                        playable_tile_count: game.playable_tile_count() as u32,
                        player_count: game.players().len() as u32,
                        draft_deadline_at: game.draft_deadline_at(),
                        ends_at: game.ends_at(),
                    })
                }
                .await;
                uow.finish(res, GameReadError::Persistence).await
            })
            .await
    }

    pub async fn game_create(
        &self,
        auth: AuthContext,
        req: requests::CreateGameRequest,
    ) -> ResultReport<GameDetail, GameUseCaseError> {
        let uc = req.into_use_case()?;
        let game = self.app.execute(auth, uc).await?;
        self.load_game_detail_after_write(auth, game.id(), GameUseCaseError::Persistence)
            .await
    }

    pub async fn game_join(
        &self,
        auth: AuthContext,
        req: requests::JoinGameRequest,
    ) -> ResultReport<GameDetail, GameUseCaseError> {
        let uc = req.into_use_case()?;
        let game = self.app.execute(auth, uc).await?;
        self.load_game_detail_after_write(auth, game.id(), GameUseCaseError::Persistence)
            .await
    }

    pub async fn game_update_settings(
        &self,
        auth: AuthContext,
        game_id: GameId,
        req: requests::UpdateGameSettingsRequest,
    ) -> ResultReport<GameDetail, GameUseCaseError> {
        let uc = req.into_use_case(game_id)?;
        let game = self.app.execute(auth, uc).await?;
        self.load_game_detail_after_write(auth, game.id(), GameUseCaseError::Persistence)
            .await
    }

    pub async fn game_start_drafting(
        &self,
        auth: AuthContext,
        game_id: GameId,
    ) -> ResultReport<GameDetail, GameUseCaseError> {
        let game = self.app.execute(auth, StartDrafting { game_id }).await?;
        self.load_game_detail_after_write(auth, game.id(), GameUseCaseError::Persistence)
            .await
    }

    pub async fn game_save_prompts(
        &self,
        auth: AuthContext,
        game_id: GameId,
        req: requests::SaveDraftPromptsRequest,
    ) -> ResultReport<GameDetail, GameUseCaseError> {
        let uc = req.into_use_case(game_id)?;
        self.app.execute(auth, uc).await?;
        self.load_game_detail_after_write(auth, game_id, GameUseCaseError::Persistence)
            .await
    }

    pub async fn game_lock_prompts(
        &self,
        auth: AuthContext,
        game_id: GameId,
    ) -> ResultReport<GameDetail, GameUseCaseError> {
        let game = self.app.execute(auth, LockDraftPrompts { game_id }).await?;
        self.load_game_detail_after_write(auth, game.id(), GameUseCaseError::Persistence)
            .await
    }

    pub async fn game_unlock_prompts(
        &self,
        auth: AuthContext,
        game_id: GameId,
    ) -> ResultReport<GameDetail, GameUseCaseError> {
        let game = self
            .app
            .execute(auth, UnlockDraftPrompts { game_id })
            .await?;
        self.load_game_detail_after_write(auth, game.id(), GameUseCaseError::Persistence)
            .await
    }

    pub async fn game_host_fill_drafts(
        &self,
        auth: AuthContext,
        game_id: GameId,
        req: requests::HostFillDraftsRequest,
    ) -> ResultReport<GameDetail, GameUseCaseError> {
        let uc: HostFillRemainingDrafts = req.into_use_case(game_id);
        let game = self.app.execute(auth, uc).await?;
        self.load_game_detail_after_write(auth, game.id(), GameUseCaseError::Persistence)
            .await
    }

    pub async fn game_set_draft_deadline(
        &self,
        auth: AuthContext,
        game_id: GameId,
        req: requests::SetDraftDeadlineRequest,
    ) -> ResultReport<GameDetail, GameUseCaseError> {
        let uc: SetDraftDeadline = req.into_use_case(game_id);
        let game = self.app.execute(auth, uc).await?;
        self.load_game_detail_after_write(auth, game.id(), GameUseCaseError::Persistence)
            .await
    }

    pub async fn game_set_watch(
        &self,
        auth: AuthContext,
        game_id: GameId,
        req: requests::SetGameWatchRequest,
    ) -> ResultReport<GameDetail, BenachrichtigungUseCaseError> {
        let uc = req.into_use_case(game_id);
        let game = self.app.execute(auth, uc).await?;
        self.load_game_detail_after_write(
            auth,
            game.id(),
            BenachrichtigungUseCaseError::Persistence,
        )
        .await
    }

    pub async fn game_leave(
        &self,
        auth: AuthContext,
        game_id: GameId,
    ) -> ResultReport<(), GameUseCaseError> {
        self.app.execute(auth, LeaveGame { game_id }).await?;
        Ok(())
    }

    pub async fn game_kick_player(
        &self,
        auth: AuthContext,
        game_id: GameId,
        target_user_id: BenutzerId,
    ) -> ResultReport<GameDetail, GameUseCaseError> {
        let game = self
            .app
            .execute(
                auth,
                KickPlayer {
                    game_id,
                    target_user_id,
                },
            )
            .await?;
        self.load_game_detail_after_write(auth, game.id(), GameUseCaseError::Persistence)
            .await
    }

    pub async fn game_transfer_host(
        &self,
        auth: AuthContext,
        game_id: GameId,
        req: requests::TransferHostRequest,
    ) -> ResultReport<GameDetail, GameUseCaseError> {
        let uc = req.into_use_case(game_id);
        let game = self.app.execute(auth, uc).await?;
        self.load_game_detail_after_write(auth, game.id(), GameUseCaseError::Persistence)
            .await
    }

    pub async fn game_abandon(
        &self,
        auth: AuthContext,
        game_id: GameId,
    ) -> ResultReport<GameDetail, GameUseCaseError> {
        let game = self.app.execute(auth, AbandonGame { game_id }).await?;
        self.load_game_detail_after_write(auth, game.id(), GameUseCaseError::Persistence)
            .await
    }

    async fn load_game_detail_after_write<E: friendarazzi_core::ThreadSafeError>(
        &self,
        auth: AuthContext,
        game_id: GameId,
        fallback_err: E,
    ) -> ResultReport<GameDetail, E> {
        self.game_detail(auth, game_id)
            .await
            .map_err(|err| err.change_context(fallback_err))
    }

    pub async fn game_detail(
        &self,
        auth: AuthContext,
        game_id: GameId,
    ) -> ResultReport<GameDetail, GameReadError> {
        self.app
            .execute_fn(auth, async |ctx| {
                let viewer_id = ctx
                    .auth()
                    .require_benutzer_id()
                    .change_context(GameReadError::Unauthorized)?;
                let now = ctx.clock().now();
                let uow = ctx
                    .enter()
                    .await
                    .change_context(GameReadError::Persistence)?;
                let res = async {
                    let game = match uow.games().find_by_id(game_id).await {
                        Ok(g) => g.into_data(),
                        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                            return Err(Report::new(GameReadError::NotFound));
                        }
                        Err(err) => return Err(err).change_context(GameReadError::Persistence),
                    };

                    if !game.has_player(viewer_id) {
                        return Err(Report::new(GameReadError::Forbidden));
                    }

                    let player_ids = game.sorted_player_ids();
                    let users = uow
                        .benutzer()
                        .find_by_ids(&player_ids)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(GameReadError::Persistence)?;

                    let badges = build_badge_map(&game, users.into_iter().map(|b| b.into_data()));
                    let player_badges: Vec<PlayerBadge> = player_ids
                        .iter()
                        .filter_map(|pid| badges.get(pid).cloned())
                        .collect();

                    let (draft_closes_at, started_at, ended_at, winner) = match &game {
                        Game::Lobby(_) => (None, None, None, None),
                        Game::Drafting(d) => (d.draft_closes_at(), None, None, None),
                        Game::Active(a) => (None, Some(a.started_at()), None, None),
                        Game::Completed(c) => (
                            None,
                            Some(c.started_at()),
                            Some(c.ended_at()),
                            c.winner_user_id().and_then(|wid| badges.get(&wid).cloned()),
                        ),
                    };

                    // Drafting quotas & caller's saved drafts
                    let (my_quotas, my_drafts) = if let Game::Drafting(drafting) = &game {
                        let quotas = target_quotas_for_author(
                            &player_ids,
                            viewer_id,
                            drafting.playable_tile_count(),
                        )
                        .map_err(|e| e.change_context(GameReadError::Persistence))?;
                        let quota_views = quotas
                            .into_iter()
                            .filter_map(|(target_id, count)| {
                                badges
                                    .get(&target_id)
                                    .cloned()
                                    .map(|badge| DraftTargetQuota {
                                        target_user: badge,
                                        required_count: count as u32,
                                    })
                            })
                            .collect();
                        let drafts = drafting
                            .prompts_by_author(viewer_id)
                            .into_iter()
                            .map(|p| PromptSubmissionView {
                                id: p.id.0,
                                target_user_id: p.target_user_id.0,
                                text: p.text.as_str().to_owned(),
                                source: p.source.into(),
                            })
                            .collect();
                        (quota_views, drafts)
                    } else {
                        (Vec::new(), Vec::new())
                    };

                    // Boards & Captures
                    let mut my_board = None;
                    let mut all_boards = Vec::new();
                    let mut feed = Vec::new();
                    let mut superlatives = Vec::new();

                    if matches!(&game, Game::Active(_) | Game::Completed(_)) {
                        let is_completed = matches!(&game, Game::Completed(_));
                        let boards = uow
                            .boards()
                            .find_by_game(game_id)
                            .await
                            .map_err(IntoReport::into_report)
                            .change_context(GameReadError::Persistence)?;
                        let captures: Vec<Capture> = uow
                            .captures()
                            .find_by_game(game_id)
                            .await
                            .map_err(IntoReport::into_report)
                            .change_context(GameReadError::Persistence)?
                            .into_iter()
                            .map(|c| c.into_data())
                            .collect();

                        let capture_by_id: HashMap<_, _> =
                            captures.iter().map(|c| (c.id(), c)).collect();

                        for b in boards {
                            let b = b.into_data();
                            let owner_id = b.owner_user_id();
                            let is_mine = owner_id == viewer_id;
                            if !is_mine && !is_completed {
                                continue;
                            }
                            let Some(owner_badge) = badges.get(&owner_id).cloned() else {
                                continue;
                            };

                            let tile_views = b
                                .tiles()
                                .iter()
                                .filter_map(|t| {
                                    let target_user = badges.get(&t.target_user_id())?.clone();
                                    let cap =
                                        t.capture_id().and_then(|cid| capture_by_id.get(&cid));
                                    let show_photo = cap.is_some_and(|c| {
                                        can_view_full_capture(&game, c, viewer_id)
                                    });
                                    Some(BoardTileView {
                                        id: t.id().0,
                                        row: t.row(),
                                        col: t.col(),
                                        prompt_text: t.prompt_text().as_str().to_owned(),
                                        target_user,
                                        is_captured: t.capture_id().is_some(),
                                        capture_id: t.capture_id().map(|c| c.0),
                                        image_storage_key: if show_photo {
                                            cap.map(|c| c.image_storage_key().to_owned())
                                        } else {
                                            None
                                        },
                                        captured_at: cap.map(|c| c.captured_at()),
                                    })
                                })
                                .collect();

                            let view = BoardView {
                                id: b.id().0,
                                owner: owner_badge,
                                has_bingo: b.has_bingo(),
                                tiles: tile_views,
                            };
                            if is_mine {
                                my_board = Some(view.clone());
                            }
                            if is_completed {
                                all_boards.push(view);
                            }
                        }

                        for cap in &captures {
                            if let Some(item) = capture_to_feed_item(&game, &badges, cap, viewer_id)
                            {
                                feed.push(item);
                            }
                        }

                        if let Game::Completed(completed) = &game
                            && let Some(awards) =
                                compute_game_awards(&player_ids, &captures, completed.started_at())
                        {
                            superlatives = awards_to_superlatives(awards, &badges);
                        }
                    }

                    Ok::<_, Report<GameReadError>>(GameDetail {
                        id: game.id().0,
                        invite_code: game.invite_code().as_str().to_owned(),
                        host_user_id: game.host_user_id().0,
                        title: game.title().as_str().to_owned(),
                        grid_size: game.grid_size() as u8,
                        visibility_mode: game.visibility_mode().into(),
                        status: (&game).into(),
                        watching: game.player_watching(viewer_id).unwrap_or(true),
                        playable_tile_count: game.playable_tile_count() as u32,
                        draft_closes_at,
                        draft_deadline_at: game.draft_deadline_at(),
                        ends_at: game.ends_at(),
                        server_now: now,
                        started_at,
                        ended_at,
                        completion_reason: game.completion_reason().map(Into::into),
                        winner,
                        created_at: game.created_at(),
                        players: player_badges,
                        my_quotas,
                        my_drafts,
                        my_board,
                        all_boards,
                        feed,
                        superlatives,
                    })
                }
                .await;
                uow.finish(res, GameReadError::Persistence).await
            })
            .await
    }

    // ---------------------------------------------------------------- Captures

    pub async fn capture_submit(
        &self,
        auth: AuthContext,
        game_id: GameId,
        tile_id: BoardTileId,
        image_bytes: Vec<u8>,
    ) -> ResultReport<CaptureOutcome, CaptureUseCaseError> {
        let outcome = self
            .app
            .execute(
                auth,
                SubmitCapture {
                    game_id,
                    tile_id,
                    image_bytes,
                },
            )
            .await?;

        let viewer_id = auth
            .require_benutzer_id()
            .change_context(CaptureUseCaseError::Unauthorized)?;

        // Load badges for the returned capture item
        let item = self
            .app
            .execute_fn(auth, async |ctx| {
                let uow = ctx
                    .enter()
                    .await
                    .change_context(CaptureUseCaseError::Persistence)?;
                let res = async {
                    let game = uow
                        .games()
                        .find_by_id(game_id)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(CaptureUseCaseError::Persistence)?
                        .into_data();
                    let ids = game.sorted_player_ids();
                    let users = uow
                        .benutzer()
                        .find_by_ids(&ids)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(CaptureUseCaseError::Persistence)?;
                    let badges = build_badge_map(&game, users.into_iter().map(|b| b.into_data()));
                    let item = capture_to_feed_item(&game, &badges, &outcome.capture, viewer_id)
                        .ok_or_else(|| Report::new(CaptureUseCaseError::Persistence))?;
                    Ok::<_, Report<CaptureUseCaseError>>(item)
                }
                .await;
                uow.finish(res, CaptureUseCaseError::Persistence).await
            })
            .await?;

        Ok(CaptureOutcome {
            capture: item,
            winning_line: outcome
                .winning_line
                .map(|line| line.into_iter().map(GridCoordinate::from).collect()),
        })
    }

    pub async fn capture_retract(
        &self,
        auth: AuthContext,
        game_id: GameId,
        capture_id: CaptureId,
    ) -> ResultReport<GameDetail, CaptureUseCaseError> {
        self.app
            .execute(
                auth,
                RetractCapture {
                    game_id,
                    capture_id,
                },
            )
            .await?;
        self.game_detail(auth, game_id)
            .await
            .change_context(CaptureUseCaseError::Persistence)
    }

    // ---------------------------------------------------------------- Profiles & Galleries

    pub async fn profile_get(
        &self,
        auth: AuthContext,
        target_user_id: BenutzerId,
    ) -> ResultReport<UserProfile, ProfileReadError> {
        self.app
            .execute_fn(auth, async |ctx| {
                let _viewer_id = ctx
                    .auth()
                    .require_benutzer_id()
                    .change_context(ProfileReadError::Unauthorized)?;
                let uow = ctx
                    .enter()
                    .await
                    .change_context(ProfileReadError::Persistence)?;
                let res = async {
                    let benutzer = match uow.benutzer().find_by_id(target_user_id).await {
                        Ok(p) => p.into_data(),
                        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                            return Err(Report::new(ProfileReadError::NotFound));
                        }
                        Err(err) => return Err(err).change_context(ProfileReadError::Persistence),
                    };

                    let games = uow
                        .games()
                        .find_by_participating_user(target_user_id)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(ProfileReadError::Persistence)?;
                    let caught = uow
                        .captures()
                        .find_by_target_user(target_user_id)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(ProfileReadError::Persistence)?;
                    let captured = uow
                        .captures()
                        .find_by_catcher_user(target_user_id)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(ProfileReadError::Persistence)?;

                    Ok::<_, Report<ProfileReadError>>(profile_to_schema(
                        &benutzer,
                        games.len() as u32,
                        captured.len() as u32,
                        caught.len() as u32,
                    ))
                }
                .await;
                uow.finish(res, ProfileReadError::Persistence).await
            })
            .await
    }

    pub async fn profile_update(
        &self,
        auth: AuthContext,
        req: requests::UpdateProfileRequest,
    ) -> ResultReport<UserProfile, ProfileUseCaseError> {
        let uc = req.into_use_case()?;
        let updated = self.app.execute(auth, uc).await?;
        self.profile_get(auth, updated.id())
            .await
            .map_err(|e| e.change_context(ProfileUseCaseError::Persistence))
    }

    pub async fn profile_upload_avatar(
        &self,
        auth: AuthContext,
        image_bytes: Vec<u8>,
    ) -> ResultReport<UserProfile, ProfileUseCaseError> {
        let updated = self.app.execute(auth, UploadAvatar { image_bytes }).await?;
        self.profile_get(auth, updated.id())
            .await
            .map_err(|e| e.change_context(ProfileUseCaseError::Persistence))
    }

    pub async fn user_gallery(
        &self,
        auth: AuthContext,
        target_user_id: BenutzerId,
    ) -> ResultReport<UserGallery, ProfileReadError> {
        self.app
            .execute_fn(auth, async |ctx| {
                let viewer_id = ctx
                    .auth()
                    .require_benutzer_id()
                    .change_context(ProfileReadError::Unauthorized)?;
                let uow = ctx
                    .enter()
                    .await
                    .change_context(ProfileReadError::Persistence)?;
                let res = async {
                    let target_benutzer = match uow.benutzer().find_by_id(target_user_id).await {
                        Ok(p) => p.into_data(),
                        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                            return Err(Report::new(ProfileReadError::NotFound));
                        }
                        Err(err) => return Err(err).change_context(ProfileReadError::Persistence),
                    };

                    let games = uow
                        .games()
                        .find_by_participating_user(target_user_id)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(ProfileReadError::Persistence)?;
                    let caught_all: Vec<Capture> = uow
                        .captures()
                        .find_by_target_user(target_user_id)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(ProfileReadError::Persistence)?
                        .into_iter()
                        .map(|c| c.into_data())
                        .collect();
                    let captured_all: Vec<Capture> = uow
                        .captures()
                        .find_by_catcher_user(target_user_id)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(ProfileReadError::Persistence)?
                        .into_iter()
                        .map(|c| c.into_data())
                        .collect();

                    type CachedGame = (Game, HashMap<BenutzerId, PlayerBadge>);
                    let mut game_cache: HashMap<GameId, CachedGame> = HashMap::new();
                    let mut all_game_ids: HashSet<GameId> = HashSet::new();
                    for c in caught_all.iter().chain(captured_all.iter()) {
                        all_game_ids.insert(c.game_id());
                    }

                    for gid in all_game_ids {
                        let Ok(game) = uow.games().find_by_id(gid).await else {
                            continue;
                        };
                        let game = game.into_data();
                        let ids = game.sorted_player_ids();
                        let users = uow
                            .benutzer()
                            .find_by_ids(&ids)
                            .await
                            .map_err(IntoReport::into_report)
                            .change_context(ProfileReadError::Persistence)?;
                        let badges =
                            build_badge_map(&game, users.into_iter().map(|b| b.into_data()));
                        game_cache.insert(gid, (game, badges));
                    }

                    let group_captures = |list: &[Capture]| -> Vec<GalleryGameGroup> {
                        let mut by_game: HashMap<GameId, Vec<CaptureFeedItem>> = HashMap::new();
                        for cap in list {
                            let Some((game, badges)) = game_cache.get(&cap.game_id()) else {
                                continue;
                            };
                            // Only show full-resolution unredacted memories in galleries once visible!
                            if !can_view_full_capture(game, cap, viewer_id) {
                                continue;
                            }
                            if let Some(item) = capture_to_feed_item(game, badges, cap, viewer_id) {
                                by_game.entry(game.id()).or_default().push(item);
                            }
                        }

                        let mut groups: Vec<GalleryGameGroup> = by_game
                            .into_iter()
                            .filter_map(|(gid, mut photos)| {
                                let (game, badges) = game_cache.get(&gid)?;
                                photos.sort_by_key(|p| std::cmp::Reverse(p.captured_at));
                                let (started_at, ended_at, winner) = match game {
                                    Game::Lobby(_) | Game::Drafting(_) => (None, None, None),
                                    Game::Active(a) => (Some(a.started_at()), None, None),
                                    Game::Completed(c) => (
                                        Some(c.started_at()),
                                        Some(c.ended_at()),
                                        c.winner_user_id()
                                            .and_then(|wid| badges.get(&wid).cloned()),
                                    ),
                                };
                                Some(GalleryGameGroup {
                                    game_id: game.id().0,
                                    game_title: game.title().as_str().to_owned(),
                                    started_at,
                                    ended_at,
                                    winner,
                                    photos,
                                })
                            })
                            .collect();
                        groups.sort_by_key(|g| std::cmp::Reverse(g.ended_at.or(g.started_at)));
                        groups
                    };

                    let caught_groups = group_captures(&caught_all);
                    let captured_groups = group_captures(&captured_all);

                    Ok::<_, Report<ProfileReadError>>(UserGallery {
                        user: profile_to_schema(
                            &target_benutzer,
                            games.len() as u32,
                            captured_all.len() as u32,
                            caught_all.len() as u32,
                        ),
                        caught: caught_groups,
                        captured: captured_groups,
                    })
                }
                .await;
                uow.finish(res, ProfileReadError::Persistence).await
            })
            .await
    }

    // ---------------------------------------------------------------- Media

    pub async fn media_get(
        &self,
        auth: AuthContext,
        storage_key: String,
    ) -> ResultReport<ObjectStream, MediaReadError> {
        if storage_key.is_empty()
            || storage_key.contains('/')
            || storage_key.contains("..")
            || storage_key.contains('\\')
        {
            return Err(Report::new(MediaReadError::NotFound));
        }

        self.app
            .execute_fn(auth, async |ctx| {
                let viewer_id = ctx
                    .auth()
                    .require_benutzer_id()
                    .change_context(MediaReadError::Unauthorized)?;

                if !storage_key.starts_with("avatar_") {
                    let uow = ctx.enter().await.change_context(MediaReadError::Storage)?;
                    let allowed = async {
                        let capture = match uow.captures().find_by_storage_key(&storage_key).await {
                            Ok(c) => c.into_data(),
                            Err(err)
                                if matches!(err.current_context(), RepositoryError::NotFound) =>
                            {
                                return Err(Report::new(MediaReadError::NotFound));
                            }
                            Err(err) => return Err(err).change_context(MediaReadError::Storage),
                        };
                        let game = uow
                            .games()
                            .find_by_id(capture.game_id())
                            .await
                            .map_err(IntoReport::into_report)
                            .change_context(MediaReadError::Storage)?
                            .into_data();

                        let permitted = if storage_key == capture.thumbnail_storage_key() {
                            can_view_blurred_capture(&game, &capture, viewer_id)
                        } else {
                            can_view_full_capture(&game, &capture, viewer_id)
                        };
                        if !permitted {
                            return Err(Report::new(MediaReadError::Forbidden));
                        }
                        Ok::<_, Report<MediaReadError>>(())
                    }
                    .await;
                    uow.finish(allowed, MediaReadError::Storage).await?;
                }

                let stream = ctx
                    .object_store()
                    .get(&storage_key)
                    .await
                    .change_context(MediaReadError::Storage)?
                    .ok_or_else(|| Report::new(MediaReadError::NotFound))?;
                Ok(stream)
            })
            .await
    }

    // ---------------------------------------------------------------- Registration & Login

    pub async fn registrieren(
        &self,
        anfrage: requests::RegistrierungRequest,
        client_key: ClientKey,
    ) -> ResultReport<SessionAusgestellt, RegistrierenFehler> {
        let use_case: Registrieren = anfrage.in_use_case(client_key)?;
        self.app.execute(AuthContext::Anonymous, use_case).await
    }

    pub async fn email_bestätigen(
        &self,
        anfrage: requests::EmailBestaetigenRequest,
        client_key: ClientKey,
    ) -> ResultReport<(), EmailBestätigenFehler> {
        let use_case: EmailBestätigen = anfrage.in_use_case(client_key)?;
        self.app.execute(AuthContext::Anonymous, use_case).await
    }

    pub async fn email_bestätigung_erneut_senden(
        &self,
        auth: AuthContext,
        client_key: ClientKey,
    ) -> ResultReport<(), EmailBestätigungSendenFehler> {
        self.app
            .execute(auth, EmailBestätigungErneutSenden { client_key })
            .await
    }

    pub async fn anmelden(
        &self,
        anfrage: requests::AnmeldungRequest,
        client_key: ClientKey,
    ) -> ResultReport<SessionAusgestellt, AnmeldungFehler> {
        let use_case: Anmeldung = anfrage.in_use_case(client_key)?;
        let ausgestellt = self.app.execute(AuthContext::Anonymous, use_case).await?;
        Ok(ausgestellt)
    }

    pub async fn session_erneuern(
        &self,
        refresh_token: String,
        client_key: ClientKey,
    ) -> ResultReport<SessionAusgestellt, ErneuerungFehler> {
        let jetzt = self.app.clock.now();
        self.app
            .throttle
            .check(&client_key, ThrottleAction::Erneuerung, jetzt)
            .await
            .change_context(ErneuerungFehler::Gedrosselt)?;

        let refresh_token = match friendarazzi_core::domain::konto::RefreshToken::try_from(
            refresh_token,
        ) {
            Ok(token) => token,
            Err(err) => {
                if let Err(fehler) = self
                    .app
                    .throttle
                    .increment(&client_key, ThrottleAction::Erneuerung, jetzt)
                    .await
                {
                    warn!(fehler = %fehler, "der Erneuerungsversuch konnte nicht gezählt werden");
                }
                return Err(Report::new(err).change_context(ErneuerungFehler::SessionUngültig));
            }
        };

        let ausgestellt = self
            .app
            .execute(
                AuthContext::Anonymous,
                Erneuerung {
                    refresh_token,
                    client_key,
                },
            )
            .await?;
        Ok(ausgestellt)
    }

    pub async fn abmelden(&self, auth: AuthContext) -> ResultReport<(), AbmeldungFehler> {
        self.app.execute(auth, Abmeldung).await
    }

    pub async fn passwort_vergessen(
        &self,
        anfrage: requests::PasswortVergessenRequest,
        client_key: ClientKey,
    ) -> ResultReport<(), PasswortVergessenFehler> {
        let use_case: PasswortVergessen = anfrage.in_use_case(client_key)?;
        self.app.execute(AuthContext::Anonymous, use_case).await
    }

    pub async fn passwort_wiederherstellen(
        &self,
        anfrage: requests::PasswortWiederherstellenRequest,
    ) -> ResultReport<(), PasswortWiederherstellenFehler> {
        let use_case: PasswortWiederherstellen = anfrage.in_use_case()?;
        self.app.execute(AuthContext::Anonymous, use_case).await
    }

    pub async fn benutzer_anzeigen(
        &self,
        auth: AuthContext,
    ) -> ResultReport<AuthenticatedBenutzer, ProfileReadError> {
        self.app
            .execute_fn(auth, async |ctx| {
                let benutzer_id = ctx
                    .auth()
                    .require_benutzer_id()
                    .change_context(ProfileReadError::Unauthorized)?;
                let uow = ctx
                    .enter()
                    .await
                    .change_context(ProfileReadError::Persistence)?;
                let ergebnis = async {
                    let benutzer = match uow.benutzer().find_by_id(benutzer_id).await {
                        Ok(b) => b.into_data(),
                        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                            return Err(Report::new(ProfileReadError::NotFound));
                        }
                        Err(err) => {
                            return Err(err).change_context(ProfileReadError::Persistence);
                        }
                    };
                    Ok(benutzer)
                }
                .await;
                let benutzer = uow.finish(ergebnis, ProfileReadError::Persistence).await?;
                Ok(benutzer.into())
            })
            .await
    }

    pub async fn benutzer_löschen(
        &self,
        auth: AuthContext,
        anfrage: requests::BenutzerLoeschungRequest,
    ) -> ResultReport<(), BenutzerLöschenFehler> {
        let use_case: BenutzerLöschen = anfrage.in_use_case()?;
        self.app.execute(auth, use_case).await
    }

    pub fn oidc_providers(&self) -> OidcProvidersResponse {
        OidcProvidersResponse {
            providers: self
                .app
                .oidc_client
                .configured_providers()
                .into_iter()
                .map(OidcProviderItem::from)
                .collect(),
        }
    }

    pub async fn oidc_start_login(
        &self,
        provider_raw: &str,
        return_to: Option<String>,
        client_key: ClientKey,
    ) -> ResultReport<String, OidcUseCaseError> {
        let provider_id = domain::konto::OidcProviderId::parse(provider_raw)
            .change_context(OidcUseCaseError::AnbieterNichtKonfiguriert)?;
        self.app
            .execute(
                AuthContext::Anonymous,
                OidcStartAuth {
                    provider_id,
                    mode: domain::konto::OidcFlowMode::Login,
                    return_to,
                    client_key,
                },
            )
            .await
    }

    pub async fn oidc_complete_callback(
        &self,
        provider_raw: &str,
        code: Option<String>,
        state: Option<String>,
        provider_error: Option<String>,
        apple_user_json: Option<&str>,
    ) -> ResultReport<String, OidcUseCaseError> {
        let provider_id = domain::konto::OidcProviderId::parse(provider_raw)
            .change_context(OidcUseCaseError::AnbieterNichtKonfiguriert)?;
        let display_name_hint = apple_user_json.and_then(parse_apple_user_display_name);
        let outcome = self
            .app
            .execute(
                AuthContext::Anonymous,
                OidcCompleteCallback {
                    provider_id,
                    code,
                    state,
                    provider_error,
                    display_name_hint,
                },
            )
            .await?;
        Ok(outcome.redirect_url)
    }

    pub async fn oidc_exchange(
        &self,
        anfrage: requests::OidcExchangeRequest,
        client_key: ClientKey,
    ) -> ResultReport<SessionAusgestellt, OidcUseCaseError> {
        let use_case = anfrage.in_use_case(client_key)?;
        self.app.execute(AuthContext::Anonymous, use_case).await
    }

    pub async fn login_methods(
        &self,
        auth: AuthContext,
    ) -> ResultReport<LoginMethodsResponse, ProfileReadError> {
        let providers: Vec<OidcProviderItem> = self
            .app
            .oidc_client
            .configured_providers()
            .into_iter()
            .map(OidcProviderItem::from)
            .collect();
        let display_by_id: HashMap<String, String> = providers
            .iter()
            .map(|p| (p.id.clone(), p.display_name.clone()))
            .collect();

        self.app
            .execute_fn(auth, async move |ctx| {
                let benutzer_id = ctx
                    .auth()
                    .require_benutzer_id()
                    .change_context(ProfileReadError::Unauthorized)?;
                let uow = ctx
                    .enter()
                    .await
                    .change_context(ProfileReadError::Persistence)?;
                let res = async {
                    let konten = uow
                        .konten()
                        .find_by_benutzer(benutzer_id)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(ProfileReadError::Persistence)?;
                    let identities = uow
                        .external_identities()
                        .find_by_benutzer(benutzer_id)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(ProfileReadError::Persistence)?;
                    let external_identities = identities
                        .into_iter()
                        .map(|row| {
                            let id_str = row.provider_id().as_str().to_owned();
                            let provider_display_name = display_by_id
                                .get(&id_str)
                                .cloned()
                                .unwrap_or_else(|| id_str.clone());
                            LinkedExternalIdentity {
                                id: row.id().0,
                                provider_id: id_str,
                                provider_display_name,
                                provider_email: row
                                    .provider_email()
                                    .map(|e| AsRef::<str>::as_ref(e).to_owned()),
                                created_at: row.created_at(),
                            }
                        })
                        .collect();
                    Ok::<_, Report<ProfileReadError>>(LoginMethodsResponse {
                        has_password: !konten.is_empty(),
                        external_identities,
                        available_providers: providers,
                    })
                }
                .await;
                uow.finish(res, ProfileReadError::Persistence).await
            })
            .await
    }

    pub async fn oidc_start_account_action(
        &self,
        auth: AuthContext,
        provider_raw: &str,
        anfrage: requests::StartOidcAccountActionRequest,
        client_key: ClientKey,
    ) -> ResultReport<OidcAuthorizationRedirect, OidcUseCaseError> {
        let benutzer_id = auth
            .require_benutzer_id()
            .change_context(OidcUseCaseError::NichtAngemeldet)?;
        let provider_id = domain::konto::OidcProviderId::parse(provider_raw)
            .change_context(OidcUseCaseError::AnbieterNichtKonfiguriert)?;
        let use_case = anfrage.in_use_case(provider_id, benutzer_id, client_key);
        let authorization_url = self.app.execute(auth, use_case).await?;
        Ok(OidcAuthorizationRedirect { authorization_url })
    }

    pub async fn unlink_external_identity(
        &self,
        auth: AuthContext,
        identity_id: domain::konto::ExternalIdentityId,
    ) -> ResultReport<(), OidcUseCaseError> {
        self.app
            .execute(auth, UnlinkExternalIdentity { identity_id })
            .await
    }

    pub async fn set_initial_password(
        &self,
        auth: AuthContext,
        anfrage: requests::SetInitialPasswordRequest,
    ) -> ResultReport<(), PasswortErstmaligSetzenFehler> {
        let use_case = anfrage.in_use_case()?;
        self.app.execute(auth, use_case).await
    }

    // ---------------------------------------------------------------- Notifications

    pub async fn notifications_list(
        &self,
        auth: AuthContext,
    ) -> ResultReport<NotificationsInbox, NotificationReadError> {
        self.app
            .execute_fn(auth, async |ctx| {
                let viewer_id = ctx
                    .auth()
                    .require_benutzer_id()
                    .change_context(NotificationReadError::Unauthorized)?;
                let uow = ctx
                    .enter()
                    .await
                    .change_context(NotificationReadError::Persistence)?;
                let res = async {
                    let raw = uow
                        .benachrichtigungen()
                        .find_by_empfänger(viewer_id)
                        .await
                        .map_err(IntoReport::into_report)
                        .change_context(NotificationReadError::Persistence)?;

                    let mut game_cache: HashMap<GameId, (Game, HashMap<BenutzerId, PlayerBadge>)> =
                        HashMap::new();
                    let mut capture_cache: HashMap<domain::CaptureId, Capture> = HashMap::new();
                    let mut views = Vec::with_capacity(raw.len());
                    let mut unread_count = 0u32;

                    for item in raw {
                        let n = item.into_data();
                        if let std::collections::hash_map::Entry::Vacant(e) =
                            game_cache.entry(n.game_id())
                        {
                            match uow.games().find_by_id(n.game_id()).await {
                                Ok(g) => {
                                    let g = g.into_data();
                                    let ids = g.sorted_player_ids();
                                    let users = uow
                                        .benutzer()
                                        .find_by_ids(&ids)
                                        .await
                                        .map_err(IntoReport::into_report)
                                        .change_context(NotificationReadError::Persistence)?;
                                    let badges = build_badge_map(
                                        &g,
                                        users.into_iter().map(|b| b.into_data()),
                                    );
                                    e.insert((g, badges));
                                }
                                Err(err)
                                    if matches!(
                                        err.current_context(),
                                        RepositoryError::NotFound
                                    ) => {}
                                Err(err) => {
                                    return Err(err)
                                        .change_context(NotificationReadError::Persistence);
                                }
                            }
                        }

                        let Some((game, badges)) = game_cache.get(&n.game_id()) else {
                            continue;
                        };

                        let mut actor = n.actor_id().and_then(|aid| badges.get(&aid).cloned());
                        let mut target = n.target_id().and_then(|tid| badges.get(&tid).cloned());
                        let mut prompt_text = None;
                        let mut is_blurred = false;
                        let mut capture_id = n.capture_id().map(|c| c.0);

                        if n.art() == BenachrichtigungArt::AufnahmeHochgeladen
                            && let Some(cid) = n.capture_id()
                        {
                            if let std::collections::hash_map::Entry::Vacant(entry) =
                                capture_cache.entry(cid)
                            {
                                match uow.captures().find_by_id(cid).await {
                                    Ok(c) => {
                                        entry.insert(c.into_data());
                                    }
                                    Err(err)
                                        if matches!(
                                            err.current_context(),
                                            RepositoryError::NotFound
                                        ) => {}
                                    Err(err) => {
                                        return Err(err)
                                            .change_context(NotificationReadError::Persistence);
                                    }
                                }
                            }
                            if let Some(cap) = capture_cache.get(&cid) {
                                if !can_view_blurred_capture(game, cap, viewer_id) {
                                    // Never expose Undercover capture details before reveal.
                                    actor = None;
                                    target = None;
                                    capture_id = None;
                                    is_blurred = true;
                                } else if can_view_full_capture(game, cap, viewer_id) {
                                    prompt_text = Some(cap.prompt_text().as_str().to_owned());
                                } else {
                                    target = None;
                                    is_blurred = true;
                                }
                            } else if !matches!(game, Game::Completed(_)) {
                                // Fail closed by game visibility mode if capture cannot be loaded.
                                match game.visibility_mode() {
                                    domain::VisibilityMode::Undercover => {
                                        actor = None;
                                        target = None;
                                        capture_id = None;
                                        is_blurred = true;
                                    }
                                    domain::VisibilityMode::BlurredTeaser => {
                                        target = None;
                                        is_blurred = true;
                                    }
                                    domain::VisibilityMode::LiveFeed => {}
                                }
                            }
                        }

                        if n.ist_ungelesen() {
                            unread_count += 1;
                        }

                        views.push(NotificationView {
                            id: n.id().0,
                            kind: n.art().into(),
                            game_id: n.game_id().0,
                            game_title: game.title().as_str().to_owned(),
                            capture_id,
                            actor,
                            target,
                            prompt_text,
                            is_blurred,
                            created_at: n.erstellt_am(),
                            read_at: n.gelesen_am(),
                        });
                    }

                    Ok::<_, Report<NotificationReadError>>(NotificationsInbox {
                        unread_count,
                        notifications: views,
                    })
                }
                .await;
                uow.finish(res, NotificationReadError::Persistence).await
            })
            .await
    }

    pub async fn notification_mark_read(
        &self,
        auth: AuthContext,
        notification_id: BenachrichtigungId,
    ) -> ResultReport<(), BenachrichtigungUseCaseError> {
        self.app
            .execute(
                auth,
                BenachrichtigungAlsGelesenMarkieren {
                    benachrichtigung_id: notification_id,
                },
            )
            .await
    }

    pub async fn notifications_mark_all_read(
        &self,
        auth: AuthContext,
    ) -> ResultReport<(), BenachrichtigungUseCaseError> {
        self.app
            .execute(auth, AlleBenachrichtigungenAlsGelesenMarkieren)
            .await
    }

    pub async fn notification_preferences_get(
        &self,
        auth: AuthContext,
    ) -> ResultReport<NotificationPreferences, NotificationReadError> {
        self.app
            .execute_fn(auth, async |ctx| {
                let user_id = ctx
                    .auth()
                    .require_benutzer_id()
                    .change_context(NotificationReadError::Unauthorized)?;
                let now = ctx.clock().now();
                let uow = ctx
                    .enter()
                    .await
                    .change_context(NotificationReadError::Persistence)?;
                let res = async {
                    let prefs = match uow.benutzer().find_by_id(user_id).await {
                        Ok(u) => u.notification_preferences(),
                        Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                            BenachrichtigungsEinstellungen::standard(now)
                        }
                        Err(err) => {
                            return Err(err).change_context(NotificationReadError::Persistence);
                        }
                    };
                    Ok::<_, Report<NotificationReadError>>(NotificationPreferences::from(prefs))
                }
                .await;
                uow.finish(res, NotificationReadError::Persistence).await
            })
            .await
    }

    pub async fn notification_preferences_update(
        &self,
        auth: AuthContext,
        req: requests::UpdateNotificationPreferencesRequest,
    ) -> ResultReport<NotificationPreferences, BenachrichtigungUseCaseError> {
        let uc = BenachrichtigungsEinstellungenAktualisieren::from(req);
        let updated = self.app.execute(auth, uc).await?;
        Ok(NotificationPreferences::from(updated))
    }

    pub async fn notification_unsubscribe(
        &self,
        req: requests::UnsubscribeNotificationRequest,
    ) -> ResultReport<UnsubscribeOutcome, BenachrichtigungUseCaseError> {
        let art = self
            .app
            .execute(
                AuthContext::Anonymous,
                BenachrichtigungAbbestellenMitToken { token: req.token },
            )
            .await?;
        Ok(UnsubscribeOutcome {
            ok: true,
            kind: art.into(),
        })
    }
}

fn game_to_summary(
    game: &Game,
    viewer_id: Option<BenutzerId>,
    my_captured_tiles: Option<u32>,
    winner: Option<PlayerBadge>,
    server_now: DateTime<Utc>,
) -> GameSummary {
    let (started_at, ended_at) = match game {
        Game::Lobby(_) | Game::Drafting(_) => (None, None),
        Game::Active(a) => (Some(a.started_at()), None),
        Game::Completed(c) => (Some(c.started_at()), Some(c.ended_at())),
    };
    let (my_draft_filled, my_draft_locked, ready_player_count, draft_closes_at) = match game {
        Game::Drafting(d) => (
            viewer_id.map(|uid| d.prompts_by_author(uid).len() as u32),
            viewer_id.map(|uid| d.is_draft_locked(uid)),
            Some(d.draft_locked().len() as u32),
            d.draft_closes_at(),
        ),
        _ => (None, None, None, None),
    };
    let needs_action = match (game, viewer_id) {
        (Game::Lobby(l), Some(uid)) => l.host_user_id() == uid && l.players().len() >= MIN_PLAYERS,
        (Game::Drafting(d), Some(uid)) => !d.is_draft_locked(uid),
        _ => false,
    };

    GameSummary {
        id: game.id().0,
        invite_code: game.invite_code().as_str().to_owned(),
        title: game.title().as_str().to_owned(),
        grid_size: game.grid_size() as u8,
        visibility_mode: game.visibility_mode().into(),
        status: game.into(),
        watching: viewer_id
            .and_then(|uid| game.player_watching(uid))
            .unwrap_or(true),
        playable_tile_count: game.playable_tile_count() as u32,
        player_count: game.players().len() as u32,
        my_captured_tiles,
        my_draft_filled,
        my_draft_locked,
        ready_player_count,
        needs_action,
        draft_closes_at,
        draft_deadline_at: game.draft_deadline_at(),
        ends_at: game.ends_at(),
        server_now,
        winner,
        started_at,
        ended_at,
        completion_reason: game.completion_reason().map(Into::into),
        created_at: game.created_at(),
    }
}

fn awards_to_superlatives(
    awards: GameAwards,
    badges: &HashMap<BenutzerId, PlayerBadge>,
) -> Vec<SuperlativeView> {
    let entries = [
        (
            SuperlativeKind::MostWanted,
            awards.most_wanted.user_id,
            awards.most_wanted.metric_value,
            None,
        ),
        (
            SuperlativeKind::Ghost,
            awards.ghost.user_id,
            awards.ghost.metric_value,
            None,
        ),
        (
            SuperlativeKind::QuickestDraw,
            awards.quickest_draw.user_id,
            awards.quickest_draw.metric_value,
            Some(awards.quickest_draw.capture_id.0),
        ),
        (
            SuperlativeKind::Sniper,
            awards.sniper.user_id,
            awards.sniper.metric_value,
            None,
        ),
    ];
    entries
        .into_iter()
        .filter_map(|(kind, user_id, metric_value, capture_id)| {
            badges.get(&user_id).cloned().map(|user| SuperlativeView {
                kind,
                user,
                metric_value,
                capture_id,
            })
        })
        .collect()
}

fn badge_from_benutzer(
    benutzer: &domain::Benutzer,
    draft_locked: bool,
    is_host: bool,
) -> PlayerBadge {
    let effective_title = benutzer.effective_title().to_owned();
    PlayerBadge {
        id: benutzer.id().0,
        display_name: benutzer.display_name().as_str().to_owned(),
        avatar_storage_key: benutzer.avatar_storage_key().map(str::to_owned),
        gold_medal_count: benutzer.gold_medal_count(),
        effective_title,
        draft_locked,
        is_host,
    }
}

fn build_badge_map(
    game: &Game,
    users: impl Iterator<Item = domain::Benutzer>,
) -> HashMap<BenutzerId, PlayerBadge> {
    let user_map: HashMap<BenutzerId, domain::Benutzer> = users.map(|b| (b.id(), b)).collect();
    let draft_locked = match game {
        Game::Drafting(d) => Some(d.draft_locked()),
        Game::Lobby(_) | Game::Active(_) | Game::Completed(_) => None,
    };
    let mut out = HashMap::new();
    for player in game.players() {
        let is_locked = draft_locked.is_some_and(|set| set.contains(&player.user_id));
        if let Some(b) = user_map.get(&player.user_id) {
            out.insert(
                player.user_id,
                badge_from_benutzer(b, is_locked, game.host_user_id() == player.user_id),
            );
        } else {
            out.insert(
                player.user_id,
                PlayerBadge {
                    id: player.user_id.0,
                    display_name: "Spieler".to_owned(),
                    avatar_storage_key: None,
                    gold_medal_count: 0,
                    effective_title: "Neuling".to_owned(),
                    draft_locked: is_locked,
                    is_host: game.host_user_id() == player.user_id,
                },
            );
        }
    }
    out
}

fn capture_to_feed_item(
    game: &Game,
    badges: &HashMap<BenutzerId, PlayerBadge>,
    cap: &Capture,
    viewer_id: BenutzerId,
) -> Option<CaptureFeedItem> {
    let can_full = can_view_full_capture(game, cap, viewer_id);
    let can_blurred = can_view_blurred_capture(game, cap, viewer_id);

    if !can_blurred {
        return None;
    }

    let catcher = badges.get(&cap.catcher_user_id())?.clone();
    if can_full {
        Some(CaptureFeedItem {
            id: cap.id().0,
            tile_id: cap.tile_id().0,
            catcher,
            target: badges.get(&cap.target_user_id()).cloned(),
            prompt_text: Some(cap.prompt_text().as_str().to_owned()),
            image_storage_key: Some(cap.image_storage_key().to_owned()),
            thumbnail_storage_key: cap.thumbnail_storage_key().to_owned(),
            captured_at: cap.captured_at(),
            is_blurred: false,
        })
    } else {
        // BlurredTeaser mode during active play: reveal teaser thumbnail & timestamp without target/prompt!
        Some(CaptureFeedItem {
            id: cap.id().0,
            tile_id: cap.tile_id().0,
            catcher,
            target: None,
            prompt_text: None,
            image_storage_key: None,
            thumbnail_storage_key: cap.thumbnail_storage_key().to_owned(),
            captured_at: cap.captured_at(),
            is_blurred: true,
        })
    }
}

fn profile_to_schema(
    benutzer: &domain::Benutzer,
    games_played: u32,
    captures_made: u32,
    times_caught: u32,
) -> UserProfile {
    UserProfile {
        id: benutzer.id().0,
        display_name: benutzer.display_name().as_str().to_owned(),
        avatar_storage_key: benutzer.avatar_storage_key().map(str::to_owned),
        gold_medal_count: benutzer.gold_medal_count(),
        equipped_title: benutzer.equipped_title().map(str::to_owned),
        effective_title: benutzer.effective_title().to_owned(),
        unlocked_titles: benutzer
            .unlocked_titles()
            .into_iter()
            .map(str::to_owned)
            .collect(),
        games_played,
        captures_made,
        times_caught,
    }
}

fn parse_apple_user_display_name(raw_json: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct AppleName {
        #[serde(rename = "firstName")]
        first_name: Option<String>,
        #[serde(rename = "lastName")]
        last_name: Option<String>,
    }
    #[derive(serde::Deserialize)]
    struct AppleUser {
        name: Option<AppleName>,
    }
    let parsed: AppleUser = serde_json::from_str(raw_json).ok()?;
    let name = parsed.name?;
    let parts: Vec<&str> = [name.first_name.as_deref(), name.last_name.as_deref()]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    (!parts.is_empty()).then(|| parts.join(" "))
}
