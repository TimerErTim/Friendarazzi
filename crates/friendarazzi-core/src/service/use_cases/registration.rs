//! Single-step registration with immediate session and async email confirmation.

use async_trait::async_trait;
use error_stack::{IntoReport, Report, ResultExt};
use tracing::{info, instrument, warn};

use crate::{
    application::{ExecutionContext, uow::UnitOfWork},
    domain::{
        EmailAdresse, NeuerBenutzer, NonEmptyString,
        konto::{
            KlartextPasswort, NeueBestätigung, NeuesKonto, RefreshToken, SecretToken, hash_password,
        },
    },
    ports::{ClientKey, Mail, RepositoryError, ThrottleAction},
    service::{SessionAusgestellt, UseCase},
};

use super::{claims_for_session, new_session};

/// Every way `Registrieren` may refuse.
#[derive(Debug, thiserror::Error)]
pub enum RegistrierenFehler {
    #[error("die Anfrage ist ungültig")]
    UngültigeAnfrage,
    #[error("Diese E-Mail-Adresse ist bereits vergeben")]
    EmailBereitsVergeben,
    #[error("zu viele Versuche; bitte später erneut versuchen")]
    Gedrosselt,
    #[error("das Passwort konnte nicht gespeichert werden")]
    PasswortSetzen,
    #[error("das Token konnte nicht erzeugt werden")]
    TokenErzeugen,
    #[error("die Registrierung konnte nicht gespeichert werden")]
    Persistence,
}

/// Register in one step: create Benutzer + password Konto, open Session, and mail confirmation.
///
/// The newly created Benutzer starts unverified (`email_verified_at = None`) and can participate
/// in one game before confirming the address.
#[derive(Debug)]
pub struct Registrieren {
    pub email: EmailAdresse,
    pub passwort: KlartextPasswort,
    pub display_name: Option<NonEmptyString>,
    pub client_key: ClientKey,
}

#[async_trait]
impl UseCase<SessionAusgestellt> for Registrieren {
    type Error = Report<RegistrierenFehler>;

    #[instrument(skip_all, fields(email = ?self.email), err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<SessionAusgestellt, Self::Error> {
        let jetzt = ctx.clock().now();

        ctx.throttle()
            .check_and_increment(&self.client_key, ThrottleAction::Registrierung, jetzt)
            .await
            .change_context(RegistrierenFehler::Gedrosselt)?;

        let hashed_passwort =
            hash_password(&self.passwort).change_context(RegistrierenFehler::PasswortSetzen)?;
        let confirmation_token =
            SecretToken::random().change_context(RegistrierenFehler::TokenErzeugen)?;
        let refresh_token =
            RefreshToken::random().change_context(RegistrierenFehler::TokenErzeugen)?;

        let email = self.email;
        let display_name = self.display_name;
        let uow = ctx
            .enter()
            .await
            .change_context(RegistrierenFehler::Persistence)?;

        let ergebnis = async {
            match uow.benutzer().find_by_email(&email).await {
                Ok(_) => return Err(Report::new(RegistrierenFehler::EmailBereitsVergeben)),
                Err(fehler) if matches!(fehler.current_context(), RepositoryError::NotFound) => {}
                Err(fehler) => return Err(fehler).change_context(RegistrierenFehler::Persistence),
            }

            let neuer_benutzer = match display_name {
                Some(name) => NeuerBenutzer::neu(email.clone(), name, jetzt),
                None => NeuerBenutzer::registrierung(email.clone(), jetzt),
            }
            .with_email_verified_at(None);

            let benutzer = match uow.benutzer().create(neuer_benutzer).await {
                Ok(b) => b,
                Err(fehler) if matches!(fehler.current_context(), RepositoryError::Conflict) => {
                    return Err(Report::new(RegistrierenFehler::EmailBereitsVergeben));
                }
                Err(fehler) => {
                    return Err(fehler)
                        .map_err(IntoReport::into_report)
                        .change_context(RegistrierenFehler::Persistence);
                }
            };

            uow.konten()
                .create(NeuesKonto::neu(benutzer.id(), hashed_passwort, jetzt))
                .await
                .map_err(IntoReport::into_report)
                .change_context(RegistrierenFehler::Persistence)?;

            spend_open_confirmations(&uow, &email, jetzt)
                .await
                .change_context(RegistrierenFehler::Persistence)?;

            let bestätigung =
                NeueBestätigung::neu(email.clone(), (&confirmation_token).into(), jetzt);
            uow.bestätigungen()
                .create(bestätigung)
                .await
                .map_err(IntoReport::into_report)
                .change_context(RegistrierenFehler::Persistence)?;

            let session = uow
                .sessions()
                .create(new_session(&ctx, benutzer.id(), &refresh_token, jetzt))
                .await
                .map_err(IntoReport::into_report)
                .change_context(RegistrierenFehler::Persistence)?;

            Ok::<_, Report<RegistrierenFehler>>(SessionAusgestellt {
                benutzer: benutzer.cloned_data(),
                session_id: session.id(),
                refresh_token,
                claims: claims_for_session(&ctx, benutzer.id(), session.id(), jetzt),
            })
        }
        .await;

