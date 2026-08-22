use async_trait::async_trait;

use crate::error::AppResult;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, prompt: String) -> AppResult<String>;
}

pub struct GeminiProvider {
    api_key: String,
    model: String,
}

impl GeminiProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "gemini-2.5-flash".to_string(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn complete(&self, _prompt: String) -> AppResult<String> {
        let _ = (&self.api_key, &self.model);
        Err(crate::error::AppError::NotImplemented(
            "Gemini HTTP client lands in Phase 6".to_string(),
        ))
    }
}
