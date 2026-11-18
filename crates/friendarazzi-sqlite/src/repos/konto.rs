use std::sync::Arc;

use async_lock::Mutex;
use async_trait::async_trait;
use error_stack::ResultExt;
use libsql::{Row, Transaction};
use uuid::Uuid;

use friendarazzi_core::{
    ErrorReportExt,
    application::uow::Versioned,
    domain::konto::{BenutzerId, HashedPasswort, Konto, KontoId, NeuesKonto},
    ports::{KontoRepository, RepositoryError, RepositoryResult},
};

use crate::{
    errors::libsql_error_to_persistence_error,
    repos::common::{
        delete_versioned, format_datetime, parse_datetime, parse_uuid, parse_version,
        update_versioned,
    },
};

const SELECT: &str =
    r#"SELECT id, benutzer_id, password_hash, bestätigt_am, created_at, _version FROM "konto""#;

fn konto_from_row(row: &Row) -> RepositoryResult<Versioned<Konto>> {
    let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let benutzer_id_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
    let password_hash_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
    let bestätigt_am_raw: String = row.get(3).contextualize(RepositoryError::Data)?;
    let created_at_raw: String = row.get(4).contextualize(RepositoryError::Data)?;

    let id = parse_uuid(&id_raw)?;
    let benutzer_id = parse_uuid(&benutzer_id_raw)?;
    let password_hash =
        HashedPasswort::from_phc(password_hash_raw).change_context(RepositoryError::Data)?;
    let bestätigt_am = parse_datetime(&bestätigt_am_raw)?;
    let created_at = parse_datetime(&created_at_raw)?;

    Ok(Versioned::new(
        parse_version(row, 5)?,
        Konto::from_parts(
            KontoId(id),
            BenutzerId(benutzer_id),
            password_hash,
            bestätigt_am,
            created_at,
        ),
    ))
}

pub struct SQLiteKontoRepository {
    pub(crate) tx: Arc<Mutex<Option<Transaction>>>,
}

#[async_trait]
impl KontoRepository for SQLiteKontoRepository {
    async fn create(&self, konto: NeuesKonto) -> RepositoryResult<Versioned<Konto>> {
        let id = KontoId(Uuid::new_v4());
        let domain = Konto::neu(id, konto.clone());

        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"INSERT INTO "konto" (id, benutzer_id, password_hash, bestätigt_am, created_at, _version)
               VALUES (?1, ?2, ?3, ?4, ?5, 0)"#,
            (
                id.0.to_string(),
                konto.benutzer_id().0.to_string(),
                konto.password_hash().as_str().to_string(),
                format_datetime(konto.bestätigt_am()),
                format_datetime(konto.created_at()),
            ),
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;

        Ok(Versioned::init(domain))
    }

    async fn find_by_id(&self, id: KontoId) -> RepositoryResult<Versioned<Konto>> {
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

        konto_from_row(&row)
    }

    async fn find_by_benutzer(
        &self,
        benutzer_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Konto>>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(r#"{SELECT} WHERE benutzer_id = ?1"#),
                [benutzer_id.0.to_string()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let mut konten = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        {
            konten.push(konto_from_row(&row)?);
        }
        Ok(konten)
    }

    /// Full-row write-back with an in-place version bump; a stale version is a
    /// `VersionMismatch`, never a silent overwrite.
    async fn update(&self, konto: &mut Versioned<Konto>) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        update_versioned(
            tx,
            r#"UPDATE "konto" SET benutzer_id = ?2, password_hash = ?3,
                   bestätigt_am = ?4, created_at = ?5, _version = ?6 + 1
               WHERE id = ?1 AND _version = ?6"#,
            (
                konto.id().0.to_string(),
                konto.benutzer_id().0.to_string(),
                konto.password_hash().as_str().to_string(),
                format_datetime(konto.bestätigt_am()),
                format_datetime(konto.created_at()),
                konto.v() as i64,
            ),
            konto.v(),
        )
        .await?;

        konto.increment();
        Ok(())
    }

    /// Delete guarded by the version the caller holds.
    async fn delete(&self, konto: Versioned<Konto>) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        delete_versioned(
            tx,
            r#"DELETE FROM "konto" WHERE id = ?1 AND _version = ?2"#,
            (konto.id().0.to_string(), konto.v() as i64),
            konto.v(),
        )
        .await
    }
}
