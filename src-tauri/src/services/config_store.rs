use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::process_manager::LaunchMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub llama_dir: Option<String>,
    pub model_dirs: Vec<String>,
    pub default_params: LaunchParams,
    pub last_preset: Option<String>,
    #[serde(default = "default_proxy_port")]
    pub proxy_port: u16,
    #[serde(default = "default_true")]
    pub proxy_cors: bool,
    #[serde(default)]
    pub proxy_allow_external: bool,
}

fn default_proxy_port() -> u16 { 8080 }
fn default_true() -> bool { true }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            llama_dir: None,
            model_dirs: Vec::new(),
            default_params: LaunchParams::default(),
            last_preset: None,
            proxy_port: 8080,
            proxy_cors: true,
            proxy_allow_external: false,
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
    pub cache_type_v: Option<String>,
    pub no_kv_offload: Option<bool>,
    pub seed: Option<i64>,
    pub mlock: Option<bool>,
    pub no_mmap: Option<bool>,
    pub api_key: Option<String>,
    pub system_prompt: Option<String>,
    pub extra_args: Option<String>,
}

impl Default for LaunchParams {
    fn default() -> Self {
        Self {
            gpu_layers: Some(99),
            ctx_size: Some(4096),
            threads: None,
            port: None,            // None → 启动时随机分配空闲端口
            host: Some("127.0.0.1".into()),
            flash_attn: Some(true),
            cont_batching: Some(true),
            batch_size: None,
            ubatch_size: None,
            parallel: None,
            cache_type_k: None,
            cache_type_v: None,
            no_kv_offload: None,
            seed: None,
            mlock: None,
            no_mmap: None,
            api_key: None,
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
    pub cache_type_v: Option<String>,
    pub no_kv_offload: Option<bool>,
    pub seed: Option<i64>,
    pub mlock: Option<bool>,
    pub no_mmap: Option<bool>,
    pub api_key: Option<String>,
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
    cache: std::sync::Mutex<Option<AppConfig>>,
    presets_cache: std::sync::Mutex<Option<Vec<Preset>>>,
}

impl ConfigStore {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            config_path: app_data_dir.join("config.json"),
            presets_path: app_data_dir.join("presets.json"),
            cache: std::sync::Mutex::new(None),
            presets_cache: std::sync::Mutex::new(None),
        }
    }

    pub fn load_config(&self) -> AppConfig {
        // Cache hit — avoid disk read
        if let Ok(guard) = self.cache.lock() {
            if let Some(ref cfg) = *guard {
                return cfg.clone();
            }
        }
        // Cache miss — read from disk
        let config = if self.config_path.exists() {
            std::fs::read_to_string(&self.config_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            AppConfig::default()
        };
        if let Ok(mut guard) = self.cache.lock() {
            *guard = Some(config.clone());
        }
        config
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), String> {
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("序列化失败: {}", e))?;
        atomic_write(&self.config_path, &json)?;
        // Update cache on successful save
        if let Ok(mut guard) = self.cache.lock() {
            *guard = Some(config.clone());
        }
        Ok(())
    }

    pub fn list_presets(&self) -> Vec<Preset> {
        if let Ok(guard) = self.presets_cache.lock() {
            if let Some(ref presets) = *guard {
                return presets.clone();
            }
        }
        let presets = if self.presets_path.exists() {
            std::fs::read_to_string(&self.presets_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        if let Ok(mut guard) = self.presets_cache.lock() {
            *guard = Some(presets.clone());
        }
        presets
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
        atomic_write(&self.presets_path, &json)?;
        if let Ok(mut guard) = self.presets_cache.lock() {
            *guard = Some(presets.to_vec());
        }
        Ok(())
    }
}

/// Write to a temp file then rename for crash safety.
/// Falls back to direct write if rename fails (e.g. cross-device on Windows).
fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, content)
        .map_err(|e| format!("写入临时文件失败: {}", e))?;
    if std::fs::rename(&tmp_path, path).is_err() {
        // Fallback: direct write (less crash-safe but avoids cross-device rename failure)
        std::fs::write(path, content)
            .map_err(|e| format!("写入配置文件失败: {}", e))?;
        let _ = std::fs::remove_file(&tmp_path);
    }
    Ok(())
}
