use tauri::State;
use std::sync::Arc;
use serde::Serialize;

use crate::services::model_scanner::{self, ModelInfo};
use crate::services::config_store::ConfigStore;

#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub models: Vec<ModelInfo>,
    pub scan_errors: Vec<String>,
}

#[tauri::command]
pub async fn scan_models(
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<ScanResult, String> {
    let config = config_store.load_config();
    let mut all_models = Vec::new();
    let mut scan_errors = Vec::new();

    for dir in &config.model_dirs {
        match model_scanner::scan_directory(dir) {
            Ok(mut models) => all_models.append(&mut models),
            Err(e) => {
                scan_errors.push(format!("{}: {}", dir, e));
            }
        }
    }

    Ok(ScanResult { models: all_models, scan_errors })
}

#[tauri::command]
pub async fn get_model_info(path: String) -> Result<ModelInfo, String> {
    let file_path = std::path::Path::new(&path);
    model_scanner::parse_model_file(file_path)
        .ok_or_else(|| format!("无法读取模型文件: {}", path))
}
