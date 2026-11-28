//! Game lobby, drafting quotas, balanced board generation, and TypeState lifecycle transitions.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use rand::{
    Rng,
    seq::{IndexedRandom, SliceRandom},
};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    ResultReport,
    domain::{
        BenutzerId, NonEmptyString,
        board::{Board, BoardTileAssignment, NewBoard},
        capture::{Capture, CaptureId},
        deck::StarterDeck,
    },
};

const CONSTRUCTING: &str = "while constructing game";

/// Minimum number of players required to start drafting and guarantee `author != owner && target != owner`.
pub const MIN_PLAYERS: usize = 3;

/// Shared grace period once all players lock prompts before boards are dealt.
pub const READY_GRACE_PERIOD_SECONDS: i64 = 60;

/// Maximum visible character count allowed for a Bingo prompt submission.
pub const PROMPT_MAX_CHARS: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GameId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PromptSubmissionId(pub Uuid);

#[derive(Debug, Error)]
pub enum GameError {
    #[error("Der Spieltitel darf nicht leer sein")]
    EmptyTitle,
    #[error("Der Auftragstext darf nicht leer sein")]
    EmptyPromptText,
    #[error("Der Auftragstext darf maximal {PROMPT_MAX_CHARS} Zeichen lang sein")]
    PromptTooLong,
    #[error("Die Frist muss in der Zukunft liegen")]
    DeadlineInPast,
    #[error("Der Einladungscode ist ungültig")]
    InvalidInviteCode,
    #[error("Die Rastergröße muss 3, 4 oder 5 sein")]
    InvalidGridSize,
    #[error("Unbekannter Sichtbarkeitsmodus")]
    InvalidVisibilityMode,
    #[error("Unbekannte Prompt-Quelle")]
    InvalidPromptSource,
    #[error("Unbekannter Abschlussgrund")]
    InvalidCompletionReason,
    #[error("Diese Aktion ist im aktuellen Spielstatus nicht möglich")]
    InvalidStateTransition,
    #[error("Mindestens {MIN_PLAYERS} Spieler sind erforderlich")]
    TooFewPlayers,
    #[error("Du kannst keinen Auftrag über dich selbst schreiben")]
    SelfTargetNotAllowed,
    #[error("Das Ziel gehört nicht zu dieser Runde")]
    TargetNotInGame,
    #[error("Der Spieler gehört nicht zu dieser Runde")]
    PlayerNotInGame,
    #[error("Die eingereichten Aufträge entsprechen nicht der gleichmäßigen Verteilung")]
    InvalidPromptQuota,
    #[error("Die Spielbretter konnten nicht konfliktfrei zusammengestellt werden")]
    BoardAssemblyFailed,
    #[error("Das Spielbrett gehört nicht zu diesem Spiel oder hat kein Bingo")]
    BoardNotCompleted,
}

/// URL-safe invite code (uppercase alphanumeric, 4..=16 characters).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InviteCode(String);

impl InviteCode {
    pub fn new<S: AsRef<str>>(raw: S) -> Result<Self, GameError> {
        let trimmed = raw.as_ref().trim().to_ascii_uppercase();
        if trimmed.len() < 4
            || trimmed.len() > 16
            || !trimmed.chars().all(|c| c.is_ascii_alphanumeric())
        {
            return Err(GameError::InvalidInviteCode);
        }
        Ok(Self(trimmed))
    }

    /// Generate a 6-character invite code by sampling without replacement so no symbol repeats.
    pub fn generate(rng: &mut (impl Rng + ?Sized)) -> Self {
        const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let code: String = ALPHABET.sample(rng, 6).map(|&b| b as char).collect();
        Self(code)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for InviteCode {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GridSize {
    Three = 3,
    Four = 4,
    Five = 5,
}

impl GridSize {
    pub fn from_u8(val: u8) -> Result<Self, GameError> {
        match val {
            3 => Ok(Self::Three),
            4 => Ok(Self::Four),
            5 => Ok(Self::Five),
            _ => Err(GameError::InvalidGridSize),
        }
    }

    pub fn side(self) -> usize {
        self as usize
    }

    pub fn playable_tile_count(self) -> usize {
        let side = self.side();
        side * side
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisibilityMode {
    Undercover,
    LiveFeed,
    BlurredTeaser,
}

impl VisibilityMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Undercover => "Undercover",
            Self::LiveFeed => "LiveFeed",
            Self::BlurredTeaser => "BlurredTeaser",
        }
    }
}

impl std::str::FromStr for VisibilityMode {
    type Err = GameError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Undercover" => Ok(Self::Undercover),
            "LiveFeed" => Ok(Self::LiveFeed),
            "BlurredTeaser" => Ok(Self::BlurredTeaser),
            _ => Err(GameError::InvalidVisibilityMode),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptSource {
    Custom,
    StarterDeck,
}

impl PromptSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Custom => "Custom",
            Self::StarterDeck => "StarterDeck",
        }
    }
}

impl std::str::FromStr for PromptSource {
    type Err = GameError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Custom" => Ok(Self::Custom),
            "StarterDeck" => Ok(Self::StarterDeck),
            _ => Err(GameError::InvalidPromptSource),
        }
    }
}

/// How a completed game reached its final state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionReason {
    Bingo,
    Deadline,
    Abandoned,
}

impl CompletionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bingo => "Bingo",
            Self::Deadline => "Deadline",
            Self::Abandoned => "Abandoned",
        }
    }
}

impl FromStr for CompletionReason {
    type Err = GameError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Bingo" => Ok(Self::Bingo),
            "Deadline" => Ok(Self::Deadline),
            "Abandoned" => Ok(Self::Abandoned),
            _ => Err(GameError::InvalidCompletionReason),
        }
    }
}

/// Lifecycle state markers for [`GameIn`].
pub mod state {
    use std::collections::HashSet;

    use chrono::{DateTime, Utc};

