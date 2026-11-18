use std::sync::Arc;

use async_lock::Mutex;
use async_trait::async_trait;
use error_stack::ResultExt;
use libsql::{Row, Transaction};
use uuid::Uuid;

use friendarazzi_core::{
    ErrorReportExt,
    application::uow::Versioned,
    domain::{Benutzer, BenutzerId, EmailAdresse, NeuerBenutzer, Rolle},
    ports::{BenutzerRepository, RepositoryError, RepositoryResult},
};

use crate::{
    errors::libsql_error_to_persistence_error,
    repos::common::{
        delete_versioned, format_datetime, parse_datetime, parse_uuid, parse_version,
        update_versioned,
    },
};

const SELECT: &str = r#"SELECT id, rolle, email_address, username, display_name, avatar_storage_key, gold_medal_count, equipped_title, created_at, _version FROM "benutzer""#;

fn benutzer_from_row(row: &Row) -> RepositoryResult<Versioned<Benutzer>> {
    let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let rolle_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
    let email_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
    let username: String = row.get(3).contextualize(RepositoryError::Data)?;
    let display_name: String = row.get(4).contextualize(RepositoryError::Data)?;
    let avatar_storage_key: Option<String> = row.get(5).contextualize(RepositoryError::Data)?;
    let gold_medal_count_raw: i64 = row.get(6).contextualize(RepositoryError::Data)?;
    let equipped_title: Option<String> = row.get(7).contextualize(RepositoryError::Data)?;
    let created_at_raw: String = row.get(8).contextualize(RepositoryError::Data)?;
    let version = parse_version(row, 9)?;

    let id = parse_uuid(&id_raw)?;
    let rolle: Rolle = rolle_raw.parse().change_context(RepositoryError::Data)?;
    let email_address =
        EmailAdresse::normalisiert(email_raw).change_context(RepositoryError::Data)?;
    let gold_medal_count =
        u32::try_from(gold_medal_count_raw).contextualize(RepositoryError::Data)?;
    let created_at = parse_datetime(&created_at_raw)?;

    let benutzer = Benutzer::from_parts(
        BenutzerId(id),
        rolle,
        email_address,
        username,
        display_name,
        avatar_storage_key,
        gold_medal_count,
        equipped_title,
        created_at,
    )
    .change_context(RepositoryError::Data)?;

    Ok(Versioned::new(version, benutzer))
}

pub struct SQLiteBenutzerRepository {
    pub(crate) tx: Arc<Mutex<Option<Transaction>>>,
}

#[async_trait]
impl BenutzerRepository for SQLiteBenutzerRepository {
    async fn create(&self, benutzer: NeuerBenutzer) -> RepositoryResult<Versioned<Benutzer>> {
        let id = BenutzerId(Uuid::new_v4());
        let domain = Benutzer::neu(id, benutzer);

        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"INSERT INTO "benutzer" (id, rolle, email_address, username, display_name, avatar_storage_key, gold_medal_count, equipped_title, created_at, _version)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0)"#,
            (
                id.0.to_string(),
                domain.rolle().as_str(),
                domain.email_address().as_ref(),
                domain.username().as_str(),
                domain.display_name().as_str(),
                domain.avatar_storage_key().map(str::to_owned),
                i64::from(domain.gold_medal_count()),
                domain.equipped_title().map(str::to_owned),
                format_datetime(domain.created_at()),
            ),
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;

        Ok(Versioned::init(domain))
    }

    async fn find_by_id(&self, id: BenutzerId) -> RepositoryResult<Versioned<Benutzer>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(&format!(r#"{SELECT} WHERE id = ?1"#), [id.0.to_string()])
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        else {
            return Err(RepositoryError::NotFound.into());
        };

        benutzer_from_row(&row)
    }

    async fn find_by_ids(&self, ids: &[BenutzerId]) -> RepositoryResult<Vec<Versioned<Benutzer>>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(r#"{SELECT} WHERE id IN ({})"#, placeholders.join(", "));
        let params: Vec<String> = ids.iter().map(|id| id.0.to_string()).collect();

        let mut rows = tx
            .query(&sql, libsql::params_from_iter(params))
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        {
            out.push(benutzer_from_row(&row)?);
        }
        Ok(out)
    }

    async fn find_by_email(&self, email: &EmailAdresse) -> RepositoryResult<Versioned<Benutzer>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(r#"{SELECT} WHERE email_address = ?1"#),
                [email.as_ref()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        else {
            return Err(RepositoryError::NotFound.into());
        };

        benutzer_from_row(&row)
    }

    async fn find_by_username(&self, username: &str) -> RepositoryResult<Versioned<Benutzer>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(r#"{SELECT} WHERE LOWER(username) = LOWER(?1)"#),
                [username],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        else {
            return Err(RepositoryError::NotFound.into());
        };

        benutzer_from_row(&row)
    }

    /// Full-row write-back with an in-place version bump; a stale version is a
    /// `VersionMismatch`, never a silent overwrite.
    async fn update(&self, benutzer: &mut Versioned<Benutzer>) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        update_versioned(
            tx,
            r#"UPDATE "benutzer"
               SET rolle = ?2, email_address = ?3, username = ?4, display_name = ?5,
                   avatar_storage_key = ?6, gold_medal_count = ?7, equipped_title = ?8,
                   created_at = ?9, _version = ?10 + 1
               WHERE id = ?1 AND _version = ?10"#,
            (
                benutzer.id().0.to_string(),
                benutzer.rolle().as_str(),
                benutzer.email_address().as_ref().to_string(),
                benutzer.username().as_str().to_string(),
                benutzer.display_name().as_str().to_string(),
                benutzer.avatar_storage_key().map(str::to_owned),
                i64::from(benutzer.gold_medal_count()),
                benutzer.equipped_title().map(str::to_owned),
                format_datetime(benutzer.created_at()),
                benutzer.v() as i64,
            ),
            benutzer.v(),
        )
        .await?;

        benutzer.increment();
        Ok(())
    }

    /// Delete guarded by the version the caller holds.
    async fn delete(&self, benutzer: Versioned<Benutzer>) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        delete_versioned(
            tx,
            r#"DELETE FROM "benutzer" WHERE id = ?1 AND _version = ?2"#,
            (benutzer.id().0.to_string(), benutzer.v() as i64),
            benutzer.v(),
        )
        .await
    }
}
