use std::sync::Arc;

use crate::gmail::OauthCoordinator;
use crate::llm::secrets::SecretStore;

pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub db_path: String,
    pub secrets: Arc<dyn SecretStore>,
    pub oauth: Arc<OauthCoordinator>,
    pub resumes_dir: std::path::PathBuf,
}
