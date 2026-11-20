use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use error_stack::ResultExt;
use friendarazzi_core::application::AuthContext;
use friendarazzi_core::service::Event;
use friendarazzi_core::{App, ErrorReportExt, ResultReport};
use futures::FutureExt;
use tracing::{Instrument, debug, error, info, info_span, trace, warn};

use crate::queue::{SqliteEventQueue, SqliteEventQueueError, format_timestamp, parse_timestamp};

#[derive(Debug, Clone, Copy)]
pub struct EventQueueConfig {
    pub idle_poll_interval: Duration,
    pub max_sleep_interval: Duration,
    pub max_attempts: u32,
    pub retry_backoff: Duration,
}

impl Default for EventQueueConfig {
    fn default() -> Self {
        Self {
            idle_poll_interval: Duration::from_secs(5),
            max_sleep_interval: Duration::from_secs(5),
            max_attempts: 3,
            retry_backoff: Duration::from_secs(5),
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
    run_at: DateTime<Utc>,
    attempts: u32,
}

/// Run the SQLite event queue worker in the background until `shutdown` resolves.
///
/// Any currently executing event finishes before returning. Pending and future events
/// remain safely persisted in SQLite for the next process startup.
pub async fn run(
    queue: Arc<SqliteEventQueue>,
    app: App,
    shutdown: impl Future<Output = ()> + Send,
) -> ResultReport<EventQueueReport, SqliteEventQueueError> {
    run_with_config(queue, app, EventQueueConfig::default(), shutdown).await
}

/// Run the SQLite event queue worker with an explicit [`EventQueueConfig`] until
/// `shutdown` resolves.
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

    loop {
        tokio::select! {
            biased;
            () = &mut shutdown => {
                info!("sqlite event queue worker received shutdown signal");
                break;
            }
            () = std::future::ready(()) => {}
        }

        let now = queue.clock().now();
        let next = fetch_next_pending(&queue).await?;

        match next {
            Some(row) if row.run_at <= now => {
                if !claim_event(&queue, &row.id, now).await? {
                    continue;
                }
                process_claimed_row(&queue, &app, &config, row, &mut report).await?;
            }
            Some(row) => {
                let wait_duration = (row.run_at - now)
                    .to_std()
                    .unwrap_or(Duration::ZERO)
                    .min(config.max_sleep_interval);
                trace!(
                    next_event_id = %row.id,
                    run_at = %row.run_at,
                    ?wait_duration,
                    "sqlite event queue waiting for next scheduled event"
                );
                tokio::select! {
                    biased;
                    () = &mut shutdown => {
                        info!("sqlite event queue worker received shutdown signal while waiting");
                        break;
                    }
                    () = queue.notify().notified() => {
                        trace!("sqlite event queue woken by notification");
                    }
                    () = tokio::time::sleep(wait_duration) => {}
                }
            }
            None => {
                trace!(
                    poll_interval = ?config.idle_poll_interval,
                    "sqlite event queue idle"
                );
                tokio::select! {
                    biased;
                    () = &mut shutdown => {
                        info!("sqlite event queue worker received shutdown signal while idle");
                        break;
                    }
                    () = queue.notify().notified() => {
                        trace!("sqlite event queue woken by notification");
                    }
                    () = tokio::time::sleep(config.idle_poll_interval) => {}
                }
            }
        }
    }

    info!(
        processed = report.processed,
        failed = report.failed,
        retried = report.retried,
        recovered = report.recovered,
        "sqlite event queue worker stopped gracefully"
    );
    Ok(report)
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
            "SELECT id, payload, run_at, attempts
             FROM _event_queue
             WHERE status = 'pending'
             ORDER BY run_at ASC, created_at ASC
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
    let run_at_raw: String = row
        .get(2)
        .contextualize(SqliteEventQueueError::Persistence)?;
    let attempts: i64 = row
        .get(3)
        .contextualize(SqliteEventQueueError::Persistence)?;
    Ok(Some(PendingRow {
        id,
        payload,
        run_at: parse_timestamp(&run_at_raw)?,
        attempts: attempts.max(0) as u32,
    }))
}

async fn claim_event(
    queue: &SqliteEventQueue,
    id: &str,
    now: DateTime<Utc>,
) -> ResultReport<bool, SqliteEventQueueError> {
    let conn = queue
        .instance()
        .create_connection()
        .await
        .change_context(SqliteEventQueueError::Persistence)?;
    let now_str = format_timestamp(now);
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
    config: &EventQueueConfig,
    row: PendingRow,
    report: &mut EventQueueReport,
) -> ResultReport<(), SqliteEventQueueError> {
    let event: Event = match serde_json::from_str(&row.payload) {
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
            report.failed += 1;
            return Ok(());
        }
    };

    let kind = event.kind();
    let attempt = row.attempts + 1;
    let span = info_span!(
        "sqlite_event_queue_process",
        event_id = %row.id,
        kind,
        attempt
    );

    let outcome = async {
        debug!("processing claimed sqlite event");
        app.execute(AuthContext::Anonymous, event).await
    }
    .instrument(span)
    .await;

    match outcome {
        Ok(()) => {
            delete_completed(queue, &row.id).await?;
            report.processed += 1;
            debug!(event_id = %row.id, kind, "sqlite event completed");
        }
        Err(err) => {
            let err_text = format!("{err:?}");
            if attempt < config.max_attempts {
                let backoff = chrono::Duration::from_std(config.retry_backoff)
                    .unwrap_or_else(|_| chrono::Duration::seconds(5));
                let retry_at = queue.clock().now() + backoff;
                reschedule_retry(queue, &row.id, attempt, retry_at, &err_text).await?;
                report.retried += 1;
                warn!(
                    event_id = %row.id,
                    kind,
                    attempt,
                    %retry_at,
                    ?err,
                    "sqlite event execution failed; scheduled retry"
                );
            } else {
                mark_failed(queue, &row.id, attempt, &err_text).await?;
                report.failed += 1;
                error!(
                    event_id = %row.id,
                    kind,
                    attempt,
                    ?err,
                    "sqlite event exhausted retries; marked failed"
                );
            }
        }
    }

    Ok(())
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

async fn reschedule_retry(
    queue: &SqliteEventQueue,
    id: &str,
    attempts: u32,
    retry_at: DateTime<Utc>,
    last_error: &str,
) -> ResultReport<(), SqliteEventQueueError> {
    let conn = queue
        .instance()
        .create_connection()
        .await
        .change_context(SqliteEventQueueError::Persistence)?;
    let now_str = format_timestamp(queue.clock().now());
    let retry_str = format_timestamp(retry_at);
    conn.execute(
        "UPDATE _event_queue
         SET status = 'pending', attempts = ?1, run_at = ?2, last_error = ?3, locked_at = NULL, updated_at = ?4
         WHERE id = ?5",
        (attempts as i64, retry_str.as_str(), last_error, now_str.as_str(), id),
    )
    .await
    .contextualize(SqliteEventQueueError::Persistence)?;
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
