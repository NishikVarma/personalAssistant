pub mod email_prompt;
pub mod extract_prompt;
pub mod jd_prompt;
pub mod secrets;

use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};

const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
pub const DEFAULT_MODEL: &str = "gemini-2.5-flash";

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, prompt: String) -> AppResult<String>;
    fn model(&self) -> &str;
}

pub struct GeminiProvider {
    api_key: String,
    model: String,
    http: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_model(api_key, DEFAULT_MODEL)
    }

    pub fn with_model(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("reqwest client");
        Self {
            api_key: api_key.into(),
            model: normalize_model(model),
            http,
        }
    }
}

fn normalize_model(model: impl Into<String>) -> String {
    let trimmed = model.into().trim().to_string();
    if trimmed.is_empty() {
        DEFAULT_MODEL.to_string()
    } else {
        trimmed
    }
}

/// Builds the generateContent request body (pure function, unit-tested).
pub fn build_request_body(prompt: &str) -> Value {
    json!({
        "contents": [{ "parts": [{ "text": prompt }] }]
    })
}

/// Extracts the concatenated text parts from a successful generateContent
/// response (pure function, unit-tested).
pub fn extract_text(response: &Value) -> AppResult<String> {
    let mut text = String::new();
    if let Some(parts) = response
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
    {
        for part in parts {
            if let Some(piece) = part.get("text").and_then(Value::as_str) {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(piece);
            }
        }
    }
    if text.trim().is_empty() {
        let finish = response
            .pointer("/candidates/0/finishReason")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        return Err(AppError::InvalidInput(format!(
            "Gemini returned no content (finish reason: {finish})"
        )));
    }
    Ok(text)
}

/// Extracts a human-readable message from a Gemini error payload.
pub fn extract_error_message(status: u16, body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.pointer("/error/message").and_then(Value::as_str).map(String::from))
        .unwrap_or_else(|| format!("Gemini API returned HTTP {status}"))
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn complete(&self, prompt: String) -> AppResult<String> {
        let url = format!("{API_BASE}/models/{}:generateContent", self.model);
        let response = self
            .http
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .json(&build_request_body(&prompt))
            .send()
            .await?;

        let status = response.status().as_u16();
        let body = response.text().await?;
        if !(200..300).contains(&status) {
            return Err(AppError::InvalidInput(extract_error_message(
                status,
                &body,
            )));
        }

        let parsed: Value =
            serde_json::from_str(&body).map_err(|e| AppError::InvalidInput(format!(
                "could not parse Gemini response: {e}"
            )))?;
        extract_text(&parsed)
    }

    fn model(&self) -> &str {
        &self.model
    }
}
