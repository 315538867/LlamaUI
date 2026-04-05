use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, State};

use crate::commands::proxy::ProxyState;
use crate::services::config_store::{ConfigStore, InstanceConfig, Preset};
use crate::services::instance_registry::{InstanceInfo, InstanceRegistry};

/// Start a named model instance.
/// On success, registers the instance port in the proxy routing table.
#[tauri::command]
pub async fn start_instance(
    app: AppHandle,
    config: InstanceConfig,
    registry: State<'_, Arc<InstanceRegistry>>,
    config_store: State<'_, Arc<ConfigStore>>,
    proxy_state: State<'_, Arc<ProxyState>>,
) -> Result<(), String> {
    let app_config = config_store.load_config();
    let llama_dir = app_config.llama_dir
        .ok_or("未配置 llama.cpp 路径，请先在设置中配置")?;

    // Persist the instance config
    config_store.save_instance_config(config.clone())?;

    // Start the process
    let port = registry.start(app, &llama_dir, config.clone()).await?;

    // Register in proxy routing table
    proxy_state.register(&config.name, port);

    Ok(())
}

/// Stop a named model instance and remove it from the proxy routing table.
#[tauri::command]
pub async fn stop_instance(
    name: String,
    registry: State<'_, Arc<InstanceRegistry>>,
    proxy_state: State<'_, Arc<ProxyState>>,
) -> Result<(), String> {
    registry.stop(&name).await?;
    proxy_state.unregister(&name);
    Ok(())
}

/// Get a snapshot of all running instances.
#[tauri::command]
pub async fn get_all_instances(
    registry: State<'_, Arc<InstanceRegistry>>,
) -> Result<HashMap<String, InstanceInfo>, String> {
    Ok(registry.get_all().await)
}

/// Delete a saved instance config (does NOT stop a running instance).
#[tauri::command]
pub async fn delete_instance_config(
    name: String,
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<(), String> {
    config_store.delete_instance_config(&name)
}

// ── Per-model presets ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_model_presets(
    model_filename: String,
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<Vec<Preset>, String> {
    Ok(config_store.list_model_presets(&model_filename))
}

#[tauri::command]
pub async fn save_model_preset(
    model_filename: String,
    preset: Preset,
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<(), String> {
    config_store.save_model_preset(&model_filename, preset)
}

#[tauri::command]
pub async fn delete_model_preset(
    model_filename: String,
    name: String,
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<(), String> {
    config_store.delete_model_preset(&model_filename, &name)
}
