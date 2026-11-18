//! The HTTP edge: one thin `#[OpenApi]` wrapper over [`FriendarazziAppApi`].

use error_stack::ResultExt;
use friendarazzi_core::{
    App, ResultReport,
    application::AuthContext,
    domain::{BenutzerId, BoardTileId, GameId},
    service::{
        AbmeldungFehler, BenutzerLöschenFehler, CaptureUseCaseError,
        PasswortWiederherstellenFehler, ProfileUseCaseError,
    },
};
use poem::web::RealIp;
use poem_openapi::{
    Multipart, OpenApi, OpenApiService, ServerObject, Tags,
    param::{Cookie, Header, Path},
    payload::Json,
    types::multipart::Upload,
};
use std::sync::{Arc, RwLock, atomic::AtomicBool};
use uuid::Uuid;

mod auth;
mod cookie;
mod extractors;
mod health;
pub mod jwt;
mod responses;
use extractors::*;
pub use responses::*;

use crate::{
    api::{
        FriendarazziAppApi,
        requests::{
            AnmeldungRequest, BenutzerLoeschungRequest, CreateGameRequest, JoinGameRequest,
            PasswortVergessenRequest, PasswortWiederherstellenRequest,
            RegistrierungAbschlussRequest, RegistrierungStartRequest, SaveDraftPromptsRequest,
            UpdateGameSettingsRequest, UpdateProfileRequest,
        },
    },
    schema::{
        AuthenticatedBenutzer, CaptureOutcome, GameDetail, GameSummary, MailHealth, Ok,
        StarterDeck, UserGallery, UserProfile,
    },
};

/// The sections the published API is grouped into.
#[derive(Tags)]
pub enum ApiTag {
    /// Friendarazzi game lobbies, drafting, and active play.
    Games,
    /// Photo proof uploads and media streaming.
    Captures,
    /// Player profiles, Gold Medals, and photo galleries.
    Profiles,
    /// Curated Quick-Start starter decks.
    Decks,
    /// Registration, sign-in, session, password recovery, and account settings.
    Konto,
    /// Liveness of the service.
    Betrieb,
}

/// Multipart form payload containing a single uploaded photo file.
#[derive(Multipart)]
pub struct PhotoUploadMultipart {
    /// Uploaded image file (JPEG, PNG, or WebP).
    pub file: Upload,
}

pub struct FriendarazziApiSpec {
    app_api: FriendarazziAppApi,
    mail_health: Arc<RwLock<MailHealth>>,
    mail_probe_running: Arc<AtomicBool>,
}

impl FriendarazziApiSpec {
    pub fn new(app_api: FriendarazziAppApi) -> Self {
        Self {
            app_api,
            mail_health: Arc::default(),
            mail_probe_running: Arc::default(),
        }
    }
}

impl From<FriendarazziAppApi> for FriendarazziApiSpec {
    fn from(app_api: FriendarazziAppApi) -> Self {
        Self::new(app_api)
    }
}

impl From<App> for FriendarazziApiSpec {
    fn from(app: App) -> Self {
        Self::new(FriendarazziAppApi::new(app))
    }
}

#[OpenApi]
impl FriendarazziApiSpec {
    /// Report that the service is up.
    ///
    /// Returns `OK` when the service is running.
    #[oai(path = "/health", method = "get", tag = "ApiTag::Betrieb")]
    async fn health(&self) -> Json<String> {
        Json("OK".to_string())
    }

    /// Report the latest result from the mail relay check.
    ///
    /// The response describes the latest probe result, or `Unknown` before the first probe.
    #[oai(path = "/health/mail", method = "get", tag = "ApiTag::Betrieb")]
    async fn health_mail(&self) -> Json<MailHealth> {
        Json(self.mail_health())
    }

    // ------------------------------------------------------------------ decks

