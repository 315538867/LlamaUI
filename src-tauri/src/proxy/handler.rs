use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use bytes::Bytes;
use futures_util::StreamExt;
use serde::Serialize;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use tokio_stream::wrappers::ReceiverStream;

use crate::services::config_store::ProxyResponsesMode;
use super::convert::{request::codex_to_anthropic, response::SseConverter};
use super::server::ProxyConfig;

const HOP_BY_HOP: &[&str] = &[
    "connection", "keep-alive", "transfer-encoding", "te",
    "trailer", "upgrade", "proxy-authenticate", "proxy-authorization", "host",
];

// ── Proxy log event (emitted to frontend) ────────────────────────────────────

#[derive(Clone, Serialize)]
struct ProxyLogEvent {
    timestamp: u64,
    level: String,      // "info" | "warn" | "error"
    message: String,
}

impl ProxyLogEvent {
    fn now(level: &str, message: impl Into<String>) -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            level: level.into(),
            message: message.into(),
        }
    }
}

fn emit_log(cfg: &ProxyConfig, level: &str, msg: impl Into<String>) {
    let event = ProxyLogEvent::now(level, msg);
    cfg.app_handle.emit("proxy://log", &event).ok();
}

fn sse_error_response(message: &str) -> Response {
    let data = json!({
        "type": "error",
        "error": {
            "type": "server_error",
            "message": message
        }
    });
    let body = format!("event: error\ndata: {}\n\ndata: [DONE]\n\n", data);
    Response::builder()
        .status(200)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .body(axum::body::Body::from(body))
        .unwrap_or_else(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())
}

fn error_resp(status: StatusCode, error_type: &str, message: impl Into<String>) -> Response {
    let message = message.into();
    (status, Json(json!({ "error": { "type": error_type, "message": message } }))).into_response()
}

// ── Auth helper ───────────────────────────────────────────────────────────────

/// Returns `None` if auth passes (no key configured, or header matches).
/// Returns `Some(Response)` with 401 if auth fails.
fn check_auth(cfg: &ProxyConfig, headers: &HeaderMap) -> Option<Response> {
    let expected = cfg.api_key.load();
    let Some(ref expected_key) = **expected else {
        return None; // No proxy key configured — open access
    };

    let provided = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");

    if provided == expected_key {
        return None; // Auth OK
    }

    emit_log(cfg, "warn", "认证失败：Authorization header 无效");
    Some(error_resp(StatusCode::UNAUTHORIZED, "authentication_error",
        "Invalid API key. Provide a valid Bearer token."))
}

// ── Route resolver ────────────────────────────────────────────────────────────

/// Look up which llama.cpp port handles `model_name`.
/// Returns `Err(Response)` with 404 if not found.
fn resolve_route(cfg: &ProxyConfig, model_name: &str) -> Result<String, Response> {
    if let Some(port) = cfg.routes.get(model_name) {
        return Ok(format!("http://127.0.0.1:{}", *port));
    }
    // Build available-models list for the error message
    let available: Vec<String> = cfg.routes.iter().map(|r| r.key().clone()).collect();
    let msg = if available.is_empty() {
        format!("没有正在运行的模型实例。请先启动一个实例。")
    } else {
        format!(
            "模型 \"{}\" 未运行。可用实例：{}",
            model_name,
            available.join(", ")
        )
    };
    emit_log(cfg, "error", &msg);
    Err((
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": {
                "type": "model_not_found",
                "message": msg,
                "available": available,
            }
        })),
    ).into_response())
}

// ── /v1/responses handler ─────────────────────────────────────────────────────

/// /v1/responses → auth check → route by model → convert → /v1/messages
pub async fn handle_responses(
    State(cfg): State<ProxyConfig>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    // 1. Auth
    if let Some(err) = check_auth(&cfg, &headers) { return err; }

    // 2. Extract model
    let model_name = body.get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if model_name.is_empty() {
        emit_log(&cfg, "warn", "请求缺少 model 字段");
        return error_resp(StatusCode::BAD_REQUEST, "invalid_request", "Missing 'model' field in request body.");
    }

    // 3. Route
    let target = match resolve_route(&cfg, model_name) {
        Ok(t) => t,
        Err(resp) => return resp,
    };

    emit_log(&cfg, "info", format!("[→] /v1/responses  model={}  →  {}", model_name, target));

    // 调试：打印 tools 字段结构
    if let Some(tools) = body.get("tools") {
        emit_log(&cfg, "info", format!("[DEBUG] tools: {}", tools));
    }

    // 4. 根据模式选择处理方式
    match cfg.responses_mode {
        ProxyResponsesMode::Direct => handle_responses_direct(cfg, target, body).await,
        ProxyResponsesMode::Anthropic => handle_responses_anthropic(cfg, target, body).await,
    }
}

