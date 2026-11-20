use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use error_stack::{Report, ResultExt};
use friendarazzi_core::adapters::SystemClock;
use friendarazzi_core::ports::{Clock, EventQueue, EventQueueError};
use friendarazzi_core::service::Event;
use friendarazzi_core::{ErrorReportExt, ResultReport};
use friendarazzi_sqlite::SQLiteInstance;
use molting::{AppliableMigration, MigrationTarget};
use tokio::sync::Notify;
use tracing::{debug, trace};
use uuid::Uuid;

use crate::migrations::MIGRATIONS;

#[derive(Debug, thiserror::Error)]
pub enum SqliteEventQueueError {
    #[error("sqlite event queue migration failed")]
    Migration,
    #[error("sqlite event queue persistence error")]
    Persistence,
    #[error("sqlite event queue serialization error")]
    Serialization,
}

/// Persistent SQLite-backed [`EventQueue`] adapter sharing an [`SQLiteInstance`].
///
/// Events are written to `_event_queue` and executed asynchronously by the background
/// worker entrypoint [`run`](crate::run) / [`run_with_config`](crate::run_with_config).
pub struct SqliteEventQueue {
    instance: Arc<SQLiteInstance>,
    clock: Arc<dyn Clock>,
    notify: Arc<Notify>,
}

impl SqliteEventQueue {
    pub fn new(instance: Arc<SQLiteInstance>) -> Self {
        Self {
            instance,
            clock: Arc::new(SystemClock),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    pub(crate) fn instance(&self) -> &SQLiteInstance {
        self.instance.as_ref()
    }

    pub(crate) fn clock(&self) -> &dyn Clock {
        self.clock.as_ref()
    }

    pub(crate) fn notify(&self) -> &Notify {
        self.notify.as_ref()
    }

    #[tracing::instrument(skip(self), err(Debug))]
    pub async fn migrate_to_latest(&self) -> ResultReport<(), SqliteEventQueueError> {
        let mut target = EventQueueMigrationTarget(self.instance.as_ref());
        MIGRATIONS
            .apply(&mut target, None)
            .await
            .contextualize(SqliteEventQueueError::Migration)
    }

    pub async fn pending_count(&self) -> ResultReport<u64, SqliteEventQueueError> {
        let conn = self
            .instance
            .create_connection()
            .await
            .change_context(SqliteEventQueueError::Persistence)?;
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM _event_queue WHERE status = 'pending'",
                (),
            )
            .await
            .contextualize(SqliteEventQueueError::Persistence)?;
        let Some(row) = rows
            .next()
            .await
            .contextualize(SqliteEventQueueError::Persistence)?
        else {
            return Ok(0);
        };
        let count: i64 = row
            .get(0)
            .contextualize(SqliteEventQueueError::Persistence)?;
        Ok(count.max(0) as u64)
    }

    pub async fn failed_count(&self) -> ResultReport<u64, SqliteEventQueueError> {
        let conn = self
            .instance
            .create_connection()
            .await
            .change_context(SqliteEventQueueError::Persistence)?;
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM _event_queue WHERE status = 'failed'",
                (),
            )
            .await
            .contextualize(SqliteEventQueueError::Persistence)?;
        let Some(row) = rows
            .next()
            .await
            .contextualize(SqliteEventQueueError::Persistence)?
        else {
            return Ok(0);
        };
        let count: i64 = row
            .get(0)
            .contextualize(SqliteEventQueueError::Persistence)?;
        Ok(count.max(0) as u64)
    }

    async fn insert_event(
        &self,
        event: Event,
        run_at: DateTime<Utc>,
        key: Option<&str>,
    ) -> ResultReport<(), EventQueueError> {
        let payload =
            serde_json::to_string(&event).contextualize(EventQueueError::Serialization)?;
        let now_str = format_timestamp(self.clock.now());
        let run_at_str = format_timestamp(run_at);
        let id = Uuid::new_v4().to_string();
        let kind = event.kind();

        let conn = self
            .instance
            .create_connection()
            .await
            .change_context(EventQueueError::Unavailable)?;
        let tx = conn
            .transaction_with_behavior(libsql::TransactionBehavior::Immediate)
            .await
            .contextualize(EventQueueError::Unavailable)?;

        if let Some(dedupe_key) = key {
            tx.execute(
                "DELETE FROM _event_queue WHERE dedupe_key = ?1 AND status = 'pending'",
                [dedupe_key],
            )
            .await
            .contextualize(EventQueueError::Unavailable)?;
        }

        tx.execute(
            "INSERT INTO _event_queue (
                id, dedupe_key, payload, run_at, status, attempts, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, 'pending', 0, ?5, ?5)",
            (
                id.as_str(),
                key,
                payload.as_str(),
                run_at_str.as_str(),
                now_str.as_str(),
            ),
        )
        .await
        .contextualize(EventQueueError::Unavailable)?;

