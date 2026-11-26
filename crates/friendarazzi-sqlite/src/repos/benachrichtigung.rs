use std::sync::Arc;

use async_lock::Mutex;
use async_trait::async_trait;
use error_stack::ResultExt;
use friendarazzi_core::{
    ErrorReportExt,
    application::uow::Versioned,
    domain::{
        Benachrichtigung, BenachrichtigungId, BenutzerId, CaptureId, GameId, NeueBenachrichtigung,
    },
    ports::{BenachrichtigungRepository, RepositoryError, RepositoryResult},
};
use libsql::{Row, Transaction};
use uuid::Uuid;

use crate::{
    errors::libsql_error_to_persistence_error,
    repos::common::{
        delete_versioned, format_datetime, parse_datetime, parse_uuid, parse_version,
        update_versioned,
    },
};

const SELECT_NOTIF: &str = r#"SELECT id, empfaenger_id, art, game_id, capture_id, actor_id, target_id, dedupe_key, erstellt_am, gelesen_am, _version FROM "benachrichtigung""#;

fn notif_from_row(row: &Row) -> RepositoryResult<Versioned<Benachrichtigung>> {
    let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let empfänger_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
    let art_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
    let game_raw: String = row.get(3).contextualize(RepositoryError::Data)?;
    let capture_raw: Option<String> = row.get(4).contextualize(RepositoryError::Data)?;
    let actor_raw: Option<String> = row.get(5).contextualize(RepositoryError::Data)?;
    let target_raw: Option<String> = row.get(6).contextualize(RepositoryError::Data)?;
    let dedupe_key: String = row.get(7).contextualize(RepositoryError::Data)?;
    let erstellt_raw: String = row.get(8).contextualize(RepositoryError::Data)?;
    let gelesen_raw: Option<String> = row.get(9).contextualize(RepositoryError::Data)?;
    let version = parse_version(row, 10)?;

    let notif = Benachrichtigung::from_parts(
        BenachrichtigungId(parse_uuid(&id_raw)?),
        BenutzerId(parse_uuid(&empfänger_raw)?),
        &art_raw,
        GameId(parse_uuid(&game_raw)?),
        capture_raw
            .map(|s| parse_uuid(&s))
            .transpose()?
            .map(CaptureId),
        actor_raw
            .map(|s| parse_uuid(&s))
            .transpose()?
            .map(BenutzerId),
        target_raw
            .map(|s| parse_uuid(&s))
            .transpose()?
            .map(BenutzerId),
        dedupe_key,
        parse_datetime(&erstellt_raw)?,
        gelesen_raw.map(|s| parse_datetime(&s)).transpose()?,
    )
    .change_context(RepositoryError::Data)?;

    Ok(Versioned::new(version, notif))
}

pub struct SQLiteBenachrichtigungRepository {
    pub(crate) tx: Arc<Mutex<Option<Transaction>>>,
}

#[async_trait]
impl BenachrichtigungRepository for SQLiteBenachrichtigungRepository {
    async fn create(
        &self,
        neu: NeueBenachrichtigung,
    ) -> RepositoryResult<Versioned<Benachrichtigung>> {
        let id = BenachrichtigungId(Uuid::new_v4());
        let domain = Benachrichtigung::neu(id, neu);

        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"INSERT INTO "benachrichtigung"
               (id, empfaenger_id, art, game_id, capture_id, actor_id, target_id, dedupe_key, erstellt_am, gelesen_am, _version)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0)"#,
            (
                id.0.to_string(),
                domain.empfänger_id().0.to_string(),
                domain.art().as_str(),
                domain.game_id().0.to_string(),
                domain.capture_id().map(|c| c.0.to_string()),
                domain.actor_id().map(|u| u.0.to_string()),
                domain.target_id().map(|u| u.0.to_string()),
                domain.dedupe_key().as_str(),
                format_datetime(domain.erstellt_am()),
                domain.gelesen_am().map(format_datetime),
            ),
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;

        Ok(Versioned::init(domain))
    }

    async fn find_by_id(
        &self,
        id: BenachrichtigungId,
    ) -> RepositoryResult<Versioned<Benachrichtigung>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(r#"{SELECT_NOTIF} WHERE id = ?1"#),
                [id.0.to_string()],
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

        notif_from_row(&row)
    }

    async fn find_by_empfänger(
        &self,
        empfänger_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Benachrichtigung>>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(
                    r#"{SELECT_NOTIF} WHERE empfaenger_id = ?1 ORDER BY erstellt_am DESC, id DESC"#
                ),
                [empfänger_id.0.to_string()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        {
            out.push(notif_from_row(&row)?);
        }
        Ok(out)
    }

    async fn update(
        &self,
        benachrichtigung: &mut Versioned<Benachrichtigung>,
    ) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        update_versioned(
            tx,
            r#"UPDATE "benachrichtigung"
               SET gelesen_am = ?2, _version = ?3 + 1
               WHERE id = ?1 AND _version = ?3"#,
            (
                benachrichtigung.id().0.to_string(),
                benachrichtigung.gelesen_am().map(format_datetime),
                benachrichtigung.v() as i64,
            ),
            benachrichtigung.v(),
        )
        .await?;

        benachrichtigung.increment();
        Ok(())
    }

    async fn delete(&self, benachrichtigung: Versioned<Benachrichtigung>) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        delete_versioned(
            tx,
            r#"DELETE FROM "benachrichtigung" WHERE id = ?1 AND _version = ?2"#,
            (
                benachrichtigung.id().0.to_string(),
                benachrichtigung.v() as i64,
            ),
            benachrichtigung.v(),
        )
        .await
    }

    async fn delete_by_benutzer(&self, empfänger_id: BenutzerId) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"DELETE FROM "benachrichtigung" WHERE empfaenger_id = ?1"#,
            [empfänger_id.0.to_string()],
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;
        Ok(())
    }
}
