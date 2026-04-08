use tauri::State;
use std::sync::Arc;

use crate::error::AppError;
use crate::services::config_store::{AppConfig, ConfigStore, Preset};
use crate::services::llama_detector::{self, LlamaInstall};

#[tauri::command]
pub async fn get_config(
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<AppConfig, AppError> {
    Ok(config_store.load_config())
}

#[tauri::command]
pub async fn save_config(
    config: AppConfig,
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<(), AppError> {
    config_store.save_config(&config).map_err(AppError::from)
}

#[tauri::command]
pub async fn list_presets(
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<Vec<Preset>, AppError> {
    Ok(config_store.list_presets())
}

#[tauri::command]
pub async fn save_preset(
    preset: Preset,
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<(), AppError> {
    config_store.save_preset(preset).map_err(AppError::from)
}

#[tauri::command]
pub async fn load_preset(
    name: String,
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<Preset, AppError> {
    config_store.load_preset(&name).map_err(|e| AppError::NotFound { path: e })
}

#[tauri::command]
pub async fn delete_preset(
    name: String,
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<(), AppError> {
    config_store.delete_preset(&name).map_err(AppError::from)
}

#[tauri::command]
pub async fn detect_llama() -> Result<Vec<LlamaInstall>, AppError> {
    Ok(llama_detector::detect_installations())
}

#[tauri::command]
pub async fn validate_llama_path(path: String) -> Result<LlamaInstall, AppError> {
    llama_detector::validate_path(&path).map_err(|e| AppError::NotFound { path: e })
}
