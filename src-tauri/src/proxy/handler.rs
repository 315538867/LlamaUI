use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use bytes::Bytes;
use futures_util::StreamExt;
use serde_json::Value;
use tokio_stream::wrappers::ReceiverStream;

use super::convert::{request::codex_to_anthropic, response::SseConverter};
use super::server::ProxyConfig;

// 跳过不应转发的逐跳头
const HOP_BY_HOP: &[&str] = &[
    "connection", "keep-alive", "transfer-encoding", "te",
    "trailer", "upgrade", "proxy-authenticate", "proxy-authorization", "host",
];

/// /v1/responses → 格式转换 → /v1/messages
pub async fn handle_responses(
    State(cfg): State<ProxyConfig>,
    Json(body): Json<Value>,
) -> Response {
    let anthropic_req = codex_to_anthropic(&body);
    let target = cfg.target.read().unwrap().clone();
    let target_url = format!("{}/v1/messages", target.trim_end_matches('/'));

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
            return (StatusCode::BAD_GATEWAY, format!("upstream error: {}", e)).into_response();
        }
    };

    if !upstream.status().is_success() {
        let status = upstream.status();
        let body = upstream.text().await.unwrap_or_default();
        return (StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY), body)
            .into_response();
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(64);

    tokio::spawn(async move {
        let mut converter = SseConverter::new();
        let _ = tx.send(Ok(Bytes::from(converter.created_event()))).await;

        let mut stream = upstream.bytes_stream();
        let mut buf = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(_) => break,
            };
            buf.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buf.find('\n') {
                let line = buf[..pos].trim_end_matches('\r').to_string();
                buf = buf[pos + 1..].to_string();
                for event in converter.feed_line(&line) {
                    if tx.send(Ok(Bytes::from(event))).await.is_err() {
                        return;
                    }
                }
            }
        }

        if !buf.trim().is_empty() {
            for event in converter.feed_line(buf.trim()) {
                let _ = tx.send(Ok(Bytes::from(event))).await;
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

/// 其他所有路径 → 原封不动透传给 llama.cpp
pub async fn handle_passthrough(
    State(cfg): State<ProxyConfig>,
    req: axum::extract::Request,
) -> Response {
    let method = req.method().clone();
    let path_and_query = req.uri().path_and_query()
        .map(|pq| pq.as_str().to_owned())
        .unwrap_or_else(|| "/".into());
    let target = cfg.target.read().unwrap().clone();
    let target_url = format!("{}{}", target.trim_end_matches('/'), path_and_query);

    let headers = req.headers().clone();
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(b) => b,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("read body: {}", e)).into_response(),
    };

    let mut rb = cfg.client.request(
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET),
        &target_url,
    );
    for (name, value) in &headers {
        if HOP_BY_HOP.contains(&name.as_str()) {
            continue;
        }
        if let Ok(v) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
            rb = rb.header(name.as_str(), v);
        }
    }

    let upstream = match rb.body(body_bytes).send().await {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("upstream error: {}", e)).into_response(),
    };

    let status = StatusCode::from_u16(upstream.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let resp_headers = upstream.headers().clone();

    let mut builder = Response::builder().status(status);
    for (name, value) in &resp_headers {
        if HOP_BY_HOP.contains(&name.as_str()) {
            continue;
        }
        builder = builder.header(name.as_str(), value.as_bytes());
    }

    let stream = upstream.bytes_stream()
        .map(|r| r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));

    builder
        .body(axum::body::Body::from_stream(stream))
        .unwrap_or_else(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())
}
