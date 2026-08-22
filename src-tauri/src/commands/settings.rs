use tauri::State;

use crate::db::{self, settings_repo};
use crate::error::AppResult;
use crate::models::AppInfo;
use crate::state::AppState;

#[tauri::command]
pub async fn get_setting(state: State<'_, AppState>, key: String) -> AppResult<Option<String>> {
    settings_repo::get(&state.pool, &key).await
}

#[tauri::command]
pub async fn set_setting(state: State<'_, AppState>, key: String, value: String) -> AppResult<()> {
    if key.trim().is_empty() {
        return Err(crate::error::AppError::InvalidInput(
            "setting key must not be empty".to_string(),
        ));
    }
    settings_repo::set(&state.pool, &key, &value).await
}

#[tauri::command]
pub async fn delete_setting(state: State<'_, AppState>, key: String) -> AppResult<bool> {
    settings_repo::delete(&state.pool, &key).await
}

#[tauri::command]
pub async fn get_app_info(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<AppInfo> {
    let schema_version = db::schema_version(&state.pool).await?;
    Ok(AppInfo {
        app_version: app.package_info().version.to_string(),
        db_path: state.db_path.clone(),
        schema_version,
    })
}
