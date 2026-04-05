use tauri::{AppHandle, State};
use std::sync::Arc;

use crate::services::process_manager::{ProcessInfo, ProcessManager};
use crate::services::config_store::{ConfigStore, LaunchConfig};
use crate::commands::proxy::ProxyState;

#[tauri::command]
pub async fn start_llama(
    app: AppHandle,
    config: LaunchConfig,
    process_manager: State<'_, Arc<ProcessManager>>,
    config_store: State<'_, Arc<ConfigStore>>,
    proxy_state: State<'_, Arc<ProxyState>>,
) -> Result<(), String> {
    let app_config = config_store.load_config();
    let llama_dir = app_config.llama_dir
        .ok_or("未配置 llama.cpp 路径，请先在设置中配置")?;

    process_manager.start(app, &llama_dir, &config).await?;

    // 将实际分配的端口同步给 Proxy
    let info = process_manager.get_info().await;
    if let Some(port) = info.port {
        let host = config.host.as_deref().unwrap_or("127.0.0.1");
        proxy_state.update_llama_target(host, port);
    }

    Ok(())
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
