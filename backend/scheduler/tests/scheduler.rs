mod cases;

use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::Duration;
use friendarazzi_core::domain::{CompletionReason, Game, READY_GRACE_PERIOD_SECONDS};
use friendarazzi_fakes::{FakeUnitOfWorkProvider, FixedClock};
use friendarazzi_scheduler::{InMemoryStateStore, run_finish_expired_games, run_start_ready_games};

#[test_log::test(tokio::test)]
async fn job_activates_ready_game_past_deadline() {
    let clock = Arc::new(FixedClock::new(cases::test_time()));
    let (app, game_id) =
        cases::setup_locked_drafting_game(Box::new(FakeUnitOfWorkProvider::empty()), clock.clone())
            .await;

    run_start_ready_games(&app).await.unwrap();
    assert!(matches!(
        cases::load_game(&app, game_id).await,
        Game::Drafting(_)
    ));

    clock.advance_by(Duration::seconds(READY_GRACE_PERIOD_SECONDS));
    run_start_ready_games(&app).await.unwrap();
    assert!(matches!(
        cases::load_game(&app, game_id).await,
        Game::Active(_)
    ));
}

#[test_log::test(tokio::test)]
async fn job_sleeps_in_task_when_deadline_is_within_domain_grace_period() {
    let clock = Arc::new(FixedClock::new(cases::test_time()));
    let (app, game_id) =
        cases::setup_locked_drafting_game(Box::new(FakeUnitOfWorkProvider::empty()), clock.clone())
            .await;

    // 50ms remaining falls inside READY_GRACE_PERIOD_SECONDS so the job sleeps in-place.
    clock.advance_by(Duration::seconds(READY_GRACE_PERIOD_SECONDS) - Duration::milliseconds(50));
    let advancer_clock = Arc::clone(&clock);
    tokio::spawn(async move {
        tokio::time::sleep(StdDuration::from_millis(20)).await;
        advancer_clock.advance_by(Duration::milliseconds(50));
    });

    run_start_ready_games(&app).await.unwrap();
    assert!(matches!(
        cases::load_game(&app, game_id).await,
        Game::Active(_)
    ));
}

#[test_log::test(tokio::test)]
async fn scheduler_framework_with_in_memory_store() {
    cases::scheduler_framework_runs_and_stops_cleanly(
        Box::new(FakeUnitOfWorkProvider::empty()),
        InMemoryStateStore::new(),
    )
    .await;
}

#[test_log::test(tokio::test)]
async fn job_finishes_expired_active_game_past_ends_at() {
    use friendarazzi_core::ports::Clock;

    let clock = Arc::new(FixedClock::new(cases::test_time()));
    let ends_at = clock.now() + Duration::minutes(10);
    let (app, game_id) = cases::setup_active_game_with_deadline(
        Box::new(FakeUnitOfWorkProvider::empty()),
        clock.clone(),
        ends_at,
    )
    .await;

    run_finish_expired_games(&app).await.unwrap();
    assert!(matches!(
        cases::load_game(&app, game_id).await,
        Game::Active(_)
    ));

    clock.advance_by(Duration::minutes(11));
    run_finish_expired_games(&app).await.unwrap();
    let Game::Completed(done) = cases::load_game(&app, game_id).await else {
        panic!("expected game to complete after ends_at");
    };
    assert_eq!(done.completion_reason(), CompletionReason::Deadline);
    assert_eq!(done.winner_user_id(), None);
}
