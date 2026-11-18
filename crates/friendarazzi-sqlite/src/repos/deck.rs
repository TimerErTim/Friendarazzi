use std::sync::Arc;

use async_lock::Mutex;
use async_trait::async_trait;
use friendarazzi_core::{
    ErrorReportExt,
    domain::{DeckId, StarterDeck},
    ports::{DeckRepository, RepositoryError, RepositoryResult},
};
use libsql::Transaction;

use crate::{errors::libsql_error_to_persistence_error, repos::common::parse_uuid};

pub struct SQLiteDeckRepository {
    pub(crate) tx: Arc<Mutex<Option<Transaction>>>,
}

#[async_trait]
impl DeckRepository for SQLiteDeckRepository {
    async fn find_all(&self) -> RepositoryResult<Vec<StarterDeck>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                r#"SELECT id, slug, name, description, prompts_json FROM "starter_deck" ORDER BY id ASC"#,
                (),
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        {
            let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
            let slug: String = row.get(1).contextualize(RepositoryError::Data)?;
            let name: String = row.get(2).contextualize(RepositoryError::Data)?;
            let description: String = row.get(3).contextualize(RepositoryError::Data)?;
            let prompts_json: String = row.get(4).contextualize(RepositoryError::Data)?;
            let prompts: Vec<String> =
                serde_json::from_str(&prompts_json).contextualize(RepositoryError::Data)?;

            out.push(StarterDeck {
                id: DeckId(parse_uuid(&id_raw)?),
                slug,
                name,
                description,
                prompts,
            });
        }
        Ok(out)
    }
}
