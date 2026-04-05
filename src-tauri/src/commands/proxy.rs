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

    /// llama.cpp 启动后调用：用最新代理设置（从配置读取）启动代理，目标指向 llama.cpp
    pub fn start_for_llama(&self, proxy_port: u16, cors: bool, allow_external: bool, llama_host: &str, llama_port: u16) {
        let target = format!("http://{}:{}", llama_host, llama_port);
        let config = ProxyConfig::new(proxy_port, target, cors, allow_external);
        let handle = start(config.clone());
        // 先停掉旧实例
        if let Ok(mut h) = self.handle.lock() {
            if let Some(old) = h.take() { old.abort(); }
            *h = Some(handle);
        }
        if let Ok(mut c) = self.config.lock() { *c = Some(config); }
    }

    /// 停止代理（llama.cpp 停止时调用）
    pub fn stop(&self) {
        if let Ok(mut h) = self.handle.lock() {
            if let Some(handle) = h.take() { handle.abort(); }
        }
        if let Ok(mut c) = self.config.lock() { *c = None; }
    }
}

/// 修改代理设置并持久化；若代理正在运行则同步重启
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

    // 只有代理正在运行时才重启（代理生命周期跟随 llama.cpp）
    let mut handle_guard = state.handle.lock().map_err(|e| e.to_string())?;
    if handle_guard.is_none() {
        return Ok(()); // 未运行，只保存设置即可
    }

    let mut config_guard = state.config.lock().map_err(|e| e.to_string())?;
    let current_target = config_guard
        .as_ref()
        .and_then(|c| c.target.read().ok().map(|t| t.clone()))
        .unwrap_or_else(|| "http://127.0.0.1:18000".into());

    if let Some(h) = handle_guard.take() { h.abort(); }

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
