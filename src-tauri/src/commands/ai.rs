use std::time::Instant;

use tauri::State;

use crate::db::settings_repo;
use crate::error::{AppError, AppResult};
use crate::llm::{GeminiProvider, LlmProvider, DEFAULT_MODEL};
use crate::models::ai::{AiConfig, AiTestResult};
use crate::state::AppState;

pub(crate) const KEY_SERVICE: &str = "job-application-copilot";
pub(crate) const KEY_ACCOUNT: &str = "gemini_api_key";
pub(crate) const MODEL_SETTING: &str = "ai.model";

async fn stored_model(pool: &sqlx::SqlitePool) -> AppResult<String> {
    Ok(settings_repo::get(pool, MODEL_SETTING)
        .await?
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string()))
}

async fn current_provider(state: &AppState) -> AppResult<GeminiProvider> {
    let api_key = state
        .secrets
        .get(KEY_SERVICE, KEY_ACCOUNT)?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| {
            AppError::InvalidInput(
                "no Gemini API key configured — add one in Settings".to_string(),
            )
        })?;
    let model = stored_model(&state.pool).await?;
    Ok(GeminiProvider::with_model(api_key, model))
}

#[tauri::command]
pub async fn ai_get_config(state: State<'_, AppState>) -> AppResult<AiConfig> {
    let has_api_key = state.secrets.get(KEY_SERVICE, KEY_ACCOUNT)?.is_some();
    let model = stored_model(&state.pool).await?;
    Ok(AiConfig { model, has_api_key })
}

#[tauri::command]
pub async fn ai_set_model(state: State<'_, AppState>, model: String) -> AppResult<()> {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput("model must not be empty".to_string()));
    }
    settings_repo::set(&state.pool, MODEL_SETTING, trimmed).await
}

#[tauri::command]
pub async fn ai_set_api_key(state: State<'_, AppState>, api_key: String) -> AppResult<()> {
    let trimmed = api_key.trim();
    if trimmed.len() < 20 {
        return Err(AppError::InvalidInput(
            "API key looks too short to be valid".to_string(),
        ));
    }
    state.secrets.set(KEY_SERVICE, KEY_ACCOUNT, trimmed)
}

#[tauri::command]
pub async fn ai_clear_api_key(state: State<'_, AppState>) -> AppResult<bool> {
    state.secrets.delete(KEY_SERVICE, KEY_ACCOUNT)
}

#[tauri::command]
pub async fn ai_test_connection(state: State<'_, AppState>) -> AppResult<AiTestResult> {
    let model = stored_model(&state.pool).await?;
    let started = Instant::now();
    let outcome = match current_provider(&state).await {
        Err(e) => Err(e.to_string()),
        Ok(provider) => provider
            .complete("Reply with exactly: OK".to_string())
            .await
            .map(|_| provider.model().to_string())
            .map_err(|e| e.to_string()),
    };
    let latency_ms = Some(started.elapsed().as_millis() as u64);
    Ok(match outcome {
        Ok(used_model) => AiTestResult {
            ok: true,
            latency_ms,
            error: None,
            model: used_model,
        },
        Err(error) => AiTestResult {
            ok: false,
            latency_ms,
            error: Some(error),
            model,
        },
    })
}
