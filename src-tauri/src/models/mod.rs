pub mod ai;
pub mod application;
pub mod contact;
pub mod email;
pub mod profile;

pub use ai::*;
pub use application::*;
pub use contact::*;
pub use email::*;
pub use profile::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub app_version: String,
    pub db_path: String,
    pub schema_version: i64,
}
