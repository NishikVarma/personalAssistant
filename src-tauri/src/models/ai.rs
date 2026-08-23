use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfig {
    pub model: String,
    pub has_api_key: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTestResult {
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
    pub model: String,
}
