use async_trait::async_trait;
use molting::UpMigration;

/// Persist optional email verification timestamp and backfill pre-existing users as verified.
pub struct Migration;

#[async_trait]
impl UpMigration<libsql::Transaction, libsql::Error> for Migration {
    fn version(&self) -> usize {
        8
    }

    fn description(&self) -> Option<&'static str> {
        Some("Add email_verified_at column to benutzer and backfill existing accounts")
    }

    async fn up(&self, transaction: &mut libsql::Transaction) -> Result<(), libsql::Error> {
        transaction
            .execute_batch(
                r#"
                ALTER TABLE "benutzer" ADD COLUMN email_verified_at TEXT;
                UPDATE "benutzer"
                    SET email_verified_at = registered_at
                    WHERE email_verified_at IS NULL;
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
        domain::{EmailAdresse, NeuerBenutzer},
        uow::UnitOfWorkProvider,
    };

    use crate::{SQLiteInstance, migrations::MIGRATIONS};

    #[test_log::test(tokio::test)]
    async fn upgrades_from_v7_and_backfills_existing_users_as_verified() {
        let sqlite = SQLiteInstance::in_temp_dir().await.unwrap();
        let mut conn = sqlite.create_connection().await.unwrap();
        MIGRATIONS.apply(&mut conn, Some(7)).await.unwrap();

        let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        let ts = "2026-10-01T10:00:00+00:00";
        conn.execute(
            r#"INSERT INTO "benutzer" (
                id, email_address, display_name,
                default_watch_updated_at, email_game_started_updated_at,
                email_capture_posted_updated_at, email_game_completed_updated_at,
                registered_at, _version
            ) VALUES (?1, ?2, ?3, ?4, ?4, ?4, ?4, ?4, 0)"#,
            (
                "00000000-0000-0000-0000-000000000001",
                "legacy@example.at",
                "Legacy",
                ts,
            ),
        )
        .await
        .unwrap();
        drop(conn);

        sqlite.migrate_repos_to_latest().await.unwrap();

        let uow = sqlite.begin().await.unwrap();
        let legacy = uow
            .benutzer()
            .find_by_email(&EmailAdresse::normalisiert("legacy@example.at").unwrap())
            .await
            .unwrap();
        assert!(legacy.ist_verifiziert());

        let unverified = uow
            .benutzer()
            .create(
                NeuerBenutzer::registrierung(
                    EmailAdresse::normalisiert("fresh@example.at").unwrap(),
                    now,
                )
                .with_email_verified_at(None),
            )
            .await
            .unwrap();
        assert!(!unverified.ist_verifiziert());
        uow.rollback().await.unwrap();
    }
}
