use std::sync::{Arc, Mutex};
use serde_json::{json, Value};
use tauri::{AppHandle, State};
use tauri::async_runtime::JoinHandle;

use crate::proxy::server::{start, ProxyConfig};
use crate::services::config_store::{ConfigStore, ProxyResponsesMode};

pub struct ProxyState {
    handle: Mutex<Option<JoinHandle<()>>>,
    /// Shared config — routes table updated in-place without restarting server
    config: Mutex<Option<ProxyConfig>>,
}

impl ProxyState {
    pub fn new() -> Self {
        Self {
            handle: Mutex::new(None),
            config: Mutex::new(None),
        }
    }

    fn with_config<F: FnOnce(&ProxyConfig)>(&self, f: F) {
        if let Ok(guard) = self.config.lock() {
            if let Some(ref cfg) = *guard { f(cfg); }
        }
    }

    /// Start proxy server at app launch with empty routes table.
    pub fn start_at_launch(
        &self,
        port: u16,
        cors: bool,
        allow_external: bool,
        api_key: Option<String>,
        responses_mode: ProxyResponsesMode,
        app_handle: AppHandle,
    ) {
        let config = ProxyConfig::new(port, cors, allow_external, api_key, responses_mode, app_handle);
        let handle = start(config.clone());
        if let Ok(mut h) = self.handle.lock() { *h = Some(handle); }
        if let Ok(mut c) = self.config.lock() { *c = Some(config); }
    }

    /// Register an instance route (called after llama.cpp starts).
    pub fn register(&self, name: &str, port: u16) {
        self.with_config(|cfg| { cfg.routes.insert(name.to_string(), port); });
    }

    /// Unregister an instance route (called after llama.cpp stops).
    pub fn unregister(&self, name: &str) {
        self.with_config(|cfg| { cfg.routes.remove(name); });
    }

    /// Stop proxy server (called on app shutdown).
    pub fn stop(&self) {
        if let Ok(mut h) = self.handle.lock() {
            if let Some(handle) = h.take() { handle.abort(); }
        }
        if let Ok(mut c) = self.config.lock() { *c = None; }
    }
}

/// Update proxy settings and restart server; preserves existing routes.
#[tauri::command]
pub fn restart_proxy(
    port: u16,
    cors: bool,
    allow_external: bool,
    api_key: Option<String>,
    responses_mode: ProxyResponsesMode,
    app_handle: AppHandle,
    state: State<Arc<ProxyState>>,
    config_store: State<Arc<ConfigStore>>,
) -> Result<(), String> {
    // Persist settings
    let mut app_config = config_store.load_config();
    app_config.proxy_port = port;
    app_config.proxy_cors = cors;
    app_config.proxy_allow_external = allow_external;
    app_config.proxy_api_key = api_key.clone();
    app_config.proxy_responses_mode = responses_mode.clone();
    config_store.save_config(&app_config)?;

    // Snapshot current routes so they survive the restart
    let existing_routes: Vec<(String, u16)> = {
        let guard = state.config.lock().map_err(|e| e.to_string())?;
        guard.as_ref()
            .map(|cfg| cfg.routes.iter().map(|r| (r.key().clone(), *r.value())).collect())
            .unwrap_or_default()
    };

    // Abort old server
    if let Ok(mut h) = state.handle.lock() {
        if let Some(old) = h.take() { old.abort(); }
    }

    // Start new server with same routes
    let new_config = ProxyConfig::new(port, cors, allow_external, api_key, responses_mode, app_handle);
    for (name, port) in existing_routes {
        new_config.routes.insert(name, port);
    }
    let handle = start(new_config.clone());
    if let Ok(mut h) = state.handle.lock() { *h = Some(handle); }
    if let Ok(mut c) = state.config.lock() { *c = Some(new_config); }

    Ok(())
}

#[tauri::command]
pub fn get_proxy_status(state: State<Arc<ProxyState>>) -> Value {
    let guard = state.config.lock().ok();
    let running = state.handle.lock().map(|g| g.is_some()).unwrap_or(false);

    match guard.as_ref().and_then(|g| g.as_ref()) {
        Some(cfg) if running => {
            let routes: Vec<Value> = cfg.routes.iter()
                .map(|r| json!({ "name": r.key(), "port": r.value() }))
                .collect();
            json!({
                "running": true,
                "port": cfg.port,
                "cors": cfg.cors,
                "allow_external": cfg.allow_external,
                "routes": routes,
            })
        }
        _ => json!({ "running": false }),
    }
}
