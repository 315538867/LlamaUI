mod commands;
pub mod error;
pub mod mcp;
pub mod proxy;
pub mod services;

use std::sync::Arc;
use tauri::Manager;

use commands::proxy::ProxyState;
use services::config_store::ConfigStore;
use services::instance_registry::InstanceRegistry;

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
            let instance_registry = Arc::new(InstanceRegistry::new());
            let proxy_state = Arc::new(ProxyState::new());

            // Start proxy at app launch with empty routes table
            let app_config = config_store.load_config();
            proxy_state.start_at_launch(
                app_config.proxy_port,
                app_config.proxy_cors,
                app_config.proxy_allow_external,
                app_config.proxy_api_key,
                app_config.proxy_responses_mode,
                app.handle().clone(),
            );

            app.manage(config_store);
            app.manage(instance_registry);
            app.manage(proxy_state);

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if window.label() != "main" {
                    return;
                }
                let registry = window.state::<Arc<InstanceRegistry>>();
                let ps = window.state::<Arc<ProxyState>>();
                tauri::async_runtime::spawn({
                    let registry = Arc::clone(&registry);
                    async move {
                        registry.stop_all().await;
                    }
                });
                ps.stop();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::instance::start_instance,
            commands::instance::stop_instance,
            commands::instance::get_all_instances,
            commands::instance::delete_instance_config,
            commands::instance::list_model_presets,
            commands::instance::save_model_preset,
            commands::instance::delete_model_preset,
            commands::models::scan_models,
            commands::models::scan_models_stream,
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
            commands::benchmark::run_benchmark,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
