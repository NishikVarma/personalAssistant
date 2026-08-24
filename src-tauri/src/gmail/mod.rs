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
