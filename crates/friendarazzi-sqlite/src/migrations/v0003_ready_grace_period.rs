use async_trait::async_trait;
use molting::UpMigration;

/// Persist the shared pre-game ready countdown deadline for reversible prompt locking.
pub struct Migration;

#[async_trait]
impl UpMigration<libsql::Transaction, libsql::Error> for Migration {
    fn version(&self) -> usize {
        3
    }

    fn description(&self) -> Option<&'static str> {
        Some("Add draft_closes_at countdown deadline column to game")
    }

    async fn up(&self, transaction: &mut libsql::Transaction) -> Result<(), libsql::Error> {
        transaction
            .execute_batch(r#"ALTER TABLE "game" ADD COLUMN draft_closes_at TEXT;"#)
            .await?;
        Ok(())
    }
}
