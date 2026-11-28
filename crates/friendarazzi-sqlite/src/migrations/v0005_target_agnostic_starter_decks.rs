use async_trait::async_trait;
use friendarazzi_core::domain::builtin_starter_decks;
use molting::UpMigration;

/// Replace seeded `{name}` starter deck templates with target-agnostic prompts.
pub struct Migration;

#[async_trait]
impl UpMigration<libsql::Transaction, libsql::Error> for Migration {
    fn version(&self) -> usize {
        5
    }

    fn description(&self) -> Option<&'static str> {
        Some("Migrate Quick-Start starter decks to target-agnostic prompts")
    }

    async fn up(&self, transaction: &mut libsql::Transaction) -> Result<(), libsql::Error> {
        for deck in builtin_starter_decks() {
            let prompts_json = serde_json::to_string(&deck.prompts)
                .map_err(|err| libsql::Error::ToSqlConversionFailure(Box::new(err)))?;
            transaction
                .execute(
                    r#"INSERT INTO "starter_deck" (id, slug, name, description, prompts_json)
                       VALUES (?1, ?2, ?3, ?4, ?5)
                       ON CONFLICT(id) DO UPDATE SET
                           slug = excluded.slug,
                           name = excluded.name,
                           description = excluded.description,
                           prompts_json = excluded.prompts_json"#,
                    (
                        deck.id.0.to_string(),
                        deck.slug,
                        deck.name,
                        deck.description,
                        prompts_json,
                    ),
                )
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use friendarazzi_core::{domain::builtin_starter_decks, uow::UnitOfWorkProvider};

    use crate::{SQLiteInstance, migrations::MIGRATIONS};

    #[test_log::test(tokio::test)]
    async fn migrates_legacy_name_templated_starter_decks_to_target_agnostic_prompts() {
        let sqlite = SQLiteInstance::in_temp_dir().await.unwrap();
        let mut conn = sqlite.create_connection().await.unwrap();

        MIGRATIONS.apply(&mut conn, Some(4)).await.unwrap();

        // Simulate a database seeded before v0005 with legacy `{name}` templates.
        let legacy_json = serde_json::to_string(&vec!["{name} macht ein Foto"]).unwrap();
        conn.execute(
            r#"UPDATE "starter_deck" SET prompts_json = ?1"#,
            [legacy_json],
        )
        .await
        .unwrap();
        drop(conn);

        sqlite.migrate_repos_to_latest().await.unwrap();

        let uow = sqlite.begin().await.unwrap();
        let migrated = uow.decks().find_all().await.unwrap();
        uow.rollback().await.unwrap();

        assert_eq!(migrated, builtin_starter_decks());
        for deck in &migrated {
            assert!(deck.prompts.iter().all(|p| !p.contains("{name}")));
        }
    }
}
