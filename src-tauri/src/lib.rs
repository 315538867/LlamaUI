mod commands;
pub mod mcp;
pub mod proxy;
pub mod services;

use std::sync::Arc;
use tauri::Manager;

use commands::proxy::ProxyState;
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

            // 从配置读取 llama.cpp 地址，构建 Proxy 目标 URL
            let app_config = config_store.load_config();
            let llama_host = app_config.default_params.host
                .unwrap_or_else(|| "127.0.0.1".into());
            let llama_port = app_config.default_params.port.unwrap_or(8000);
            let proxy_target = format!("http://{}:{}", llama_host, llama_port);

            let proxy_state = Arc::new(ProxyState::new());
            proxy_state.auto_start(
                app_config.proxy_port,
                proxy_target,
                app_config.proxy_cors,
                app_config.proxy_allow_external,
            );

            app.manage(config_store);
            app.manage(process_manager);
            app.manage(proxy_state);

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if window.label() != "main" {
                    return;
                }
                let pm = window.state::<Arc<ProcessManager>>();
                let pm_clone = Arc::clone(&pm);
                tauri::async_runtime::spawn(async move {
                    pm_clone.stop().await.ok();
                });
            }
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
            commands::proxy::restart_proxy,
            commands::proxy::get_proxy_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
