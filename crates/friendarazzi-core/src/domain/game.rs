//! Game lobby, drafting quotas, balanced board generation, and lifecycle transitions.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use rand::{Rng, seq::SliceRandom};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    ResultReport,
    domain::{
        NonEmptyString,
        board::{Board, BoardId, BoardTile, BoardTileId, NewBoard, NewBoardTile},
        konto::BenutzerId,
    },
};

const CONSTRUCTING: &str = "while constructing game";

/// Minimum number of players required to start drafting and guarantee `author != owner && target != owner`.
pub const MIN_PLAYERS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GameId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PromptSubmissionId(pub Uuid);

#[derive(Debug, Error)]
pub enum GameError {
    #[error("Der Spieltitel darf nicht leer sein")]
    EmptyTitle,
    #[error("Der Einladungscode ist ungültig")]
    InvalidInviteCode,
    #[error("Die Rastergröße muss 3, 4 oder 5 sein")]
    InvalidGridSize,
    #[error("Unbekannter Sichtbarkeitsmodus")]
    InvalidVisibilityMode,
    #[error("Unbekannter Spielstatus")]
    InvalidStatus,
    #[error("Unbekannte Prompt-Quelle")]
    InvalidPromptSource,
    #[error("Diese Aktion ist im aktuellen Spielstatus nicht möglich")]
    InvalidStateTransition,
    #[error("Mindestens {MIN_PLAYERS} Spieler sind erforderlich")]
    TooFewPlayers,
    #[error("Du kannst keinen Auftrag über dich selbst schreiben")]
    SelfTargetNotAllowed,
    #[error("Das Ziel gehört nicht zu dieser Runde")]
    TargetNotInGame,
    #[error("Die eingereichten Aufträge entsprechen nicht der gleichmäßigen Verteilung")]
    InvalidPromptQuota,
    #[error("Die Spielbretter konnten nicht konfliktfrei zusammengestellt werden")]
    BoardAssemblyFailed,
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

    pub fn generate(rng: &mut impl Rng) -> Self {
        const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let code: String = (0..6)
            .map(|_| {
                let idx = rng.gen_range(0..ALPHABET.len());
                ALPHABET[idx] as char
            })
            .collect();
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

    pub fn has_center_free_space(self, include_free_space: bool) -> bool {
        include_free_space && self != Self::Four
    }

    pub fn playable_tile_count(self, include_free_space: bool) -> usize {
        let side = self.side();
        side * side - usize::from(self.has_center_free_space(include_free_space))
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
pub enum GameStatus {
    Lobby,
    Drafting,
    Active,
    Completed,
}

impl GameStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lobby => "Lobby",
            Self::Drafting => "Drafting",
            Self::Active => "Active",
            Self::Completed => "Completed",
        }
    }
}

impl std::str::FromStr for GameStatus {
    type Err = GameError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Lobby" => Ok(Self::Lobby),
            "Drafting" => Ok(Self::Drafting),
            "Active" => Ok(Self::Active),
            "Completed" => Ok(Self::Completed),
            _ => Err(GameError::InvalidStatus),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGame {
    pub invite_code: InviteCode,
    pub host_user_id: BenutzerId,
    pub title: NonEmptyString,
    pub grid_size: GridSize,
    pub visibility_mode: VisibilityMode,
    pub include_free_space: bool,
    pub created_at: DateTime<Utc>,
}

/// A Friendarazzi social photo-hunting Bingo game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    id: GameId,
    invite_code: InviteCode,
    host_user_id: BenutzerId,
    title: NonEmptyString,
    grid_size: GridSize,
    visibility_mode: VisibilityMode,
    include_free_space: bool,
    status: GameStatus,
    started_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    winner_user_id: Option<BenutzerId>,
    created_at: DateTime<Utc>,
}

