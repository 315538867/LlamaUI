use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::process_manager::LaunchMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub llama_dir: Option<String>,
    pub model_dirs: Vec<String>,
    pub default_params: LaunchParams,
    pub last_preset: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            llama_dir: None,
            model_dirs: Vec::new(),
            default_params: LaunchParams::default(),
            last_preset: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchParams {
    pub gpu_layers: Option<i32>,
    pub ctx_size: Option<u32>,
    pub threads: Option<u32>,
    pub port: Option<u16>,
    pub host: Option<String>,
    pub flash_attn: Option<bool>,
    pub cont_batching: Option<bool>,
    pub batch_size: Option<u32>,
    pub ubatch_size: Option<u32>,
    pub parallel: Option<u32>,
    pub cache_type_k: Option<String>,
    pub seed: Option<i64>,
    pub mlock: Option<bool>,
    pub no_mmap: Option<bool>,
    pub api_key: Option<String>,
    pub cors_allow_origins: Option<String>,
    pub system_prompt: Option<String>,
    pub extra_args: Option<String>,
}

impl Default for LaunchParams {
    fn default() -> Self {
        Self {
            gpu_layers: Some(99),
            ctx_size: Some(4096),
            threads: None,
            port: Some(8000),
            host: Some("127.0.0.1".into()),
            flash_attn: Some(true),
            cont_batching: Some(true),
            batch_size: None,
            ubatch_size: None,
            parallel: None,
            cache_type_k: None,
            seed: None,
            mlock: None,
            no_mmap: None,
            api_key: None,
            cors_allow_origins: None,
            system_prompt: None,
            extra_args: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchConfig {
    pub model_path: String,
    pub mode: LaunchMode,
    pub gpu_layers: Option<i32>,
    pub ctx_size: Option<u32>,
    pub threads: Option<u32>,
    pub port: Option<u16>,
    pub host: Option<String>,
    pub flash_attn: Option<bool>,
    pub cont_batching: Option<bool>,
    pub batch_size: Option<u32>,
    pub ubatch_size: Option<u32>,
    pub parallel: Option<u32>,
    pub cache_type_k: Option<String>,
    pub seed: Option<i64>,
    pub mlock: Option<bool>,
    pub no_mmap: Option<bool>,
    pub api_key: Option<String>,
    pub cors_allow_origins: Option<String>,
    pub system_prompt: Option<String>,
    pub prompt: Option<String>,
    pub predict: Option<u32>,
    pub extra_args: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub params: LaunchParams,
    pub mode: LaunchMode,
}

pub struct ConfigStore {
    config_path: PathBuf,
    presets_path: PathBuf,
}

impl ConfigStore {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            config_path: app_data_dir.join("config.json"),
            presets_path: app_data_dir.join("presets.json"),
        }
    }

    pub fn load_config(&self) -> AppConfig {
        if self.config_path.exists() {
            std::fs::read_to_string(&self.config_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            AppConfig::default()
        }
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), String> {
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("序列化失败: {}", e))?;
        atomic_write(&self.config_path, &json)
    }

    pub fn list_presets(&self) -> Vec<Preset> {
        if self.presets_path.exists() {
            std::fs::read_to_string(&self.presets_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    pub fn save_preset(&self, preset: Preset) -> Result<(), String> {
        let mut presets = self.list_presets();
        if let Some(existing) = presets.iter_mut().find(|p| p.name == preset.name) {
            *existing = preset;
        } else {
            presets.push(preset);
        }
        self.write_presets(&presets)
    }

    pub fn load_preset(&self, name: &str) -> Result<Preset, String> {
        self.list_presets()
            .into_iter()
            .find(|p| p.name == name)
            .ok_or_else(|| format!("预设不存在: {}", name))
    }

    pub fn delete_preset(&self, name: &str) -> Result<(), String> {
        let mut presets = self.list_presets();
        let len_before = presets.len();
        presets.retain(|p| p.name != name);
        if presets.len() == len_before {
            return Err(format!("预设不存在: {}", name));
        }
        self.write_presets(&presets)
    }

    fn write_presets(&self, presets: &[Preset]) -> Result<(), String> {
        let json = serde_json::to_string_pretty(presets)
            .map_err(|e| format!("序列化失败: {}", e))?;
        atomic_write(&self.presets_path, &json)
    }
}

/// Write to a temp file then rename for crash safety (#9 fix)
fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, content)
        .map_err(|e| format!("写入临时文件失败: {}", e))?;
    std::fs::rename(&tmp_path, path)
        .map_err(|e| format!("重命名失败: {}", e))?;
    Ok(())
}
