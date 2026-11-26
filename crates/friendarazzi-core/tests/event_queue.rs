use std::sync::Arc;

use chrono::Utc;
use error_stack::Report;
use friendarazzi_core::{
    App,
    adapters::BlockingEventQueue,
    application::{AuthContext, IdentityProviderConfiguration},
    domain::{BenutzerId, CaptureId, GameId},
    ports::RepositoryError,
    service::{CaptureUpdated, Event, handle_event},
};
use friendarazzi_fakes::{FakeUnitOfWorkProvider, RecordingEventQueue};
use uuid::Uuid;

fn sample_event() -> CaptureUpdated {
    CaptureUpdated {
        game_id: GameId(Uuid::new_v4()),
        capture_id: CaptureId(Uuid::new_v4()),
        uploader_id: BenutzerId(Uuid::new_v4()),
        target_id: BenutzerId(Uuid::new_v4()),
        occurred_at: Utc::now(),
    }
}

#[test_log::test(pollster::test)]
async fn staged_events_are_only_dispatched_after_execution_succeeds() {
    let recorder = Arc::new(RecordingEventQueue::new());
    let app = App::builder()
        .uow_provider(Box::new(FakeUnitOfWorkProvider::empty()))
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .event_queue(Arc::clone(&recorder))
        .build();

    let dummy = sample_event();

    let err_res = app
        .execute_fn::<_, (), RepositoryError>(AuthContext::Anonymous, {
            let ev = dummy.clone();
            async move |ctx| {
                let uow = ctx.enter().await?;
                ctx.events().submit(ev);
                uow.rollback().await?;
                Err(Report::new(RepositoryError::Conflict))
            }
        })
        .await;
    assert!(err_res.is_err());
    assert_eq!(recorder.total_events(), 0);

    app.execute_fn::<_, (), RepositoryError>(AuthContext::Anonymous, {
        let ev = dummy.clone();
        async move |ctx| {
            let uow = ctx.enter().await?;
            ctx.events().submit(ev);
            uow.commit().await
        }
    })
    .await
    .unwrap();
    assert_eq!(recorder.total_events(), 1);
}

#[test_log::test(pollster::test)]
async fn blocking_event_queue_runs_after_uow_commit_without_deadlock() {
    let app = App::builder()
        .uow_provider(Box::new(FakeUnitOfWorkProvider::empty()))
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .event_queue(BlockingEventQueue::from_app)
        .build();

    let ev = sample_event();
    app.execute_fn::<_, (), RepositoryError>(AuthContext::Anonymous, async move |ctx| {
        let uow = ctx.enter().await?;
        ctx.events().submit(ev);
        uow.commit().await
    })
    .await
    .unwrap();
}

#[test_log::test(pollster::test)]
async fn default_event_queue_executes_submitted_events() {
    let app = App::builder()
        .uow_provider(Box::new(FakeUnitOfWorkProvider::empty()))
        .konto(IdentityProviderConfiguration::neu(
            "friendarazzi.app",
            "friendarazzi-api",
        ))
        .build();

    let ev = sample_event();
    handle_event(&app, Event::from(ev.clone())).await.unwrap();

    app.execute_fn::<_, (), RepositoryError>(AuthContext::Anonymous, async move |ctx| {
        let uow = ctx.enter().await?;
        ctx.events().submit(ev);
        uow.commit().await
    })
    .await
    .unwrap();

    let drop_start = std::time::Instant::now();
    drop(app);
    assert!(drop_start.elapsed() < std::time::Duration::from_secs(1));
}
