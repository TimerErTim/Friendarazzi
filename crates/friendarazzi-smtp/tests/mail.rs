//! The adapter's envelope and MIME parts are asserted without a relay: recipient, subject,
//! plain-text + HTML parts, and `List-Unsubscribe` headers are checked directly.

use friendarazzi_core::{
    domain::{CaptureId, EmailAdresse, GameId},
    ports::{Mail, MailError},
};
use friendarazzi_smtp::{SmtpKonfiguration, SmtpMailSender};
use lettre::message::header::Subject;
use uuid::Uuid;

fn absender() -> EmailAdresse {
    EmailAdresse::normalisiert("noreply@friendarazzi.app").expect("static address is valid")
}

fn sender() -> SmtpMailSender {
    let mut konfiguration = SmtpKonfiguration::mailpit(absender());
    konfiguration.frontend_basis_url = "https://friendarazzi.app".to_owned();
    SmtpMailSender::neu(konfiguration).expect("mailpit relay builds")
}

#[test_log::test(tokio::test)]
async fn die_nachricht_trägt_empfänger_betreff_klartext_und_html() {
    let sender = sender();
    let mail = Mail::email_bestätigung(
        EmailAdresse::normalisiert("anna@muster.at").expect("address is valid"),
        "abc",
    );

    let nachricht = sender.baue_nachricht(&mail).expect("the message builds");

    let empfänger: Vec<String> = nachricht
        .envelope()
        .to()
        .iter()
        .map(|adresse| adresse.to_string())
        .collect();
    assert_eq!(empfänger, vec!["anna@muster.at".to_owned()]);

    let betreff = nachricht
        .headers()
        .get::<Subject>()
        .expect("the subject header is set");
    assert_eq!(
        betreff,
        Subject::from("Bestätige deine E-Mail-Adresse".to_owned())
    );

    let roh = String::from_utf8_lossy(&nachricht.formatted()).into_owned();
    assert!(roh.contains("Content-Type: multipart/alternative"), "{roh}");
    assert!(roh.contains("Content-Type: text/plain"), "{roh}");
    assert!(roh.contains("Content-Type: text/html"), "{roh}");
    let rumpf = entfalte_quoted_printable(&roh);
    assert!(
        rumpf.contains("https://friendarazzi.app/registrierung?token=abc"),
        "der Link steht unversehrt im Rumpf: {rumpf}"
    );
    assert!(!roh.contains("List-Unsubscribe:"));
}

#[test_log::test(tokio::test)]
async fn benachrichtigungs_mail_trägt_list_unsubscribe_header_und_html_details() {
    let sender = sender();
    let gid = GameId(Uuid::nil());
    let cid = CaptureId(Uuid::nil());
    let mail = Mail::aufnahme_hochgeladen(
        EmailAdresse::normalisiert("anna@muster.at").expect("address is valid"),
        gid,
        "Sommerfest",
        cid,
        "Max",
        Some("Anna".to_owned()),
        Some("Lacht laut".to_owned()),
        false,
        "unsub-123",
    );

    let nachricht = sender.baue_nachricht(&mail).expect("the message builds");
    let roh = String::from_utf8_lossy(&nachricht.formatted()).into_owned();
    assert!(
        roh.contains("List-Unsubscribe: <https://friendarazzi.app/abbestellen?token=unsub-123>"),
        "{roh}"
    );
    assert!(
        roh.contains("List-Unsubscribe-Post: List-Unsubscribe=One-Click"),
        "{roh}"
    );
    let rumpf = entfalte_quoted_printable(&roh);
    assert!(rumpf.contains("Lacht laut"));
    assert!(rumpf.contains("NEUER SCHNAPPSCHUSS"));
}

/// A message that cannot even be built is a delivery failure, never a panic and never a
/// silent success: the use case logs it and the request still answers (NFR-5).
#[test_log::test(tokio::test)]
async fn eine_ungültige_adresse_ist_ein_zustellungsfehler() {
    let sender = sender();
    // `EmailAdresse` only checks for `@` and `.`; the space makes the mailbox unparseable.
    let mail = Mail::email_bestätigung(
        EmailAdresse::normalisiert("anna muster@muster.at").expect("domain address is lenient"),
        "abc",
    );

    let fehler = sender
        .baue_nachricht(&mail)
        .expect_err("die Adresse lässt sich nicht als Mailbox parsen");
    assert!(matches!(fehler.current_context(), MailError::Zustellung));
}

/// Undo `quoted-printable`: soft line breaks and `=XX` escapes both have to go before a
/// link can be compared.
fn entfalte_quoted_printable(nachricht: &str) -> String {
    let rumpf = nachricht
        .split_once("\r\n\r\n")
        .map(|(_, rumpf)| rumpf)
        .unwrap_or(nachricht);
    let ohne_umbruch = rumpf.replace("=\r\n", "");

    let bytes = ohne_umbruch.as_bytes();
    let mut ergebnis = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'='
            && i + 2 < bytes.len()
            && let Ok(byte) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
        {
            ergebnis.push(byte);
            i += 3;
            continue;
        }
        ergebnis.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&ergebnis).into_owned()
}