        let ausgestellt = uow
            .finish(ergebnis, RegistrierenFehler::Persistence)
            .await?;

        if let Err(fehler) = ctx
            .mail_sender()
            .send(Mail::email_bestätigung(
                email,
                confirmation_token.base64url().as_str(),
            ))
            .await
        {
            warn!(fehler = %fehler, "Bestätigungsmail konnte nicht zugestellt werden");
        }

        info!(
            benutzer_id = ?ausgestellt.benutzer.id(),
            "registrierung in einem Schritt abgeschlossen"
        );
        Ok(ausgestellt)
    }
}

/// Every way `EmailBestätigen` may refuse.
#[derive(Debug, thiserror::Error)]
pub enum EmailBestätigenFehler {
    #[error("der Bestätigungslink ist ungültig oder abgelaufen")]
    UngültigerLink,
    #[error("zu viele Versuche; bitte später erneut versuchen")]
    Gedrosselt,
    #[error("die Bestätigung konnte nicht gespeichert werden")]
    Persistence,
}

/// Confirm the Benutzer's e-mail address with the single-use confirmation token.
#[derive(Debug)]
pub struct EmailBestätigen {
    pub token: SecretToken,
    pub client_key: ClientKey,
}

#[async_trait]
impl UseCase<()> for EmailBestätigen {
    type Error = Report<EmailBestätigenFehler>;

    #[instrument(skip_all, fields(client_ip = %self.client_key.ip), err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        let jetzt = ctx.clock().now();

        ctx.throttle()
            .check_and_increment(&self.client_key, ThrottleAction::EmailBestätigung, jetzt)
            .await
            .change_context(EmailBestätigenFehler::Gedrosselt)?;

        let uow = ctx
            .enter()
            .await
            .change_context(EmailBestätigenFehler::Persistence)?;

        let ergebnis = async {
            let bestätigung = match uow
                .bestätigungen()
                .find_by_hash(&(&self.token).into())
                .await
            {
                Ok(b) => b,
                Err(fehler) if matches!(fehler.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(EmailBestätigenFehler::UngültigerLink));
                }
                Err(fehler) => {
                    return Err(fehler).change_context(EmailBestätigenFehler::Persistence);
                }
            };

            if !bestätigung.ist_gültig(jetzt) {
                return Err(Report::new(EmailBestätigenFehler::UngültigerLink));
            }

            let mut verbraucht = bestätigung.clone();
            verbraucht.verbrauchen(jetzt);
            uow.bestätigungen()
                .update(&mut verbraucht)
                .await
                .map_err(IntoReport::into_report)
                .change_context(EmailBestätigenFehler::Persistence)?;

            let mut benutzer = match uow.benutzer().find_by_email(bestätigung.email()).await {
                Ok(b) => b,
                Err(fehler) if matches!(fehler.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(EmailBestätigenFehler::UngültigerLink));
                }
                Err(fehler) => {
                    return Err(fehler).change_context(EmailBestätigenFehler::Persistence);
                }
            };

            benutzer.email_verifizieren(jetzt);
            uow.benutzer()
                .update(&mut benutzer)
                .await
                .map_err(IntoReport::into_report)
                .change_context(EmailBestätigenFehler::Persistence)?;

            uow.bestätigungen()
                .delete_by_email(bestätigung.email())
                .await
                .map_err(IntoReport::into_report)
                .change_context(EmailBestätigenFehler::Persistence)?;

            Ok::<_, Report<EmailBestätigenFehler>>(benutzer.id())
        }
        .await;

        let benutzer_id = uow
            .finish(ergebnis, EmailBestätigenFehler::Persistence)
            .await?;
        info!(?benutzer_id, "E-Mail-Adresse bestätigt");
        Ok(())
    }
}

