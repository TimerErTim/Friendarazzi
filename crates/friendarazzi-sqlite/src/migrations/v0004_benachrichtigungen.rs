use async_trait::async_trait;
use molting::UpMigration;

/// In-app notifications, per-user notification preferences, and per-game watch state.
pub struct Migration;

#[async_trait]
impl UpMigration<libsql::Transaction, libsql::Error> for Migration {
    fn version(&self) -> usize {
        4
    }

    fn description(&self) -> Option<&'static str> {
        Some("Add in-app notifications, notification preferences, and game watching flag")
    }

    async fn up(&self, transaction: &mut libsql::Transaction) -> Result<(), libsql::Error> {
        transaction
            .execute_batch(
                r#"
            ALTER TABLE "game_player" ADD COLUMN watching INTEGER NOT NULL DEFAULT 1;

            CREATE TABLE IF NOT EXISTS "benachrichtigung" (
                id TEXT PRIMARY KEY,
                empfaenger_id TEXT NOT NULL REFERENCES "benutzer" (id) ON DELETE CASCADE,
                art TEXT NOT NULL,
                game_id TEXT NOT NULL REFERENCES "game" (id) ON DELETE CASCADE,
                capture_id TEXT,
                actor_id TEXT,
                target_id TEXT,
                dedupe_key TEXT NOT NULL,
                abbestell_token_hash TEXT NOT NULL,
                erstellt_am TEXT NOT NULL,
                gelesen_am TEXT,
                _version INTEGER NOT NULL DEFAULT 0,
                UNIQUE (empfaenger_id, dedupe_key),
                UNIQUE (abbestell_token_hash)
            );

            CREATE INDEX IF NOT EXISTS "idx_benachrichtigung_empfaenger"
                ON "benachrichtigung" (empfaenger_id, erstellt_am DESC);

            ALTER TABLE "benutzer" ADD COLUMN default_watch_games INTEGER NOT NULL DEFAULT 1;
            ALTER TABLE "benutzer" ADD COLUMN default_watch_updated_at TEXT NOT NULL DEFAULT '';
            ALTER TABLE "benutzer" ADD COLUMN email_game_started INTEGER NOT NULL DEFAULT 1;
            ALTER TABLE "benutzer" ADD COLUMN email_game_started_updated_at TEXT NOT NULL DEFAULT '';
            ALTER TABLE "benutzer" ADD COLUMN email_capture_posted INTEGER NOT NULL DEFAULT 1;
            ALTER TABLE "benutzer" ADD COLUMN email_capture_posted_updated_at TEXT NOT NULL DEFAULT '';
            ALTER TABLE "benutzer" ADD COLUMN email_game_completed INTEGER NOT NULL DEFAULT 1;
            ALTER TABLE "benutzer" ADD COLUMN email_game_completed_updated_at TEXT NOT NULL DEFAULT '';

            UPDATE "benutzer" SET
                default_watch_updated_at = registered_at,
                email_game_started_updated_at = registered_at,
                email_capture_posted_updated_at = registered_at,
                email_game_completed_updated_at = registered_at
            WHERE default_watch_updated_at = '';
        "#,
            )
            .await?;
        Ok(())
    }
}
