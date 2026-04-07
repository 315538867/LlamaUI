use axum::{routing::post, Router};
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::Arc;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use tauri::{AppHandle, async_runtime::JoinHandle};
use tower_http::cors::CorsLayer;

use crate::services::config_store::ProxyResponsesMode;
use super::handler::{handle_passthrough, handle_responses};

#[derive(Clone)]
pub struct ProxyConfig {
    pub port: u16,
    /// instance_name → llama.cpp port
    pub routes: Arc<DashMap<String, u16>>,
    /// proxy-level API key (validates incoming Codex requests)
    pub api_key: Arc<ArcSwap<Option<String>>>,
    pub client: Arc<Client>,
    pub cors: bool,
    pub allow_external: bool,
    pub responses_mode: ProxyResponsesMode,
    pub app_handle: AppHandle,
}

impl ProxyConfig {
    pub fn new(
        port: u16,
        cors: bool,
        allow_external: bool,
        api_key: Option<String>,
        responses_mode: ProxyResponsesMode,
        app_handle: AppHandle,
    ) -> Self {
        Self {
            port,
            routes: Arc::new(DashMap::new()),
            api_key: Arc::new(ArcSwap::from_pointee(api_key)),
            client: Arc::new(
                Client::builder()
                    .pool_max_idle_per_host(4)
                    .pool_idle_timeout(std::time::Duration::from_secs(30))
                    .connect_timeout(std::time::Duration::from_secs(5))
                    .build()
                    .expect("reqwest client build failed"),
            ),
            cors,
            allow_external,
            responses_mode,
            app_handle,
        }
    }
}

/// 启动代理服务器，返回 JoinHandle（用于控制生命周期）
pub fn start(config: ProxyConfig) -> JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        run_server(config).await;
    })
}

async fn run_server(config: ProxyConfig) {
    let router = Router::new()
        .route("/v1/responses", post(handle_responses))
        .fallback(handle_passthrough)
        .with_state(config.clone());

    let app = if config.cors {
        router.layer(
            CorsLayer::permissive()
                .max_age(std::time::Duration::from_secs(3600)),
        )
    } else {
        router
    };

    let ip = if config.allow_external { [0, 0, 0, 0] } else { [127, 0, 0, 1] };
    let addr = SocketAddr::from((ip, config.port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[proxy] bind {}:{} failed: {}", addr.ip(), config.port, e);
            return;
        }
    };
    eprintln!("[proxy] listening on http://{}", addr);
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("[proxy] server error: {}", e);
    }
}
