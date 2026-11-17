use std::sync::Arc;

use async_lock::Mutex;
use async_trait::async_trait;
use error_stack::ResultExt;
use libsql::{Row, Transaction};
use logandaim_core::{
    ErrorReportExt,
    application::uow::Versioned,
    domain::{BoardTileId, Capture, CaptureId, GameId, NewCapture, konto::BenutzerId},
    ports::{CaptureRepository, RepositoryError, RepositoryResult},
};
use uuid::Uuid;

use crate::{
    errors::libsql_error_to_persistence_error,
    repos::common::{format_datetime, parse_datetime, parse_uuid, parse_version},
};

const SELECT: &str = r#"SELECT id, game_id, tile_id, catcher_user_id, target_user_id, prompt_text, image_storage_key, thumbnail_storage_key, captured_at, _version FROM "captures""#;

fn capture_from_row(row: &Row) -> RepositoryResult<Versioned<Capture>> {
    let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let game_id_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
    let tile_id_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
    let catcher_raw: String = row.get(3).contextualize(RepositoryError::Data)?;
    let target_raw: Option<String> = row.get(4).contextualize(RepositoryError::Data)?;
    let prompt_text: String = row.get(5).contextualize(RepositoryError::Data)?;
    let image_storage_key: String = row.get(6).contextualize(RepositoryError::Data)?;
    let thumbnail_storage_key: String = row.get(7).contextualize(RepositoryError::Data)?;
    let captured_at_raw: String = row.get(8).contextualize(RepositoryError::Data)?;
    let version = parse_version(row, 9)?;

    let capture = Capture::from_parts(
        CaptureId(parse_uuid(&id_raw)?),
        GameId(parse_uuid(&game_id_raw)?),
        BoardTileId(parse_uuid(&tile_id_raw)?),
        BenutzerId(parse_uuid(&catcher_raw)?),
        target_raw
            .map(|s| parse_uuid(&s))
            .transpose()?
            .map(BenutzerId),
        prompt_text,
        image_storage_key,
        thumbnail_storage_key,
        parse_datetime(&captured_at_raw)?,
    )
    .change_context(RepositoryError::Data)?;

    Ok(Versioned::new(version, capture))
}

pub struct SQLiteCaptureRepository {
    pub(crate) tx: Arc<Mutex<Option<Transaction>>>,
}

#[async_trait]
impl CaptureRepository for SQLiteCaptureRepository {
    async fn create(&self, capture: NewCapture) -> RepositoryResult<Versioned<Capture>> {
        let id = CaptureId(Uuid::new_v4());
        let domain = Capture::new(id, capture);
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"INSERT INTO "captures" (id, game_id, tile_id, catcher_user_id, target_user_id, prompt_text, image_storage_key, thumbnail_storage_key, captured_at, _version)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0)"#,
            (
                id.0.to_string(),
                domain.game_id().0.to_string(),
                domain.tile_id().0.to_string(),
                domain.catcher_user_id().0.to_string(),
                domain.target_user_id().map(|u| u.0.to_string()),
                domain.prompt_text().as_str(),
                domain.image_storage_key(),
                domain.thumbnail_storage_key(),
                format_datetime(domain.captured_at()),
            ),
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;

        Ok(Versioned::init(domain))
    }

    async fn find_by_id(&self, id: CaptureId) -> RepositoryResult<Versioned<Capture>> {
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
        capture_from_row(&row)
    }

    async fn find_by_game(&self, game_id: GameId) -> RepositoryResult<Vec<Versioned<Capture>>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(r#"{SELECT} WHERE game_id = ?1 ORDER BY captured_at ASC"#),
                [game_id.0.to_string()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        {
            out.push(capture_from_row(&row)?);
        }
        Ok(out)
    }

    async fn find_by_target_user(
        &self,
        user_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Capture>>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(r#"{SELECT} WHERE target_user_id = ?1 ORDER BY captured_at DESC"#),
                [user_id.0.to_string()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        {
            out.push(capture_from_row(&row)?);
        }
        Ok(out)
    }

    async fn find_by_catcher_user(
        &self,
        user_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<Capture>>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(r#"{SELECT} WHERE catcher_user_id = ?1 ORDER BY captured_at DESC"#),
                [user_id.0.to_string()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;

        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        {
            out.push(capture_from_row(&row)?);
        }
        Ok(out)
    }

    async fn find_by_storage_key(&self, key: &str) -> RepositoryResult<Versioned<Capture>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(
                    r#"{SELECT} WHERE image_storage_key = ?1 OR thumbnail_storage_key = ?1 LIMIT 1"#
                ),
                [key],
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
        capture_from_row(&row)
    }
}
