use std::time::Duration;

use serde::Deserialize;

use crate::error::{AppError, AppResult};

pub const GMAIL_SEND_SCOPE: &str = "https://www.googleapis.com/auth/gmail.send";
const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const REVOKE_ENDPOINT: &str = "https://oauth2.googleapis.com/revoke";
const USERINFO_ENDPOINT: &str = "https://openidconnect.googleapis.com/v1/userinfo";

pub fn scopes() -> String {
    format!("openid email {GMAIL_SEND_SCOPE}")
}

/// Builds the Google OAuth consent URL for the loopback desktop flow.
pub fn build_auth_url(client_id: &str, redirect_uri: &str) -> String {
    format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
        AUTH_ENDPOINT,
        urlencode(client_id),
        urlencode(redirect_uri),
        urlencode(&scopes()),
    )
}

fn urlencode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Extracts the `code` query parameter from a loopback redirect request line
/// such as `GET /?code=abc&scope=... HTTP/1.1`.
pub fn extract_code_from_request_line(line: &str) -> Option<String> {
    let path = line.split_whitespace().nth(1)?;
    let query = path.split('?').nth(1)?;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=')?;
        if key == "code" {
            return Some(value.to_string());
        }
    }
    None
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub expires_in: Option<i64>,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("reqwest client")
}

pub async fn exchange_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> AppResult<TokenResponse> {
    let response = http_client()
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await?;
    parse_token_response(response).await
}

pub async fn refresh_access_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> AppResult<TokenResponse> {
    let response = http_client()
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;
    parse_token_response(response).await
}

async fn parse_token_response(response: reqwest::Response) -> AppResult<TokenResponse> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(AppError::InvalidInput(format!(
            "Google token endpoint returned HTTP {status}: {body}"
        )));
    }
    let parsed: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| AppError::InvalidInput(format!("could not parse token response: {e}")))?;
    Ok(parsed)
}

pub async fn fetch_user_email(access_token: &str) -> AppResult<String> {
    let response = http_client()
        .get(USERINFO_ENDPOINT)
        .bearer_auth(access_token)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(AppError::InvalidInput(format!(
            "could not read Google account info (HTTP {})",
            response.status()
        )));
    }
    let payload: serde_json::Value = response.json().await?;
    payload
        .get("email")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| AppError::InvalidInput("Google account has no email".to_string()))
}

pub async fn revoke_token(token: &str) -> AppResult<()> {
    let response = http_client()
        .post(REVOKE_ENDPOINT)
        .form(&[("token", token)])
        .send()
        .await?;
    if !response.status().is_success() {
        // revocation failures are non-fatal; local credentials are removed regardless
        return Ok(());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_url_contains_required_params() {
        let url = build_auth_url("my-client.apps.googleusercontent.com", "http://127.0.0.1:54321");
        assert!(url.starts_with(AUTH_ENDPOINT));
        assert!(url.contains("client_id=my-client.apps.googleusercontent.com"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A54321"));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("prompt=consent"));
        assert!(url.contains(urlencode(GMAIL_SEND_SCOPE).as_str()));
    }

    #[test]
    fn extracts_code_from_loopback_request() {
        let line = "GET /?code=4%2F0abc&scope=email HTTP/1.1";
        assert_eq!(extract_code_from_request_line(line).as_deref(), Some("4%2F0abc"));
        assert!(extract_code_from_request_line("GET /favicon.ico HTTP/1.1").is_none());
        assert!(extract_code_from_request_line("garbage").is_none());
    }
}
