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
                    dedupe_key TEXT,
                    payload TEXT NOT NULL,
                    run_at TEXT NOT NULL,
                    status TEXT NOT NULL,
                    attempts INTEGER NOT NULL DEFAULT 0,
                    last_error TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    locked_at TEXT
                );
                CREATE UNIQUE INDEX IF NOT EXISTS idx_event_queue_pending_dedupe
                    ON _event_queue (dedupe_key)
                    WHERE dedupe_key IS NOT NULL AND status = 'pending';
                CREATE INDEX IF NOT EXISTS idx_event_queue_due
                    ON _event_queue (status, run_at);
                ",
            )
            .await?;
        Ok(())
    }
}
