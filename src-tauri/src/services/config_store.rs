use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LaunchMode {
    Server,
    Cli,
}

// ── Launch params (per-instance configuration) ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchParams {
    pub gpu_layers: Option<i32>,
    pub ctx_size: Option<u32>,
    pub threads: Option<u32>,
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
    pub extra_args: Option<String>,
    pub no_context_shift: Option<bool>,
    pub keep: Option<i32>,
    pub model_draft: Option<String>,
    pub gpu_layers_draft: Option<i32>,
    pub draft_max: Option<i32>,
    pub draft_min: Option<i32>,
    pub draft_p_min: Option<f32>,
    pub ctx_size_draft: Option<i32>,
    pub spec_type: Option<String>,
}

impl Default for LaunchParams {
    fn default() -> Self {
        Self {
            gpu_layers: Some(99), // 99 = 全部卸载至 GPU，llama.cpp 自动限制不超过模型实际层数
            ctx_size: Some(4096),
            threads: None,
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
            extra_args: None,
            no_context_shift: None,
            keep: None,
            model_draft: None,
            gpu_layers_draft: None,
            draft_max: None,
            draft_min: None,
            draft_p_min: None,
            ctx_size_draft: None,
            spec_type: None,
        }
    }
}

// ── Instance config (named deployment unit) ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub name: String,          // routing key (= body.model in Codex requests)
    pub model_path: String,
    pub mode: LaunchMode,
    pub params: LaunchParams,
}

impl InstanceConfig {
    pub fn new(name: String, model_path: String) -> Self {
        Self {
            name,
            model_path,
            mode: LaunchMode::Server,
            params: LaunchParams::default(),
        }
    }
}

// ── Legacy LaunchConfig (kept for backward compat, used in process commands) ─

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchConfig {
    pub model_path: String,
    pub mode: LaunchMode,
    pub gpu_layers: Option<i32>,
    pub ctx_size: Option<u32>,
    pub threads: Option<u32>,
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
    pub prompt: Option<String>,
    pub predict: Option<u32>,
    pub extra_args: Option<String>,
}

// ── Preset ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub params: LaunchParams,
    pub mode: LaunchMode,
}

// ── AppConfig ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub llama_dir: Option<String>,
    pub model_dirs: Vec<String>,
    /// Saved instance configurations (user-created named deployments)
    #[serde(default)]
    pub instances: Vec<InstanceConfig>,
    /// Per-model-file presets: key = model filename (not full path)
    #[serde(default)]
    pub model_presets: HashMap<String, Vec<Preset>>,
    /// Legacy global defaults (kept for migration)
    pub default_params: LaunchParams,
    pub last_preset: Option<String>,
    #[serde(default = "default_proxy_port")]
    pub proxy_port: u16,
    #[serde(default = "default_true")]
    pub proxy_cors: bool,
    #[serde(default)]
    pub proxy_allow_external: bool,
    /// Proxy-level API key (validates incoming Codex requests)
    #[serde(default)]
    pub proxy_api_key: Option<String>,
    /// /v1/responses 处理模式：direct = 直透传到 llama.cpp，anthropic = 转 Anthropic 格式
    #[serde(default)]
    pub proxy_responses_mode: ProxyResponsesMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyResponsesMode {
    #[default]
    Direct,
    Anthropic,
}

fn default_proxy_port() -> u16 { 8080 }
fn default_true() -> bool { true }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            llama_dir: None,
            model_dirs: Vec::new(),
            instances: Vec::new(),
            model_presets: HashMap::new(),
            default_params: LaunchParams::default(),
            last_preset: None,
            proxy_port: 8080,
            proxy_cors: true,
            proxy_allow_external: false,
            proxy_api_key: None,
            proxy_responses_mode: ProxyResponsesMode::Direct,
        }
    }
}

// ── ConfigStore ───────────────────────────────────────────────────────────────

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
        if let Ok(guard) = self.cache.lock() {
            if let Some(ref cfg) = *guard { return cfg.clone(); }
        }
        let config = if self.config_path.exists() {
            std::fs::read_to_string(&self.config_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            AppConfig::default()
        };
        if let Ok(mut guard) = self.cache.lock() { *guard = Some(config.clone()); }
        config
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), String> {
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("序列化失败: {}", e))?;
        atomic_write(&self.config_path, &json)?;
        if let Ok(mut guard) = self.cache.lock() { *guard = Some(config.clone()); }
        Ok(())
    }

    // ── Instance config persistence ───────────────────────────────────────────

    pub fn save_instance_config(&self, instance: InstanceConfig) -> Result<(), String> {
        let mut cfg = self.load_config();
        if let Some(existing) = cfg.instances.iter_mut().find(|i| i.name == instance.name) {
            *existing = instance;
        } else {
            cfg.instances.push(instance);
        }
        self.save_config(&cfg)
    }

    pub fn delete_instance_config(&self, name: &str) -> Result<(), String> {
        let mut cfg = self.load_config();
        cfg.instances.retain(|i| i.name != name);
        self.save_config(&cfg)
    }

    // ── Per-model presets ─────────────────────────────────────────────────────

    pub fn list_model_presets(&self, model_filename: &str) -> Vec<Preset> {
        self.load_config()
            .model_presets
            .get(model_filename)
            .cloned()
            .unwrap_or_default()
    }

    pub fn save_model_preset(&self, model_filename: &str, preset: Preset) -> Result<(), String> {
        let mut cfg = self.load_config();
        let presets = cfg.model_presets.entry(model_filename.to_string()).or_default();
        if let Some(existing) = presets.iter_mut().find(|p| p.name == preset.name) {
            *existing = preset;
        } else {
            presets.push(preset);
        }
        self.save_config(&cfg)
    }

    pub fn delete_model_preset(&self, model_filename: &str, name: &str) -> Result<(), String> {
        let mut cfg = self.load_config();
        if let Some(presets) = cfg.model_presets.get_mut(model_filename) {
            let before = presets.len();
            presets.retain(|p| p.name != name);
            if presets.len() == before {
                return Err(format!("预设不存在: {}", name));
            }
        }
        self.save_config(&cfg)
    }

    // ── Legacy global presets (kept for migration) ────────────────────────────

    pub fn list_presets(&self) -> Vec<Preset> {
        if let Ok(guard) = self.presets_cache.lock() {
            if let Some(ref presets) = *guard { return presets.clone(); }
        }
        let presets = if self.presets_path.exists() {
            std::fs::read_to_string(&self.presets_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        if let Ok(mut guard) = self.presets_cache.lock() { *guard = Some(presets.clone()); }
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
        let before = presets.len();
        presets.retain(|p| p.name != name);
        if presets.len() == before {
            return Err(format!("预设不存在: {}", name));
        }
        self.write_presets(&presets)
    }

    fn write_presets(&self, presets: &[Preset]) -> Result<(), String> {
        let json = serde_json::to_string_pretty(presets)
            .map_err(|e| format!("序列化失败: {}", e))?;
        atomic_write(&self.presets_path, &json)?;
        if let Ok(mut guard) = self.presets_cache.lock() { *guard = Some(presets.to_vec()); }
        Ok(())
    }
}

// ── Atomic write ──────────────────────────────────────────────────────────────

fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, content)
        .map_err(|e| format!("写入临时文件失败: {}", e))?;
    if std::fs::rename(&tmp_path, path).is_err() {
        std::fs::write(path, content)
            .map_err(|e| format!("写入配置文件失败: {}", e))?;
        let _ = std::fs::remove_file(&tmp_path);
    }
    Ok(())
}
