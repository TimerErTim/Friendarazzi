use std::sync::Arc;

use async_lock::Mutex;
use async_trait::async_trait;
use error_stack::ResultExt;
use friendarazzi_core::{
    ErrorReportExt,
    application::uow::Versioned,
    domain::{
        BenutzerId, EmailAdresse,
        konto::{
            ExternalIdentity, ExternalIdentityId, NeueExternalIdentity, NeuerOidcAuthState,
            NeuerOidcExchangeCode, OidcAuthState, OidcAuthStateId, OidcExchangeCode,
            OidcExchangeCodeId, OidcExchangePurpose, OidcFlowMode, OidcProviderId, SecretTokenHash,
        },
    },
    ports::{
        ExternalIdentityRepository, OidcAuthStateRepository, OidcExchangeCodeRepository,
        RepositoryError, RepositoryResult,
    },
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

const SELECT_IDENTITY: &str = r#"SELECT id, benutzer_id, provider_id, subject, provider_email, created_at, _version FROM "external_identity""#;

const SELECT_STATE: &str = r#"SELECT id, provider_id, state_hash, nonce, pkce_verifier, mode, benutzer_id, return_to, created_at, expires_at, spent_at, _version FROM "oidc_auth_state""#;

const SELECT_CODE: &str = r#"SELECT id, code_hash, benutzer_id, purpose, expires_at, spent_at, _version FROM "oidc_exchange_code""#;

fn identity_from_row(row: &Row) -> RepositoryResult<Versioned<ExternalIdentity>> {
    let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let benutzer_id_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
    let provider_id_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
    let subject: String = row.get(3).contextualize(RepositoryError::Data)?;
    let email_raw: Option<String> = row.get(4).contextualize(RepositoryError::Data)?;
    let created_at_raw: String = row.get(5).contextualize(RepositoryError::Data)?;

    let provider_email = email_raw
        .map(|raw| EmailAdresse::normalisiert(&raw))
        .transpose()
        .change_context(RepositoryError::Data)?;
    let provider_id =
        OidcProviderId::parse(&provider_id_raw).change_context(RepositoryError::Data)?;

    Ok(Versioned::new(
        parse_version(row, 6)?,
        ExternalIdentity::from_parts(
            ExternalIdentityId(parse_uuid(&id_raw)?),
            BenutzerId(parse_uuid(&benutzer_id_raw)?),
            provider_id,
            subject,
            provider_email,
            parse_datetime(&created_at_raw)?,
        ),
    ))
}

fn state_from_row(row: &Row) -> RepositoryResult<Versioned<OidcAuthState>> {
    let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let provider_id_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
    let state_hash_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
    let nonce: String = row.get(3).contextualize(RepositoryError::Data)?;
    let pkce_verifier: String = row.get(4).contextualize(RepositoryError::Data)?;
    let mode_raw: String = row.get(5).contextualize(RepositoryError::Data)?;
    let benutzer_id_raw: Option<String> = row.get(6).contextualize(RepositoryError::Data)?;
    let return_to: Option<String> = row.get(7).contextualize(RepositoryError::Data)?;
    let created_at_raw: String = row.get(8).contextualize(RepositoryError::Data)?;
    let expires_at_raw: String = row.get(9).contextualize(RepositoryError::Data)?;
    let spent_at_raw: Option<String> = row.get(10).contextualize(RepositoryError::Data)?;

    let mode = match mode_raw.as_str() {
        "login" => OidcFlowMode::Login,
        "link" => OidcFlowMode::Link {
            benutzer_id: BenutzerId(parse_uuid(
                benutzer_id_raw.as_deref().ok_or(RepositoryError::Data)?,
            )?),
        },
        "delete_reauth" => OidcFlowMode::DeleteReauth {
            benutzer_id: BenutzerId(parse_uuid(
                benutzer_id_raw.as_deref().ok_or(RepositoryError::Data)?,
            )?),
        },
        _ => return Err(RepositoryError::Data.into()),
    };

    Ok(Versioned::new(
        parse_version(row, 11)?,
        OidcAuthState::from_parts(
            OidcAuthStateId(parse_uuid(&id_raw)?),
            OidcProviderId::parse(&provider_id_raw).change_context(RepositoryError::Data)?,
            SecretTokenHash::from_hex(state_hash_raw).change_context(RepositoryError::Data)?,
            nonce,
            pkce_verifier,
            mode,
            return_to,
            parse_datetime(&created_at_raw)?,
            parse_datetime(&expires_at_raw)?,
            spent_at_raw.map(|raw| parse_datetime(&raw)).transpose()?,
        ),
    ))
}

fn code_from_row(row: &Row) -> RepositoryResult<Versioned<OidcExchangeCode>> {
    let id_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let code_hash_raw: String = row.get(1).contextualize(RepositoryError::Data)?;
    let benutzer_id_raw: String = row.get(2).contextualize(RepositoryError::Data)?;
    let purpose_raw: String = row.get(3).contextualize(RepositoryError::Data)?;
    let expires_at_raw: String = row.get(4).contextualize(RepositoryError::Data)?;
    let spent_at_raw: Option<String> = row.get(5).contextualize(RepositoryError::Data)?;

    Ok(Versioned::new(
        parse_version(row, 6)?,
        OidcExchangeCode::from_parts(
            OidcExchangeCodeId(parse_uuid(&id_raw)?),
            SecretTokenHash::from_hex(code_hash_raw).change_context(RepositoryError::Data)?,
            BenutzerId(parse_uuid(&benutzer_id_raw)?),
            OidcExchangePurpose::parse(&purpose_raw).change_context(RepositoryError::Data)?,
            parse_datetime(&expires_at_raw)?,
            spent_at_raw.map(|raw| parse_datetime(&raw)).transpose()?,
        ),
    ))
}

pub struct SQLiteExternalIdentityRepository {
    pub(crate) tx: Arc<Mutex<Option<Transaction>>>,
}

#[async_trait]
impl ExternalIdentityRepository for SQLiteExternalIdentityRepository {
    async fn create(
        &self,
        identity: NeueExternalIdentity,
    ) -> RepositoryResult<Versioned<ExternalIdentity>> {
        let id = ExternalIdentityId(Uuid::new_v4());
        let entity = ExternalIdentity::from_parts(
            id,
            identity.benutzer_id(),
            identity.provider_id().clone(),
            identity.subject().to_owned(),
            identity.provider_email().cloned(),
            identity.created_at(),
        );
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;
        tx.execute(
            r#"INSERT INTO "external_identity"
                (id, benutzer_id, provider_id, subject, provider_email, created_at, _version)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)"#,
            (
                id.0.to_string(),
                entity.benutzer_id().0.to_string(),
                entity.provider_id().as_str().to_owned(),
                entity.subject().to_owned(),
                entity
                    .provider_email()
                    .map(|email| AsRef::<str>::as_ref(email).to_owned()),
                format_datetime(entity.created_at()),
            ),
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;
        Ok(Versioned::init(entity))
    }

    async fn find_by_id(
        &self,
        id: ExternalIdentityId,
    ) -> RepositoryResult<Versioned<ExternalIdentity>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;
        let mut rows = tx
            .query(
                &format!("{SELECT_IDENTITY} WHERE id = ?1"),
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
        identity_from_row(&row)
    }

    async fn find_by_provider_subject(
        &self,
        provider_id: &OidcProviderId,
        subject: &str,
    ) -> RepositoryResult<Versioned<ExternalIdentity>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;
        let mut rows = tx
            .query(
                &format!("{SELECT_IDENTITY} WHERE provider_id = ?1 AND subject = ?2"),
                (provider_id.as_str().to_owned(), subject.to_owned()),
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
        identity_from_row(&row)
    }

    async fn find_by_benutzer(
        &self,
        benutzer_id: BenutzerId,
    ) -> RepositoryResult<Vec<Versioned<ExternalIdentity>>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;
        let mut rows = tx
            .query(
                &format!("{SELECT_IDENTITY} WHERE benutzer_id = ?1 ORDER BY created_at ASC"),
                [benutzer_id.0.to_string()],
            )
            .await
            .contextualize_with(libsql_error_to_persistence_error)?;
        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .contextualize_with(libsql_error_to_persistence_error)?
        {
            out.push(identity_from_row(&row)?);
        }
        Ok(out)
    }

    async fn delete(&self, identity: Versioned<ExternalIdentity>) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;
        delete_versioned(
            tx,
            r#"DELETE FROM "external_identity" WHERE id = ?1 AND _version = ?2"#,
            (identity.id().0.to_string(), identity.v() as i64),
            identity.v(),
        )
        .await
    }

    async fn delete_by_benutzer(&self, benutzer_id: BenutzerId) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;
        tx.execute(
            r#"DELETE FROM "external_identity" WHERE benutzer_id = ?1"#,
            [benutzer_id.0.to_string()],
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;
        Ok(())
    }
}

