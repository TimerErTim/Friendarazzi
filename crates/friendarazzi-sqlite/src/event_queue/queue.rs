use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use error_stack::ResultExt;
use friendarazzi_core::domain::{BenutzerId, CaptureId, CompletionReason, GameId};
use friendarazzi_core::ports::{EventQueue, EventQueueError};
use friendarazzi_core::service::{
    CaptureUploaded, Event, GameFinished, GameStarted, NearBingoReached,
};
use friendarazzi_core::{ErrorReportExt, ResultReport};
use molting::{AppliableMigration, MigrationTarget};
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;
use tracing::debug;
use uuid::Uuid;

use crate::SQLiteInstance;
use crate::event_queue::migrations::MIGRATIONS;

#[derive(Debug, thiserror::Error)]
pub enum SqliteEventQueueError {
    #[error("sqlite event queue migration failed")]
    Migration,
    #[error("sqlite event queue persistence error")]
    Persistence,
    #[error("sqlite event queue serialization error")]
    Serialization,
}

fn default_completion_reason() -> String {
    CompletionReason::Bingo.as_str().to_owned()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
enum PersistedEvent {
    GameStarted {
        game_id: Uuid,
        started_at: DateTime<Utc>,
    },
    CaptureUploaded {
        game_id: Uuid,
        capture_id: Uuid,
        uploader_id: Uuid,
        target_id: Uuid,
        occurred_at: DateTime<Utc>,
    },
    NearBingoReached {
        game_id: Uuid,
        leader_id: Uuid,
        occurred_at: DateTime<Utc>,
    },
    GameFinished {
        game_id: Uuid,
        winner_id: Option<Uuid>,
        #[serde(default = "default_completion_reason")]
        completion_reason: String,
        finished_at: DateTime<Utc>,
    },
}

impl From<&Event> for PersistedEvent {
    fn from(event: &Event) -> Self {
        match event {
            Event::GameStarted(ev) => Self::GameStarted {
                game_id: ev.game_id.0,
                started_at: ev.started_at,
            },
            Event::CaptureUploaded(ev) => Self::CaptureUploaded {
                game_id: ev.game_id.0,
                capture_id: ev.capture_id.0,
                uploader_id: ev.uploader_id.0,
                target_id: ev.target_id.0,
                occurred_at: ev.occurred_at,
            },
            Event::NearBingoReached(ev) => Self::NearBingoReached {
                game_id: ev.game_id.0,
                leader_id: ev.leader_id.0,
                occurred_at: ev.occurred_at,
            },
            Event::GameFinished(ev) => Self::GameFinished {
                game_id: ev.game_id.0,
                winner_id: ev.winner_id.map(|id| id.0),
                completion_reason: ev.completion_reason.as_str().to_owned(),
                finished_at: ev.finished_at,
            },
        }
    }
}

impl From<PersistedEvent> for Event {
    fn from(persisted: PersistedEvent) -> Self {
        match persisted {
            PersistedEvent::GameStarted {
                game_id,
                started_at,
            } => Event::GameStarted(Box::new(GameStarted {
                game_id: GameId(game_id),
                started_at,
            })),
            PersistedEvent::CaptureUploaded {
                game_id,
                capture_id,
                uploader_id,
                target_id,
                occurred_at,
            } => Event::CaptureUploaded(Box::new(CaptureUploaded {
                game_id: GameId(game_id),
                capture_id: CaptureId(capture_id),
                uploader_id: BenutzerId(uploader_id),
                target_id: BenutzerId(target_id),
                occurred_at,
            })),
            PersistedEvent::NearBingoReached {
                game_id,
                leader_id,
                occurred_at,
            } => Event::NearBingoReached(Box::new(NearBingoReached {
                game_id: GameId(game_id),
                leader_id: BenutzerId(leader_id),
                occurred_at,
            })),
            PersistedEvent::GameFinished {
                game_id,
                winner_id,
                completion_reason,
                finished_at,
            } => Event::GameFinished(Box::new(GameFinished {
                game_id: GameId(game_id),
                winner_id: winner_id.map(BenutzerId),
                completion_reason: completion_reason.parse().unwrap_or(CompletionReason::Bingo),
                finished_at,
            })),
        }
    }
}

pub(crate) fn serialize_event(event: &Event) -> Result<String, serde_json::Error> {
    serde_json::to_string(&PersistedEvent::from(event))
}

pub(crate) fn deserialize_event(raw: &str) -> Result<Event, serde_json::Error> {
    serde_json::from_str::<PersistedEvent>(raw).map(Event::from)
}

/// Persistent SQLite-backed [`EventQueue`] adapter sharing an [`SQLiteInstance`].
///
/// Events are persisted to `_event_queue` and processed asynchronously by the background
/// worker entrypoint [`run`](crate::event_queue::run) / [`run_with_config`](crate::event_queue::run_with_config).
pub struct SqliteEventQueue {
    instance: Arc<SQLiteInstance>,
    notify: Arc<Notify>,
}

impl SqliteEventQueue {
    pub fn new(instance: Arc<SQLiteInstance>) -> Self {
        Self {
            instance,
            notify: Arc::new(Notify::new()),
        }
    }

    pub(crate) fn instance(&self) -> &SQLiteInstance {
        self.instance.as_ref()
    }

    pub(crate) fn notify(&self) -> &Notify {
        self.notify.as_ref()
    }

    #[tracing::instrument(skip(self), err(Debug))]
    pub async fn migrate_to_latest(&mut self) -> ResultReport<(), SqliteEventQueueError> {
        MIGRATIONS
            .apply(self, None)
            .await
            .contextualize(SqliteEventQueueError::Migration)
    }

    pub async fn pending_count(&self) -> ResultReport<u64, SqliteEventQueueError> {
        self.count_by_status("pending").await
    }

    pub async fn failed_count(&self) -> ResultReport<u64, SqliteEventQueueError> {
        self.count_by_status("failed").await
    }

    async fn count_by_status(&self, status: &str) -> ResultReport<u64, SqliteEventQueueError> {
        let conn = self
            .instance
            .create_connection()
            .await
            .change_context(SqliteEventQueueError::Persistence)?;
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM _event_queue WHERE status = ?1",
                [status],
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
}

#[async_trait]
impl EventQueue for SqliteEventQueue {
    async fn enqueue(&self, events: Vec<Event>) -> ResultReport<(), EventQueueError> {
        if events.is_empty() {
            return Ok(());
        }

        let now_str = format_timestamp(Utc::now());
        let mut serialized = Vec::with_capacity(events.len());
        for event in &events {
            let id = Uuid::new_v4().to_string();
            let payload = serialize_event(event).contextualize(EventQueueError::Serialization)?;
            serialized.push((id, payload));
        }

        let conn = self
            .instance
            .create_connection()
            .await
            .change_context(EventQueueError::Unavailable)?;
        let tx = conn
            .transaction_with_behavior(libsql::TransactionBehavior::Immediate)
            .await
            .contextualize(EventQueueError::Unavailable)?;

        for (id, payload) in &serialized {
            tx.execute(
                "INSERT INTO _event_queue (
                    id, payload, status, attempts, created_at, updated_at
                ) VALUES (?1, ?2, 'pending', 0, ?3, ?3)",
                (id.as_str(), payload.as_str(), now_str.as_str()),
            )
            .await
            .contextualize(EventQueueError::Unavailable)?;
        }

        tx.commit()
            .await
            .contextualize(EventQueueError::Unavailable)?;

        debug!(
            count = serialized.len(),
            "persisted submitted events in sqlite queue"
        );
        self.notify.notify_one();
        Ok(())
    }
}

#[async_trait]
impl MigrationTarget<libsql::Transaction, libsql::Error> for SqliteEventQueue {
    fn get_current_version(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<usize>, libsql::Error>> + '_>> {
        Box::pin(async move {
            let conn = self.instance.create_connection().await.map_err(|_| {
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
            self.instance.create_connection().await.map_err(|_| {
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

pub(crate) fn format_timestamp(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Nanos, true)
}
