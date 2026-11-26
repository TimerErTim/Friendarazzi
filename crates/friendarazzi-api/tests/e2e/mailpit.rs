//! Read a confirmation or recovery token out of Mailpit's HTTP API.
//!
//! The e2e suite goes through a real SMTP relay for AC-19 — that is the point of the case:
//! the mail must survive the adapter, the transport, and a real server. Everything else uses
//! the in-memory fake, because a suite where every case needs a daemon is a suite that gets
//! skipped.
//!
//! A missing Mailpit is a **failure with instructions**, never a skip: a silently skipped
//! AC-19 is the exact failure this module exists to prevent.

use std::time::Duration;

use serde_json::Value;

/// Where Mailpit listens, hardcoded to the port `[daemons.mailpit]` in `mise.toml` binds.
///
/// The port is the daemon's, so an env variable here would only add a way to point the
/// suite at a Mailpit the task did not start.
fn basis_url() -> String {
    "http://127.0.0.1:8034".to_owned()
}

/// Whether Mailpit answers at all.
pub async fn erreichbar() -> bool {
    reqwest::Client::new()
        .get(format!("{}/api/v1/info", basis_url()))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .map(|resp| resp.status().is_success())
        .unwrap_or(false)
}

/// Panic with an actionable message when Mailpit is not running.
pub async fn verlange_mailpit() {
    assert!(
        erreichbar().await,
        "Mailpit ist unter {} nicht erreichbar. Starte ihn mit `mise daemons start mailpit`.",
        basis_url()
    );
}

/// Every message in the mailbox, oldest first.
pub async fn nachrichten() -> Vec<Value> {
    let antwort = reqwest::Client::new()
        .get(format!("{}/api/v1/messages", basis_url()))
        .send()
        .await
        .unwrap_or_else(|fehler| {
            panic!(
                "Mailpit ist unter {} nicht erreichbar ({fehler}). Starte ihn mit \
                 `mise daemons start mailpit`.",
                basis_url()
            )
        })
        .json::<Value>()
        .await
        .expect("Mailpit antwortet mit JSON");

    let mut messages = antwort["messages"].as_array().cloned().unwrap_or_default();
    messages.reverse();
    messages
}

/// Message details returned by Mailpit's REST API, including plain text, HTML, and headers.
#[derive(Debug, Clone)]
pub struct MailpitNachricht {
    pub subject: String,
    pub text: String,
    pub html: String,
    pub headers: Value,
}

impl MailpitNachricht {
    pub fn header_first(&self, name: &str) -> Option<&str> {
        self.headers
            .as_object()?
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .and_then(|(_, v)| v.as_array()?.first()?.as_str())
    }
}

fn hat_empfänger(nachricht: &Value, empfänger: &str) -> bool {
    nachricht["To"]
        .as_array()
        .map(|empfänger_liste| {
            empfänger_liste.iter().any(|eintrag| {
                eintrag["Address"]
                    .as_str()
                    .map(|adresse| adresse.eq_ignore_ascii_case(empfänger))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

async fn lade_nachricht(id: &str) -> MailpitNachricht {
    let client = reqwest::Client::new();
    let detail = client
        .get(format!("{}/api/v1/message/{id}", basis_url()))
        .send()
        .await
        .expect("Mailpit liefert die Nachricht")
        .json::<Value>()
        .await
        .expect("Mailpit antwortet mit JSON");

    let headers = client
        .get(format!("{}/api/v1/message/{id}/headers", basis_url()))
        .send()
        .await
        .expect("Mailpit liefert die Header")
        .json::<Value>()
        .await
        .expect("Mailpit antwortet mit Header-JSON");

    MailpitNachricht {
        subject: detail["Subject"].as_str().unwrap_or_default().to_owned(),
        text: detail["Text"].as_str().unwrap_or_default().to_owned(),
        html: detail["HTML"].as_str().unwrap_or_default().to_owned(),
        headers,
    }
}

/// Every message addressed to `empfänger`, oldest first, with decoded plain-text, HTML, and headers.
pub async fn nachrichten_an(empfänger: &str) -> Vec<MailpitNachricht> {
    let all = nachrichten().await;
    let mut out = Vec::new();
    for msg in all.iter().filter(|m| hat_empfänger(m, empfänger)) {
        let id = msg["ID"].as_str().expect("jede Nachricht hat eine ID");
        out.push(lade_nachricht(id).await);
    }
    out
}

/// Full details of the newest message addressed to `empfänger`.
pub async fn letzte_nachricht_an(empfänger: &str) -> MailpitNachricht {
    let msgs = nachrichten_an(empfänger).await;
    msgs.into_iter().last().unwrap_or_else(|| {
        panic!("keine Mail an {empfänger} in Mailpit");
    })
}

/// The body of the newest message addressed to `empfänger`, decoded from Mailpit's
/// base64-free `Text` field.
pub async fn letzter_text_an(empfänger: &str) -> String {
    let msg = letzte_nachricht_an(empfänger).await;
    if msg.text.is_empty() {
        msg.html
    } else {
        msg.text
    }
}

/// Forget every message, so a case asserts on its own mail only.
pub async fn leeren() {
    reqwest::Client::new()
        .delete(format!("{}/api/v1/messages", basis_url()))
        .send()
        .await
        .expect("Mailpit leert die Mailbox");
}

pub use super::token_aus;
