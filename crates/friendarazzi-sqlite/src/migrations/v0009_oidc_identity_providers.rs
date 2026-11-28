use async_trait::async_trait;
use molting::UpMigration;

/// Persist external OIDC identities, single-use auth states, and short-lived exchange codes.
pub struct Migration;

#[async_trait]
impl UpMigration<libsql::Transaction, libsql::Error> for Migration {
    fn version(&self) -> usize {
        9
    }

    fn description(&self) -> Option<&'static str> {
        Some("Add external_identity, oidc_auth_state, and oidc_exchange_code tables")
    }

    async fn up(&self, transaction: &mut libsql::Transaction) -> Result<(), libsql::Error> {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS "external_identity" (
                    id TEXT PRIMARY KEY,
                    benutzer_id TEXT NOT NULL REFERENCES "benutzer" (id) ON DELETE CASCADE,
                    provider_id TEXT NOT NULL,
                    subject TEXT NOT NULL,
                    provider_email TEXT,
                    created_at TEXT NOT NULL,
                    _version INTEGER NOT NULL DEFAULT 0,
                    UNIQUE (provider_id, subject)
                );

                CREATE INDEX IF NOT EXISTS "idx_external_identity_benutzer_id"
                    ON "external_identity" (benutzer_id);

                CREATE TABLE IF NOT EXISTS "oidc_auth_state" (
                    id TEXT PRIMARY KEY,
                    provider_id TEXT NOT NULL,
                    state_hash TEXT NOT NULL UNIQUE,
                    nonce TEXT NOT NULL,
                    pkce_verifier TEXT NOT NULL,
                    mode TEXT NOT NULL,
                    benutzer_id TEXT REFERENCES "benutzer" (id) ON DELETE CASCADE,
                    return_to TEXT,
                    created_at TEXT NOT NULL,
                    expires_at TEXT NOT NULL,
                    spent_at TEXT,
                    _version INTEGER NOT NULL DEFAULT 0
                );

                CREATE INDEX IF NOT EXISTS "idx_oidc_auth_state_benutzer_id"
                    ON "oidc_auth_state" (benutzer_id);

                CREATE TABLE IF NOT EXISTS "oidc_exchange_code" (
                    id TEXT PRIMARY KEY,
                    code_hash TEXT NOT NULL UNIQUE,
                    benutzer_id TEXT NOT NULL REFERENCES "benutzer" (id) ON DELETE CASCADE,
                    purpose TEXT NOT NULL,
                    expires_at TEXT NOT NULL,
                    spent_at TEXT,
                    _version INTEGER NOT NULL DEFAULT 0
                );

                CREATE INDEX IF NOT EXISTS "idx_oidc_exchange_code_benutzer_id"
                    ON "oidc_exchange_code" (benutzer_id);
                "#,
            )
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use friendarazzi_core::{
        domain::{
            EmailAdresse, NeuerBenutzer,
            konto::{NeueExternalIdentity, OidcProviderId},
        },
        uow::UnitOfWorkProvider,
    };

    use crate::{SQLiteInstance, migrations::MIGRATIONS};

    #[test_log::test(tokio::test)]
    async fn upgrades_from_v8_and_persists_external_identities_without_konto() {
        let sqlite = SQLiteInstance::in_temp_dir().await.unwrap();
        let mut conn = sqlite.create_connection().await.unwrap();
        MIGRATIONS.apply(&mut conn, Some(8)).await.unwrap();
        drop(conn);

        sqlite.migrate_repos_to_latest().await.unwrap();

        let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        let uow = sqlite.begin().await.unwrap();
        let user = uow
            .benutzer()
            .create(NeuerBenutzer::registrierung(
                EmailAdresse::normalisiert("oidc@example.at").unwrap(),
                now,
            ))
            .await
            .unwrap();
        let identity = uow
            .external_identities()
            .create(
                NeueExternalIdentity::neu(
                    user.id(),
                    OidcProviderId::parse("google").unwrap(),
                    "sub-123",
                    Some(user.email_address().clone()),
                    now,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(identity.provider_id().as_str(), "google");
        assert!(
            uow.konten()
                .find_by_benutzer(user.id())
                .await
                .unwrap()
                .is_empty()
        );
        uow.commit().await.unwrap();
    }
}
