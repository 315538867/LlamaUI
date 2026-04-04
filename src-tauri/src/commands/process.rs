use tauri::{AppHandle, State};
use std::sync::Arc;

use crate::services::process_manager::{ProcessInfo, ProcessManager};
use crate::services::config_store::{ConfigStore, LaunchConfig};

#[tauri::command]
pub async fn start_llama(
    app: AppHandle,
    config: LaunchConfig,
    process_manager: State<'_, Arc<ProcessManager>>,
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<(), String> {
    let app_config = config_store.load_config();
    let llama_dir = app_config.llama_dir
        .ok_or("未配置 llama.cpp 路径，请先在设置中配置")?;

    process_manager.start(app, &llama_dir, &config).await
}

#[tauri::command]
pub async fn stop_llama(
    process_manager: State<'_, Arc<ProcessManager>>,
) -> Result<(), String> {
    process_manager.stop().await
}

#[tauri::command]
pub async fn get_llama_status(
    process_manager: State<'_, Arc<ProcessManager>>,
) -> Result<ProcessInfo, String> {
    Ok(process_manager.get_info().await)
}
