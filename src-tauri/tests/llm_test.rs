use assistant_lib::db::{self, settings_repo};
use assistant_lib::llm::{
    secrets::{MemoryStore, SecretStore},
    build_request_body, extract_error_message, extract_text, GeminiProvider, LlmProvider,
    DEFAULT_MODEL,
};
use serde_json::json;
use sqlx::SqlitePool;

async fn test_pool() -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let pool = db::open(dir.path().join("test.db")).await.expect("pool");
    (pool, dir)
}

const SERVICE: &str = "job-application-copilot";
const ACCOUNT: &str = "gemini_api_key";

#[test]
fn request_body_shape() {
    let body = build_request_body("hello world");
    assert_eq!(
        body,
        json!({ "contents": [{ "parts": [{ "text": "hello world" }] }] })
    );
}

#[test]
fn extracts_and_concatenates_text_parts() {
    let response = json!({
        "candidates": [{
            "content": { "parts": [{ "text": "Hello" }, { "text": "world" }] },
            "finishReason": "STOP"
        }]
    });
    assert_eq!(extract_text(&response).unwrap(), "Hello\nworld");
}

#[test]
fn empty_candidates_surface_finish_reason() {
    let response = json!({
        "candidates": [{ "content": { "parts": [] }, "finishReason": "SAFETY" }]
    });
    let err = extract_text(&response).unwrap_err().to_string();
    assert!(err.contains("SAFETY"), "error should mention finish reason: {err}");
}

#[test]
fn error_messages_are_extracted_from_payloads() {
    let body = r#"{"error": {"code": 400, "message": "API key not valid. Please pass a valid API key."}}"#;
    assert_eq!(
        extract_error_message(400, body),
        "API key not valid. Please pass a valid API key."
    );
    assert_eq!(extract_error_message(502, "<html>bad gateway</html>"), "Gemini API returned HTTP 502");
}

#[test]
fn provider_normalizes_model_names() {
    assert_eq!(GeminiProvider::new("key").model(), DEFAULT_MODEL);
    assert_eq!(GeminiProvider::with_model("k", " gemini-2.5-pro ").model(), "gemini-2.5-pro");
    assert_eq!(GeminiProvider::with_model("k", "   ").model(), DEFAULT_MODEL);
}

#[test]
fn memory_store_roundtrip() {
    let store = MemoryStore::new();
    assert!(store.get(SERVICE, ACCOUNT).unwrap().is_none());
    assert!(!store.delete(SERVICE, ACCOUNT).unwrap());

    store.set(SERVICE, ACCOUNT, "abc123").unwrap();
    assert_eq!(store.get(SERVICE, ACCOUNT).unwrap().as_deref(), Some("abc123"));
    assert!(store.delete(SERVICE, ACCOUNT).unwrap());
    assert!(store.get(SERVICE, ACCOUNT).unwrap().is_none());
}

/// Live round-trip against the real Gemini API.
/// Run manually with: GEMINI_API_KEY=... cargo test -- --ignored
#[tokio::test]
#[ignore = "requires network + GEMINI_API_KEY"]
async fn live_gemini_call() {
    let api_key =
        std::env::var("GEMINI_API_KEY").expect("set GEMINI_API_KEY to run the live test");
    let provider = GeminiProvider::with_model(api_key, DEFAULT_MODEL);
    let text = provider.complete("Reply with exactly: OK".to_string()).await.unwrap();
    assert!(!text.trim().is_empty());
}

#[tokio::test]
async fn model_setting_roundtrips_through_settings_table() {
    let (pool, _dir) = test_pool().await;
    assert!(settings_repo::get(&pool, "ai.model").await.unwrap().is_none());
    settings_repo::set(&pool, "ai.model", "gemini-2.5-pro").await.unwrap();
    assert_eq!(
        settings_repo::get(&pool, "ai.model").await.unwrap().as_deref(),
        Some("gemini-2.5-pro")
    );
}
