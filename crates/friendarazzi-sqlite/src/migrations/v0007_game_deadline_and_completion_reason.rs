use async_trait::async_trait;
use molting::UpMigration;

/// Persist optional game completion deadline (`ends_at`) and completion reason (`Bingo`, `Deadline`, `Abandoned`).
pub struct Migration;

#[async_trait]
impl UpMigration<libsql::Transaction, libsql::Error> for Migration {
    fn version(&self) -> usize {
        7
    }

    fn description(&self) -> Option<&'static str> {
        Some("Add ends_at and completion_reason columns to game")
    }

    async fn up(&self, transaction: &mut libsql::Transaction) -> Result<(), libsql::Error> {
        transaction
            .execute_batch(
                r#"
                ALTER TABLE "game" ADD COLUMN ends_at TEXT;
                ALTER TABLE "game" ADD COLUMN completion_reason TEXT;
                CREATE INDEX IF NOT EXISTS "idx_game_active_ends_at"
                    ON "game" (status, ends_at)
                    WHERE ends_at IS NOT NULL;
                UPDATE "game"
                    SET completion_reason = 'Bingo'
                    WHERE status = 'Completed' AND completion_reason IS NULL;
                "#,
            )
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use friendarazzi_core::uow::UnitOfWorkProvider;

    use crate::{SQLiteInstance, migrations::MIGRATIONS};

    #[test_log::test(tokio::test)]
    async fn upgrades_from_v6_with_ends_at_and_completion_reason() {
        let sqlite = SQLiteInstance::in_temp_dir().await.unwrap();
        let mut conn = sqlite.create_connection().await.unwrap();
        MIGRATIONS.apply(&mut conn, Some(6)).await.unwrap();
        drop(conn);

        sqlite.migrate_repos_to_latest().await.unwrap();

        let uow = sqlite.begin().await.unwrap();
        let decks = uow.decks().find_all().await.unwrap();
        assert_eq!(decks.len(), 4);
        uow.rollback().await.unwrap();
    }
}
