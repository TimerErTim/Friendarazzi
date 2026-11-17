//! Permission predicates for Friendarazzi games and captures.

use crate::domain::{Capture, Game, GamePlayer, GameStatus, VisibilityMode, konto::BenutzerId};

pub fn is_player_in_game(players: &[GamePlayer], user_id: BenutzerId) -> bool {
    players.iter().any(|p| p.user_id == user_id)
}

pub fn can_view_full_capture(
    game: &Game,
    players: &[GamePlayer],
    capture: &Capture,
    viewer: BenutzerId,
) -> bool {
    if !is_player_in_game(players, viewer) {
        return false;
    }
    game.status() == GameStatus::Completed
        || game.visibility_mode() == VisibilityMode::LiveFeed
        || capture.catcher_user_id() == viewer
}

pub fn can_view_blurred_capture(
    game: &Game,
    players: &[GamePlayer],
    capture: &Capture,
    viewer: BenutzerId,
) -> bool {
    if !is_player_in_game(players, viewer) {
        return false;
    }
    can_view_full_capture(game, players, capture, viewer)
        || game.visibility_mode() == VisibilityMode::BlurredTeaser
}