/// Every way `EmailBestätigungErneutSenden` may refuse.
#[derive(Debug, thiserror::Error)]
pub enum EmailBestätigungSendenFehler {
    #[error("für diese Aktion ist eine Anmeldung erforderlich")]
    Unauthorized,
    #[error("deine E-Mail-Adresse ist bereits bestätigt")]
    BereitsBestätigt,
    #[error("zu viele Versuche; bitte später erneut versuchen")]
    Gedrosselt,
    #[error("das Bestätigungs-Token konnte nicht erzeugt werden")]
    TokenErzeugen,
    #[error("die Bestätigung konnte nicht gespeichert werden")]
    Persistence,
}

/// Issue a fresh confirmation token and email for the currently signed-in unverified Benutzer.
#[derive(Debug, Clone)]
pub struct EmailBestätigungErneutSenden {
    pub client_key: ClientKey,
}

#[async_trait]
impl UseCase<()> for EmailBestätigungErneutSenden {
    type Error = Report<EmailBestätigungSendenFehler>;

    #[instrument(skip_all, fields(client_ip = %self.client_key.ip), err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(EmailBestätigungSendenFehler::Unauthorized)?;
        let jetzt = ctx.clock().now();

        ctx.throttle()
            .check_and_increment(
                &self.client_key,
                ThrottleAction::EmailBestätigungErneut,
                jetzt,
            )
            .await
            .change_context(EmailBestätigungSendenFehler::Gedrosselt)?;

        let token =
            SecretToken::random().change_context(EmailBestätigungSendenFehler::TokenErzeugen)?;

        let uow = ctx
            .enter()
            .await
            .change_context(EmailBestätigungSendenFehler::Persistence)?;

        let ergebnis = async {
            let benutzer = match uow.benutzer().find_by_id(user_id).await {
                Ok(b) => b,
                Err(fehler) if matches!(fehler.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(EmailBestätigungSendenFehler::Unauthorized));
                }
                Err(fehler) => {
                    return Err(fehler).change_context(EmailBestätigungSendenFehler::Persistence);
                }
            };
            if benutzer.ist_verifiziert() {
                return Err(Report::new(EmailBestätigungSendenFehler::BereitsBestätigt));
            }

            let email = benutzer.email_address().clone();
            spend_open_confirmations(&uow, &email, jetzt)
                .await
                .change_context(EmailBestätigungSendenFehler::Persistence)?;

            let bestätigung = NeueBestätigung::neu(email.clone(), (&token).into(), jetzt);
            uow.bestätigungen()
                .create(bestätigung)
                .await
                .map_err(IntoReport::into_report)
                .change_context(EmailBestätigungSendenFehler::Persistence)?;

            Ok::<_, Report<EmailBestätigungSendenFehler>>(email)
        }
        .await;

        let email = uow
            .finish(ergebnis, EmailBestätigungSendenFehler::Persistence)
            .await?;

        if let Err(fehler) = ctx
            .mail_sender()
            .send(Mail::email_bestätigung(email, token.base64url().as_str()))
            .await
        {
            warn!(fehler = %fehler, "Bestätigungsmail konnte nicht erneut zugestellt werden");
        }
        Ok(())
    }
}

async fn spend_open_confirmations(
    uow: &UnitOfWork<'_>,
    email: &EmailAdresse,
    jetzt: chrono::DateTime<chrono::Utc>,
) -> Result<(), Report<RepositoryError>> {
    let offene = uow
        .bestätigungen()
        .find_by_email(email)
        .await
        .map_err(IntoReport::into_report)?;
    for bestätigung in offene {
        if bestätigung.ist_verbraucht() {
            continue;
        }
        let mut geladen = bestätigung;
        geladen.verbrauchen(jetzt);
        uow.bestätigungen()
            .update(&mut geladen)
            .await
            .map_err(IntoReport::into_report)?;
    }
    Ok(())
}
