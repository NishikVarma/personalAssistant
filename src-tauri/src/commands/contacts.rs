use tauri::State;

use crate::db::{contacts_repo, tags_repo};
use crate::error::AppResult;
use crate::models::contact::{Contact, ContactInput, Tag, TagInput};
use crate::state::AppState;

#[tauri::command]
pub async fn contact_list(
    state: State<'_, AppState>,
    search: Option<String>,
) -> AppResult<Vec<Contact>> {
    contacts_repo::list(&state.pool, search.as_deref().unwrap_or("")).await
}

#[tauri::command]
pub async fn contact_create(
    state: State<'_, AppState>,
    input: ContactInput,
) -> AppResult<Contact> {
    contacts_repo::create(&state.pool, &input).await
}

#[tauri::command]
pub async fn contact_update(
    state: State<'_, AppState>,
    id: i64,
    input: ContactInput,
) -> AppResult<Contact> {
    contacts_repo::update(&state.pool, id, &input).await
}

#[tauri::command]
pub async fn contact_set_last_contacted(
    state: State<'_, AppState>,
    id: i64,
    last_contacted_at: Option<String>,
) -> AppResult<()> {
    contacts_repo::set_last_contacted(&state.pool, id, last_contacted_at).await
}

#[tauri::command]
pub async fn contact_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    contacts_repo::delete(&state.pool, id).await
}

#[tauri::command]
pub async fn tag_list(state: State<'_, AppState>) -> AppResult<Vec<Tag>> {
    tags_repo::list(&state.pool).await
}

#[tauri::command]
pub async fn tag_create(state: State<'_, AppState>, input: TagInput) -> AppResult<Tag> {
    tags_repo::create(&state.pool, &input).await
}

#[tauri::command]
pub async fn tag_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    tags_repo::delete(&state.pool, id).await
}

#[tauri::command]
pub async fn contact_list_tags(
    state: State<'_, AppState>,
    contact_id: i64,
) -> AppResult<Vec<Tag>> {
    tags_repo::list_for_contact(&state.pool, contact_id).await
}

#[tauri::command]
pub async fn contact_replace_tags(
    state: State<'_, AppState>,
    contact_id: i64,
    tag_ids: Vec<i64>,
) -> AppResult<()> {
    tags_repo::replace_contact_tags(&state.pool, contact_id, &tag_ids).await
}
