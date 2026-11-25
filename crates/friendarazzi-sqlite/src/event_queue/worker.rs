use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use error_stack::ResultExt;
use friendarazzi_core::{App, ErrorReportExt, ResultReport, service::handle_event};
use futures::FutureExt;
use tokio::task::JoinSet;
use tracing::{Instrument, debug, error, info, info_span, trace, warn};

use crate::event_queue::queue::{
    SqliteEventQueue, SqliteEventQueueError, deserialize_event, format_timestamp,
};

#[derive(Debug, Clone, Copy)]
pub struct EventQueueConfig {
    pub idle_poll_interval: Duration,
    pub max_attempts: u32,
    pub max_concurrency: usize,
    pub shutdown_grace_period: Duration,
}

impl Default for EventQueueConfig {
    fn default() -> Self {
        Self {
            idle_poll_interval: Duration::from_secs(5),
            max_attempts: 3,
            max_concurrency: 32,
            shutdown_grace_period: Duration::from_millis(250),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EventQueueReport {
    pub processed: u64,
    pub failed: u64,
    pub retried: u64,
    pub recovered: u64,
}

struct PendingRow {
    id: String,
    payload: String,
    attempts: u32,
}

enum TaskOutcome {
    Processed,
    Retried,
    Failed,
}

/// Run the SQLite event queue worker in the background until `shutdown` resolves.
///
/// Claimed events execute concurrently. On shutdown, active handlers are given a short
/// grace period to finish; any incomplete handlers are cancelled and reset to `'pending'`
/// so they resume cleanly on the next startup.
pub async fn run(
    queue: Arc<SqliteEventQueue>,
    app: App,
    shutdown: impl Future<Output = ()> + Send,
) -> ResultReport<EventQueueReport, SqliteEventQueueError> {
    run_with_config(queue, app, EventQueueConfig::default(), shutdown).await
}

#[tracing::instrument(skip(queue, app, shutdown), fields(?config), err(Debug))]
pub async fn run_with_config(
    queue: Arc<SqliteEventQueue>,
    app: App,
    config: EventQueueConfig,
    shutdown: impl Future<Output = ()> + Send,
) -> ResultReport<EventQueueReport, SqliteEventQueueError> {
    let mut report = EventQueueReport::default();

    let recovered = recover_stale_processing(&queue).await?;
    report.recovered = recovered;
    if recovered > 0 {
        warn!(recovered, "recovered stale processing events on startup");
    }

    info!("sqlite event queue worker started");

    let shutdown = shutdown.fuse();
    tokio::pin!(shutdown);
    let mut in_flight: JoinSet<ResultReport<TaskOutcome, SqliteEventQueueError>> = JoinSet::new();

    loop {
        tokio::select! {
            biased;
            () = &mut shutdown => {
                info!("sqlite event queue worker received shutdown signal");
                break;
            }
            Some(joined) = in_flight.join_next(), if !in_flight.is_empty() => {
                record_task_result(joined, &mut report);
                continue;
            }
            () = std::future::ready(()) => {}
        }

        if in_flight.len() < config.max_concurrency.max(1)
            && let Some(row) = fetch_next_pending(&queue).await?
        {
            if claim_event(&queue, &row.id).await? {
                let q = Arc::clone(&queue);
                let app_clone = app.clone();
                let max_attempts = config.max_attempts;
                in_flight.spawn(async move {
                    process_claimed_row(&q, &app_clone, max_attempts, row).await
                });
            }
            continue;
        }

        trace!(
            poll_interval = ?config.idle_poll_interval,
            in_flight = in_flight.len(),
            "sqlite event queue waiting"
        );
        tokio::select! {
            biased;
            () = &mut shutdown => {
                info!("sqlite event queue worker received shutdown signal while waiting");
                break;
            }
            Some(joined) = in_flight.join_next(), if !in_flight.is_empty() => {
                record_task_result(joined, &mut report);
            }
            () = queue.notify().notified() => {
                trace!("sqlite event queue woken by notification");
            }
            () = tokio::time::sleep(config.idle_poll_interval) => {}
        }
    }

    drain_on_shutdown(
        &queue,
        &mut in_flight,
        config.shutdown_grace_period,
        &mut report,
    )
    .await?;

    info!(
        processed = report.processed,
        failed = report.failed,
        retried = report.retried,
        recovered = report.recovered,
        "sqlite event queue worker stopped gracefully"
    );
    Ok(report)
}

fn record_task_result(
    joined: Result<ResultReport<TaskOutcome, SqliteEventQueueError>, tokio::task::JoinError>,
    report: &mut EventQueueReport,
) {
    match joined {
        Ok(Ok(TaskOutcome::Processed)) => report.processed += 1,
        Ok(Ok(TaskOutcome::Retried)) => report.retried += 1,
        Ok(Ok(TaskOutcome::Failed)) => report.failed += 1,
        Ok(Err(err)) => error!(?err, "sqlite event task failed updating queue state"),
        Err(err) if err.is_cancelled() => {}
        Err(err) => error!(?err, "sqlite event task panicked"),
    }
}

async fn drain_on_shutdown(
    queue: &SqliteEventQueue,
    in_flight: &mut JoinSet<ResultReport<TaskOutcome, SqliteEventQueueError>>,
    grace_period: Duration,
    report: &mut EventQueueReport,
) -> ResultReport<(), SqliteEventQueueError> {
    if in_flight.is_empty() {
        return Ok(());
    }

    let deadline = tokio::time::sleep(grace_period);
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            biased;
            Some(joined) = in_flight.join_next() => {
                record_task_result(joined, report);
                if in_flight.is_empty() {
                    break;
                }
            }
            () = &mut deadline => {
                debug!(
                    remaining = in_flight.len(),
                    "shutdown grace period elapsed; aborting sleeping event handlers"
                );
                in_flight.shutdown().await;
                break;
            }
        }
    }

    let reset = recover_stale_processing(queue).await?;
    if reset > 0 {
        debug!(
            reset,
            "reset uncompleted processing events back to pending on shutdown"
        );
    }
    Ok(())
}

async fn recover_stale_processing(
    queue: &SqliteEventQueue,
) -> ResultReport<u64, SqliteEventQueueError> {
    let conn = queue
        .instance()
        .create_connection()
        .await
        .change_context(SqliteEventQueueError::Persistence)?;
    let now_str = format_timestamp(queue.clock().now());
    let recovered = conn
        .execute(
            "UPDATE _event_queue
             SET status = 'pending', locked_at = NULL, updated_at = ?1
             WHERE status = 'processing'",
            [now_str.as_str()],
        )
        .await
        .contextualize(SqliteEventQueueError::Persistence)?;
    Ok(recovered)
}

async fn fetch_next_pending(
    queue: &SqliteEventQueue,
) -> ResultReport<Option<PendingRow>, SqliteEventQueueError> {
    let conn = queue
        .instance()
        .create_connection()
        .await
        .change_context(SqliteEventQueueError::Persistence)?;
    let mut rows = conn
        .query(
            "SELECT id, payload, attempts
             FROM _event_queue
             WHERE status = 'pending'
             ORDER BY created_at ASC
             LIMIT 1",
            (),
        )
        .await
        .contextualize(SqliteEventQueueError::Persistence)?;
    let Some(row) = rows
        .next()
        .await
        .contextualize(SqliteEventQueueError::Persistence)?
    else {
        return Ok(None);
    };
    let id: String = row
        .get(0)
        .contextualize(SqliteEventQueueError::Persistence)?;
    let payload: String = row
        .get(1)
        .contextualize(SqliteEventQueueError::Persistence)?;
    let attempts: i64 = row
        .get(2)
        .contextualize(SqliteEventQueueError::Persistence)?;
    Ok(Some(PendingRow {
        id,
        payload,
        attempts: attempts.max(0) as u32,
    }))
}

async fn claim_event(
    queue: &SqliteEventQueue,
    id: &str,
) -> ResultReport<bool, SqliteEventQueueError> {
    let conn = queue
        .instance()
        .create_connection()
        .await
        .change_context(SqliteEventQueueError::Persistence)?;
    let now_str = format_timestamp(queue.clock().now());
    let updated = conn
        .execute(
            "UPDATE _event_queue
             SET status = 'processing', locked_at = ?1, updated_at = ?1
             WHERE id = ?2 AND status = 'pending'",
            (now_str.as_str(), id),
        )
        .await
        .contextualize(SqliteEventQueueError::Persistence)?;
    Ok(updated > 0)
}

async fn process_claimed_row(
    queue: &SqliteEventQueue,
    app: &App,
    max_attempts: u32,
    row: PendingRow,
) -> ResultReport<TaskOutcome, SqliteEventQueueError> {
    let event = match deserialize_event(&row.payload) {
        Ok(event) => event,
        Err(err) => {
            error!(
                event_id = %row.id,
                ?err,
                "failed to deserialize persisted event; marking failed"
            );
            mark_failed(
                queue,
                &row.id,
                row.attempts + 1,
                &format!("deserialization error: {err}"),
            )
            .await?;
            return Ok(TaskOutcome::Failed);
        }
    };

    let attempt = row.attempts + 1;
    let span = info_span!("sqlite_event_queue_process", event_id = %row.id, attempt);

    let outcome = async {
        debug!("processing claimed sqlite event");
        handle_event(app, event).await
    }
    .instrument(span)
    .await;

    match outcome {
        Ok(()) => {
            delete_completed(queue, &row.id).await?;
            debug!(event_id = %row.id, "sqlite event completed");
            Ok(TaskOutcome::Processed)
        }
        Err(err_text) => {
            if attempt < max_attempts {
                requeue_retry(queue, &row.id, attempt, &err_text).await?;
                warn!(
                    event_id = %row.id,
                    attempt,
                    err = %err_text,
                    "sqlite event execution failed; requeued for retry"
                );
                Ok(TaskOutcome::Retried)
            } else {
                mark_failed(queue, &row.id, attempt, &err_text).await?;
                error!(
                    event_id = %row.id,
                    attempt,
                    err = %err_text,
                    "sqlite event exhausted retries; marked failed"
                );
                Ok(TaskOutcome::Failed)
            }
        }
    }
}

async fn delete_completed(
    queue: &SqliteEventQueue,
    id: &str,
) -> ResultReport<(), SqliteEventQueueError> {
    let conn = queue
        .instance()
        .create_connection()
        .await
        .change_context(SqliteEventQueueError::Persistence)?;
    conn.execute("DELETE FROM _event_queue WHERE id = ?1", [id])
        .await
        .contextualize(SqliteEventQueueError::Persistence)?;
    Ok(())
}

async fn requeue_retry(
    queue: &SqliteEventQueue,
    id: &str,
    attempts: u32,
    last_error: &str,
) -> ResultReport<(), SqliteEventQueueError> {
    let conn = queue
        .instance()
        .create_connection()
        .await
        .change_context(SqliteEventQueueError::Persistence)?;
    let now_str = format_timestamp(queue.clock().now());
    conn.execute(
        "UPDATE _event_queue
         SET status = 'pending', attempts = ?1, last_error = ?2, locked_at = NULL, updated_at = ?3
         WHERE id = ?4",
        (attempts as i64, last_error, now_str.as_str(), id),
    )
    .await
    .contextualize(SqliteEventQueueError::Persistence)?;
    queue.notify().notify_one();
    Ok(())
}

async fn mark_failed(
    queue: &SqliteEventQueue,
    id: &str,
    attempts: u32,
    last_error: &str,
) -> ResultReport<(), SqliteEventQueueError> {
    let conn = queue
        .instance()
        .create_connection()
        .await
        .change_context(SqliteEventQueueError::Persistence)?;
    let now_str = format_timestamp(queue.clock().now());
    conn.execute(
        "UPDATE _event_queue
         SET status = 'failed', attempts = ?1, last_error = ?2, locked_at = NULL, updated_at = ?3
         WHERE id = ?4",
        (attempts as i64, last_error, now_str.as_str(), id),
    )
    .await
    .contextualize(SqliteEventQueueError::Persistence)?;
    Ok(())
}
