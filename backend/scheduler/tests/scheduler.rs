mod cases;

use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::Duration;
use friendarazzi_core::domain::{Game, READY_GRACE_PERIOD_SECONDS};
use friendarazzi_fakes::{FakeUnitOfWorkProvider, FixedClock};
use friendarazzi_scheduler::{InMemoryStateStore, run_start_ready_games};

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