        tx.commit()
            .await
            .contextualize(EventQueueError::Unavailable)?;

        debug!(event_id = %id, kind, %run_at, key, "persisted event in sqlite queue");
        self.notify.notify_one();
        Ok(())
    }
}

#[async_trait]
impl EventQueue for SqliteEventQueue {
    async fn enqueue(&self, event: Event) -> ResultReport<(), EventQueueError> {
        let now = self.clock.now();
        self.insert_event(event, now, None).await
    }

    async fn schedule(
        &self,
        event: Event,
        run_at: DateTime<Utc>,
        key: Option<&str>,
    ) -> ResultReport<(), EventQueueError> {
        self.insert_event(event, run_at, key).await
    }

    async fn cancel(&self, key: &str) -> ResultReport<(), EventQueueError> {
        let conn = self
            .instance
            .create_connection()
            .await
            .change_context(EventQueueError::Unavailable)?;
        let removed = conn
            .execute(
                "DELETE FROM _event_queue WHERE dedupe_key = ?1 AND status = 'pending'",
                [key],
            )
            .await
            .contextualize(EventQueueError::Unavailable)?;
        trace!(key, removed, "cancelled pending event in sqlite queue");
        if removed > 0 {
            self.notify.notify_one();
        }
        Ok(())
    }
}

struct EventQueueMigrationTarget<'a>(&'a SQLiteInstance);

#[async_trait]
impl MigrationTarget<libsql::Transaction, libsql::Error> for EventQueueMigrationTarget<'_> {
    fn get_current_version(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<usize>, libsql::Error>> + '_>> {
        Box::pin(async move {
            let conn = self.0.create_connection().await.map_err(|_| {
                libsql::Error::Misuse("event queue migration connection failed".into())
            })?;
            let tx = conn
                .transaction_with_behavior(libsql::TransactionBehavior::Exclusive)
                .await?;
            tx.execute(
                "CREATE TABLE IF NOT EXISTS _event_queue_migration_history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    version INTEGER,
                    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )",
                (),
            )
            .await?;

            let mut rows = tx
                .query(
                    "SELECT version FROM _event_queue_migration_history ORDER BY applied_at DESC, version DESC LIMIT 1",
                    (),
                )
                .await?;

            tx.commit().await?;
            let Some(row) = rows.next().await? else {
                return Ok(None);
            };
            let version: Option<u64> = row.get(0)?;
            Ok(version.map(|v| v as usize))
        })
    }

    async fn apply_migration(
        &mut self,
        new_version: Option<usize>,
        implementation: impl AppliableMigration<libsql::Transaction, libsql::Error> + Send,
    ) -> Result<(), libsql::Error> {
        let conn =
            self.0.create_connection().await.map_err(|_| {
                libsql::Error::Misuse("event queue migration connection failed".into())
            })?;
        let mut tx = conn
            .transaction_with_behavior(libsql::TransactionBehavior::Exclusive)
            .await?;

        implementation.run(&mut tx).await?;

        tx.execute(
            "INSERT INTO _event_queue_migration_history (version) VALUES (?1)",
            [new_version.map(|v| v as i64)],
        )
        .await?;

        tx.commit().await?;
        Ok(())
    }
}

/// Fixed-width UTC RFC3339 format (nanosecond precision) so SQLite `ORDER BY` and `<=`
/// string comparisons match chronological order even across sub-second timestamps.
pub(crate) fn format_timestamp(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

pub(crate) fn parse_timestamp(raw: &str) -> Result<DateTime<Utc>, Report<SqliteEventQueueError>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| Report::new(SqliteEventQueueError::Serialization))
}
