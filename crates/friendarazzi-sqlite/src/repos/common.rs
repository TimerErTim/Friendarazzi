use std::str::FromStr;

use chrono::{DateTime, Utc};
use friendarazzi_core::{
    ErrorReportExt,
    ports::{RepositoryError, RepositoryResult},
};
use uuid::Uuid;

use crate::errors::libsql_error_to_persistence_error;

/// Parse a stored UUID.
pub fn parse_uuid(s: &str) -> RepositoryResult<Uuid> {
    Uuid::from_str(s).contextualize(RepositoryError::Data)
}

pub fn format_datetime(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

pub fn parse_datetime(s: &str) -> RepositoryResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .contextualize(RepositoryError::Data)
}

/// Read the `_version` column of a row.
pub fn parse_version(row: &libsql::Row, index: i32) -> RepositoryResult<u64> {
    let version: i64 = row.get(index).contextualize(RepositoryError::Data)?;
    u64::try_from(version).contextualize(RepositoryError::Data)
}

/// Conditional full-row update.
pub async fn update_versioned(
    tx: &libsql::Transaction,
    sql: &str,
    params: impl libsql::params::IntoParams,
    expected: u64,
) -> RepositoryResult<()> {
    let affected = tx
        .execute(sql, params)
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;
    if affected == 0 {
        return Err(RepositoryError::VersionMismatch {
            expected,
            actual: None,
        }
        .into());
    }
    Ok(())
}

/// Conditional delete guarded by `_version`.
pub async fn delete_versioned(
    tx: &libsql::Transaction,
    sql: &str,
    params: impl libsql::params::IntoParams,
    expected: u64,
) -> RepositoryResult<()> {
    let affected = tx
        .execute(sql, params)
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;
    if affected == 0 {
        return Err(RepositoryError::VersionMismatch {
            expected,
            actual: None,
        }
        .into());
    }
    Ok(())
}
