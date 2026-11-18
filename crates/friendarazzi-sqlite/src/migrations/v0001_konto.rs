use async_trait::async_trait;
use molting::UpMigration;

/// The `konto` + `Benutzer` baseline: Benutzer, Konto, session, and single-use token tables.
pub struct Migration;

#[async_trait]
impl UpMigration<libsql::Transaction, libsql::Error> for Migration {
    fn version(&self) -> usize {
        1
    }

    fn description(&self) -> Option<&'static str> {
        Some("Benutzer, Konto, session, and single-use tokens")
    }

    async fn up(&self, transaction: &mut libsql::Transaction) -> Result<(), libsql::Error> {
        transaction
            .execute_batch(
                r#"
            CREATE TABLE IF NOT EXISTS "benutzer" (
                id TEXT PRIMARY KEY,
                email_address TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                avatar_storage_key TEXT,
                gold_medal_count INTEGER NOT NULL DEFAULT 0,
                equipped_title TEXT,
                registered_at TEXT NOT NULL,
                _version INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS "konto" (
                id TEXT PRIMARY KEY,
                benutzer_id TEXT NOT NULL REFERENCES "benutzer" (id) ON DELETE CASCADE,
                password_hash TEXT NOT NULL,
                bestätigt_am TEXT NOT NULL,
                created_at TEXT NOT NULL,
                _version INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS "idx_konto_benutzer_id" ON "konto" (benutzer_id);

            CREATE TABLE IF NOT EXISTS "session" (
                id TEXT PRIMARY KEY,
                benutzer_id TEXT NOT NULL REFERENCES "benutzer" (id) ON DELETE CASCADE,
                refresh_token_hash TEXT NOT NULL UNIQUE,
                public_key TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                spent_at TEXT,
                revoked_at TEXT,
                _version INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS "idx_session_benutzer_id" ON "session" (benutzer_id);
            CREATE INDEX IF NOT EXISTS "idx_session_revoked_at" ON "session" (revoked_at);
            CREATE INDEX IF NOT EXISTS "idx_session_expires_at" ON "session" (expires_at);

            CREATE TABLE IF NOT EXISTS "email_bestätigung" (
                id TEXT PRIMARY KEY,
                email TEXT NOT NULL,
                token_hash TEXT NOT NULL UNIQUE,
                expires_at TEXT NOT NULL,
                spent_at TEXT,
                _version INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS "idx_email_bestätigung_email"
                ON "email_bestätigung" (email);

            CREATE TABLE IF NOT EXISTS "passwort_wiederherstellung" (
                id TEXT PRIMARY KEY,
                konto_id TEXT NOT NULL REFERENCES "konto" (id) ON DELETE CASCADE,
                token_hash TEXT NOT NULL UNIQUE,
                expires_at TEXT NOT NULL,
                spent_at TEXT,
                _version INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS "idx_passwort_wiederherstellung_konto_id"
                ON "passwort_wiederherstellung" (konto_id);
        "#,
            )
            .await?;
        Ok(())
    }
}
