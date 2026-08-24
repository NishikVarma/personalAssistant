use tauri::State;

use crate::db::applications_repo;
use crate::error::AppResult;
use crate::models::application::{Application, ApplicationInput, ApplicationStatus};
use crate::state::AppState;

#[tauri::command]
pub async fn application_list(
    state: State<'_, AppState>,
    status: Option<ApplicationStatus>,
) -> AppResult<Vec<Application>> {
    applications_repo::list(&state.pool, status).await
}

#[tauri::command]
pub async fn application_create(
    state: State<'_, AppState>,
    input: ApplicationInput,
) -> AppResult<Application> {
    applications_repo::create(&state.pool, &input).await
}

#[tauri::command]
pub async fn application_update(
    state: State<'_, AppState>,
    id: i64,
    input: ApplicationInput,
) -> AppResult<Application> {
    applications_repo::update(&state.pool, id, &input).await
}

#[tauri::command]
pub async fn application_set_status(
    state: State<'_, AppState>,
    id: i64,
    status: ApplicationStatus,
) -> AppResult<Application> {
    let updated = applications_repo::set_status(&state.pool, id, status).await?;
    // closing an application suppresses its pending follow-ups
    if matches!(updated.status.as_str(), "rejected" | "withdrawn") {
        crate::db::follow_ups_repo::suppress_for(&state.pool, id, None, "application closed")
            .await?;
    }
    Ok(updated)
}

#[tauri::command]
pub async fn application_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    applications_repo::delete(&state.pool, id).await
}
