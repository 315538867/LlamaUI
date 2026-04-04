use tauri::State;
use std::sync::Arc;

use crate::services::model_scanner::{self, ModelInfo};
use crate::services::config_store::ConfigStore;

#[tauri::command]
pub async fn scan_models(
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<Vec<ModelInfo>, String> {
    let config = config_store.load_config();
    let mut all_models = Vec::new();

    for dir in &config.model_dirs {
        match model_scanner::scan_directory(dir) {
            Ok(mut models) => all_models.append(&mut models),
            Err(e) => eprintln!("扫描目录失败 {}: {}", dir, e),
        }
    }

    Ok(all_models)
}

#[tauri::command]
pub async fn get_model_info(path: String) -> Result<ModelInfo, String> {
    let file_path = std::path::Path::new(&path);
    model_scanner::parse_model_file(file_path)
        .ok_or_else(|| format!("无法读取模型文件: {}", path))
}