impl Game {
    pub fn new(id: GameId, input: NewGame) -> Self {
        Self {
            id,
            invite_code: input.invite_code,
            host_user_id: input.host_user_id,
            title: input.title,
            grid_size: input.grid_size,
            visibility_mode: input.visibility_mode,
            include_free_space: input.include_free_space,
            status: GameStatus::Lobby,
            started_at: None,
            ended_at: None,
            winner_user_id: None,
            created_at: input.created_at,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: GameId,
        invite_code: String,
        host_user_id: BenutzerId,
        title: String,
        grid_size: u8,
        visibility_mode: String,
        include_free_space: bool,
        status: String,
        started_at: Option<DateTime<Utc>>,
        ended_at: Option<DateTime<Utc>>,
        winner_user_id: Option<BenutzerId>,
        created_at: DateTime<Utc>,
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
        let status: GameStatus = status.parse().map_err(Report::new).attach(CONSTRUCTING)?;

        Ok(Self {
            id,
            invite_code,
            host_user_id,
            title,
            grid_size,
            visibility_mode,
            include_free_space,
            status,
            started_at,
            ended_at,
            winner_user_id,
            created_at,
        })
    }

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

    pub fn include_free_space(&self) -> bool {
        self.include_free_space
    }

    pub fn status(&self) -> GameStatus {
        self.status
    }

    pub fn started_at(&self) -> Option<DateTime<Utc>> {
        self.started_at
    }

    pub fn ended_at(&self) -> Option<DateTime<Utc>> {
        self.ended_at
    }

    pub fn winner_user_id(&self) -> Option<BenutzerId> {
        self.winner_user_id
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn playable_tile_count(&self) -> usize {
        self.grid_size.playable_tile_count(self.include_free_space)
    }

    pub fn has_center_free_space(&self) -> bool {
        self.grid_size
            .has_center_free_space(self.include_free_space)
    }

    pub fn update_settings(
        &mut self,
        title: NonEmptyString,
        grid_size: GridSize,
        visibility_mode: VisibilityMode,
        include_free_space: bool,
    ) -> ResultReport<(), GameError> {
        if self.status != GameStatus::Lobby {
            return Err(Report::new(GameError::InvalidStateTransition));
        }
        self.title = title;
        self.grid_size = grid_size;
        self.visibility_mode = visibility_mode;
        self.include_free_space = include_free_space;
        Ok(())
    }

    pub fn start_drafting(&mut self, player_count: usize) -> ResultReport<(), GameError> {
        if self.status != GameStatus::Lobby {
            return Err(Report::new(GameError::InvalidStateTransition));
        }
        if player_count < MIN_PLAYERS {
            return Err(Report::new(GameError::TooFewPlayers));
        }
        self.status = GameStatus::Drafting;
        Ok(())
    }

    pub fn start_active(&mut self, now: DateTime<Utc>) -> ResultReport<(), GameError> {
        if self.status != GameStatus::Drafting {
            return Err(Report::new(GameError::InvalidStateTransition));
        }
        self.status = GameStatus::Active;
        self.started_at = Some(now);
        Ok(())
    }

    pub fn complete(
        &mut self,
        winner_user_id: BenutzerId,
        now: DateTime<Utc>,
    ) -> ResultReport<(), GameError> {
        if self.status != GameStatus::Active {
            return Err(Report::new(GameError::InvalidStateTransition));
        }
        self.status = GameStatus::Completed;
        self.winner_user_id = Some(winner_user_id);
        self.ended_at = Some(now);
        Ok(())
    }
}

/// Participation record of one player in a game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GamePlayer {
    pub game_id: GameId,
    pub user_id: BenutzerId,
    pub joined_at: DateTime<Utc>,
    pub draft_locked: bool,
}

/// One submitted prompt authored by `author_user_id` targeting `target_user_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptSubmission {
    pub id: PromptSubmissionId,
    pub game_id: GameId,
    pub author_user_id: BenutzerId,
    pub target_user_id: BenutzerId,
    pub text: NonEmptyString,
    pub source: PromptSource,
}

impl PromptSubmission {
    pub fn new(
        id: PromptSubmissionId,
        game_id: GameId,
        author_user_id: BenutzerId,
        target_user_id: BenutzerId,
        text: NonEmptyString,
        source: PromptSource,
    ) -> ResultReport<Self, GameError> {
        if author_user_id == target_user_id {
            return Err(Report::new(GameError::SelfTargetNotAllowed));
        }
        Ok(Self {
            id,
            game_id,
            author_user_id,
            target_user_id,
            text,
            source,
        })
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
    game: &Game,
    players: &[BenutzerId],
    prompts: &[PromptSubmission],
    rng: &mut impl Rng,
) -> ResultReport<Vec<(Board, Vec<BoardTile>)>, GameError> {
    let n = players.len();
    if n < MIN_PLAYERS {
        return Err(Report::new(GameError::TooFewPlayers));
    }

    let playable_count = game.playable_tile_count();
    let expected_total = n * playable_count;
    if prompts.len() != expected_total {
        return Err(Report::new(GameError::InvalidPromptQuota));
    }

    // Shuffle prompts first so matching picks a random valid assignment every game.
    let mut shuffled_prompts: Vec<&PromptSubmission> = prompts.iter().collect();
    shuffled_prompts.shuffle(rng);

    // Create slots: `n * playable_count` slots, each owned by `players[slot_idx / playable_count]`.
    let mut slot_owners: Vec<BenutzerId> = Vec::with_capacity(expected_total);
    for &owner in players {
        for _ in 0..playable_count {
            slot_owners.push(owner);
        }
    }

    // Build adjacency list: for each slot `s`, eligible prompt indices in randomized order.
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

    // Kuhn's bipartite matching: match every slot to a distinct eligible prompt.
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

    let side = game.grid_size().side() as u8;
    let center = side / 2;
    let has_free = game.has_center_free_space();

    let mut result = Vec::with_capacity(n);
    for (player_idx, &owner) in players.iter().enumerate() {
        let board_id = BoardId(Uuid::new_v4());
        let board = Board::new(
            board_id,
            NewBoard {
                game_id: game.id(),
                owner_user_id: owner,
            },
        );

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

        let mut tiles = Vec::with_capacity((side as usize) * (side as usize));
        let mut prompt_iter = assigned_prompts.into_iter();
        let mut seen_prompts: HashSet<PromptSubmissionId> = HashSet::new();

        for r in 0..side {
            for c in 0..side {
                if has_free && r == center && c == center {
                    tiles.push(BoardTile::new(
                        BoardTileId(Uuid::new_v4()),
                        NewBoardTile {
                            board_id,
                            row: r,
                            col: c,
                            prompt_submission_id: None,
                            target_user_id: None,
                            prompt_text: NonEmptyString::neu("Group Selfie / Free Space")
                                .expect("static non-empty"),
                            is_free_space: true,
                        },
                    ));
                } else {
                    let p = prompt_iter
                        .next()
                        .ok_or_else(|| Report::new(GameError::BoardAssemblyFailed))?;
                    seen_prompts.insert(p.id);
                    tiles.push(BoardTile::new(
                        BoardTileId(Uuid::new_v4()),
                        NewBoardTile {
                            board_id,
                            row: r,
                            col: c,
                            prompt_submission_id: Some(p.id),
                            target_user_id: Some(p.target_user_id),
                            prompt_text: p.text.clone(),
                            is_free_space: false,
                        },
                    ));
                }
            }
        }
        if seen_prompts.len() != playable_count {
            return Err(Report::new(GameError::BoardAssemblyFailed));
        }

        result.push((board, tiles));
    }

    Ok(result)
}

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

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;

