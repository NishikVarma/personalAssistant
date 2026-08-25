pub mod commands;
pub mod db;
pub mod error;
pub mod gmail;
pub mod llm;
pub mod models;
pub mod state;
use tauri::Manager;

use crate::gmail::OauthCoordinator;
use crate::llm::secrets::KeyringStore;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let (pool, db_path) = tauri::async_runtime::block_on(db::init(data_dir.clone()))?;
            let resumes_dir = data_dir.join("resumes");
            std::fs::create_dir_all(&resumes_dir)?;
            app.manage(AppState {
                pool,
                db_path,
                secrets: std::sync::Arc::new(KeyringStore::new()),
                oauth: std::sync::Arc::new(OauthCoordinator::new()),
                resumes_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::delete_setting,
            commands::settings::get_app_info,
            commands::profile::profile_get,
            commands::profile::profile_update,
            commands::profile::profile_set_verified,
            commands::profile::education_list,
            commands::profile::education_create,
            commands::profile::education_update,
            commands::profile::education_set_verified,
            commands::profile::education_delete,
            commands::profile::experience_list,
            commands::profile::experience_create,
            commands::profile::experience_update,
            commands::profile::experience_set_verified,
            commands::profile::experience_delete,
            commands::profile::project_list,
            commands::profile::project_create,
            commands::profile::project_update,
            commands::profile::project_set_verified,
            commands::profile::project_delete,
            commands::profile::skill_list,
            commands::profile::skill_create,
            commands::profile::skill_update,
            commands::profile::skill_delete,
            commands::profile::skill_list_for_entity,
            commands::profile::skill_replace_for_entity,
            commands::profile::bullet_list_for_entity,
            commands::profile::bullet_create,
            commands::profile::bullet_update,
            commands::profile::bullet_set_verified,
            commands::profile::bullet_delete,
            commands::profile::certification_list,
            commands::profile::certification_create,
            commands::profile::certification_update,
            commands::profile::certification_set_verified,
            commands::profile::certification_delete,
            commands::profile::achievement_list,
            commands::profile::achievement_create,
            commands::profile::achievement_update,
            commands::profile::achievement_set_verified,
            commands::profile::achievement_delete,
            commands::profile::link_list,
            commands::profile::link_create,
            commands::profile::link_update,
            commands::profile::link_delete,
            commands::contacts::contact_list,
            commands::contacts::contact_create,
            commands::contacts::contact_update,
            commands::contacts::contact_set_last_contacted,
            commands::contacts::contact_delete,
            commands::contacts::tag_list,
            commands::contacts::tag_create,
            commands::contacts::tag_delete,
            commands::contacts::contact_list_tags,
            commands::contacts::contact_replace_tags,
            commands::applications::application_list,
            commands::applications::application_create,
            commands::applications::application_update,
            commands::applications::application_set_status,
            commands::applications::application_delete,
            commands::ai::ai_get_config,
            commands::ai::ai_set_model,
            commands::ai::ai_set_api_key,
            commands::ai::ai_clear_api_key,
            commands::ai::ai_test_connection,
            commands::emails::generated_email_list,
            commands::emails::generated_email_get,
            commands::emails::generated_email_create,
            commands::emails::generated_email_update,
            commands::emails::generated_email_set_status,
            commands::emails::generated_email_delete,
            commands::emails::ai_generate_email,
            commands::emails::ai_extract_contact,
            commands::emails::email_send,
            commands::emails::email_history_list,
            commands::emails::email_history_set_response,
            commands::emails::email_history_record_incoming,
            commands::emails::email_template_list,
            commands::emails::email_template_create,
            commands::emails::email_template_update,
            commands::emails::email_template_delete,
            commands::emails::email_template_save_from_email,
            commands::gmail::google_set_client_secret,
            commands::gmail::google_has_client_secret,
            commands::gmail::google_begin_connect,
            commands::gmail::google_complete_connect,
            commands::gmail::google_status,
            commands::gmail::google_disconnect,
            commands::gmail::gmail_sync_replies,
            commands::follow_up::follow_up_list,
            commands::follow_up::follow_up_due,
            commands::follow_up::follow_up_due_count,
            commands::follow_up::follow_up_sweep,
            commands::follow_up::follow_up_reschedule,
            commands::follow_up::follow_up_cancel,
            commands::follow_up::follow_up_config_get,
            commands::follow_up::follow_up_config_set,
            commands::follow_up::follow_up_draft,
            commands::bulk::bulk_import_preview,
            commands::bulk::bulk_batch_create,
            commands::bulk::bulk_batch_list,
            commands::bulk::bulk_batch_get,
            commands::bulk::bulk_generate,
            commands::bulk::bulk_batch_remove_draft,
            commands::bulk::bulk_batch_finish,
            commands::resumes::resume_file_upload,
            commands::resumes::resume_file_list,
            commands::resumes::resume_file_delete,
            commands::resumes::resume_file_tex_content,
            commands::resumes::latex_detect,
            commands::resumes::resume_extract_profile,
            commands::resumes::resume_extract_from_text,
            commands::resumes::profile_import_extracted,
            commands::resumes::resume_match_jd,
            commands::resumes::resume_generate_variant,
            commands::resumes::resume_variant_list,
            commands::resumes::resume_variant_tex_content,
            commands::resumes::resume_variant_approve,
            commands::resumes::resume_variant_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
