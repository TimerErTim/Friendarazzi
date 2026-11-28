use std::future::Future;
use std::pin::Pin;
use std::sync::LazyLock;

use async_trait::async_trait;
use molting::{AppliableMigration, MigrationRegistry, MigrationTarget, UpMigration};

use crate::SQLiteConnection;

mod v0001_konto;
mod v0002_mvp;
mod v0003_ready_grace_period;
mod v0004_benachrichtigungen;
mod v0005_target_agnostic_starter_decks;
mod v0006_draft_deadline_and_expanded_decks;
mod v0007_game_deadline_and_completion_reason;
mod v0008_email_verified_at;
mod v0009_oidc_identity_providers;

type Registry = MigrationRegistry<dyn UpMigration<libsql::Transaction, libsql::Error>>;

/// Baseline plus one migration per schema change, registered in ascending version order.
pub static MIGRATIONS: LazyLock<Registry> = LazyLock::new(|| {
    let mut registry: Registry = MigrationRegistry::new();
    registry.add(v0001_konto::Migration);
    registry.add(v0002_mvp::Migration);
    registry.add(v0003_ready_grace_period::Migration);
    registry.add(v0004_benachrichtigungen::Migration);
    registry.add(v0005_target_agnostic_starter_decks::Migration);
    registry.add(v0006_draft_deadline_and_expanded_decks::Migration);
    registry.add(v0007_game_deadline_and_completion_reason::Migration);
    registry.add(v0008_email_verified_at::Migration);
    registry.add(v0009_oidc_identity_providers::Migration);
    registry
});

#[async_trait]
impl MigrationTarget<libsql::Transaction, libsql::Error> for SQLiteConnection {
    fn get_current_version(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<usize>, libsql::Error>> + '_>> {
        Box::pin(async move {
            let tx = self
                .transaction_with_behavior(libsql::TransactionBehavior::Exclusive)
                .await?;
            tx.execute(
                "CREATE TABLE IF NOT EXISTS _migration_history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    version INTEGER,
                    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )",
                (),
            )
            .await?;

            let mut rows = tx
                .query(
                    "SELECT version FROM _migration_history ORDER BY applied_at DESC, version DESC LIMIT 1",
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
        let mut tx = self
            .transaction_with_behavior(libsql::TransactionBehavior::Exclusive)
            .await?;

        implementation.run(&mut tx).await?;

        tx.execute(
            "INSERT INTO _migration_history (version) VALUES (?1)",
            [new_version.map(|v| v as i64)],
        )
        .await?;
        tx.query(
            &format!(
                "PRAGMA user_version = {}",
                new_version.map(|v| v as i64).unwrap_or(0)
            ),
            (),
        )
        .await?;

        tx.commit().await?;

        Ok(())
    }
}
