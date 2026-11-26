use std::sync::Arc;

use async_lock::Mutex;
use async_trait::async_trait;
use error_stack::ResultExt;
use friendarazzi_core::{
    ErrorReportExt,
    application::uow::Versioned,
    domain::{
        Benachrichtigung, BenachrichtigungId, BenachrichtigungsEinstellungen, BenutzerId,
        CaptureId, GameId, NeueBenachrichtigung, NeueBenachrichtigungsEinstellungen,
        PreferenceToggle,
    },
    ports::{
        BenachrichtigungRepository, BenachrichtigungsEinstellungenRepository, RepositoryError,
        RepositoryResult,
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

const SELECT_NOTIF: &str = r#"SELECT id, empfaenger_id, art, game_id, capture_id, actor_id, target_id, dedupe_key, erstellt_am, gelesen_am, _version FROM "benachrichtigung""#;

const SELECT_PREFS: &str = r#"SELECT benutzer_id, default_watch_games, default_watch_updated_at, email_game_started, email_game_started_updated_at, email_capture_posted, email_capture_posted_updated_at, email_game_completed, email_game_completed_updated_at, _version FROM "benachrichtigungs_einstellungen""#;

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

fn prefs_from_row(row: &Row) -> RepositoryResult<Versioned<BenachrichtigungsEinstellungen>> {
    let bid_raw: String = row.get(0).contextualize(RepositoryError::Data)?;
    let watch_active: i64 = row.get(1).contextualize(RepositoryError::Data)?;
    let watch_at: String = row.get(2).contextualize(RepositoryError::Data)?;
    let started_active: i64 = row.get(3).contextualize(RepositoryError::Data)?;
    let started_at: String = row.get(4).contextualize(RepositoryError::Data)?;
    let capture_active: i64 = row.get(5).contextualize(RepositoryError::Data)?;
    let capture_at: String = row.get(6).contextualize(RepositoryError::Data)?;
    let completed_active: i64 = row.get(7).contextualize(RepositoryError::Data)?;
    let completed_at: String = row.get(8).contextualize(RepositoryError::Data)?;
    let version = parse_version(row, 9)?;

    let prefs = BenachrichtigungsEinstellungen::neu(NeueBenachrichtigungsEinstellungen {
        benutzer_id: BenutzerId(parse_uuid(&bid_raw)?),
        standard_spiel_beobachten: PreferenceToggle::neu(
            watch_active != 0,
            parse_datetime(&watch_at)?,
        ),
        email_spiel_gestartet: PreferenceToggle::neu(
            started_active != 0,
            parse_datetime(&started_at)?,
        ),
        email_aufnahme_hochgeladen: PreferenceToggle::neu(
            capture_active != 0,
            parse_datetime(&capture_at)?,
        ),
        email_spiel_beendet: PreferenceToggle::neu(
            completed_active != 0,
            parse_datetime(&completed_at)?,
        ),
    });

    Ok(Versioned::new(version, prefs))
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

pub struct SQLiteBenachrichtigungsEinstellungenRepository {
    pub(crate) tx: Arc<Mutex<Option<Transaction>>>,
}

#[async_trait]
impl BenachrichtigungsEinstellungenRepository for SQLiteBenachrichtigungsEinstellungenRepository {
    async fn create(
        &self,
        neu: NeueBenachrichtigungsEinstellungen,
    ) -> RepositoryResult<Versioned<BenachrichtigungsEinstellungen>> {
        let domain = BenachrichtigungsEinstellungen::neu(neu);

        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"INSERT INTO "benachrichtigungs_einstellungen"
               (benutzer_id, default_watch_games, default_watch_updated_at,
                email_game_started, email_game_started_updated_at,
                email_capture_posted, email_capture_posted_updated_at,
                email_game_completed, email_game_completed_updated_at, _version)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0)"#,
            (
                domain.benutzer_id().0.to_string(),
                i64::from(domain.standard_spiel_beobachten().aktiv),
                format_datetime(domain.standard_spiel_beobachten().aktualisiert_am),
                i64::from(domain.email_spiel_gestartet().aktiv),
                format_datetime(domain.email_spiel_gestartet().aktualisiert_am),
                i64::from(domain.email_aufnahme_hochgeladen().aktiv),
                format_datetime(domain.email_aufnahme_hochgeladen().aktualisiert_am),
                i64::from(domain.email_spiel_beendet().aktiv),
                format_datetime(domain.email_spiel_beendet().aktualisiert_am),
            ),
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;

        Ok(Versioned::init(domain))
    }

    async fn find_by_benutzer(
        &self,
        benutzer_id: BenutzerId,
    ) -> RepositoryResult<Versioned<BenachrichtigungsEinstellungen>> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        let mut rows = tx
            .query(
                &format!(r#"{SELECT_PREFS} WHERE benutzer_id = ?1"#),
                [benutzer_id.0.to_string()],
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

        prefs_from_row(&row)
    }

    async fn update(
        &self,
        einstellungen: &mut Versioned<BenachrichtigungsEinstellungen>,
    ) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        update_versioned(
            tx,
            r#"UPDATE "benachrichtigungs_einstellungen"
               SET default_watch_games = ?2, default_watch_updated_at = ?3,
                   email_game_started = ?4, email_game_started_updated_at = ?5,
                   email_capture_posted = ?6, email_capture_posted_updated_at = ?7,
                   email_game_completed = ?8, email_game_completed_updated_at = ?9,
                   _version = ?10 + 1
               WHERE benutzer_id = ?1 AND _version = ?10"#,
            (
                einstellungen.benutzer_id().0.to_string(),
                i64::from(einstellungen.standard_spiel_beobachten().aktiv),
                format_datetime(einstellungen.standard_spiel_beobachten().aktualisiert_am),
                i64::from(einstellungen.email_spiel_gestartet().aktiv),
                format_datetime(einstellungen.email_spiel_gestartet().aktualisiert_am),
                i64::from(einstellungen.email_aufnahme_hochgeladen().aktiv),
                format_datetime(einstellungen.email_aufnahme_hochgeladen().aktualisiert_am),
                i64::from(einstellungen.email_spiel_beendet().aktiv),
                format_datetime(einstellungen.email_spiel_beendet().aktualisiert_am),
                einstellungen.v() as i64,
            ),
            einstellungen.v(),
        )
        .await?;

        einstellungen.increment();
        Ok(())
    }

    async fn delete(
        &self,
        einstellungen: Versioned<BenachrichtigungsEinstellungen>,
    ) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        delete_versioned(
            tx,
            r#"DELETE FROM "benachrichtigungs_einstellungen" WHERE benutzer_id = ?1 AND _version = ?2"#,
            (
                einstellungen.benutzer_id().0.to_string(),
                einstellungen.v() as i64,
            ),
            einstellungen.v(),
        )
        .await
    }

    async fn delete_by_benutzer(&self, benutzer_id: BenutzerId) -> RepositoryResult<()> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or(RepositoryError::Conflict)?;

        tx.execute(
            r#"DELETE FROM "benachrichtigungs_einstellungen" WHERE benutzer_id = ?1"#,
            [benutzer_id.0.to_string()],
        )
        .await
        .contextualize_with(libsql_error_to_persistence_error)?;
        Ok(())
    }
}
