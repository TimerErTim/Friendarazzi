use crate::application::ResultReport;
use crate::domain::{CaptureId, EmailAdresse, GameId};

/// What a transaction or notification mail is about.
///
/// The use case names the *kind* and carries domain identifiers/tokens; it never builds a
/// URL. The frontend base URL is deployment data that only the sending adapter has, and the
/// routes below are the product's, so the adapter is the one place that knows how a token or
/// entity becomes a clickable link (SPEC-konto Q-17).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailArt {
    /// Confirmation of an address during registration step 1.
    EmailBestätigung { token: String },
    /// A requested password recovery.
    PasswortWiederherstellung { token: String },
    /// Game started after the drafting grace period elapsed.
    SpielGestartet {
        game_id: GameId,
        game_title: String,
        unsubscribe_token: String,
    },
    /// Another player posted a capture targeting the recipient (no image embedded).
    AufnahmeHochgeladen {
        game_id: GameId,
        game_title: String,
        capture_id: CaptureId,
        catcher_name: String,
        target_name: Option<String>,
        prompt_text: Option<String>,
        is_blurred_teaser: bool,
        unsubscribe_token: String,
    },
    /// Another player completed a game with Bingo.
    SpielBeendet {
        game_id: GameId,
        game_title: String,
        winner_name: String,
        unsubscribe_token: String,
    },
}

impl MailArt {
    fn primary_url(&self, frontend_basis_url: &str) -> String {
        let base = frontend_basis_url.trim_end_matches('/');
        match self {
            Self::EmailBestätigung { token } => format!("{base}/registrierung?token={token}"),
            Self::PasswortWiederherstellung { token } => format!("{base}/passwort?token={token}"),
            Self::SpielGestartet { game_id, .. } | Self::SpielBeendet { game_id, .. } => {
                format!("{base}/games?id={}", game_id.0)
            }
            Self::AufnahmeHochgeladen {
                game_id,
                capture_id,
                ..
            } => format!("{base}/games?id={}&capture={}", game_id.0, capture_id.0),
        }
    }

    fn unsubscribe_url(&self, frontend_basis_url: &str) -> Option<String> {
        let base = frontend_basis_url.trim_end_matches('/');
        match self {
            Self::EmailBestätigung { .. } | Self::PasswortWiederherstellung { .. } => None,
            Self::SpielGestartet {
                unsubscribe_token, ..
            }
            | Self::AufnahmeHochgeladen {
                unsubscribe_token, ..
            }
            | Self::SpielBeendet {
                unsubscribe_token, ..
            } => Some(format!("{base}/abbestellen?token={unsubscribe_token}")),
        }
    }
}

/// Outbound mail carrying both plain-text and styled HTML representations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mail {
    pub empfänger: EmailAdresse,
    pub art: MailArt,
}

impl Mail {
    pub fn email_bestätigung(empfänger: EmailAdresse, token: impl Into<String>) -> Self {
        Self {
            empfänger,
            art: MailArt::EmailBestätigung {
                token: token.into(),
            },
        }
    }

    pub fn passwort_wiederherstellung(empfänger: EmailAdresse, token: impl Into<String>) -> Self {
        Self {
            empfänger,
            art: MailArt::PasswortWiederherstellung {
                token: token.into(),
            },
        }
    }