pub struct SQLiteOidcAuthStateRepository {
    pub(crate) tx: Arc<Mutex<Option<Transaction>>>,
}

#[async_trait]
impl OidcAuthStateRepository for SQLiteOidcAuthStateRepository {
    async fn create(
        &self,
        state: NeuerOidcAuthState,
    ) -> RepositoryResult<Versioned<OidcAuthState>> {
        let id = OidcAuthStateId(Uuid::new_v4());
        let entity = OidcAuthState::from_parts(
            id,
            state.provider_id().clone(),
            state.state_hash().clone(),
            state.nonce().to_owned(),
            state.pkce_verifier().to_owned(),
            state.mode(),
            state.return_to().map(str::to_owned),
            state.created_at(),
            state.expires_at(),
            None,
        );
        let (mode_str, benutzer_id_opt) = match entity.mode() {
            OidcFlowMode::Login => ("login", None),
            OidcFlowMode::Link { benutzer_id } => ("link", Some(benutzer_id.0.to_string())),
            OidcFlowMode::DeleteReauth { benutzer_id } => {
                ("delete_reauth", Some(benutzer_id.0.to_string()))
            }
        };
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;
        tx.execute(
            r#"INSERT INTO "oidc_auth_state"
                (id, provider_id, state_hash, nonce, pkce_verifier, mode, benutzer_id,
                 return_to, created_at, expires_at, spent_at, _version)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, 0)"#,
            libsql::params![
                id.0.to_string(),
                entity.provider_id().as_str().to_owned(),
                entity.state_hash().as_str().to_owned(),
                entity.nonce().to_owned(),
                entity.pkce_verifier().to_owned(),
                mode_str,
                benutzer_id_opt,
                entity.return_to().map(str::to_owned),
                format_datetime(entity.created_at()),
                format_datetime(entity.expires_at()),
            ],
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;
        Ok(Versioned::init(entity))
    }

