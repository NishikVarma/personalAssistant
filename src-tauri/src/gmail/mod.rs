pub mod mime;
pub mod oauth;

use std::time::Duration;

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::error::{AppError, AppResult};

/// A pending loopback redirect: the code receiver plus the redirect URI that
/// must be echoed back during the token exchange.
pub struct PendingAuth {
    pub receiver: oneshot::Receiver<String>,
    pub redirect_uri: String,
}

/// Holds the in-flight loopback OAuth redirect.
#[derive(Default)]
pub struct OauthCoordinator {
    slot: tokio::sync::Mutex<Option<PendingAuth>>,
}

impl OauthCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn store(&self, pending: PendingAuth) {
        if let Ok(mut slot) = self.slot.try_lock() {
            *slot = Some(pending);
        }
    }

    pub fn take(&self) -> Option<PendingAuth> {
        self.slot.try_lock().ok().and_then(|mut slot| slot.take())
    }
}

/// Waits for the Google redirect on the loopback listener (max 5 minutes),
/// answering each request; resolves with the authorization code.
pub async fn wait_for_code(listener: TcpListener, sender: oneshot::Sender<String>) {
    let _ = tokio::time::timeout(Duration::from_secs(300), async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            if let Some(code) = handle_connection(&mut stream).await {
                let _ = sender.send(code);
                return;
            }
        }
    })
    .await;
}

async fn handle_connection(stream: &mut tokio::net::TcpStream) -> Option<String> {
    let mut buffer = [0u8; 4096];
    let read = stream.read(&mut buffer).await.ok()?;
    let request = String::from_utf8_lossy(&buffer[..read]).to_string();
    let request_line = request.lines().next()?.to_string();

    let body = if request_line.contains("code=") {
        "<html><body><h3>Email authorized.</h3><p>You can close this window and return to the app.</p></body></html>"
    } else {
        "<html><body><p>Waiting for authorization…</p></body></html>"
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await.ok()?;

    oauth::extract_code_from_request_line(&request_line)
}

/// Sends a raw MIME message through the Gmail API; returns (message id, thread id).
pub async fn send_message(
    access_token: &str,
    raw_base64url: &str,
) -> AppResult<(String, Option<String>)> {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?
        .post("https://gmail.googleapis.com/gmail/v1/users/me/messages/send")
        .bearer_auth(access_token)
        .json(&json!({ "raw": raw_base64url }))
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(AppError::InvalidInput(format!(
            "Gmail send failed (HTTP {status}): {body}"
        )));
    }
    let parsed: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| AppError::InvalidInput(format!("could not parse Gmail response: {e}")))?;
    let id = parsed
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Gmail response missing message id".to_string()))?
        .to_string();
    let thread_id = parsed
        .get("threadId")
        .and_then(|v| v.as_str())
        .map(String::from);
    Ok((id, thread_id))
}

/// A detected reply to one of our sent emails, ready for history recording.
pub struct ParsedReply {
    pub gmail_message_id: String,
    pub from_email: String,
    pub subject: Option<String>,
    pub snippet: Option<String>,
    pub occurred_at: String,
}

pub struct ThreadMessage {
    pub id: String,
    pub from_email: Option<String>,
    pub subject: Option<String>,
    pub snippet: Option<String>,
    pub internal_date_ms: Option<i64>,
}

/// Extracts the bare address from a From header like `"Jane" <jane@acme.com>`.
pub fn extract_email_address(from_header: &str) -> Option<String> {
    let trimmed = from_header.trim();
    if let Some(open) = trimmed.rfind('<') {
        let close = trimmed[open..].find('>')? + open;
        return Some(trimmed[open + 1..close].trim().to_string());
    }
    if trimmed.contains('@') {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn header_value(message: &serde_json::Value, name: &str) -> Option<String> {
    message
        .pointer("/payload/headers")
        .and_then(|headers| headers.as_array())
        .and_then(|headers| {
            headers
                .iter()
                .find(|h| {
                    h.get("name").and_then(|n| n.as_str()).map(|n| n.eq_ignore_ascii_case(name)).unwrap_or(false)
                })
                .and_then(|h| h.get("value"))
                .and_then(|v| v.as_str())
                .map(String::from)
        })
}

/// Parses a Gmail threads.get (format=metadata) response body.
pub fn parse_thread_messages(body: &serde_json::Value) -> Vec<ThreadMessage> {
    let mut out = Vec::new();
    if let Some(messages) = body.get("messages").and_then(|m| m.as_array()) {
        for message in messages {
            let id = match message.get("id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => continue,
            };
            let internal_date_ms = message
                .get("internalDate")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok());
            out.push(ThreadMessage {
                id,
                from_email: header_value(message, "From").and_then(|h| extract_email_address(&h)),
                subject: header_value(message, "Subject"),
                snippet: message
                    .get("snippet")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                internal_date_ms,
            });
        }
    }
    out
}

/// Finds the first message in a thread that is a genuine reply: not ours,
/// from a different sender, newer than our sent message.
pub fn find_reply(
    messages: &[ThreadMessage],
    own_email: &str,
    own_message_id: Option<&str>,
    sent_at_ms: i64,
) -> Option<ParsedReply> {
    let mut candidate: Option<&ThreadMessage> = None;
    for message in messages {
        if Some(message.id.as_str()) == own_message_id {
            continue;
        }
        match &message.from_email {
            Some(email) if !email.eq_ignore_ascii_case(own_email) => {}
            _ => continue,
        }
        if message.internal_date_ms.unwrap_or(0) <= sent_at_ms {
            continue;
        }
        if candidate
            .map(|c| c.internal_date_ms.unwrap_or(0) < message.internal_date_ms.unwrap_or(0))
            .unwrap_or(true)
        {
            candidate = Some(message);
        }
    }
    candidate.map(|m| ParsedReply {
        gmail_message_id: m.id.clone(),
        from_email: m.from_email.clone().unwrap_or_default(),
        subject: m.subject.clone(),
        snippet: m.snippet.clone(),
        occurred_at: internal_date_to_rfc3339(m.internal_date_ms),
    })
}

fn internal_date_to_rfc3339(ms: Option<i64>) -> String {
    ms.and_then(chrono::DateTime::from_timestamp_millis)
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339()
}

/// Fetches a thread (metadata only: headers + snippet, no bodies).
pub async fn fetch_thread_metadata(
    access_token: &str,
    thread_id: &str,
) -> AppResult<serde_json::Value> {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?
        .get(format!(
            "https://gmail.googleapis.com/gmail/v1/users/me/threads/{thread_id}"
        ))
        .bearer_auth(access_token)
        .query(&[("format", "metadata")])
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(AppError::InvalidInput(format!(
            "Gmail thread fetch failed (HTTP {status})"
        )));
    }
    serde_json::from_str(&body)
        .map_err(|e| AppError::InvalidInput(format!("could not parse Gmail thread: {e}")))
}