    pub fn spiel_gestartet(
        empfänger: EmailAdresse,
        game_id: GameId,
        game_title: impl Into<String>,
        unsubscribe_token: impl Into<String>,
    ) -> Self {
        Self {
            empfänger,
            art: MailArt::SpielGestartet {
                game_id,
                game_title: game_title.into(),
                unsubscribe_token: unsubscribe_token.into(),
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn aufnahme_hochgeladen(
        empfänger: EmailAdresse,
        game_id: GameId,
        game_title: impl Into<String>,
        capture_id: CaptureId,
        catcher_name: impl Into<String>,
        target_name: Option<String>,
        prompt_text: Option<String>,
        is_blurred_teaser: bool,
        unsubscribe_token: impl Into<String>,
    ) -> Self {
        Self {
            empfänger,
            art: MailArt::AufnahmeHochgeladen {
                game_id,
                game_title: game_title.into(),
                capture_id,
                catcher_name: catcher_name.into(),
                target_name,
                prompt_text,
                is_blurred_teaser,
                unsubscribe_token: unsubscribe_token.into(),
            },
        }
    }

    pub fn spiel_beendet(
        empfänger: EmailAdresse,
        game_id: GameId,
        game_title: impl Into<String>,
        winner_name: impl Into<String>,
        unsubscribe_token: impl Into<String>,
    ) -> Self {
        Self {
            empfänger,
            art: MailArt::SpielBeendet {
                game_id,
                game_title: game_title.into(),
                winner_name: winner_name.into(),
                unsubscribe_token: unsubscribe_token.into(),
            },
        }
    }

    pub fn betreff(&self) -> String {
        match &self.art {
            MailArt::EmailBestätigung { .. } => "Bestätige deine E-Mail-Adresse".to_owned(),
            MailArt::PasswortWiederherstellung { .. } => "Setze dein Passwort zurück".to_owned(),
            MailArt::SpielGestartet { game_title, .. } => {
                format!("Die Jagd beginnt: „{game_title}“ ist gestartet!")
            }
            MailArt::AufnahmeHochgeladen {
                game_title,
                catcher_name,
                target_name,
                is_blurred_teaser,
                ..
            } => {
                if *is_blurred_teaser {
                    format!("Neuer Teaser von {catcher_name} in „{game_title}“")
                } else if let Some(target) = target_name {
                    format!("{catcher_name} hat {target} in „{game_title}“ erwischt!")
                } else {
                    format!("{catcher_name} hat ein Foto in „{game_title}“ hochgeladen")
                }
            }
            MailArt::SpielBeendet {
                game_title,
                winner_name,
                ..
            } => format!("Bingo! {winner_name} hat „{game_title}“ gewonnen!"),
        }
    }

    pub fn unsubscribe_url(&self, frontend_basis_url: &str) -> Option<String> {
        self.art.unsubscribe_url(frontend_basis_url)
    }

    /// Plain-text body, with links spelled from `frontend_basis_url`.
    pub fn text(&self, frontend_basis_url: &str) -> String {
        let link = self.art.primary_url(frontend_basis_url);
        let unsub_footer = self
            .art
            .unsubscribe_url(frontend_basis_url)
            .map(|url| {
                format!(
                    "\n---\nDu möchtest diese E-Mail-Benachrichtigungen nicht mehr erhalten?\nAbbestellen: {url}\n"
                )
            })
            .unwrap_or_default();

        match &self.art {
            MailArt::EmailBestätigung { .. } => format!(
                "Hallo,\n\n\
                 bitte bestätige deine E-Mail-Adresse für Friendarazzi:\n\n\
                 {link}\n\n\
                 Der Link ist 24 Stunden gültig und kann nur einmal verwendet werden.\n\
                 Wenn du dich nicht registriert hast, ignoriere diese Mail.\n"
            ),
            MailArt::PasswortWiederherstellung { .. } => format!(
                "Hallo,\n\n\
                 über diesen Link kannst du ein neues Passwort für Friendarazzi setzen:\n\n\
                 {link}\n\n\
                 Der Link ist 60 Minuten gültig und kann nur einmal verwendet werden.\n\
                 Alle bestehenden Anmeldungen werden dabei beendet.\n\
                 Wenn du ihn nicht angefordert hast, ignoriere diese Mail.\n"
            ),
            MailArt::SpielGestartet { game_title, .. } => format!(
                "Hallo,\n\n\
                 die Entwurfsphase für „{game_title}“ ist abgeschlossen und alle Spielbretter wurden ausgeteilt!\n\
                 Die Foto-Jagd ist jetzt eröffnet:\n\n\
                 {link}\n{unsub_footer}"
            ),
            MailArt::AufnahmeHochgeladen {
                game_title,
                catcher_name,
                target_name,
                prompt_text,
                is_blurred_teaser,
                ..
            } => {
                if *is_blurred_teaser {
                    format!(
                        "Hallo,\n\n\
                         {catcher_name} hat in „{game_title}“ einen verpixelten Teaser hochgeladen.\n\n\
                         Teaser ansehen:\n{link}\n{unsub_footer}"
                    )
                } else {
                    let target = target_name.as_deref().unwrap_or("dich");
                    let activity = prompt_text
                        .as_deref()
                        .map(|p| format!(" bei „{p}“"))
                        .unwrap_or_default();
                    format!(
                        "Hallo,\n\n\
                         {catcher_name} hat {target}{activity} in „{game_title}“ erwischt!\n\n\
                         Beweisfoto ansehen:\n{link}\n{unsub_footer}"
                    )
                }
            }
            MailArt::SpielBeendet {
                game_title,
                winner_name,
                ..
            } => format!(
                "Hallo,\n\n\
                 Bingo! {winner_name} hat „{game_title}“ gewonnen.\n\
                 Alle Spielbretter, Fotos und Auszeichnungen sind jetzt freigeschaltet:\n\n\
                 {link}\n{unsub_footer}"
            ),
        }
    }

    /// Neubrutalist HTML representation matching Friendarazzi's visual identity (no embedded capture photos).
    pub fn html(&self, frontend_basis_url: &str) -> String {
        let link = self.art.primary_url(frontend_basis_url);
        let unsub_url = self.art.unsubscribe_url(frontend_basis_url);

        let (badge_label, badge_bg, headline, body_html, cta_label, footnote_html) =
            match &self.art {
                MailArt::EmailBestätigung { .. } => (
                    "E-MAIL BESTÄTIGEN",
                    "#ffe600",
                    "Willkommen bei Friendarazzi!".to_owned(),
                    "<p style=\"margin:0 0 16px;font-size:16px;line-height:1.55;color:#0d0d0d;\">\
                     Bitte bestätige deine E-Mail-Adresse, um dein Konto zu aktivieren und loszuspielen.\
                     </p>".to_owned(),
                    "E-Mail-Adresse bestätigen",
                    "Der Link ist 24 Stunden gültig und kann nur einmal verwendet werden. Wenn du dich nicht registriert hast, kannst du diese Nachricht ignorieren.".to_owned(),
                ),
                MailArt::PasswortWiederherstellung { .. } => (
                    "PASSWORT ZURÜCKSETZEN",
                    "#ff3b9a",
                    "Neues Passwort setzen".to_owned(),
                    "<p style=\"margin:0 0 16px;font-size:16px;line-height:1.55;color:#0d0d0d;\">\
                     Über den Button unten kannst du ein neues Passwort für dein Friendarazzi-Konto vergeben. Alle bestehenden Anmeldungen werden dabei beendet.\
                     </p>".to_owned(),
                    "Neues Passwort festlegen",
                    "Der Link ist 60 Minuten gültig und kann nur einmal verwendet werden. Wenn du das Zurücksetzen nicht angefordert hast, ignoriere diese Mail.".to_owned(),
                ),
                MailArt::SpielGestartet { game_title, .. } => {
                    let safe_title = escape_html(game_title);
                    (
                        "SPIELSTART",
                        "#00e5ff",
                        format!("Die Jagd beginnt: „{safe_title}“"),
                        format!(
                            "<p style=\"margin:0 0 16px;font-size:16px;line-height:1.55;color:#0d0d0d;\">\
                             Alle Aufträge sind eingeloggt und der Countdown ist abgelaufen. Dein persönliches Bingo-Brett für <strong>{safe_title}</strong> liegt bereit!\
                             </p>"
                        ),
                        "Jetzt zum Spielbrett",
                        "Tipp: Halte deine Kamera bereit – wer zuerst eine Reihe, Spalte oder Diagonale voll hat, holt das Bingo.".to_owned(),
                    )
                }
                MailArt::AufnahmeHochgeladen {
                    game_title,
                    catcher_name,
                    target_name,
                    prompt_text,
                    is_blurred_teaser,
                    ..
                } => {
                    let safe_game = escape_html(game_title);
                    let safe_catcher = escape_html(catcher_name);
                    if *is_blurred_teaser {
                        (
                            "TEASER-FOTO",
                            "#ff3b9a",
                            format!("Neuer Teaser in „{safe_game}“"),
                            format!(
                                "<p style=\"margin:0 0 16px;font-size:16px;line-height:1.55;color:#0d0d0d;\">\
                                 <strong>{safe_catcher}</strong> hat gerade ein neues verpixeltes Teaser-Foto hochgeladen.\
                                 </p>\
                                 <div style=\"margin:0 0 20px;padding:14px 16px;background-color:#f0ece0;border:2px solid #0d0d0d;border-radius:6px;\">\
                                 <p style=\"margin:0;font-size:13px;font-weight:700;color:#454545;text-transform:uppercase;letter-spacing:0.05em;\">Spiel</p>\
                                 <p style=\"margin:4px 0 0;font-size:16px;font-weight:700;color:#0d0d0d;\">{safe_game}</p>\
                                 </div>"
                            ),
                            "Teaser im Spiel ansehen",
                            "Aus Datenschutz- und Spielmodus-Gründen wird das Foto nicht per E-Mail mitgeschickt.".to_owned(),
                        )
                    } else {
                        let safe_target = escape_html(target_name.as_deref().unwrap_or("dich"));
                        let prompt_block = prompt_text
                            .as_deref()
                            .map(|p| {
                                let safe_prompt = escape_html(p);
                                format!(
                                    "<div style=\"margin:0 0 20px;padding:14px 16px;background-color:#fff7b3;border:2px solid #0d0d0d;border-radius:6px;\">\
                                     <p style=\"margin:0;font-size:12px;font-weight:700;color:#454545;text-transform:uppercase;letter-spacing:0.06em;\">Auftrag in „{safe_game}“</p>\
                                     <p style=\"margin:6px 0 0;font-size:17px;font-weight:700;color:#0d0d0d;\">„{safe_prompt}“</p>\
                                     </div>"
                                )
                            })
                            .unwrap_or_default();
                        (
                            "NEUER SCHNAPPSCHUSS",
                            "#ffe600",
                            format!("{safe_catcher} hat {safe_target} erwischt!"),
                            format!(
                                "<p style=\"margin:0 0 16px;font-size:16px;line-height:1.55;color:#0d0d0d;\">\
                                 <strong>{safe_catcher}</strong> hat ein Beweisfoto von <strong>{safe_target}</strong> im Spiel <strong>„{safe_game}“</strong> eingereicht.\
                                 </p>{prompt_block}"
                            ),
                            "Beweisfoto ansehen",
                            "Das Beweisfoto wird nicht direkt in dieser E-Mail eingebettet – öffne das Spiel, um den Moment anzusehen.".to_owned(),
                        )
                    }
                }
                MailArt::SpielBeendet {
                    game_title,
                    winner_name,
                    ..
                } => {
                    let safe_game = escape_html(game_title);
                    let safe_winner = escape_html(winner_name);
                    (
                        "BINGO-SIEG",
                        "#ffe600",
                        format!("{safe_winner} hat „{safe_game}“ gewonnen!"),
                        format!(
                            "<p style=\"margin:0 0 16px;font-size:16px;line-height:1.55;color:#0d0d0d;\">\
                             Das Bingo ist komplett! <strong>{safe_winner}</strong> holt die Goldmedaille in <strong>„{safe_game}“</strong>.\
                             </p>\
                             <div style=\"margin:0 0 20px;padding:14px 16px;background-color:#d6fbff;border:2px solid #0d0d0d;border-radius:6px;\">\
                             <p style=\"margin:0;font-size:14px;font-weight:700;color:#0d0d0d;\">\
                             Alle Spielbretter, Beweisfotos, die Zeitachse und die Auszeichnungen sind jetzt für alle freigeschaltet.\
                             </p>\
                             </div>"
                        ),
                        "Siegerehrung & Galerie ansehen",
                        "Schau dir alle aufgedeckten Bretter und Superlative direkt im Spiel an.".to_owned(),
                    )
                }
            };

        let safe_link = escape_html(&link);
        let safe_headline = headline;
        let safe_footnote = escape_html(&footnote_html);
        let unsubscribe_block = unsub_url
            .map(|u| {
                let safe_unsub = escape_html(&u);
                format!(
                    "<p style=\"margin:14px 0 0;font-size:12px;line-height:1.5;color:#454545;\">\
                     Du möchtest diese E-Mail-Benachrichtigungen nicht mehr erhalten? \
                     <a href=\"{safe_unsub}\" style=\"color:#0d0d0d;font-weight:700;text-decoration:underline;\">Hier abbestellen</a>.\
                     </p>"
                )
            })
            .unwrap_or_default();

        format!(
            "<!DOCTYPE html>\
            <html lang=\"de\">\
            <head>\
              <meta charset=\"utf-8\">\
              <meta name=\"viewport\" content=\"width=device-width,initial-scale=1.0\">\
              <title>{safe_title}</title>\
            </head>\
            <body style=\"margin:0;padding:0;background-color:#faf8f2;font-family:'Space Grotesk',-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;color:#0d0d0d;\">\
              <table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" style=\"background-color:#faf8f2;padding:28px 14px;\">\
                <tr>\
                  <td align=\"center\">\
                    <table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" style=\"max-width:560px;background-color:#ffffff;border:3px solid #0d0d0d;border-radius:8px;box-shadow:6px 6px 0 #0d0d0d;overflow:hidden;\">\
                      <tr>\
                        <td style=\"padding:18px 24px;background-color:#ffffff;border-bottom:3px solid #0d0d0d;\">\
                          <table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\">\
                            <tr>\
                              <td>\
                                <span style=\"display:inline-block;padding:4px 8px;background-color:#ffe600;border:2px solid #0d0d0d;font-weight:900;font-size:16px;letter-spacing:-0.02em;color:#0d0d0d;\">Friend</span>\
                                <span style=\"display:inline-block;padding:4px 8px;background-color:#00e5ff;border:2px solid #0d0d0d;border-left:none;font-weight:900;font-size:16px;letter-spacing:-0.02em;color:#0d0d0d;\">araz</span>\
                                <span style=\"display:inline-block;padding:4px 8px;background-color:#ff3b9a;border:2px solid #0d0d0d;border-left:none;font-weight:900;font-size:16px;letter-spacing:-0.02em;color:#0d0d0d;\">zi</span>\
                              </td>\
                              <td align=\"right\">\
                                <span style=\"display:inline-block;padding:4px 10px;background-color:{badge_bg};border:2px solid #0d0d0d;border-radius:4px;font-size:11px;font-weight:800;letter-spacing:0.06em;color:#0d0d0d;\">{badge_label}</span>\
                              </td>\
                            </tr>\
                          </table>\
                        </td>\
                      </tr>\
                      <tr>\
                        <td style=\"padding:26px 24px 22px;\">\
                          <h1 style=\"margin:0 0 14px;font-size:24px;line-height:1.2;font-weight:900;color:#0d0d0d;\">{safe_headline}</h1>\
                          {body_html}\
                          <table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\" style=\"margin:22px 0 18px;\">\
                            <tr>\
                              <td style=\"background-color:#ffe600;border:2.5px solid #0d0d0d;border-radius:6px;box-shadow:4px 4px 0 #0d0d0d;\">\
                                <a href=\"{safe_link}\" style=\"display:inline-block;padding:13px 22px;font-size:15px;font-weight:800;color:#0d0d0d;text-decoration:none;\">{cta_label} &rarr;</a>\
                              </td>\
                            </tr>\
                          </table>\
                          <p style=\"margin:0;font-size:12px;line-height:1.5;color:#454545;word-break:break-all;\">\
                            Falls der Button nicht funktioniert, kopiere diesen Link:<br>\
                            <a href=\"{safe_link}\" style=\"color:#0d0d0d;text-decoration:underline;\">{safe_link}</a>\
                          </p>\
                        </td>\
                      </tr>\
                      <tr>\
                        <td style=\"padding:16px 24px 20px;background-color:#f0ece0;border-top:2px solid #0d0d0d;\">\
                          <p style=\"margin:0;font-size:12px;line-height:1.5;color:#454545;\">{safe_footnote}</p>\
                          {unsubscribe_block}\
                        </td>\
                      </tr>\
                    </table>\
                  </td>\
                </tr>\
              </table>\
            </body>\
            </html>",
            safe_title = escape_html(&self.betreff()),
        )
    }
}

fn escape_html(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

#[derive(Debug, thiserror::Error)]
pub enum MailError {
    #[error("die Mail konnte nicht zugestellt werden")]
    Zustellung,
    #[error("der Mailversand ist nicht konfiguriert")]
    NichtKonfiguriert,
    #[error("der Mailversand antwortet nicht auf NOOP")]
    AntwortetNichtAufNoop,
    #[error("für den verschlüsselten Mailversand fehlen Benutzername oder Passwort")]
    OhneZugangsdaten,
}

/// Outbound transaction and notification mail. One adapter per deployment; Mailpit in
/// development and tests, the configured relay in production (SPEC-konto FR-11, AC-19).
///
/// A failing send never fails the request that caused it: use cases call this after their
/// unit of work commits and log the failure instead of propagating it (NFR-5).
#[async_trait::async_trait]
pub trait MailSender: Send + Sync {
    async fn send(&self, mail: Mail) -> ResultReport<(), MailError>;

    /// Whether a send would reach the relay right now, proven without delivering a message.
    async fn prüfe_verbindung(&self) -> ResultReport<(), MailError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn adresse() -> EmailAdresse {
        EmailAdresse::normalisiert("anna@muster.at").expect("static address is valid")
    }

    #[test_log::test]
    fn der_link_wird_aus_der_basis_url_und_der_route_gebaut() {
        let mail = Mail::email_bestätigung(adresse(), "abc");
        assert!(
            mail.text("http://localhost:3000")
                .contains("http://localhost:3000/registrierung?token=abc")
        );
        assert!(
            mail.html("http://localhost:3000")
                .contains("http://localhost:3000/registrierung?token=abc")
        );

        let mail = Mail::passwort_wiederherstellung(adresse(), "xyz");
        assert!(
            mail.text("http://localhost:3000")
                .contains("http://localhost:3000/passwort?token=xyz")
        );
    }

    #[test_log::test]
    fn ein_schrägstrich_am_ende_erzeugt_keinen_doppelten() {
        let mail = Mail::email_bestätigung(adresse(), "abc");
        assert!(
            mail.text("https://friendarazzi.app/")
                .contains("https://friendarazzi.app/registrierung?token=abc")
        );
    }

    #[test_log::test]
    fn benachrichtigungs_mails_enthalten_deep_links_und_html_escaping() {
        let gid = GameId(Uuid::nil());
        let cid = CaptureId(Uuid::nil());
        let cap_mail = Mail::aufnahme_hochgeladen(
            adresse(),
            gid,
            "Abend <Test>",
            cid,
            "Max",
            Some("Anna".to_owned()),
            Some("Trinkt <Espresso>".to_owned()),
            false,
            "unsub-tok",
        );

        assert!(cap_mail.betreff().contains("Max"));
        assert!(cap_mail.betreff().contains("Anna"));
        let html = cap_mail.html("https://friendarazzi.app");
        assert!(html.contains("Abend &lt;Test&gt;"));
        assert!(html.contains("Trinkt &lt;Espresso&gt;"));
        assert!(html.contains("https://friendarazzi.app/games?id=00000000-0000-0000-0000-000000000000&amp;capture=00000000-0000-0000-0000-000000000000"));
        assert!(html.contains("https://friendarazzi.app/abbestellen?token=unsub-tok"));
        assert!(!html.contains("<img"));

        let win_mail = Mail::spiel_beendet(adresse(), gid, "Freitag", "Lisa", "unsub-win");
        assert!(win_mail.betreff().contains("Lisa"));
        assert!(win_mail.betreff().contains("Freitag"));
        assert!(
            win_mail
                .text("https://friendarazzi.app")
                .contains("https://friendarazzi.app/games?id=00000000-0000-0000-0000-000000000000")
        );
    }
}
