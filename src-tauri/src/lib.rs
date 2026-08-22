mod commands;
pub mod db;
pub mod error;
pub mod llm;
pub mod models;
mod state;

use tauri::Manager;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let (pool, db_path) = tauri::async_runtime::block_on(db::init(data_dir))?;
            app.manage(AppState { pool, db_path });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::delete_setting,
            commands::settings::get_app_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
