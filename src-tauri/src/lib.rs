mod commands;
pub mod mcp;
pub mod services;

use std::sync::Arc;
use tauri::Manager;

use services::config_store::ConfigStore;
use services::process_manager::ProcessManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).ok();

            let config_store = Arc::new(ConfigStore::new(&app_data_dir));
            let process_manager = Arc::new(ProcessManager::new());

            app.manage(config_store);
            app.manage(process_manager);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::process::start_llama,
            commands::process::stop_llama,
            commands::process::get_llama_status,
            commands::models::scan_models,
            commands::models::get_model_info,
            commands::config::get_config,
            commands::config::save_config,
            commands::config::list_presets,
            commands::config::save_preset,
            commands::config::load_preset,
            commands::config::delete_preset,
            commands::config::detect_llama,
            commands::config::validate_llama_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