    use super::{CompletionReason, PromptSubmission};
    use crate::domain::BenutzerId;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Lobby;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Drafting {
        pub prompts: Vec<PromptSubmission>,
        pub draft_locked: HashSet<BenutzerId>,
        pub draft_closes_at: Option<DateTime<Utc>>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Active {
        pub prompts: Vec<PromptSubmission>,
        pub started_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Completed {
        pub prompts: Vec<PromptSubmission>,
        pub started_at: DateTime<Utc>,
        pub ended_at: DateTime<Utc>,
        pub winner_user_id: Option<BenutzerId>,
        pub completion_reason: CompletionReason,
    }
}

/// Participation entity owned by a `Game` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GamePlayer {
    pub user_id: BenutzerId,
    pub joined_at: DateTime<Utc>,
    pub watching: bool,
}

/// Validated prompt text (`1..=PROMPT_MAX_CHARS` characters, trimmed).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PromptText(String);

impl PromptText {
    pub fn new<S: AsRef<str>>(raw: S) -> Result<Self, GameError> {
        let trimmed = raw.as_ref().trim();
        if trimmed.is_empty() {
            return Err(GameError::EmptyPromptText);
        }
        if trimmed.chars().count() > PROMPT_MAX_CHARS {
            return Err(GameError::PromptTooLong);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for PromptText {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PromptText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PromptText {
    type Err = GameError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl From<PromptText> for NonEmptyString {
    fn from(value: PromptText) -> Self {
        NonEmptyString::neu(value.0).expect("PromptText is guaranteed non-empty")
    }
}

/// One submitted prompt entity owned by a `Game` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptSubmission {
    pub id: PromptSubmissionId,
    pub author_user_id: BenutzerId,
    pub target_user_id: BenutzerId,
    pub text: PromptText,
    pub source: PromptSource,
}

impl PromptSubmission {
    pub fn new(
        id: PromptSubmissionId,
        author_user_id: BenutzerId,
        target_user_id: BenutzerId,
        text: PromptText,
        source: PromptSource,
    ) -> ResultReport<Self, GameError> {
        if author_user_id == target_user_id {
            return Err(Report::new(GameError::SelfTargetNotAllowed));
        }
        Ok(Self {
            id,
            author_user_id,
            target_user_id,
            text,
            source,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGame {
    pub invite_code: InviteCode,
    pub host_user_id: BenutzerId,
    pub host_watching: bool,
    pub title: NonEmptyString,
    pub grid_size: GridSize,
    pub visibility_mode: VisibilityMode,
    pub draft_deadline_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// A Friendarazzi social photo-hunting Bingo game parameterized by lifecycle state `S`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameIn<S> {
    id: GameId,
    invite_code: InviteCode,
    host_user_id: BenutzerId,
    title: NonEmptyString,
    grid_size: GridSize,
    visibility_mode: VisibilityMode,
    draft_deadline_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
    players: Vec<GamePlayer>,
    created_at: DateTime<Utc>,
    state: S,
}

pub type LobbyGame = GameIn<state::Lobby>;
pub type DraftingGame = GameIn<state::Drafting>;
pub type ActiveGame = GameIn<state::Active>;
pub type CompletedGame = GameIn<state::Completed>;

/// Sum of all valid game lifecycle states for code that handles any persisted `Game`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Game {
    Lobby(LobbyGame),
    Drafting(DraftingGame),
    Active(ActiveGame),
    Completed(CompletedGame),
}

impl From<LobbyGame> for Game {
    fn from(g: LobbyGame) -> Self {
        Self::Lobby(g)
    }
}

impl From<DraftingGame> for Game {
    fn from(g: DraftingGame) -> Self {
        Self::Drafting(g)
    }
}

impl From<ActiveGame> for Game {
    fn from(g: ActiveGame) -> Self {
        Self::Active(g)
    }
}

impl From<CompletedGame> for Game {
    fn from(g: CompletedGame) -> Self {
        Self::Completed(g)
    }
}

/// Result of evaluating whether a ready drafting game should start active play.
pub enum DraftStartOutcome {
    StillDrafting(DraftingGame),
    Started {
        game: ActiveGame,
        boards: Vec<NewBoard>,
    },
}

impl<S> GameIn<S> {
    pub fn id(&self) -> GameId {
        self.id
    }

    pub fn invite_code(&self) -> &InviteCode {
        &self.invite_code
    }

    pub fn host_user_id(&self) -> BenutzerId {
        self.host_user_id
    }

    pub fn title(&self) -> &NonEmptyString {
        &self.title
    }

    pub fn grid_size(&self) -> GridSize {
        self.grid_size
    }

    pub fn visibility_mode(&self) -> VisibilityMode {
        self.visibility_mode
    }

    pub fn draft_deadline_at(&self) -> Option<DateTime<Utc>> {
        self.draft_deadline_at
    }

    pub fn ends_at(&self) -> Option<DateTime<Utc>> {
        self.ends_at
    }

    /// Transfer host ownership to another player currently in this game.
    pub fn transfer_host(
        &mut self,
        actor: BenutzerId,
        new_host: BenutzerId,
    ) -> ResultReport<(), GameError> {
        if self.host_user_id != actor || !self.has_player(new_host) {
            return Err(Report::new(GameError::PlayerNotInGame));
        }
        self.host_user_id = new_host;
        Ok(())
    }

    fn earliest_other_player(&self, excluding: BenutzerId) -> Option<BenutzerId> {
        self.sorted_player_ids()
            .into_iter()
            .find(|&uid| uid != excluding)
    }

    pub fn players(&self) -> &[GamePlayer] {
        &self.players
    }

    /// Deterministically sorted player IDs matching UI & quota turn order.
    pub fn sorted_player_ids(&self) -> Vec<BenutzerId> {
        let mut sorted = self.players.clone();
        sorted.sort_by(|a, b| {
            a.joined_at
                .cmp(&b.joined_at)
                .then(a.user_id.cmp(&b.user_id))
        });
        sorted.into_iter().map(|p| p.user_id).collect()
    }

    pub fn has_player(&self, user_id: BenutzerId) -> bool {
        self.players.iter().any(|p| p.user_id == user_id)
    }

    pub fn player_watching(&self, user_id: BenutzerId) -> Option<bool> {
        self.players
            .iter()
            .find(|p| p.user_id == user_id)
            .map(|p| p.watching)
    }

    pub fn set_player_watching(
        &mut self,
        user_id: BenutzerId,
        watching: bool,
    ) -> ResultReport<(), GameError> {
        let Some(player) = self.players.iter_mut().find(|p| p.user_id == user_id) else {
            return Err(Report::new(GameError::PlayerNotInGame));
        };
        player.watching = watching;
        Ok(())
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn playable_tile_count(&self) -> usize {
        self.grid_size.playable_tile_count()
    }

    pub fn state(&self) -> &S {
        &self.state
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: GameId,
        invite_code: String,
        host_user_id: BenutzerId,
        title: String,
        grid_size: u8,
        visibility_mode: String,
        draft_deadline_at: Option<DateTime<Utc>>,
        ends_at: Option<DateTime<Utc>>,
        mut players: Vec<GamePlayer>,
        created_at: DateTime<Utc>,
        state: S,
    ) -> ResultReport<Self, GameError> {
        let invite_code = InviteCode::new(invite_code)
            .map_err(Report::new)
            .attach(CONSTRUCTING)?;
        let title = NonEmptyString::neu(title)
            .change_context(GameError::EmptyTitle)
            .attach(CONSTRUCTING)?;
        let grid_size = GridSize::from_u8(grid_size)
            .map_err(Report::new)
            .attach(CONSTRUCTING)?;
        let visibility_mode: VisibilityMode = visibility_mode
            .parse()
            .map_err(Report::new)
            .attach(CONSTRUCTING)?;
        players.sort_by(|a, b| {
            a.joined_at
                .cmp(&b.joined_at)
                .then(a.user_id.cmp(&b.user_id))
        });

        Ok(Self {
            id,
            invite_code,
            host_user_id,
            title,
            grid_size,
            visibility_mode,
            draft_deadline_at,
            ends_at,
            players,
            created_at,
            state,
        })
    }
}

impl LobbyGame {
    pub fn new(id: GameId, input: NewGame) -> Self {
        let host_player = GamePlayer {
            user_id: input.host_user_id,
            joined_at: input.created_at,
            watching: input.host_watching,
        };
        Self {
            id,
            invite_code: input.invite_code,
            host_user_id: input.host_user_id,
            title: input.title,
            grid_size: input.grid_size,
            visibility_mode: input.visibility_mode,
            draft_deadline_at: input.draft_deadline_at,
            ends_at: input.ends_at,
            players: vec![host_player],
            created_at: input.created_at,
            state: state::Lobby,
        }
    }

    /// Update configurable lobby parameters; only available while in `Lobby` state.
    pub fn update_settings(
        &mut self,
        title: NonEmptyString,
        grid_size: GridSize,
        visibility_mode: VisibilityMode,
        draft_deadline_at: Option<DateTime<Utc>>,
        ends_at: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> ResultReport<(), GameError> {
        if let Some(deadline) = draft_deadline_at
            && deadline <= now
        {
            return Err(Report::new(GameError::DeadlineInPast));
        }
        if let Some(end) = ends_at
            && (end <= now || draft_deadline_at.is_some_and(|draft| end <= draft))
        {
            return Err(Report::new(GameError::DeadlineInPast));
        }
        self.title = title;
        self.grid_size = grid_size;
        self.visibility_mode = visibility_mode;
        self.draft_deadline_at = draft_deadline_at;
        self.ends_at = ends_at;
        Ok(())
    }

    /// Join `user_id` to the lobby (idempotent if already present).
    pub fn join_player(&mut self, user_id: BenutzerId, now: DateTime<Utc>, watching: bool) {
        if self.has_player(user_id) {
            return;
        }
        self.players.push(GamePlayer {
            user_id,
            joined_at: now,
            watching,
        });
    }

    /// Leave the lobby (auto-transferring host if other players remain; sole player must abandon instead).
    pub fn leave_player(&mut self, user_id: BenutzerId) -> ResultReport<(), GameError> {
        if !self.has_player(user_id) {
            return Err(Report::new(GameError::PlayerNotInGame));
        }
        let Some(next_host) = self.earliest_other_player(user_id) else {
            return Err(Report::new(GameError::InvalidStateTransition));
        };
        if self.host_user_id == user_id {
            self.host_user_id = next_host;
        }
        self.players.retain(|p| p.user_id != user_id);
        Ok(())
    }

    /// Remove a guest player from the lobby.
    pub fn kick_player(&mut self, target: BenutzerId) -> ResultReport<(), GameError> {
        if target == self.host_user_id {
            return Err(Report::new(GameError::InvalidStateTransition));
        }
        if !self.has_player(target) {
            return Err(Report::new(GameError::PlayerNotInGame));
        }
        self.players.retain(|p| p.user_id != target);
        Ok(())
    }

    /// Mark an unstarted lobby as abandoned.
    pub fn abandon(self, now: DateTime<Utc>) -> CompletedGame {
        GameIn {
            id: self.id,
            invite_code: self.invite_code,
            host_user_id: self.host_user_id,
            title: self.title,
            grid_size: self.grid_size,
            visibility_mode: self.visibility_mode,
            draft_deadline_at: self.draft_deadline_at,
            ends_at: self.ends_at,
            players: self.players,
            created_at: self.created_at,
            state: state::Completed {
                prompts: Vec::new(),
                started_at: now,
                ended_at: now,
                winner_user_id: None,
                completion_reason: CompletionReason::Abandoned,
            },
        }
    }

    pub fn start_drafting(self) -> ResultReport<DraftingGame, GameError> {
        if self.players.len() < MIN_PLAYERS {
            return Err(Report::new(GameError::TooFewPlayers));
        }
        Ok(GameIn {
            id: self.id,
            invite_code: self.invite_code,
            host_user_id: self.host_user_id,
            title: self.title,
            grid_size: self.grid_size,
            visibility_mode: self.visibility_mode,
            draft_deadline_at: self.draft_deadline_at,
            ends_at: self.ends_at,
            players: self.players,
            created_at: self.created_at,
            state: state::Drafting {
                prompts: Vec::new(),
                draft_locked: HashSet::new(),
                draft_closes_at: None,
            },
        })
    }
}

impl DraftingGame {
    pub fn prompts(&self) -> &[PromptSubmission] {
        &self.state.prompts
    }

    pub fn prompts_by_author(&self, author_user_id: BenutzerId) -> Vec<&PromptSubmission> {
        self.state
            .prompts
            .iter()
            .filter(|p| p.author_user_id == author_user_id)
            .collect()
    }

    pub fn draft_locked(&self) -> &HashSet<BenutzerId> {
        &self.state.draft_locked
    }

    pub fn is_draft_locked(&self, user_id: BenutzerId) -> bool {
        self.state.draft_locked.contains(&user_id)
    }

    pub fn draft_closes_at(&self) -> Option<DateTime<Utc>> {
        self.state.draft_closes_at
    }

    pub fn all_players_locked(&self) -> bool {
        self.players
            .iter()
            .all(|p| self.state.draft_locked.contains(&p.user_id))
    }

    pub fn is_ready_to_start(&self, now: DateTime<Utc>) -> bool {
        self.all_players_locked() && self.state.draft_closes_at.is_some_and(|t| now >= t)
    }

    pub fn is_draft_deadline_reached(&self, now: DateTime<Utc>) -> bool {
        self.draft_deadline_at.is_some_and(|t| now >= t)
    }

    pub fn set_draft_deadline(
        &mut self,
        deadline: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> ResultReport<(), GameError> {
        if let Some(at) = deadline
            && at <= now
        {
            return Err(Report::new(GameError::DeadlineInPast));
        }
        self.draft_deadline_at = deadline;
        Ok(())
    }

    /// Fill missing quota slots for any unlocked player from `decks` and mark all players locked.
    pub fn autofill_missing_prompts(
        &mut self,
        decks: &[StarterDeck],
        now: DateTime<Utc>,
        rng: &mut (impl Rng + ?Sized),
    ) -> ResultReport<usize, GameError> {
        let player_ids = self.sorted_player_ids();
        let total_tiles = self.playable_tile_count();
        let mut used_texts: HashSet<String> = self
            .state
            .prompts
            .iter()
            .map(|p| p.text.as_str().trim().to_lowercase())
            .collect();

        let mut pool: Vec<String> = decks
            .iter()
            .flat_map(|d| d.prompts.iter())
            .filter_map(|raw| PromptText::new(raw).ok().map(PromptText::into_string))
            .collect();
        if pool.is_empty() {
            return Err(Report::new(GameError::InvalidPromptQuota));
        }
        pool.shuffle(rng);

        let mut added = 0usize;
        for &author in &player_ids {
            if self.state.draft_locked.contains(&author) {
                continue;
            }
            let quotas = target_quotas_for_author(&player_ids, author, total_tiles)?;
            let mut existing_by_target: HashMap<BenutzerId, Vec<PromptSubmission>> = HashMap::new();
            for p in self
                .state
                .prompts
                .iter()
                .filter(|p| p.author_user_id == author)
            {
                existing_by_target
                    .entry(p.target_user_id)
                    .or_default()
                    .push(p.clone());
            }

            let mut rebuilt: Vec<PromptSubmission> = Vec::with_capacity(total_tiles);
            for (target, req) in quotas {
                let mut current = existing_by_target.remove(&target).unwrap_or_default();
                current.truncate(req);
                while current.len() < req {
                    let next_text = if let Some(pos) = pool
                        .iter()
                        .position(|cand| !used_texts.contains(&cand.trim().to_lowercase()))
                    {
                        let chosen = pool[pos].clone();
                        used_texts.insert(chosen.trim().to_lowercase());
                        chosen
                    } else {
                        pool.choose(rng).expect("pool is non-empty").clone()
                    };
                    let text = PromptText::new(next_text)?;
                    current.push(PromptSubmission::new(
                        PromptSubmissionId(Uuid::new_v4()),
                        author,
                        target,
                        text,
                        PromptSource::StarterDeck,
                    )?);
                    added += 1;
                }
                rebuilt.extend(current);
            }

            self.state.prompts.retain(|p| p.author_user_id != author);
            self.state.prompts.extend(rebuilt);
            self.state.draft_locked.insert(author);
        }

        if self.all_players_locked() && self.state.draft_closes_at.is_none() {
            self.state.draft_closes_at =
                Some(now + chrono::Duration::seconds(READY_GRACE_PERIOD_SECONDS));
        }
        Ok(added)
    }

    /// Fill any open prompts from `decks`, lock all players, deal boards, and transition immediately to `Active`.
    pub fn finalize_with_autofill(
        mut self,
        decks: &[StarterDeck],
        now: DateTime<Utc>,
        rng: &mut (impl Rng + ?Sized),
    ) -> ResultReport<(ActiveGame, Vec<NewBoard>), GameError> {
        self.autofill_missing_prompts(decks, now, rng)?;
        self.deal_active_boards(now, rng)
    }

    fn deal_active_boards(
        self,
        now: DateTime<Utc>,
        rng: &mut (impl Rng + ?Sized),
    ) -> ResultReport<(ActiveGame, Vec<NewBoard>), GameError> {
        let player_ids = self.sorted_player_ids();
        let boards = assemble_boards(
            self.id,
            self.grid_size,
            &player_ids,
            &self.state.prompts,
            rng,
        )?;
        let active = GameIn {
            id: self.id,
            invite_code: self.invite_code,
            host_user_id: self.host_user_id,
            title: self.title,
            grid_size: self.grid_size,
            visibility_mode: self.visibility_mode,
            draft_deadline_at: self.draft_deadline_at,
            ends_at: self.ends_at,
            players: self.players,
            created_at: self.created_at,
            state: state::Active {
                prompts: self.state.prompts,
                started_at: now,
            },
        };
        Ok((active, boards))
    }

    /// Remove a player during drafting, resetting draft readiness and reverting to Lobby if `< MIN_PLAYERS`.
    fn remove_participant_or_revert(mut self, target: BenutzerId) -> Game {
        if let Some(next_host) = (self.host_user_id == target)
            .then(|| self.earliest_other_player(target))
            .flatten()
        {
            self.host_user_id = next_host;
        }
        self.players.retain(|p| p.user_id != target);
        self.state
            .prompts
            .retain(|p| p.author_user_id != target && p.target_user_id != target);
        self.state.draft_locked.clear();
        self.state.draft_closes_at = None;

        if self.players.len() < MIN_PLAYERS {
            Game::Lobby(GameIn {
                id: self.id,
                invite_code: self.invite_code,
                host_user_id: self.host_user_id,
                title: self.title,
                grid_size: self.grid_size,
                visibility_mode: self.visibility_mode,
                draft_deadline_at: self.draft_deadline_at,
                ends_at: self.ends_at,
                players: self.players,
                created_at: self.created_at,
                state: state::Lobby,
            })
        } else {
            Game::Drafting(self)
        }
    }

    pub fn leave_player(
        self,
        user_id: BenutzerId,
        now: DateTime<Utc>,
    ) -> ResultReport<Game, GameError> {
        if !self.has_player(user_id) {
            return Err(Report::new(GameError::PlayerNotInGame));
        }
        if self.is_ready_to_start(now) || self.players.len() <= 1 {
            return Err(Report::new(GameError::InvalidStateTransition));
        }
        Ok(self.remove_participant_or_revert(user_id))
    }

    pub fn kick_player(
        self,
        target: BenutzerId,
        now: DateTime<Utc>,
    ) -> ResultReport<Game, GameError> {
        if target == self.host_user_id {
            return Err(Report::new(GameError::InvalidStateTransition));
        }
        if !self.has_player(target) {
            return Err(Report::new(GameError::PlayerNotInGame));
        }
        if self.is_ready_to_start(now) {
            return Err(Report::new(GameError::InvalidStateTransition));
        }
        Ok(self.remove_participant_or_revert(target))
    }

    pub fn abandon(self, now: DateTime<Utc>) -> CompletedGame {
        GameIn {
            id: self.id,
            invite_code: self.invite_code,
            host_user_id: self.host_user_id,
            title: self.title,
            grid_size: self.grid_size,
            visibility_mode: self.visibility_mode,
            draft_deadline_at: self.draft_deadline_at,
            ends_at: self.ends_at,
            players: self.players,
            created_at: self.created_at,
            state: state::Completed {
                prompts: self.state.prompts,
                started_at: now,
                ended_at: now,
                winner_user_id: None,
                completion_reason: CompletionReason::Abandoned,
            },
        }
    }

    /// Replace all prompts authored by `author`, unlock their draft, and cancel any pending countdown.
    pub fn save_author_prompts(
        &mut self,
        author: BenutzerId,
        submissions: Vec<PromptSubmission>,
        now: DateTime<Utc>,
    ) -> ResultReport<(), GameError> {
        if !self.has_player(author) {
            return Err(Report::new(GameError::PlayerNotInGame));
        }
        if self.is_ready_to_start(now) {
            return Err(Report::new(GameError::InvalidStateTransition));
        }
        for sub in &submissions {
            if sub.author_user_id != author || sub.target_user_id == author {
                return Err(Report::new(GameError::SelfTargetNotAllowed));
            }
            if !self.has_player(sub.target_user_id) {
                return Err(Report::new(GameError::TargetNotInGame));
            }
        }

        self.state.prompts.retain(|p| p.author_user_id != author);
        self.state.prompts.extend(submissions);
        self.state.draft_locked.remove(&author);
        self.state.draft_closes_at = None;
        Ok(())
    }

    /// Validate `author`'s prompt quotas, mark them ready, and begin the shared countdown once all players lock.
    pub fn lock_author_prompts(
        &mut self,
        author: BenutzerId,
        now: DateTime<Utc>,
    ) -> ResultReport<(), GameError> {
        if !self.has_player(author) {
            return Err(Report::new(GameError::PlayerNotInGame));
        }
        if self.is_ready_to_start(now) {
            return Err(Report::new(GameError::InvalidStateTransition));
        }

        let player_ids = self.sorted_player_ids();
        let author_prompts: Vec<PromptSubmission> = self
            .state
            .prompts
            .iter()
            .filter(|p| p.author_user_id == author)
            .cloned()
            .collect();

        validate_author_prompts(
            &player_ids,
            author,
            self.playable_tile_count(),
            &author_prompts,
        )?;

        self.state.draft_locked.insert(author);
        if self.all_players_locked() && self.state.draft_closes_at.is_none() {
            self.state.draft_closes_at =
                Some(now + chrono::Duration::seconds(READY_GRACE_PERIOD_SECONDS));
        }
        Ok(())
    }

    /// Withdraw `author`'s ready lock while still in drafting and cancel any active countdown.
    pub fn unlock_author_prompts(
        &mut self,
        author: BenutzerId,
        now: DateTime<Utc>,
    ) -> ResultReport<(), GameError> {
        if !self.has_player(author) {
            return Err(Report::new(GameError::PlayerNotInGame));
        }
        if self.is_ready_to_start(now) {
            return Err(Report::new(GameError::InvalidStateTransition));
        }
        self.state.draft_locked.remove(&author);
        self.state.draft_closes_at = None;
        Ok(())
    }

    /// Deal boards and transition to `Active` once all players stayed ready through the grace period.
    pub fn finalize_ready_start(
        self,
        now: DateTime<Utc>,
        rng: &mut (impl Rng + ?Sized),
    ) -> ResultReport<DraftStartOutcome, GameError> {
        if !self.is_ready_to_start(now) {
            return Ok(DraftStartOutcome::StillDrafting(self));
        }

        let (game, boards) = self.deal_active_boards(now, rng)?;
        Ok(DraftStartOutcome::Started { game, boards })
    }
}

impl ActiveGame {
    pub fn prompts(&self) -> &[PromptSubmission] {
        &self.state.prompts
    }

    pub fn started_at(&self) -> DateTime<Utc> {
        self.state.started_at
    }

    pub fn is_deadline_reached(&self, now: DateTime<Utc>) -> bool {
        self.ends_at.is_some_and(|at| now >= at)
    }

    /// Deal a starter-deck Bingo board for a latecomer without disturbing existing boards.
    pub fn join_late_player(
        &mut self,
        user_id: BenutzerId,
        now: DateTime<Utc>,
        watching: bool,
        decks: &[StarterDeck],
        rng: &mut (impl Rng + ?Sized),
    ) -> ResultReport<Option<NewBoard>, GameError> {
        if self.has_player(user_id) {
            return Ok(None);
        }
        let existing_players = self.sorted_player_ids();
        if existing_players.len() < MIN_PLAYERS {
            return Err(Report::new(GameError::TooFewPlayers));
        }

        let total_tiles = self.playable_tile_count();
        let mut players_with_latecomer = existing_players;
        players_with_latecomer.push(user_id);
        let quotas = target_quotas_for_author(&players_with_latecomer, user_id, total_tiles)?;
        let mut used_texts: HashSet<String> = self
            .state
            .prompts
            .iter()
            .map(|p| p.text.as_str().trim().to_lowercase())
            .collect();
        let mut pool: Vec<String> = decks
            .iter()
            .flat_map(|d| d.prompts.iter())
            .filter_map(|raw| PromptText::new(raw).ok().map(PromptText::into_string))
            .collect();
        if pool.is_empty() {
            return Err(Report::new(GameError::InvalidPromptQuota));
        }
        pool.shuffle(rng);

        let mut assignments = Vec::with_capacity(total_tiles);
        for (target, count) in quotas {
            for _ in 0..count {
                let next_text = if let Some(pos) = pool
                    .iter()
                    .position(|cand| !used_texts.contains(&cand.trim().to_lowercase()))
                {
                    let chosen = pool[pos].clone();
                    used_texts.insert(chosen.trim().to_lowercase());
                    chosen
                } else {
                    pool.choose(rng).expect("pool is non-empty").clone()
                };
                let text = PromptText::new(next_text)?;
                let submission = PromptSubmission::new(
                    PromptSubmissionId(Uuid::new_v4()),
                    user_id,
                    target,
                    text.clone(),
                    PromptSource::StarterDeck,
                )?;
                assignments.push(BoardTileAssignment {
                    prompt_submission_id: submission.id,
                    target_user_id: target,
                    prompt_text: text.into(),
                });
                self.state.prompts.push(submission);
            }
        }
        assignments.shuffle(rng);
        self.players.push(GamePlayer {
            user_id,
            joined_at: now,
            watching,
        });

        let board = NewBoard::new(self.id, user_id, self.grid_size, assignments)
            .change_context(GameError::BoardAssemblyFailed)?;
        Ok(Some(board))
    }

    /// Complete an active game when a player's `winner_board` has achieved Bingo.
    pub fn complete(
        self,
        winner_board: &Board,
        now: DateTime<Utc>,
    ) -> ResultReport<CompletedGame, GameError> {
        if winner_board.game_id() != self.id || !winner_board.has_bingo() {
            return Err(Report::new(GameError::BoardNotCompleted));
        }
        Ok(GameIn {
            id: self.id,
            invite_code: self.invite_code,
            host_user_id: self.host_user_id,
            title: self.title,
            grid_size: self.grid_size,
            visibility_mode: self.visibility_mode,
            draft_deadline_at: self.draft_deadline_at,
            ends_at: self.ends_at,
            players: self.players,
            created_at: self.created_at,
            state: state::Completed {
                prompts: self.state.prompts,
                started_at: self.state.started_at,
                ended_at: now,
                winner_user_id: Some(winner_board.owner_user_id()),
                completion_reason: CompletionReason::Bingo,
            },
        })
    }

    /// Finalize an active game once `ends_at` is reached: most filled tiles wins;
    /// ties break by earliest latest capture timestamp; 0 total captures yields a draw (`None`).
    pub fn complete_by_deadline(
        self,
        boards: &[Board],
        captures: &[Capture],
        now: DateTime<Utc>,
    ) -> CompletedGame {
        let capture_times: HashMap<CaptureId, DateTime<Utc>> =
            captures.iter().map(|c| (c.id(), c.captured_at())).collect();

        let mut best: Option<(usize, DateTime<Utc>, BenutzerId)> = None;
        for board in boards {
            let filled = board.tiles().iter().filter(|t| t.is_filled()).count();
            if filled == 0 {
                continue;
            }
            let latest_capture = board
                .tiles()
                .iter()
                .filter_map(|t| {
                    t.capture_id()
                        .and_then(|cid| capture_times.get(&cid).copied())
                })
                .max()
                .unwrap_or(now);
            let owner = board.owner_user_id();
            let beats = match best {
                None => true,
                Some((best_filled, best_time, best_owner)) => {
                    filled > best_filled
                        || (filled == best_filled
                            && (latest_capture < best_time
                                || (latest_capture == best_time && owner < best_owner)))
                }
            };
            if beats {
                best = Some((filled, latest_capture, owner));
            }
        }

        GameIn {
            id: self.id,
            invite_code: self.invite_code,
            host_user_id: self.host_user_id,
            title: self.title,
            grid_size: self.grid_size,
            visibility_mode: self.visibility_mode,
            draft_deadline_at: self.draft_deadline_at,
            ends_at: self.ends_at,
            players: self.players,
            created_at: self.created_at,
            state: state::Completed {
                prompts: self.state.prompts,
                started_at: self.state.started_at,
                ended_at: now,
                winner_user_id: best.map(|(_, _, owner)| owner),
                completion_reason: CompletionReason::Deadline,
            },
        }
    }

    pub fn abandon(self, now: DateTime<Utc>) -> CompletedGame {
        GameIn {
            id: self.id,
            invite_code: self.invite_code,
            host_user_id: self.host_user_id,
            title: self.title,
            grid_size: self.grid_size,
            visibility_mode: self.visibility_mode,
            draft_deadline_at: self.draft_deadline_at,
            ends_at: self.ends_at,
            players: self.players,
            created_at: self.created_at,
            state: state::Completed {
                prompts: self.state.prompts,
                started_at: self.state.started_at,
                ended_at: now,
                winner_user_id: None,
                completion_reason: CompletionReason::Abandoned,
            },
        }
    }
}

impl CompletedGame {
    pub fn prompts(&self) -> &[PromptSubmission] {
        &self.state.prompts
    }

    pub fn started_at(&self) -> DateTime<Utc> {
        self.state.started_at
    }

    pub fn ended_at(&self) -> DateTime<Utc> {
        self.state.ended_at
    }

    pub fn winner_user_id(&self) -> Option<BenutzerId> {
        self.state.winner_user_id
    }

    pub fn completion_reason(&self) -> CompletionReason {
        self.state.completion_reason
    }
}

impl Game {
    pub fn new(id: GameId, input: NewGame) -> Self {
        Self::Lobby(LobbyGame::new(id, input))
    }

    pub fn id(&self) -> GameId {
        match self {
            Self::Lobby(g) => g.id(),
            Self::Drafting(g) => g.id(),
            Self::Active(g) => g.id(),
            Self::Completed(g) => g.id(),
        }
    }

    pub fn invite_code(&self) -> &InviteCode {
        match self {
            Self::Lobby(g) => g.invite_code(),
            Self::Drafting(g) => g.invite_code(),
            Self::Active(g) => g.invite_code(),
            Self::Completed(g) => g.invite_code(),
        }
    }

    pub fn host_user_id(&self) -> BenutzerId {
        match self {
            Self::Lobby(g) => g.host_user_id(),
            Self::Drafting(g) => g.host_user_id(),
            Self::Active(g) => g.host_user_id(),
            Self::Completed(g) => g.host_user_id(),
        }
    }

    pub fn title(&self) -> &NonEmptyString {
        match self {
            Self::Lobby(g) => g.title(),
            Self::Drafting(g) => g.title(),
            Self::Active(g) => g.title(),
            Self::Completed(g) => g.title(),
        }
    }

    pub fn grid_size(&self) -> GridSize {
        match self {
            Self::Lobby(g) => g.grid_size(),
            Self::Drafting(g) => g.grid_size(),
            Self::Active(g) => g.grid_size(),
            Self::Completed(g) => g.grid_size(),
        }
    }

    pub fn visibility_mode(&self) -> VisibilityMode {
        match self {
            Self::Lobby(g) => g.visibility_mode(),
            Self::Drafting(g) => g.visibility_mode(),
            Self::Active(g) => g.visibility_mode(),
            Self::Completed(g) => g.visibility_mode(),
        }
    }

    pub fn draft_deadline_at(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::Lobby(g) => g.draft_deadline_at(),
            Self::Drafting(g) => g.draft_deadline_at(),
            Self::Active(g) => g.draft_deadline_at(),
            Self::Completed(g) => g.draft_deadline_at(),
        }
    }

    pub fn ends_at(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::Lobby(g) => g.ends_at(),
            Self::Drafting(g) => g.ends_at(),
            Self::Active(g) => g.ends_at(),
            Self::Completed(g) => g.ends_at(),
        }
    }

    pub fn completion_reason(&self) -> Option<CompletionReason> {
        match self {
            Self::Completed(g) => Some(g.completion_reason()),
            _ => None,
        }
    }

    pub fn transfer_host(
        &mut self,
        actor: BenutzerId,
        new_host: BenutzerId,
    ) -> ResultReport<(), GameError> {
        match self {
            Self::Lobby(g) => g.transfer_host(actor, new_host),
            Self::Drafting(g) => g.transfer_host(actor, new_host),
            Self::Active(g) => g.transfer_host(actor, new_host),
            Self::Completed(_) => Err(Report::new(GameError::InvalidStateTransition)),
        }
    }

    pub fn players(&self) -> &[GamePlayer] {
        match self {
            Self::Lobby(g) => g.players(),
            Self::Drafting(g) => g.players(),
            Self::Active(g) => g.players(),
            Self::Completed(g) => g.players(),
        }
    }

    pub fn sorted_player_ids(&self) -> Vec<BenutzerId> {
        match self {
            Self::Lobby(g) => g.sorted_player_ids(),
            Self::Drafting(g) => g.sorted_player_ids(),
            Self::Active(g) => g.sorted_player_ids(),
            Self::Completed(g) => g.sorted_player_ids(),
        }
    }

    pub fn has_player(&self, user_id: BenutzerId) -> bool {
        match self {
            Self::Lobby(g) => g.has_player(user_id),
            Self::Drafting(g) => g.has_player(user_id),
            Self::Active(g) => g.has_player(user_id),
            Self::Completed(g) => g.has_player(user_id),
        }
    }

    pub fn player_watching(&self, user_id: BenutzerId) -> Option<bool> {
        match self {
            Self::Lobby(g) => g.player_watching(user_id),
            Self::Drafting(g) => g.player_watching(user_id),
            Self::Active(g) => g.player_watching(user_id),
            Self::Completed(g) => g.player_watching(user_id),
        }
    }

    pub fn set_player_watching(
        &mut self,
        user_id: BenutzerId,
        watching: bool,
    ) -> ResultReport<(), GameError> {
        match self {
            Self::Lobby(g) => g.set_player_watching(user_id, watching),
            Self::Drafting(g) => g.set_player_watching(user_id, watching),
            Self::Active(g) => g.set_player_watching(user_id, watching),
            Self::Completed(g) => g.set_player_watching(user_id, watching),
        }
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        match self {
            Self::Lobby(g) => g.created_at(),
            Self::Drafting(g) => g.created_at(),
            Self::Active(g) => g.created_at(),
            Self::Completed(g) => g.created_at(),
        }
    }

    pub fn playable_tile_count(&self) -> usize {
        self.grid_size().playable_tile_count()
    }
}

/// Compute the exact per-target prompt quota for `author` in cyclic player order.
///
/// Distributing the remainder `T % (N - 1)` cyclically starting right after `author`
/// guarantees every player is targeted evenly (at most +/- 1) across the entire game.
pub fn target_quotas_for_author(
    players: &[BenutzerId],
    author: BenutzerId,
    total_tiles: usize,
) -> ResultReport<Vec<(BenutzerId, usize)>, GameError> {
    if players.len() < MIN_PLAYERS {
        return Err(Report::new(GameError::TooFewPlayers));
    }
    let Some(author_idx) = players.iter().position(|&id| id == author) else {
        return Err(Report::new(GameError::TargetNotInGame));
    };

    let others_count = players.len() - 1;
    let base = total_tiles / others_count;
    let rem = total_tiles % others_count;

    let mut quotas = Vec::with_capacity(others_count);
    for offset in 1..players.len() {
        let target = players[(author_idx + offset) % players.len()];
        let count = base + usize::from(offset - 1 < rem);
        quotas.push((target, count));
    }
    Ok(quotas)
}

/// Validate that `prompts` for `author` exactly satisfy the equal-distribution quota.
pub fn validate_author_prompts(
    players: &[BenutzerId],
    author: BenutzerId,
    total_tiles: usize,
    prompts: &[PromptSubmission],
) -> ResultReport<(), GameError> {
    let quotas = target_quotas_for_author(players, author, total_tiles)?;
    let expected: HashMap<BenutzerId, usize> = quotas.into_iter().collect();

    let mut actual: HashMap<BenutzerId, usize> = HashMap::new();
    for p in prompts {
        if p.author_user_id != author || p.target_user_id == author {
            return Err(Report::new(GameError::SelfTargetNotAllowed));
        }
        if !expected.contains_key(&p.target_user_id) {
            return Err(Report::new(GameError::TargetNotInGame));
        }
        *actual.entry(p.target_user_id).or_insert(0) += 1;
    }

    for (target, req) in expected {
        if actual.get(&target).copied().unwrap_or(0) != req {
            return Err(Report::new(GameError::InvalidPromptQuota));
        }
    }
    Ok(())
}

/// Assemble unique Bingo boards for every player using balanced bipartite matching.
///
/// Hard constraints strictly enforced:
/// - `prompt.author_user_id != board.owner_user_id`
/// - `prompt.target_user_id != board.owner_user_id`
/// - Every submitted prompt is used exactly once across all boards.
pub fn assemble_boards(
    game_id: GameId,
    grid_size: GridSize,
    players: &[BenutzerId],
    prompts: &[PromptSubmission],
    rng: &mut (impl Rng + ?Sized),
) -> ResultReport<Vec<NewBoard>, GameError> {
    let n = players.len();
    if n < MIN_PLAYERS {
        return Err(Report::new(GameError::TooFewPlayers));
    }

    let playable_count = grid_size.playable_tile_count();
    let expected_total = n * playable_count;
    if prompts.len() != expected_total {
        return Err(Report::new(GameError::InvalidPromptQuota));
    }

    let mut shuffled_prompts: Vec<&PromptSubmission> = prompts.iter().collect();
    shuffled_prompts.shuffle(rng);

    let mut slot_owners: Vec<BenutzerId> = Vec::with_capacity(expected_total);
    for &owner in players {
        for _ in 0..playable_count {
            slot_owners.push(owner);
        }
    }

    let mut adj: Vec<Vec<usize>> = Vec::with_capacity(expected_total);
    for &owner in &slot_owners {
        let mut eligible: Vec<usize> = shuffled_prompts
            .iter()
            .enumerate()
            .filter(|(_, p)| p.author_user_id != owner && p.target_user_id != owner)
            .map(|(idx, _)| idx)
            .collect();
        eligible.shuffle(rng);
        adj.push(eligible);
    }

    let mut prompt_matched_slot: Vec<Option<usize>> = vec![None; expected_total];
    let mut slot_matched_prompt: Vec<Option<usize>> = vec![None; expected_total];

    for slot in 0..expected_total {
        let mut visited = vec![false; expected_total];
        if !kuhn_augment(slot, &adj, &mut visited, &mut prompt_matched_slot) {
            return Err(Report::new(GameError::BoardAssemblyFailed));
        }
    }
    for (prompt_idx, maybe_slot) in prompt_matched_slot.iter().enumerate() {
        if let Some(slot) = maybe_slot {
            slot_matched_prompt[*slot] = Some(prompt_idx);
        }
    }

    dedupe_board_prompt_texts(
        players,
        playable_count,
        &shuffled_prompts,
        &mut slot_matched_prompt,
    );

    let mut result = Vec::with_capacity(n);
    for (player_idx, &owner) in players.iter().enumerate() {
        let mut assigned_prompts: Vec<&PromptSubmission> = Vec::with_capacity(playable_count);
        for offset in 0..playable_count {
            let slot = player_idx * playable_count + offset;
            let p_idx = slot_matched_prompt[slot]
                .ok_or_else(|| Report::new(GameError::BoardAssemblyFailed))?;
            let prompt = shuffled_prompts[p_idx];
            if prompt.author_user_id == owner || prompt.target_user_id == owner {
                return Err(Report::new(GameError::BoardAssemblyFailed));
            }
            assigned_prompts.push(prompt);
        }
        assigned_prompts.shuffle(rng);

        let mut seen_prompts: HashSet<PromptSubmissionId> = HashSet::new();
        let assignments: Vec<BoardTileAssignment> = assigned_prompts
            .into_iter()
            .map(|p| {
                seen_prompts.insert(p.id);
                BoardTileAssignment {
                    prompt_submission_id: p.id,
                    target_user_id: p.target_user_id,
                    prompt_text: p.text.clone().into(),
                }
            })
            .collect();

        if seen_prompts.len() != playable_count {
            return Err(Report::new(GameError::BoardAssemblyFailed));
        }

        let new_board = NewBoard::new(game_id, owner, grid_size, assignments)
            .change_context(GameError::BoardAssemblyFailed)?;
        result.push(new_board);
    }

    Ok(result)
}

/// Depth-first augmenting-path step of Kuhn's maximum bipartite matching algorithm.
///
/// Attempts to assign `slot` to an eligible prompt in `adj[slot]`, recursively reassigning any
/// already-matched prompt to free a valid matching across all board slots.
fn kuhn_augment(
    slot: usize,
    adj: &[Vec<usize>],
    visited: &mut [bool],
    prompt_matched_slot: &mut [Option<usize>],
) -> bool {
    for &prompt_idx in &adj[slot] {
        if visited[prompt_idx] {
            continue;
        }
        visited[prompt_idx] = true;
        if prompt_matched_slot[prompt_idx].is_none()
            || kuhn_augment(
                prompt_matched_slot[prompt_idx].unwrap(),
                adj,
                visited,
                prompt_matched_slot,
            )
        {
            prompt_matched_slot[prompt_idx] = Some(slot);
            return true;
        }
    }
    false
}

/// Best-effort pairwise swap pass reducing duplicate normalized prompt texts on the same board
/// while strictly preserving `author != owner && target != owner`.
fn dedupe_board_prompt_texts(
    players: &[BenutzerId],
    playable_count: usize,
    shuffled_prompts: &[&PromptSubmission],
    slot_matched_prompt: &mut [Option<usize>],
) {
    let norm =
        |p_idx: usize| -> String { shuffled_prompts[p_idx].text.as_str().trim().to_lowercase() };
    let board_count_for = |player_idx: usize, key: &str, matched: &[Option<usize>]| -> usize {
        let start = player_idx * playable_count;
        (start..start + playable_count)
            .filter_map(|s| matched[s])
            .filter(|&p_idx| norm(p_idx) == key)
            .count()
    };

    let total_slots = players.len() * playable_count;
    for _pass in 0..4 {
        let mut swapped = false;
        for slot_a in 0..total_slots {
            let Some(p_a) = slot_matched_prompt[slot_a] else {
                continue;
            };
            let owner_a_idx = slot_a / playable_count;
            let owner_a = players[owner_a_idx];
            let key_a = norm(p_a);
            if board_count_for(owner_a_idx, &key_a, slot_matched_prompt) <= 1 {
                continue;
            }

            for slot_b in 0..total_slots {
                let owner_b_idx = slot_b / playable_count;
                if owner_a_idx == owner_b_idx {
                    continue;
                }
                let Some(p_b) = slot_matched_prompt[slot_b] else {
                    continue;
                };
                let key_b = norm(p_b);
                if key_a == key_b {
                    continue;
                }
                let owner_b = players[owner_b_idx];
                let prompt_a = shuffled_prompts[p_a];
                let prompt_b = shuffled_prompts[p_b];
                if prompt_a.author_user_id == owner_b
                    || prompt_a.target_user_id == owner_b
                    || prompt_b.author_user_id == owner_a
                    || prompt_b.target_user_id == owner_a
                {
                    continue;
                }
                if board_count_for(owner_a_idx, &key_b, slot_matched_prompt) == 0
                    && board_count_for(owner_b_idx, &key_a, slot_matched_prompt) == 0
                {
                    slot_matched_prompt.swap(slot_a, slot_b);
                    swapped = true;
                    break;
                }
            }
        }
        if !swapped {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;
    use crate::domain::board::BoardId;

    fn test_players(count: usize) -> Vec<BenutzerId> {
        (1..=count)
            .map(|i| BenutzerId(Uuid::from_u128(i as u128)))
            .collect()
    }

    fn build_valid_prompts(players: &[BenutzerId], total_tiles: usize) -> Vec<PromptSubmission> {
        let mut out = Vec::new();
        for &author in players {
            let quotas = target_quotas_for_author(players, author, total_tiles).unwrap();
            for (target, count) in quotas {
                for idx in 0..count {
                    out.push(
                        PromptSubmission::new(
                            PromptSubmissionId(Uuid::new_v4()),
                            author,
                            target,
                            PromptText::new(format!(
                                "{}->{} #{idx}",
                                author.0.as_u128(),
                                target.0.as_u128()
                            ))
                            .unwrap(),
                            PromptSource::Custom,
                        )
                        .unwrap(),
                    );
                }
            }
        }
        out
    }

    #[test_log::test]
    fn invite_code_draws_without_replacement() {
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..50 {
            let code = InviteCode::generate(&mut rng);
            let chars: HashSet<char> = code.as_str().chars().collect();
            assert_eq!(code.as_str().len(), 6);
            assert_eq!(
                chars.len(),
                6,
                "invite code must not repeat symbols: {}",
                code.as_str()
            );
        }
    }

    #[test_log::test]
    fn assemble_boards_enforces_author_and_target_exclusion_for_all_grid_sizes() {
        let mut rng = StdRng::seed_from_u64(42);
        for player_count in [3usize, 4, 5, 7] {
            for grid_size in [GridSize::Three, GridSize::Four, GridSize::Five] {
                let players = test_players(player_count);
                let game_id = GameId(Uuid::new_v4());
                let prompts = build_valid_prompts(&players, grid_size.playable_tile_count());
                for &author in &players {
                    let mine: Vec<_> = prompts
                        .iter()
                        .filter(|p| p.author_user_id == author)
                        .cloned()
                        .collect();
                    validate_author_prompts(
                        &players,
                        author,
                        grid_size.playable_tile_count(),
                        &mine,
                    )
                    .unwrap();
                }

                let new_boards =
                    assemble_boards(game_id, grid_size, &players, &prompts, &mut rng).unwrap();
                assert_eq!(new_boards.len(), player_count);

                let mut used_ids = HashSet::new();
                for nb in new_boards {
                    let board = Board::new(BoardId(Uuid::new_v4()), nb).unwrap();
                    let owner = board.owner_user_id();
                    assert_eq!(board.tiles().len(), grid_size.side() * grid_size.side());
                    for tile in board.tiles() {
                        let pid = tile.prompt_submission_id();
                        assert!(used_ids.insert(pid), "prompt reused across boards");
                        let orig = prompts.iter().find(|p| p.id == pid).unwrap();
                        assert_ne!(orig.author_user_id, owner);
                        assert_ne!(orig.target_user_id, owner);
                        assert_ne!(tile.target_user_id(), owner);
                    }
                }
                assert_eq!(used_ids.len(), prompts.len());
            }
        }
    }

    #[test_log::test]
    fn game_typestate_transitions_enforce_lifecycle() {
        let mut rng = StdRng::seed_from_u64(99);
        let players = test_players(3);
        let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

        let mut lobby = LobbyGame::new(
            GameId(Uuid::new_v4()),
            NewGame {
                invite_code: InviteCode::new("TEST12").unwrap(),
                host_user_id: players[0],
                host_watching: true,
                title: NonEmptyString::neu("Testabend").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::Undercover,
                draft_deadline_at: None,
                ends_at: None,
                created_at: now,
            },
        );
        assert!(lobby.clone().start_drafting().is_err());

        lobby.join_player(players[1], now, true);
        lobby.join_player(players[2], now, false);
        assert_eq!(lobby.player_watching(players[0]), Some(true));
        assert_eq!(lobby.player_watching(players[2]), Some(false));
        lobby.set_player_watching(players[2], true).unwrap();
        assert_eq!(lobby.player_watching(players[2]), Some(true));
        lobby
            .update_settings(
                NonEmptyString::neu("Updated").unwrap(),
                GridSize::Three,
                VisibilityMode::LiveFeed,
                None,
                None,
                now,
            )
            .unwrap();

        let mut drafting = lobby.start_drafting().unwrap();
        let all_prompts = build_valid_prompts(&players, drafting.playable_tile_count());
        for &author in &players {
            let mine: Vec<_> = all_prompts
                .iter()
                .filter(|p| p.author_user_id == author)
                .cloned()
                .collect();
            drafting.save_author_prompts(author, mine, now).unwrap();
        }

        drafting.lock_author_prompts(players[0], now).unwrap();
        assert!(drafting.draft_closes_at().is_none());
        drafting.lock_author_prompts(players[1], now).unwrap();
        assert!(drafting.draft_closes_at().is_none());

        // Last player locks: enters 60s grace period rather than starting immediately.
        drafting.lock_author_prompts(players[2], now).unwrap();
        assert_eq!(
            drafting.draft_closes_at(),
            Some(now + chrono::Duration::seconds(READY_GRACE_PERIOD_SECONDS))
        );
        assert!(!drafting.is_ready_to_start(now + chrono::Duration::seconds(59)));
        assert!(drafting.is_ready_to_start(now + chrono::Duration::seconds(60)));

        // Withdrawing readiness cancels the pending start and preserves prompts.
        drafting
            .unlock_author_prompts(players[2], now + chrono::Duration::seconds(30))
            .unwrap();
        assert!(drafting.draft_closes_at().is_none());
        assert!(!drafting.is_draft_locked(players[2]));
        assert_eq!(drafting.prompts_by_author(players[2]).len(), 9);

        // Re-locking starts a fresh 60s countdown from the new timestamp.
        let relock_at = now + chrono::Duration::seconds(40);
        drafting.lock_author_prompts(players[2], relock_at).unwrap();
        assert_eq!(
            drafting.draft_closes_at(),
            Some(relock_at + chrono::Duration::seconds(READY_GRACE_PERIOD_SECONDS))
        );
        let DraftStartOutcome::StillDrafting(drafting) = drafting
            .finalize_ready_start(relock_at + chrono::Duration::seconds(59), &mut rng)
            .unwrap()
        else {
            panic!("must not start before full grace period expires");
        };
        let DraftStartOutcome::Started { game, boards } = drafting
            .finalize_ready_start(relock_at + chrono::Duration::seconds(60), &mut rng)
            .unwrap()
        else {
            panic!("should start active once grace period expires");
        };
        assert_eq!(boards.len(), 3);

        let mut winner_board = Board::new(BoardId(Uuid::new_v4()), boards[1].clone()).unwrap();
        assert!(game.clone().complete(&winner_board, now).is_err());
        let row0: Vec<_> = winner_board
            .tiles()
            .iter()
            .filter(|t| t.row() == 0)
            .map(|t| t.id())
            .collect();
        for tid in row0 {
            winner_board
                .record_capture(tid, crate::domain::CaptureId(Uuid::new_v4()))
                .unwrap();
        }
        let completed = game.complete(&winner_board, now).unwrap();
        assert_eq!(
            completed.winner_user_id(),
            Some(winner_board.owner_user_id())
        );
        assert_eq!(completed.completion_reason(), CompletionReason::Bingo);
    }

    #[test_log::test]
    fn self_target_prompt_is_rejected() {
        let p = BenutzerId(Uuid::new_v4());
        let err = PromptSubmission::new(
            PromptSubmissionId(Uuid::new_v4()),
            p,
            p,
            PromptText::new("Selfie").unwrap(),
            PromptSource::Custom,
        )
        .unwrap_err();
        assert!(matches!(
            err.current_context(),
            GameError::SelfTargetNotAllowed
        ));
    }

    #[test_log::test]
    fn prompt_text_enforces_80_char_limit_and_non_empty() {
        assert!(matches!(
            PromptText::new("   "),
            Err(GameError::EmptyPromptText)
        ));
        let exact_80 = "a".repeat(PROMPT_MAX_CHARS);
        assert_eq!(PromptText::new(&exact_80).unwrap().as_str().len(), 80);
        let too_long = "ä".repeat(PROMPT_MAX_CHARS + 1);
        assert!(matches!(
            PromptText::new(too_long),
            Err(GameError::PromptTooLong)
        ));
    }

    #[test_log::test]
    fn autofill_missing_prompts_fills_and_locks_unlocked_players() {
        let mut rng = StdRng::seed_from_u64(11);
        let players = test_players(3);
        let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        let mut lobby = LobbyGame::new(
            GameId(Uuid::new_v4()),
            NewGame {
                invite_code: InviteCode::new("AUTO12").unwrap(),
                host_user_id: players[0],
                host_watching: true,
                title: NonEmptyString::neu("Autofill").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::Undercover,
                draft_deadline_at: Some(now + chrono::Duration::minutes(30)),
                ends_at: None,
                created_at: now,
            },
        );
        lobby.join_player(players[1], now, true);
        lobby.join_player(players[2], now, true);

        let mut drafting = lobby.start_drafting().unwrap();
        assert!(!drafting.is_draft_deadline_reached(now));
        assert!(drafting.is_draft_deadline_reached(now + chrono::Duration::minutes(30)));

        let decks = crate::domain::builtin_starter_decks();
        let added = drafting
            .autofill_missing_prompts(&decks, now, &mut rng)
            .unwrap();
        assert_eq!(added, 27);
        assert!(drafting.all_players_locked());
        let (active, boards) = drafting
            .finalize_with_autofill(&decks, now, &mut rng)
            .unwrap();
        assert_eq!(boards.len(), 3);
        assert_eq!(active.prompts().len(), 27);
    }

    #[test_log::test]
    fn assemble_boards_avoids_duplicate_prompt_texts_on_same_board() {
        let mut rng = StdRng::seed_from_u64(5);
        let players = test_players(4);
        let game_id = GameId(Uuid::new_v4());
        let mut prompts = Vec::new();
        for (a_idx, &author) in players.iter().enumerate() {
            let quotas = target_quotas_for_author(&players, author, 9).unwrap();
            let mut slot_idx = 0usize;
            for (target, count) in quotas {
                for _ in 0..count {
                    // Three authors pick rotated targets so boards can separate all duplicates.
                    let text = if a_idx < 3 && slot_idx == a_idx * 3 {
                        "Trinkt Wasser".to_owned()
                    } else {
                        format!("Unique {a_idx}-{slot_idx}")
                    };
                    prompts.push(
                        PromptSubmission::new(
                            PromptSubmissionId(Uuid::new_v4()),
                            author,
                            target,
                            PromptText::new(text).unwrap(),
                            PromptSource::Custom,
                        )
                        .unwrap(),
                    );
                    slot_idx += 1;
                }
            }
        }

        let boards =
            assemble_boards(game_id, GridSize::Three, &players, &prompts, &mut rng).unwrap();
        for nb in boards {
            let texts: HashSet<String> = nb
                .assignments()
                .iter()
                .map(|a| a.prompt_text.as_str().to_lowercase())
                .collect();
            assert_eq!(
                texts.len(),
                nb.assignments().len(),
                "board had duplicate prompt text"
            );
        }
    }

    #[test_log::test]
    fn deadline_completion_picks_most_filled_tiebreaks_early_and_zero_captures_is_draw() {
        let mut rng = StdRng::seed_from_u64(31);
        let players = test_players(3);
        let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        let mut lobby = LobbyGame::new(
            GameId(Uuid::new_v4()),
            NewGame {
                invite_code: InviteCode::new("END123").unwrap(),
                host_user_id: players[0],
                host_watching: true,
                title: NonEmptyString::neu("Deadline Game").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
                draft_deadline_at: None,
                ends_at: Some(now + chrono::Duration::hours(2)),
                created_at: now,
            },
        );
        lobby.join_player(players[1], now, true);
        lobby.join_player(players[2], now, true);

        let drafting = lobby.start_drafting().unwrap();
        let decks = crate::domain::builtin_starter_decks();
        let (active, new_boards) = drafting
            .finalize_with_autofill(&decks, now, &mut rng)
            .unwrap();
        let mut boards: Vec<Board> = new_boards
            .into_iter()
            .map(|nb| Board::new(BoardId(Uuid::new_v4()), nb).unwrap())
            .collect();

        // 0 captures at deadline -> draw (winner_user_id = None)
        let draw =
            active
                .clone()
                .complete_by_deadline(&boards, &[], now + chrono::Duration::hours(2));
        assert_eq!(draw.winner_user_id(), None);
        assert_eq!(draw.completion_reason(), CompletionReason::Deadline);

        // Player 1 fills 1 tile at T+10m; Player 2 fills 1 tile at T+5m -> tie breaks to Player 2
        let cap1_id = CaptureId(Uuid::new_v4());
        let cap2_id = CaptureId(Uuid::new_v4());
        let t1 = boards[1].tiles()[0].id();
        let t2 = boards[2].tiles()[0].id();
        boards[1].record_capture(t1, cap1_id).unwrap();
        boards[2].record_capture(t2, cap2_id).unwrap();
        let make_cap = |id: CaptureId, board: &Board, ts: DateTime<Utc>| -> Capture {
            let t = board.tiles()[0].clone();
            Capture::from_parts(
                id,
                board.game_id(),
                t.id(),
                board.owner_user_id(),
                t.target_user_id(),
                t.prompt_text().as_str().to_owned(),
                format!("{}.webp", id.0),
                format!("t_{}.webp", id.0),
                ts,
            )
            .unwrap()
        };
        let caps = vec![
            make_cap(cap1_id, &boards[1], now + chrono::Duration::minutes(10)),
            make_cap(cap2_id, &boards[2], now + chrono::Duration::minutes(5)),
        ];
        let win_tie = active.complete_by_deadline(&boards, &caps, now + chrono::Duration::hours(2));
        assert_eq!(win_tie.winner_user_id(), Some(boards[2].owner_user_id()));
    }

    #[test_log::test]
    fn roster_leave_kick_revert_and_late_join_enforced() {
        let mut rng = StdRng::seed_from_u64(77);
        let players = test_players(4);
        let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        let mut lobby = LobbyGame::new(
            GameId(Uuid::new_v4()),
            NewGame {
                invite_code: InviteCode::new("ROS123").unwrap(),
                host_user_id: players[0],
                host_watching: true,
                title: NonEmptyString::neu("Roster").unwrap(),
                grid_size: GridSize::Three,
                visibility_mode: VisibilityMode::LiveFeed,
                draft_deadline_at: None,
                ends_at: None,
                created_at: now,
            },
        );
        lobby.join_player(players[1], now + chrono::Duration::seconds(1), true);
        lobby.join_player(players[2], now + chrono::Duration::seconds(2), true);

        // Host leaving Lobby transfers host to earliest guest (players[1])
        lobby.leave_player(players[0]).unwrap();
        assert_eq!(lobby.host_user_id(), players[1]);
        assert!(!lobby.has_player(players[0]));

        // Add back player 0 so we reach MIN_PLAYERS=3 and enter Drafting, then kick player 2 -> reverts to Lobby!
        lobby.join_player(players[0], now + chrono::Duration::seconds(3), true);
        let drafting = lobby.start_drafting().unwrap();
        let after_kick = drafting
            .kick_player(players[2], now + chrono::Duration::seconds(4))
            .unwrap();
        assert!(matches!(after_kick, Game::Lobby(_)));

        let Game::Lobby(mut back_in_lobby) = after_kick else {
            unreachable!();
        };
        back_in_lobby.join_player(players[2], now + chrono::Duration::seconds(5), true);
        let decks = crate::domain::builtin_starter_decks();
        let (mut active, _) = back_in_lobby
            .start_drafting()
            .unwrap()
            .finalize_with_autofill(&decks, now + chrono::Duration::seconds(6), &mut rng)
            .unwrap();

        // Late player joins Active -> receives full 9-tile board targeting existing players
        let late_board = active
            .join_late_player(
                players[3],
                now + chrono::Duration::seconds(10),
                true,
                &decks,
                &mut rng,
            )
            .unwrap()
            .expect("late board created");
        assert_eq!(late_board.assignments().len(), 9);
        assert!(active.has_player(players[3]));
    }
}