    fn test_players(count: usize) -> Vec<BenutzerId> {
        (1..=count)
            .map(|i| BenutzerId(Uuid::from_u128(i as u128)))
            .collect()
    }

    fn build_valid_prompts(
        game_id: GameId,
        players: &[BenutzerId],
        total_tiles: usize,
    ) -> Vec<PromptSubmission> {
        let mut out = Vec::new();
        for &author in players {
            let quotas = target_quotas_for_author(players, author, total_tiles).unwrap();
            for (target, count) in quotas {
                for idx in 0..count {
                    out.push(
                        PromptSubmission::new(
                            PromptSubmissionId(Uuid::new_v4()),
                            game_id,
                            author,
                            target,
                            NonEmptyString::neu(format!("{author:?}->{target:?} #{idx}")).unwrap(),
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
    fn assemble_boards_enforces_author_and_target_exclusion_for_all_grid_sizes() {
        let mut rng = StdRng::seed_from_u64(42);
        for player_count in [3usize, 4, 5, 7] {
            for (grid_size, free) in [
                (GridSize::Three, true),
                (GridSize::Three, false),
                (GridSize::Four, false),
                (GridSize::Five, true),
            ] {
                let players = test_players(player_count);
                let game = Game::new(
                    GameId(Uuid::new_v4()),
                    NewGame {
                        invite_code: InviteCode::new("TEST12").unwrap(),
                        host_user_id: players[0],
                        title: NonEmptyString::neu("Testabend").unwrap(),
                        grid_size,
                        visibility_mode: VisibilityMode::Undercover,
                        include_free_space: free,
                        created_at: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
                    },
                );
                let prompts = build_valid_prompts(game.id(), &players, game.playable_tile_count());
                for &author in &players {
                    let mine: Vec<_> = prompts
                        .iter()
                        .filter(|p| p.author_user_id == author)
                        .cloned()
                        .collect();
                    validate_author_prompts(&players, author, game.playable_tile_count(), &mine)
                        .unwrap();
                }

                let boards = assemble_boards(&game, &players, &prompts, &mut rng).unwrap();
                assert_eq!(boards.len(), player_count);

                let mut used_ids = HashSet::new();
                for (board, tiles) in boards {
                    let owner = board.owner_user_id();
                    assert_eq!(tiles.len(), grid_size.side() * grid_size.side());
                    for tile in tiles {
                        if tile.is_free_space() {
                            assert!(game.has_center_free_space());
                            assert_eq!(tile.target_user_id(), None);
                            continue;
                        }
                        let pid = tile.prompt_submission_id().unwrap();
                        assert!(used_ids.insert(pid), "prompt reused across boards");
                        let orig = prompts.iter().find(|p| p.id == pid).unwrap();
                        assert_ne!(orig.author_user_id, owner);
                        assert_ne!(orig.target_user_id, owner);
                        assert_ne!(tile.target_user_id().unwrap(), owner);
                    }
                }
                assert_eq!(used_ids.len(), prompts.len());
            }
        }
    }

    #[test_log::test]
    fn self_target_prompt_is_rejected() {
        let p = BenutzerId(Uuid::new_v4());
        let err = PromptSubmission::new(
            PromptSubmissionId(Uuid::new_v4()),
            GameId(Uuid::new_v4()),
            p,
            p,
            NonEmptyString::neu("Selfie").unwrap(),
            PromptSource::Custom,
        )
        .unwrap_err();
        assert!(matches!(
            err.current_context(),
            GameError::SelfTargetNotAllowed
        ));
    }
}
