use async_trait::async_trait;
use friendarazzi_core::domain::builtin_starter_decks;
use molting::UpMigration;

/// Persist optional drafting auto-close deadline on games and expand Quick-Start starter decks to 36 prompts.
pub struct Migration;

#[async_trait]
impl UpMigration<libsql::Transaction, libsql::Error> for Migration {
    fn version(&self) -> usize {
        6
    }

    fn description(&self) -> Option<&'static str> {
        Some("Add draft_deadline_at to game and expand starter decks to 36 prompts")
    }

    async fn up(&self, transaction: &mut libsql::Transaction) -> Result<(), libsql::Error> {
        transaction
            .execute_batch(
                r#"
                ALTER TABLE "game" ADD COLUMN draft_deadline_at TEXT;
                CREATE INDEX IF NOT EXISTS "idx_game_draft_timers"
                    ON "game" (status, draft_closes_at, draft_deadline_at);
                "#,
            )
            .await?;

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
    async fn upgrades_from_v5_with_draft_deadline_column_and_36_prompt_decks() {
        let sqlite = SQLiteInstance::in_temp_dir().await.unwrap();
        let mut conn = sqlite.create_connection().await.unwrap();

        MIGRATIONS.apply(&mut conn, Some(5)).await.unwrap();
        drop(conn);

        sqlite.migrate_repos_to_latest().await.unwrap();

        let uow = sqlite.begin().await.unwrap();
        let decks = uow.decks().find_all().await.unwrap();
        uow.rollback().await.unwrap();

        assert_eq!(decks, builtin_starter_decks());
        for deck in &decks {
            assert_eq!(deck.prompts.len(), 36);
        }
    }
}
