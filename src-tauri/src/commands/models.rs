use tauri::{AppHandle, Emitter, State};
use std::sync::Arc;
use serde::Serialize;
use tokio::task::JoinSet;

use crate::error::AppError;
use crate::services::model_scanner::{self, ModelInfo};
use crate::services::config_store::ConfigStore;

#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub models: Vec<ModelInfo>,
    pub scan_errors: Vec<String>,
}

#[derive(Clone, Serialize)]
pub struct ScanDonePayload {
    pub errors: Vec<String>,
}

#[tauri::command]
pub async fn scan_models(
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<ScanResult, AppError> {
    let config = config_store.load_config();
    let mut all_models = Vec::new();
    let mut scan_errors = Vec::new();

    for dir in &config.model_dirs {
        match model_scanner::scan_directory(dir) {
            Ok(mut models) => all_models.append(&mut models),
            Err(e) => scan_errors.push(format!("{}: {}", dir, e)),
        }
    }

    Ok(ScanResult { models: all_models, scan_errors })
}

#[tauri::command]
pub async fn get_model_info(path: String) -> Result<ModelInfo, AppError> {
    let file_path = std::path::Path::new(&path);
    model_scanner::parse_model_file(file_path)
        .ok_or_else(|| AppError::NotFound { path: path.clone() })
}

/// Fire-and-forget: spawn per-directory concurrent scans.
/// Emits `model://found` for each model, then `model://scan-done`.
#[tauri::command]
pub async fn scan_models_stream(
    app: AppHandle,
    config_store: State<'_, Arc<ConfigStore>>,
) -> Result<(), AppError> {
    let dirs = config_store.load_config().model_dirs;
    let mut join_set = JoinSet::new();

    for dir in dirs {
        let app_clone = app.clone();
        join_set.spawn(tokio::task::spawn_blocking(move || {
            model_scanner::scan_directory_streaming(&dir, |info| {
                app_clone.emit("model://found", &info).ok();
            })
        }));
    }

    let mut all_errors: Vec<String> = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(errs)) => all_errors.extend(errs),
            Ok(Err(e)) => all_errors.push(e.to_string()),
            Err(e) => all_errors.push(format!("扫描任务异常: {}", e)),
        }
    }

    app.emit("model://scan-done", ScanDonePayload { errors: all_errors }).ok();
    Ok(())
}