/// 直连模式：直接透传到 llama.cpp /v1/responses
async fn handle_responses_direct(cfg: ProxyConfig, target: String, body: Value) -> Response {
    let target_url = format!("{}/v1/responses", target.trim_end_matches('/'));

    let upstream = match cfg.client
        .post(&target_url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("上游错误: {}", e);
            emit_log(&cfg, "error", &msg);
            return (StatusCode::BAD_GATEWAY, msg).into_response();
        }
    };

    if !upstream.status().is_success() {
        let status = upstream.status();
        let body_text = upstream.text().await.unwrap_or_default();
        emit_log(&cfg, "error", format!("[✗] {} {}", status.as_u16(), body_text));
        return sse_error_response(&body_text);
    }

    emit_log(&cfg, "info", format!("[←] 200 OK (direct)  →  {}", target_url));

    let status = StatusCode::from_u16(upstream.status().as_u16())
        .unwrap_or(StatusCode::OK);
    let resp_headers = upstream.headers().clone();
    let stream = upstream.bytes_stream()
        .map(|r| r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));

    let mut builder = Response::builder().status(status);
    for (name, value) in &resp_headers {
        if HOP_BY_HOP.contains(&name.as_str()) { continue; }
        builder = builder.header(name.as_str(), value.as_bytes());
    }
    builder
        .body(axum::body::Body::from_stream(stream))
        .unwrap_or_else(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())
}

/// Anthropic 转换模式：Codex → Anthropic Messages API → 转回 Codex 格式
async fn handle_responses_anthropic(cfg: ProxyConfig, target: String, body: Value) -> Response {
    // 4. Convert Codex Responses API → Anthropic Messages API
    let anthropic_req = codex_to_anthropic(&body);
    let target_url = format!("{}/v1/messages", target.trim_end_matches('/'));

    // 5. Build request to llama.cpp
    let upstream = match cfg.client
        .post(&target_url)
        .header("Content-Type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .json(&anthropic_req)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("上游错误: {}", e);
            emit_log(&cfg, "error", &msg);
            return (StatusCode::BAD_GATEWAY, msg).into_response();
        }
    };

    if !upstream.status().is_success() {
        let status = upstream.status();
        let body_text = upstream.text().await.unwrap_or_default();
        emit_log(&cfg, "error", format!("[✗] {} {}", status.as_u16(), body_text));
        return sse_error_response(&body_text);
    }

    emit_log(&cfg, "info", format!("[←] 200 OK (anthropic)  →  {}", target_url));

    // 6. Stream response back
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(128);

    tokio::spawn(async move {
        let mut converter = SseConverter::new();
        let _ = tx.send(Ok(Bytes::from(converter.created_event()))).await;

        let mut stream = upstream.bytes_stream();
        let mut buf = Vec::with_capacity(4096);

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk { Ok(c) => c, Err(_) => break };
            buf.extend_from_slice(&chunk);

            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let line = String::from_utf8_lossy(&buf[..pos])
                    .trim_end_matches('\r')
                    .to_string();
                buf.drain(..=pos);

                for event in converter.feed_line(&line) {
                    if tx.send(Ok(Bytes::from(event))).await.is_err() { return; }
                }
            }
        }

        if !buf.is_empty() {
            let line = String::from_utf8_lossy(&buf).trim().to_string();
            if !line.is_empty() {
                for event in converter.feed_line(&line) {
                    let _ = tx.send(Ok(Bytes::from(event))).await;
                }
            }
        }
    });

    Response::builder()
        .status(200)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(axum::body::Body::from_stream(ReceiverStream::new(rx)))
        .unwrap()
}

// ── Fallback passthrough ──────────────────────────────────────────────────────

/// All other paths → route to first available instance, or 503
pub async fn handle_passthrough(
    State(cfg): State<ProxyConfig>,
    headers: HeaderMap,
    req: axum::extract::Request,
) -> Response {
    if let Some(err) = check_auth(&cfg, &headers) { return err; }

    // Find first running instance
    let target: Option<String> = cfg.routes.iter()
        .next()
        .map(|r| format!("http://127.0.0.1:{}", r.value()));

    let target = match target {
        Some(t) => t,
        None => {
            let msg = "没有正在运行的模型实例";
            emit_log(&cfg, "error", msg);
            return error_resp(StatusCode::SERVICE_UNAVAILABLE, "no_instance", msg);
        }
    };

    let method = req.method().clone();
    let method_str = method.as_str().to_string();
    let path_and_query = req.uri().path_and_query()
        .map(|pq| pq.as_str().to_owned())
        .unwrap_or_else(|| "/".into());
    let target_url = format!("{}{}", target.trim_end_matches('/'), path_and_query);

    let body_stream = req.into_body().into_data_stream();
    let reqwest_body = reqwest::Body::wrap_stream(body_stream);

    let mut rb = cfg.client.request(
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET),
        &target_url,
    );
    for (name, value) in &headers {
        if HOP_BY_HOP.contains(&name.as_str()) { continue; }
        if let Ok(v) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
            rb = rb.header(name.as_str(), v);
        }
    }

    let upstream = match rb.body(reqwest_body).send().await {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("upstream error: {}", e);
            emit_log(&cfg, "error", &msg);
            return sse_error_response(&msg);
        }
    };

    emit_log(&cfg, "info", format!("[→] passthrough {} {} → {}", method_str, path_and_query, target));

    if !upstream.status().is_success() {
        let status = upstream.status();
        let body_text = upstream.text().await.unwrap_or_default();
        emit_log(&cfg, "error", format!("[✗] {} {}", status.as_u16(), body_text));
        return sse_error_response(&body_text);
    }

    let status = StatusCode::from_u16(upstream.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let resp_headers = upstream.headers().clone();

    let mut builder = Response::builder().status(status);
    for (name, value) in &resp_headers {
        if HOP_BY_HOP.contains(&name.as_str()) { continue; }
        builder = builder.header(name.as_str(), value.as_bytes());
    }

    let stream = upstream.bytes_stream()
        .map(|r| r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));

    builder
        .body(axum::body::Body::from_stream(stream))
        .unwrap_or_else(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())
}
