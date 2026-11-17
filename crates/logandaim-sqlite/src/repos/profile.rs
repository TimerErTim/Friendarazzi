use std::sync::Arc;

use async_lock::Mutex;
use async_trait::async_trait;
use error_stack::ResultExt;
use libsql::{Row, Transaction};
use logandaim_core::{
    ErrorReportExt,
    application::uow::Versioned,
    domain::{NewUserProfile, UserProfile, konto::BenutzerId},
    ports::{RepositoryError, RepositoryResult, UserProfileRepository},
};

use crate::{
    errors::libsql_error_to_persistence_error,
    repos::common::{format_datetime, parse_datetime, parse_uuid, parse_version, update_versioned},
};

const SELECT: &str = r#"SELECT user_id, username, display_name, avatar_storage_key, gold_medal_count, equipped_title, created_at, _version FROM "user_medals""#;

fn profile_from_row(row: &Row) -> RepositoryResult<Versioned<UserProfile>> {
    let user_id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let username: String = row.get(1).contextualize(RepositoryError::Data)?;
    let display_name: String = row.get(2).contextualize(RepositoryError::Data)?;
    let avatar_storage_key: Option<String> = row.get(3).contextualize(RepositoryError::Data)?;
    let gold_medal_count_raw: i64 = row.get(4).contextualize(RepositoryError::Data)?;
    let equipped_title: Option<String> = row.get(5).contextualize(RepositoryError::Data)?;
    let created_at_raw: String = row.get(6).contextualize(RepositoryError::Data)?;
    let version = parse_version(row, 7)?;

    let user_id = BenutzerId(parse_uuid(&user_id_raw)?);
    let gold_medal_count =
        u32::try_from(gold_medal_count_raw).contextualize(RepositoryError::Data)?;
    let created_at = parse_datetime(&created_at_raw)?;

    let profile = UserProfile::from_parts(
        user_id,
        username,
        display_name,
        avatar_storage_key,
        gold_medal_count,
        equipped_title,
        created_at,
    )
    .change_context(RepositoryError::Data)?;

    Ok(Versioned::new(version, profile))
}

pub struct SQLiteUserProfileRepository {
    pub(crate) tx: Arc<Mutex<Option<Transaction>>>,
}

#[async_trait]
impl UserProfileRepository for SQLiteUserProfileRepository {
    async fn create(&self, profile: NewUserProfile) -> RepositoryResult<Versioned<UserProfile>> {
        let domain = UserProfile::new(profile);
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"INSERT INTO "user_medals" (user_id, username, display_name, avatar_storage_key, gold_medal_count, equipped_title, created_at, _version)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)"#,
            (
                domain.user_id().0.to_string(),
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

    async fn find_by_user_id(
        &self,
        user_id: BenutzerId,
    ) -> RepositoryResult<Versioned<UserProfile>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(r#"{SELECT} WHERE user_id = ?1"#),
                [user_id.0.to_string()],
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
        profile_from_row(&row)
    }

    async fn find_by_user_ids(
        &self,
        user_ids: &[BenutzerId],
    ) -> RepositoryResult<Vec<Versioned<UserProfile>>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let placeholders: Vec<String> = (1..=user_ids.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(r#"{SELECT} WHERE user_id IN ({})"#, placeholders.join(", "));
        let params: Vec<String> = user_ids.iter().map(|id| id.0.to_string()).collect();

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
            out.push(profile_from_row(&row)?);
        }
        Ok(out)
    }

    async fn find_by_username(&self, username: &str) -> RepositoryResult<Versioned<UserProfile>> {
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
        profile_from_row(&row)
    }

    async fn update(&self, profile: &mut Versioned<UserProfile>) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        update_versioned(
            tx,
            r#"UPDATE "user_medals"
               SET username = ?2, display_name = ?3, avatar_storage_key = ?4, gold_medal_count = ?5, equipped_title = ?6, _version = ?7 + 1
               WHERE user_id = ?1 AND _version = ?7"#,
            (
                profile.user_id().0.to_string(),
                profile.username().as_str(),
                profile.display_name().as_str(),
                profile.avatar_storage_key().map(str::to_owned),
                i64::from(profile.gold_medal_count()),
                profile.equipped_title().map(str::to_owned),
                profile.v() as i64,
            ),
            profile.v(),
        )
        .await?;
        profile.increment();
        Ok(())
    }

    async fn delete_by_user_id(&self, user_id: BenutzerId) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"DELETE FROM "user_medals" WHERE user_id = ?1"#,
            [user_id.0.to_string()],
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;
        Ok(())
    }
}
