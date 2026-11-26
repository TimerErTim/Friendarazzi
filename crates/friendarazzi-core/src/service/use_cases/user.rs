//! Deleting a Benutzer.
//!
//! One error enum per use case. Authorisation is decided in the use case body from the
//! aggregates it loads, never by the endpoint (SPEC-konto rule 17).

use async_trait::async_trait;
use error_stack::{IntoReport, Report, ResultExt};
use tracing::instrument;

use crate::{
    application::ExecutionContext,
    domain::konto::{KlartextPasswort, verify_password},
    service::UseCase,
};

/// Every way `BenutzerLöschen` may refuse.
#[derive(Debug, thiserror::Error)]
pub enum BenutzerLöschenFehler {
    #[error("die Anfrage ist ungültig")]
    UngültigeAnfrage,
    #[error("für diese Aktion ist eine Anmeldung erforderlich")]
    NichtAngemeldet,
    #[error("das Passwort ist falsch")]
    PasswortFalsch,
    #[error("das Passwort konnte nicht geprüft werden")]
    PasswortPrüfen,
    #[error("das Konto konnte nicht geladen werden")]
    KontoLaden,
    #[error("die Wiederherstellung konnte nicht gelöscht werden")]
    WiederherstellungLöschen,
    #[error("die Bestätigung konnte nicht gelöscht werden")]
    BestätigungLöschen,
    #[error("das Konto konnte nicht gelöscht werden")]
    KontoLöschen,
    #[error("der Benutzer konnte nicht geladen werden")]
    BenutzerLaden,
    #[error("die Sitzungen konnten nicht gelöscht werden")]
    SessionLöschen,
    #[error("der Benutzer konnte nicht gelöscht werden")]
    BenutzerLöschen,
}

/// Delete the caller's own Benutzer, its Konto, sessions, and outstanding tokens.
///
/// Irreversible and gated by re-entering the Passwort plus an explicit confirmation
/// (rule 8, AC-12).
#[derive(Debug)]
pub struct BenutzerLöschen {
    pub passwort: KlartextPasswort,
    /// Explicit confirmation, so a stray request cannot destroy an account.
    pub bestätigt: bool,
}

#[async_trait]
impl UseCase<()> for BenutzerLöschen {
    type Error = Report<BenutzerLöschenFehler>;

    #[instrument(skip_all, fields(bestätigt = self.bestätigt), err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        let (benutzer_id, _) = ctx
            .auth()
            .require_authenticated()
            .change_context(BenutzerLöschenFehler::NichtAngemeldet)?;

        if !self.bestätigt {
            return Err(Report::new(BenutzerLöschenFehler::UngültigeAnfrage));
        }

        let uow = ctx
            .enter()
            .await
            .change_context(BenutzerLöschenFehler::KontoLaden)?;

        let ergebnis = async {
            // The row is loaded first: `delete` takes the aggregate as it was read, so a
            // Benutzer changed since then is a conflict rather than a vanished record.
            let benutzer = uow
                .benutzer()
                .find_by_id(benutzer_id)
                .await
                .map_err(IntoReport::into_report)
                .change_context(BenutzerLöschenFehler::BenutzerLaden)?;

            let konten = uow
                .konten()
                .find_by_benutzer(benutzer_id)
                .await
                .map_err(IntoReport::into_report)
                .change_context(BenutzerLöschenFehler::NichtAngemeldet)?;

            // Re-entering the Passwort must match *a* Konto of this Benutzer; a wrong
            // value refuses before anything is removed.
            let mut stimmt = false;
            for konto in &konten {
                if verify_password(&self.passwort, konto.password_hash())
                    .change_context(BenutzerLöschenFehler::PasswortPrüfen)?
                {
                    stimmt = true;
                    break;
                }
            }
            if !stimmt {
                return Err(Report::new(BenutzerLöschenFehler::PasswortFalsch));
            }

            for konto in konten {
                uow.wiederherstellungen()
                    .delete_by_konto(konto.id())
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(BenutzerLöschenFehler::WiederherstellungLöschen)?;
                uow.bestätigungen()
                    .delete_by_email(benutzer.email_address())
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(BenutzerLöschenFehler::BestätigungLöschen)?;
                // Delete the Konto explicitly rather than relying on a foreign-key
                // cascade: every adapter must free the address the same way, and a fake
                // has no `ON DELETE CASCADE`. The row goes in as loaded, so a Konto that
                // somebody changed meanwhile is a conflict, not a deletion.
                uow.konten()
                    .delete(konto)
                    .await
                    .map_err(IntoReport::into_report)
                    .change_context(BenutzerLöschenFehler::KontoLöschen)?;
            }

            uow.sessions()
                .delete_by_benutzer(benutzer_id)
                .await
                .map_err(IntoReport::into_report)
                .change_context(BenutzerLöschenFehler::SessionLöschen)?;

            uow.benachrichtigungen()
                .delete_by_empfänger(benutzer_id)
                .await
                .map_err(IntoReport::into_report)
                .change_context(BenutzerLöschenFehler::BenutzerLöschen)?;

            uow.benutzer()
                .delete(benutzer)
                .await
                .map_err(IntoReport::into_report)
                .change_context(BenutzerLöschenFehler::NichtAngemeldet)?;

            Ok::<_, Report<BenutzerLöschenFehler>>(())
        }
        .await;

        uow.finish(ergebnis, BenutzerLöschenFehler::KontoLaden)
            .await?;
        tracing::info!(benutzer_id = ?benutzer_id, "benutzer gelöscht");
        Ok(())
    }
}