    /// List curated Quick-Start starter decks.
    ///
    /// Returns themed prompt collections used during the drafting phase.
    #[oai(path = "/decks", method = "get", tag = "ApiTag::Decks")]
    async fn decks_list(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
    ) -> TypicalJsonResponse<Vec<StarterDeck>> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        self.app_api.decks_list(auth).await.into()
    }

    // ------------------------------------------------------------------ games

    /// List all games the caller participates in.
    ///
    /// Sorted newest first, including active lobbies, ongoing hunts, and completed games.
    #[oai(path = "/games", method = "get", tag = "ApiTag::Games")]
    async fn games_list(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
    ) -> TypicalJsonResponse<Vec<GameSummary>> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        self.app_api.games_list(auth).await.into()
    }

    /// Create a new Friendarazzi game lobby.
    ///
    /// The caller becomes the host and first player of the lobby.
    #[oai(path = "/games", method = "post", tag = "ApiTag::Games")]
    async fn game_create(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        body: Json<CreateGameRequest>,
    ) -> TypicalJsonResponse<GameDetail> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        self.app_api.game_create(auth, body.0).await.into()
    }

    /// Look up a game summary by invite code.
    ///
    /// Used by `/join` invite links to preview lobby details before joining.
    #[oai(
        path = "/games/by-code/:invite_code",
        method = "get",
        tag = "ApiTag::Games"
    )]
    async fn game_by_invite_code(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        invite_code: Path<String>,
    ) -> TypicalJsonResponse<GameSummary> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        self.app_api
            .game_by_invite_code(auth, invite_code.0)
            .await
            .into()
    }

    /// Join an existing game lobby using its invite code.
    ///
    /// Joining an already-joined lobby is idempotent and returns the game state.
    #[oai(path = "/games/join", method = "post", tag = "ApiTag::Games")]
    async fn game_join(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        body: Json<JoinGameRequest>,
    ) -> TypicalJsonResponse<GameDetail> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        self.app_api.game_join(auth, body.0).await.into()
    }

    /// Read full adaptive state for a game.
    ///
    /// Adapts payload contents to the game's current status and visibility mode.
    #[oai(path = "/games/:game_id", method = "get", tag = "ApiTag::Games")]
    async fn game_detail(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        game_id: Path<Uuid>,
    ) -> TypicalJsonResponse<GameDetail> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        self.app_api
            .game_detail(auth, GameId(game_id.0))
            .await
            .into()
    }

    /// Update lobby settings before drafting begins.
    ///
    /// Host only; allows configuring title, grid size, and visibility mode.
    #[oai(
        path = "/games/:game_id/settings",
        method = "put",
        tag = "ApiTag::Games"
    )]
    async fn game_update_settings(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        game_id: Path<Uuid>,
        body: Json<UpdateGameSettingsRequest>,
    ) -> TypicalJsonResponse<GameDetail> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        self.app_api
            .game_update_settings(auth, GameId(game_id.0), body.0)
            .await
            .into()
    }

    /// Transition a game from Lobby to Drafting.
    ///
    /// Host only; requires at least 3 players in the lobby.
    #[oai(
        path = "/games/:game_id/start-drafting",
        method = "post",
        tag = "ApiTag::Games"
    )]
    async fn game_start_drafting(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        game_id: Path<Uuid>,
    ) -> TypicalJsonResponse<GameDetail> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        self.app_api
            .game_start_drafting(auth, GameId(game_id.0))
            .await
            .into()
    }

    /// Save or replace the caller's draft prompts for a game.
    ///
    /// Rejects self-targeted prompts and resets the caller's draft lock.
    #[oai(
        path = "/games/:game_id/prompts",
        method = "put",
        tag = "ApiTag::Games"
    )]
    async fn game_save_prompts(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        game_id: Path<Uuid>,
        body: Json<SaveDraftPromptsRequest>,
    ) -> TypicalJsonResponse<GameDetail> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        self.app_api
            .game_save_prompts(auth, GameId(game_id.0), body.0)
            .await
            .into()
    }

    /// Lock in the caller's draft prompts and start active play when all players are locked.
    ///
    /// Validates equal-distribution quotas and generates conflict-free Bingo boards once everyone locks.
    #[oai(
        path = "/games/:game_id/prompts/lock",
        method = "post",
        tag = "ApiTag::Games"
    )]
    async fn game_lock_prompts(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        game_id: Path<Uuid>,
    ) -> TypicalJsonResponse<GameDetail> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        self.app_api
            .game_lock_prompts(auth, GameId(game_id.0))
            .await
            .into()
    }

    // ------------------------------------------------------------------ captures & media

    /// Upload photo proof for a tile on the caller's Bingo board.
    ///
    /// Compresses the photo, generates a 64px blurred teaser, evaluates Bingo win atomically, and awards a Gold Medal on victory.
    #[oai(
        path = "/games/:game_id/tiles/:tile_id/capture",
        method = "post",
        tag = "ApiTag::Captures"
    )]
    async fn capture_submit(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        game_id: Path<Uuid>,
        tile_id: Path<Uuid>,
        multipart: PhotoUploadMultipart,
    ) -> TypicalJsonResponse<CaptureOutcome> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        let bytes = match multipart
            .file
            .into_vec()
            .await
            .change_context(CaptureUseCaseError::InvalidImage)
        {
            Ok(b) => b,
            Err(err) => {
                return TypicalJsonResponse::from(Err(err));
            }
        };
        self.app_api
            .capture_submit(auth, GameId(game_id.0), BoardTileId(tile_id.0), bytes)
            .await
            .into()
    }

    /// Retrieve an uploaded capture photo, blurred teaser, or user avatar.
    ///
    /// Enforces game membership and visibility-mode rules before streaming the image.
    #[oai(path = "/media/:storage_key", method = "get", tag = "ApiTag::Captures")]
    async fn media_get(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        storage_key: Path<String>,
    ) -> StreamImageResponse {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        self.app_api.media_get(auth, storage_key.0).await.into()
    }

    // ------------------------------------------------------------------ profiles & galleries

    /// Read a user's public profile, Gold Medals, and statistics.
    ///
    /// Returns rank titles unlocked by Gold Medals and lifetime capture counts.
    #[oai(path = "/users/:user_id", method = "get", tag = "ApiTag::Profiles")]
    async fn profile_get(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        user_id: Path<Uuid>,
    ) -> TypicalJsonResponse<UserProfile> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        self.app_api
            .profile_get(auth, BenutzerId(user_id.0))
            .await
            .into()
    }

    /// Read a user's Caught and Captured photo galleries grouped by game.
    ///
    /// Respects active visibility mode restrictions so undercover captures are never leaked early.
    #[oai(
        path = "/users/:user_id/gallery",
        method = "get",
        tag = "ApiTag::Profiles"
    )]
    async fn user_gallery(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        user_id: Path<Uuid>,
    ) -> TypicalJsonResponse<UserGallery> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        self.app_api
            .user_gallery(auth, BenutzerId(user_id.0))
            .await
            .into()
    }

    /// Update the authenticated caller's display name and equipped rank title.
    ///
    /// Rejects locked rank titles.
    #[oai(path = "/profil", method = "put", tag = "ApiTag::Profiles")]
    async fn profile_update(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        body: Json<UpdateProfileRequest>,
    ) -> TypicalJsonResponse<UserProfile> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        self.app_api.profile_update(auth, body.0).await.into()
    }

    /// Upload and set the authenticated caller's avatar image.
    ///
    /// Normalizes the image to a 400x400 WebP stored in the object store.
    #[oai(path = "/profil/avatar", method = "post", tag = "ApiTag::Profiles")]
    async fn profile_upload_avatar(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        multipart: PhotoUploadMultipart,
    ) -> TypicalJsonResponse<UserProfile> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        let bytes = match multipart
            .file
            .into_vec()
            .await
            .change_context(ProfileUseCaseError::BadRequest)
        {
            Ok(b) => b,
            Err(err) => {
                return TypicalJsonResponse::from(Err(err));
            }
        };
        self.app_api.profile_upload_avatar(auth, bytes).await.into()
    }

    // ------------------------------------------------------------------ konto

    /// Registration step 1: request a confirmation mail for an e-mail address.
    ///
    /// Returns the same response whether the address is free or already registered.
    #[oai(path = "/registrierung/start", method = "post", tag = "ApiTag::Konto")]
    async fn registrierung_starten(
        &self,
        real_ip: RealIp,
        #[oai(name = "User-Agent")] user_agent: Header<Option<String>>,
        #[oai(name = "Accept-Language")] accept_language: Header<Option<String>>,
        body: Json<RegistrierungStartRequest>,
    ) -> UncachedJsonResponse<Ok> {
        let client_key = client_key(
            real_ip,
            user_agent.0.as_deref(),
            accept_language.0.as_deref(),
        );
        UncachedJsonResponse::from(
            self.app_api
                .registrierung_starten(body.0, client_key)
                .await
                .map(|()| Ok { ok: true }),
        )
    }

    /// Complete registration with the token from the confirmation email.
    ///
    /// Returns the new session credentials.
    #[oai(
        path = "/registrierung/abschluss",
        method = "post",
        tag = "ApiTag::Konto"
    )]
    async fn registrierung_abschließen(
        &self,
        real_ip: RealIp,
        #[oai(name = "User-Agent")] user_agent: Header<Option<String>>,
        #[oai(name = "Accept-Language")] accept_language: Header<Option<String>>,
        body: Json<RegistrierungAbschlussRequest>,
    ) -> NewSessionResponse {
        let client_key = client_key(
            real_ip,
            user_agent.0.as_deref(),
            accept_language.0.as_deref(),
        );
        NewSessionResponse::from(
            self.app_api
                .registrierung_abschließen(body.0, client_key)
                .await
                .map(|ausgestellt| {
                    jwt::encode_session(ausgestellt, self.app_api.inner_app().signing.as_ref())
                        .expect("the JOSE header serialises")
                }),
        )
    }

    /// Sign in with e-mail address and password.
    ///
    /// Returns the new session credentials.
    #[oai(path = "/anmeldung", method = "post", tag = "ApiTag::Konto")]
    async fn anmelden(
        &self,
        real_ip: RealIp,
        #[oai(name = "User-Agent")] user_agent: Header<Option<String>>,
        #[oai(name = "Accept-Language")] accept_language: Header<Option<String>>,
        body: Json<AnmeldungRequest>,
    ) -> NewSessionResponse {
        let client_key = client_key(
            real_ip,
            user_agent.0.as_deref(),
            accept_language.0.as_deref(),
        );
        NewSessionResponse::from(self.app_api.anmelden(body.0, client_key).await.map(
            |ausgestellt| {
                jwt::encode_session(ausgestellt, self.app_api.inner_app().signing.as_ref())
                    .expect("the JOSE header serialises")
            },
        ))
    }

    /// Renew the current session.
    ///
    /// Returns a new session when the refresh token is valid.
    #[oai(path = "/session/erneuern", method = "post", tag = "ApiTag::Konto")]
    async fn session_erneuern(
        &self,
        real_ip: RealIp,
        #[oai(name = "User-Agent")] user_agent: Header<Option<String>>,
        #[oai(name = "Accept-Language")] accept_language: Header<Option<String>>,
        #[oai(name = "__Host-friendarazzi_refresh")] refresh: Cookie<Option<String>>,
    ) -> NewSessionResponse {
        let client_key = client_key(
            real_ip,
            user_agent.0.as_deref(),
            accept_language.0.as_deref(),
        );
        let ergebnis = self
            .app_api
            .session_erneuern(refresh.0.unwrap_or_default(), client_key)
            .await;
        NewSessionResponse::from(ergebnis.map(|ausgestellt| {
            jwt::encode_session(ausgestellt, self.app_api.inner_app().signing.as_ref())
                .expect("the JOSE header serialises")
        }))
    }

    /// End the current session.
    ///
    /// The current session is ended.
    #[oai(path = "/session/abmelden", method = "post", tag = "ApiTag::Konto")]
    async fn abmelden(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
    ) -> EndSessionResponse<Ok> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        let ergebnis: ResultReport<(), AbmeldungFehler> = self.app_api.abmelden(auth).await;
        EndSessionResponse::from(ergebnis.map(|()| Ok { ok: true }))
    }

    /// Request a password reset mail for an e-mail address.
    ///
    /// Returns the same response whether the address belongs to an account or not.
    #[oai(path = "/passwort/vergessen", method = "post", tag = "ApiTag::Konto")]
    async fn passwort_vergessen(
        &self,
        real_ip: RealIp,
        #[oai(name = "User-Agent")] user_agent: Header<Option<String>>,
        #[oai(name = "Accept-Language")] accept_language: Header<Option<String>>,
        body: Json<PasswortVergessenRequest>,
    ) -> UncachedJsonResponse<Ok> {
        let client_key = client_key(
            real_ip,
            user_agent.0.as_deref(),
            accept_language.0.as_deref(),
        );
        UncachedJsonResponse::from(
            self.app_api
                .passwort_vergessen(body.0, client_key)
                .await
                .map(|()| Ok { ok: true }),
        )
    }

    /// Set a new password from a mailed reset token.
    ///
    /// The reset token can be used once. All existing sessions end when the password changes.
    #[oai(
        path = "/passwort/wiederherstellen",
        method = "post",
        tag = "ApiTag::Konto"
    )]
    async fn passwort_wiederherstellen(
        &self,
        body: Json<PasswortWiederherstellenRequest>,
    ) -> EndSessionResponse<Ok> {
        let ergebnis: ResultReport<(), PasswortWiederherstellenFehler> =
            self.app_api.passwort_wiederherstellen(body.0).await;
        EndSessionResponse::from(ergebnis.map(|()| Ok { ok: true }))
    }

    /// Read the authenticated caller, including profile summary and Gold Medals.
    ///
    /// Requires a valid sign-in.
    #[oai(path = "/benutzer", method = "get", tag = "ApiTag::Konto")]
    async fn benutzer_anzeigen(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
    ) -> UncachedJsonResponse<AuthenticatedBenutzer> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        UncachedJsonResponse::from(self.app_api.benutzer_anzeigen(auth).await)
    }

    /// Delete the caller's own Benutzer.
    ///
    /// The caller confirms the deletion by re-entering their password.
    #[oai(path = "/benutzer/loeschung", method = "post", tag = "ApiTag::Konto")]
    async fn benutzer_löschen(
        &self,
        #[oai(name = "Authorization")] authorization: RequiredBearerAuthorization,
        body: Json<BenutzerLoeschungRequest>,
    ) -> EndSessionResponse<Ok> {
        let auth = self
            .auth_context(authorization.0.as_deref())
            .await
            .unwrap_or(AuthContext::Anonymous);
        let ergebnis: ResultReport<(), BenutzerLöschenFehler> =
            self.app_api.benutzer_löschen(auth, body.0).await;
        EndSessionResponse::from(ergebnis.map(|()| Ok { ok: true }))
    }
}

pub fn openapi_service(
    app: App,
    server_urls: impl IntoIterator<Item = impl Into<ServerObject>>,
) -> OpenApiService<FriendarazziApiSpec, ()> {
    openapi_service_from_api(FriendarazziAppApi::new(app), server_urls)
}

pub fn openapi_service_from_api(
    app_api: FriendarazziAppApi,
    server_urls: impl IntoIterator<Item = impl Into<ServerObject>>,
) -> OpenApiService<FriendarazziApiSpec, ()> {
    let mut service = OpenApiService::new(
        FriendarazziApiSpec::from(app_api),
        "Friendarazzi API",
        env!("CARGO_PKG_VERSION"),
    )
    .summary("Social photo-hunting Bingo API: lobbies, drafting, captures, and galleries.")
    .description(include_str!("README.md"));
    for server_url in server_urls {
        service = service.server(server_url);
    }
    service
}
