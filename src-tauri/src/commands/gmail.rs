use std::sync::Arc;

use tauri::State;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::db::settings_repo;
use crate::error::{AppError, AppResult};
use crate::gmail;
use crate::gmail::oauth;
use crate::llm::secrets::SecretStore;
use crate::state::AppState;

pub(crate) const CLIENT_ID_SETTING: &str = "google.client_id";
pub(crate) const CLIENT_SECRET_SERVICE: &str = "job-application-copilot";
pub(crate) const CLIENT_SECRET_ACCOUNT: &str = "google_client_secret";
pub(crate) const REFRESH_TOKEN_ACCOUNT: &str = "google_refresh_token";

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleStatus {
    pub connected: bool,
    pub account_email: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectStart {
    pub auth_url: String,
}

async fn stored_client_id(pool: &sqlx::SqlitePool) -> AppResult<String> {
    settings_repo::get(pool, CLIENT_ID_SETTING)
        .await?
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            AppError::InvalidInput(
                "no Google client ID configured — add one in Settings".to_string(),
            )
        })
}

fn stored_client_secret(secrets: &Arc<dyn SecretStore>) -> AppResult<String> {
    secrets
        .get(CLIENT_SECRET_SERVICE, CLIENT_SECRET_ACCOUNT)?
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            AppError::InvalidInput(
                "no Google client secret configured — add one in Settings".to_string(),
            )
        })
}

#[tauri::command]
pub async fn google_set_client_secret(
    state: State<'_, AppState>,
    secret: String,
) -> AppResult<()> {
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput(
            "client secret must not be empty".to_string(),
        ));
    }
    state
        .secrets
        .set(CLIENT_SECRET_SERVICE, CLIENT_SECRET_ACCOUNT, trimmed)
}

#[tauri::command]
pub async fn google_has_client_secret(state: State<'_, AppState>) -> AppResult<bool> {
    Ok(state
        .secrets
        .get(CLIENT_SECRET_SERVICE, CLIENT_SECRET_ACCOUNT)?
        .is_some())
}

/// Starts the OAuth flow: binds a loopback listener, returns the consent URL
/// for the frontend to open in the browser.
#[tauri::command]
pub async fn google_begin_connect(state: State<'_, AppState>) -> AppResult<ConnectStart> {
    let client_id = stored_client_id(&state.pool).await?;
    stored_client_secret(&state.secrets)?;

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}");
    let auth_url = oauth::build_auth_url(&client_id, &redirect_uri);

    let (sender, receiver) = oneshot::channel();
    state.oauth.store(gmail::PendingAuth { receiver, redirect_uri: redirect_uri.clone() });

    tauri::async_runtime::spawn(async move {
        gmail::wait_for_code(listener, sender).await;
    });

    Ok(ConnectStart { auth_url })
}

/// Awaits the browser redirect, exchanges the code and stores credentials.
#[tauri::command]
pub async fn google_complete_connect(state: State<'_, AppState>) -> AppResult<GoogleStatus> {
    let pending = state.oauth.take().ok_or_else(|| {
        AppError::InvalidInput("no connection attempt in progress".to_string())
    })?;
    let code = pending
        .receiver
        .await
        .map_err(|_| AppError::InvalidInput("authorization was cancelled or timed out".to_string()))?;

    let client_id = stored_client_id(&state.pool).await?;
    let client_secret = stored_client_secret(&state.secrets)?;

    let tokens =
        oauth::exchange_code(&client_id, &client_secret, &code, &pending.redirect_uri).await?;
    let refresh_token = tokens.refresh_token.clone().ok_or_else(|| {
        AppError::InvalidInput(
            "Google did not return a refresh token — revoke app access and reconnect".to_string(),
        )
    })?;
    let email = oauth::fetch_user_email(&tokens.access_token).await?;

    state
        .secrets
        .set(CLIENT_SECRET_SERVICE, REFRESH_TOKEN_ACCOUNT, &refresh_token)?;

    let ts = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO oauth_accounts
             (provider, account_email, scopes_json, keyring_service, keyring_account,
              connected_at, updated_at)
         VALUES ('google', ?1, ?2, ?3, ?4, ?5, ?5)
         ON CONFLICT(provider, account_email) DO UPDATE SET
             scopes_json = excluded.scopes_json,
             keyring_service = excluded.keyring_service,
             keyring_account = excluded.keyring_account,
             updated_at = excluded.updated_at",
    )
    .bind(&email)
    .bind(oauth::scopes())
    .bind(CLIENT_SECRET_SERVICE)
    .bind(REFRESH_TOKEN_ACCOUNT)
    .bind(&ts)
    .execute(&state.pool)
    .await?;

    Ok(GoogleStatus { connected: true, account_email: Some(email) })
}

#[tauri::command]
pub async fn google_status(state: State<'_, AppState>) -> AppResult<GoogleStatus> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT account_email FROM oauth_accounts WHERE provider = 'google' LIMIT 1")
            .fetch_optional(&state.pool)
            .await?;
    Ok(match row {
        Some((email,)) => GoogleStatus { connected: true, account_email: Some(email) },
        None => GoogleStatus { connected: false, account_email: None },
    })
}

#[tauri::command]
pub async fn google_disconnect(state: State<'_, AppState>) -> AppResult<bool> {
    let refresh = state.secrets.get(CLIENT_SECRET_SERVICE, REFRESH_TOKEN_ACCOUNT)?;
    if let Some(token) = refresh {
        let _ = oauth::revoke_token(&token).await;
    }
    state
        .secrets
        .delete(CLIENT_SECRET_SERVICE, REFRESH_TOKEN_ACCOUNT)?;
    let result = sqlx::query("DELETE FROM oauth_accounts WHERE provider = 'google'")
        .execute(&state.pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Returns a fresh access token using the stored refresh token.
pub(crate) async fn fresh_access_token(state: &AppState) -> AppResult<(String, String)> {
    let account: Option<(String,)> =
        sqlx::query_as("SELECT account_email FROM oauth_accounts WHERE provider = 'google' LIMIT 1")
            .fetch_optional(&state.pool)
            .await?;
    let account_email = account
        .map(|(e,)| e)
        .ok_or_else(|| AppError::InvalidInput("Gmail is not connected".to_string()))?;

    let refresh_token = state
        .secrets
        .get(CLIENT_SECRET_SERVICE, REFRESH_TOKEN_ACCOUNT)?
        .ok_or_else(|| AppError::InvalidInput("Gmail is not connected".to_string()))?;
    let client_id = stored_client_id(&state.pool).await?;
    let client_secret = stored_client_secret(&state.secrets)?;

    let tokens =
        oauth::refresh_access_token(&client_id, &client_secret, &refresh_token).await?;
    if tokens.access_token.is_empty() {
        return Err(AppError::InvalidInput(
            "Google returned an empty access token — try reconnecting".to_string(),
        ));
    }
    Ok((tokens.access_token, account_email))
}
