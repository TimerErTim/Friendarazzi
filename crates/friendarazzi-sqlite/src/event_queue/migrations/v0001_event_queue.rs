use async_trait::async_trait;
use molting::UpMigration;

pub struct Migration;

#[async_trait]
impl UpMigration<libsql::Transaction, libsql::Error> for Migration {
    fn version(&self) -> usize {
        1
    }

    fn description(&self) -> Option<&'static str> {
        Some("Persistent event queue table")
    }

    async fn up(&self, transaction: &mut libsql::Transaction) -> Result<(), libsql::Error> {
        transaction
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS _event_queue (
                    id TEXT PRIMARY KEY NOT NULL,
                    payload TEXT NOT NULL,
                    status TEXT NOT NULL,
                    attempts INTEGER NOT NULL DEFAULT 0,
                    last_error TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    locked_at TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_event_queue_pending
                    ON _event_queue (status, created_at);
                ",
            )
            .await?;
        Ok(())
    }
}