    async fn find_by_state_hash(
        &self,
        state_hash: &SecretTokenHash,
    ) -> RepositoryResult<Versioned<OidcAuthState>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;
        let mut rows = tx
            .query(
                &format!("{SELECT_STATE} WHERE state_hash = ?1"),
                [state_hash.as_str().to_owned()],
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
        state_from_row(&row)
    }

    async fn update(&self, state: &mut Versioned<OidcAuthState>) -> RepositoryResult<()> {
        let expected = state.v();
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;
        update_versioned(
            tx,
            r#"UPDATE "oidc_auth_state"
               SET spent_at = ?1, _version = ?2
               WHERE id = ?3 AND _version = ?4"#,
            (
                state.spent_at().map(format_datetime),
                (expected + 1) as i64,
                state.id().0.to_string(),
                expected as i64,
            ),
            expected,
        )
        .await?;
        state.increment();
        Ok(())
    }

    async fn delete_by_benutzer(&self, benutzer_id: BenutzerId) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;
        tx.execute(
            r#"DELETE FROM "oidc_auth_state" WHERE benutzer_id = ?1"#,
            [benutzer_id.0.to_string()],
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;
        Ok(())
    }
}

pub struct SQLiteOidcExchangeCodeRepository {
    pub(crate) tx: Arc<Mutex<Option<Transaction>>>,
}

#[async_trait]
impl OidcExchangeCodeRepository for SQLiteOidcExchangeCodeRepository {
    async fn create(
        &self,
        code: NeuerOidcExchangeCode,
    ) -> RepositoryResult<Versioned<OidcExchangeCode>> {
        let id = OidcExchangeCodeId(Uuid::new_v4());
        let entity = OidcExchangeCode::from_parts(
            id,
            code.code_hash().clone(),
            code.benutzer_id(),
            code.purpose(),
            code.expires_at(),
            None,
        );
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;
        tx.execute(
            r#"INSERT INTO "oidc_exchange_code"
                (id, code_hash, benutzer_id, purpose, expires_at, spent_at, _version)
               VALUES (?1, ?2, ?3, ?4, ?5, NULL, 0)"#,
            (
                id.0.to_string(),
                entity.code_hash().as_str().to_owned(),
                entity.benutzer_id().0.to_string(),
                entity.purpose().as_str(),
                format_datetime(entity.expires_at()),
            ),
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;
        Ok(Versioned::init(entity))
    }

    async fn find_by_code_hash(
        &self,
        code_hash: &SecretTokenHash,
    ) -> RepositoryResult<Versioned<OidcExchangeCode>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;
        let mut rows = tx
            .query(
                &format!("{SELECT_CODE} WHERE code_hash = ?1"),
                [code_hash.as_str().to_owned()],
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
        code_from_row(&row)
    }

    async fn update(&self, code: &mut Versioned<OidcExchangeCode>) -> RepositoryResult<()> {
        let expected = code.v();
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;
        update_versioned(
            tx,
            r#"UPDATE "oidc_exchange_code"
               SET spent_at = ?1, _version = ?2
               WHERE id = ?3 AND _version = ?4"#,
            (
                code.spent_at().map(format_datetime),
                (expected + 1) as i64,
                code.id().0.to_string(),
                expected as i64,
            ),
            expected,
        )
        .await?;
        code.increment();
        Ok(())
    }

    async fn delete_by_benutzer(&self, benutzer_id: BenutzerId) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;
        tx.execute(
            r#"DELETE FROM "oidc_exchange_code" WHERE benutzer_id = ?1"#,
            [benutzer_id.0.to_string()],
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;
        Ok(())
    }
}
