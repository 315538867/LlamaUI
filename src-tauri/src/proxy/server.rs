use axum::{routing::post, Router};
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::task::JoinHandle;
use tower_http::cors::{Any, CorsLayer};

use super::handler::{handle_passthrough, handle_responses};

#[derive(Clone)]
pub struct ProxyConfig {
    pub port: u16,
    pub target: Arc<RwLock<String>>,
    pub client: Arc<Client>,
    pub cors: bool,
    pub allow_external: bool,
}

impl ProxyConfig {
    pub fn new(port: u16, target: String, cors: bool, allow_external: bool) -> Self {
        Self {
            port,
            target: Arc::new(RwLock::new(target)),
            client: Arc::new(Client::new()),
            cors,
            allow_external,
        }
    }
}

/// 启动代理服务器，返回 JoinHandle（用于 Tauri 命令控制）
pub fn start(config: ProxyConfig) -> JoinHandle<()> {
    tokio::spawn(async move {
        run_server(config).await;
    })
}

/// CLI 阻塞入口（默认开启 CORS，仅本地）
pub fn run_proxy_server(port: u16, target: &str) {
    let config = ProxyConfig::new(port, target.to_string(), true, false);
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(run_server(config));
}

async fn run_server(config: ProxyConfig) {
    let router = Router::new()
        .route("/v1/responses", post(handle_responses))
        .fallback(handle_passthrough)
        .with_state(config.clone());

    let app = if config.cors {
        router.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
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
    eprintln!("[proxy] listening on http://{} → {}", addr, config.target.read().unwrap());
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("[proxy] server error: {}", e);
    }
}
