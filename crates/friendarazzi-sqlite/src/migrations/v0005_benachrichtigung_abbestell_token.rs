use async_trait::async_trait;
use molting::UpMigration;

/// Persist per-notification unsubscribe tokens in SQLite so one-click unsubscribe survives restarts.
pub struct Migration;

#[async_trait]
impl UpMigration<libsql::Transaction, libsql::Error> for Migration {
    fn version(&self) -> usize {
        5
    }

    fn description(&self) -> Option<&'static str> {
        Some("Add abbestell_token column to benachrichtigung table")
    }

    async fn up(&self, transaction: &mut libsql::Transaction) -> Result<(), libsql::Error> {
        transaction
            .execute_batch(
                r#"
            ALTER TABLE "benachrichtigung" ADD COLUMN abbestell_token TEXT;

            UPDATE "benachrichtigung"
               SET abbestell_token = lower(hex(randomblob(16)))
             WHERE abbestell_token IS NULL;

            CREATE UNIQUE INDEX IF NOT EXISTS "idx_benachrichtigung_abbestell_token"
                ON "benachrichtigung" (abbestell_token);
        "#,
            )
            .await?;
        Ok(())
    }
}
