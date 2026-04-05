use std::sync::{Arc, Mutex};
use serde_json::{json, Value};
use tauri::State;
use tauri::async_runtime::JoinHandle;

use crate::proxy::server::{start, ProxyConfig};
use crate::services::config_store::ConfigStore;

pub struct ProxyState {
    handle: Mutex<Option<JoinHandle<()>>>,
    config: Mutex<Option<ProxyConfig>>,
}

impl ProxyState {
    pub fn new() -> Self {
        Self {
            handle: Mutex::new(None),
            config: Mutex::new(None),
        }
    }

    /// App 启动时自动调用
    pub fn auto_start(&self, port: u16, target: String, cors: bool, allow_external: bool) {
        let mut handle_guard = self.handle.lock().unwrap();
        if handle_guard.is_some() {
            return;
        }
        let config = ProxyConfig::new(port, target, cors, allow_external);
        let handle = start(config.clone());
        *handle_guard = Some(handle);
        *self.config.lock().unwrap() = Some(config);
    }

    /// llama.cpp 启动后更新转发目标地址
    pub fn update_llama_target(&self, host: &str, port: u16) {
        let target = format!("http://{}:{}", host, port);
        if let Ok(config_guard) = self.config.lock() {
            if let Some(ref config) = *config_guard {
                if let Ok(mut t) = config.target.write() {
                    *t = target;
                }
            }
        }
    }
}

/// 停止并以新参数重启代理，同时持久化设置
#[tauri::command]
pub fn restart_proxy(
    port: u16,
    cors: bool,
    allow_external: bool,
    state: State<Arc<ProxyState>>,
    config_store: State<Arc<ConfigStore>>,
) -> Result<(), String> {
    // 持久化到 AppConfig
    let mut app_config = config_store.load_config();
    app_config.proxy_port = port;
    app_config.proxy_cors = cors;
    app_config.proxy_allow_external = allow_external;
    config_store.save_config(&app_config)?;

    // 重启 Proxy
    let mut handle_guard = state.handle.lock().map_err(|e| e.to_string())?;
    let mut config_guard = state.config.lock().map_err(|e| e.to_string())?;

    let current_target = config_guard
        .as_ref()
        .and_then(|c| c.target.read().ok().map(|t| t.clone()))
        .unwrap_or_else(|| "http://127.0.0.1:8000".into());

    if let Some(h) = handle_guard.take() {
        h.abort();
    }

    let config = ProxyConfig::new(port, current_target, cors, allow_external);
    let handle = start(config.clone());
    *handle_guard = Some(handle);
    *config_guard = Some(config);
    Ok(())
}

#[tauri::command]
pub fn get_proxy_status(state: State<Arc<ProxyState>>) -> Value {
    let running = state.handle.lock().map(|g| g.is_some()).unwrap_or(false);
    let config = state.config.lock().ok().and_then(|g| g.clone());
    match config {
        Some(c) if running => json!({
            "running": true,
            "port": c.port,
            "cors": c.cors,
            "allow_external": c.allow_external,
            "target": c.target.read().map(|t| t.clone()).unwrap_or_default(),
        }),
        _ => json!({ "running": false }),
    }
}
