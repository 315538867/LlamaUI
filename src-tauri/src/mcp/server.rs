use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::process::Child;
use std::sync::{Arc, Mutex};

use crate::services::config_store::{ConfigStore, LaunchParams};
use crate::services::llama_detector;
use crate::services::instance_registry::append_draft_args;
use crate::services::model_scanner::{self, ModelInfo};

/// MCP Server running over stdio (JSON-RPC 2.0)
/// Invoked as `llamaui mcp` subcommand.

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

const SERVER_NAME: &str = "llamaui";
const SERVER_VERSION: &str = "0.1.0";

fn tool_definitions() -> Value {
    json!([
        {
            "name": "llamaui_start",
            "description": "启动 llama.cpp 模型服务（llama-server）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "model": { "type": "string", "description": "模型名称或完整路径" },
                    "gpu_layers": { "type": "integer", "description": "GPU 卸载层数（99 表示全部卸载，实际上限由模型层数决定）", "default": 99 },
                    "ctx_size": { "type": "integer", "description": "上下文长度", "default": 4096 },
                    "port": { "type": "integer", "description": "Server 端口", "default": 8000 },
                    "model_draft": { "type": "string", "description": "草稿模型路径（用于 speculative decoding，需 llama.cpp 支持）" },
                    "gpu_layers_draft": { "type": "integer", "description": "草稿模型 GPU 卸载层数", "default": 99 },
                    "draft_max": { "type": "integer", "description": "最大草稿长度（默认 16）" },
                    "draft_min": { "type": "integer", "description": "最小草稿长度（默认 1）" },
                    "draft_p_min": { "type": "number", "description": "草稿 token 最小概率（0.0-1.0）" },
                    "ctx_size_draft": { "type": "integer", "description": "草稿模型上下文大小（默认与主模型相同）" },
                    "spec_type": { "type": "string", "description": "Speculative decoding 类型：none|ngram-cache|ngram-simple|ngram-map-k|ngram-map-k4v|ngram-mod" }
                },
                "required": ["model"]
            }
        },
        {
            "name": "llamaui_stop",
            "description": "停止当前通过 MCP 启动的 llama.cpp 模型服务"
        },
        {
            "name": "llamaui_status",
            "description": "查询 llama.cpp 模型运行状态（仅限通过 MCP 启动的进程）"
        },
        {
            "name": "llamaui_list_models",
            "description": "列出所有可用的本地 GGUF 模型"
        },
        {
            "name": "llamaui_get_config",
            "description": "获取当前 LlamaUI 配置"
        }
    ])
}

pub fn run_stdio_server(config_store: Arc<ConfigStore>) {
    let child: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));

    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let response = JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                };
                let mut out = stdout.lock();
                let _ = writeln!(out, "{}", serde_json::to_string(&response).unwrap_or_else(|_| r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"serialize error"}}"#.into()));
                let _ = out.flush();
                continue;
            }
        };

        let id = request.id.unwrap_or(Value::Null);
        let response = handle_request(&request.method, request.params, &config_store, &child, id);

        let mut out = stdout.lock();
        let _ = writeln!(out, "{}", serde_json::to_string(&response).unwrap_or_else(|_| r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"serialize error"}}"#.into()));
        let _ = out.flush();
    }

    // Clean up child process when stdin closes (MCP host disconnected).
    // Use into_inner() to recover even if the mutex was poisoned by a prior panic.
    let mut guard = child.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ref mut c) = *guard {
        let _ = c.kill();
        let _ = c.wait(); // reap to avoid zombie
    };
}

fn handle_request(
    method: &str,
    params: Option<Value>,
    config_store: &ConfigStore,
    child: &Arc<Mutex<Option<Child>>>,
    id: Value,
) -> JsonRpcResponse {
    match method {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION }
            })),
            error: None,
        },

        "tools/list" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(json!({ "tools": tool_definitions() })),
            error: None,
        },

        "tools/call" => {
            let params = params.unwrap_or(Value::Null);
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

            let result = handle_tool_call(tool_name, &arguments, config_store, child);
            match result {
                Ok(content) => JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id,
                    result: Some(json!({
                        "content": [{ "type": "text", "text": content }]
                    })),
                    error: None,
                },
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id,
                    result: Some(json!({
                        "content": [{ "type": "text", "text": e }],
                        "isError": true
                    })),
                    error: None,
                },
            }
        }

        "notifications/initialized" | "ping" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(json!({})),
            error: None,
        },

        _ => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", method),
            }),
        },
    }
}

fn handle_tool_call(
    name: &str,
    arguments: &Value,
    config_store: &ConfigStore,
    child: &Arc<Mutex<Option<Child>>>,
) -> Result<String, String> {
    match name {
        "llamaui_status" => {
            let running = child.lock().map(|g| g.is_some()).unwrap_or(false);
            Ok(json!({ "running": running, "managed_by": "mcp" }).to_string())
        }

        "llamaui_list_models" => {
            let models = collect_all_models(config_store);
            Ok(serde_json::to_string_pretty(&models).unwrap_or_else(|_| "[]".into()))
        }

        "llamaui_get_config" => {
            let mut config = config_store.load_config();
            config.proxy_api_key = None; // never expose the proxy API key over MCP
            Ok(serde_json::to_string_pretty(&config).unwrap_or_else(|_| "{}".into()))
        }

        "llamaui_start" => {
            // Reject if already running
            {
                let guard = child.lock().map_err(|e| e.to_string())?;
                if guard.is_some() {
                    return Err("模型已在运行，请先调用 llamaui_stop".into());
                }
            }

            let model = arguments
                .get("model")
                .and_then(|v| v.as_str())
                .ok_or("缺少 model 参数")?;

            let config = config_store.load_config();
            let (llama_dir, auto_detected) = match config.llama_dir {
                Some(dir) => (dir, false),
                None => {
                    let installs = crate::services::llama_detector::detect_installations();
                    let found = installs.into_iter().find(|i| i.has_server)
                        .ok_or("未配置 llama.cpp 路径，且自动检测未找到 llama-server")?;
                    (found.path, true)
                }
            };
            let gpu_layers = arguments.get("gpu_layers").and_then(|v| v.as_i64()).unwrap_or(99);
            let ctx_size = arguments.get("ctx_size").and_then(|v| v.as_i64()).unwrap_or(4096);

            // Use requested port if provided and available, otherwise find a free port
            let port: u16 = if let Some(p) = arguments.get("port").and_then(|v| v.as_i64()) {
                let p = p as u16;
                match std::net::TcpListener::bind(format!("127.0.0.1:{}", p)) {
                    Ok(_) => p,
                    Err(_) => {
                        // Requested port is in use — find a free one
                        std::net::TcpListener::bind("127.0.0.1:0")
                            .and_then(|l| l.local_addr())
                            .map(|a| a.port())
                            .unwrap_or(18000)
                    }
                }
            } else {
                std::net::TcpListener::bind("127.0.0.1:0")
                    .and_then(|l| l.local_addr())
                    .map(|a| a.port())
                    .unwrap_or(18000)
            };

            let bin_path = crate::services::llama_detector::get_binary_path(&llama_dir, "llama-server");
            if !bin_path.exists() {
                return Err(format!("找不到 llama-server: {}", bin_path.display()));
            }

            let caps = llama_detector::probe_capabilities(&bin_path);

            // Resolve model: absolute path first, then search model_dirs by name
            let model_path = if std::path::Path::new(model).exists() {
                model.to_string()
            } else {
                let mut found: Option<String> = None;
                'search: for dir in &config.model_dirs {
                    if let Ok(models) = model_scanner::scan_directory(dir) {
                        for m in models {
                            if m.name == model || m.path.ends_with(model) {
                                found = Some(m.path);
                                break 'search;
                            }
                        }
                    }
                }
                found.ok_or_else(|| format!("找不到模型: {}", model))?
            };

            let mut cmd = std::process::Command::new(&bin_path);
            let mut mcp_args: Vec<String> = vec![
                "-m".into(), model_path.clone(),
                "-ngl".into(), gpu_layers.to_string(),
                "-c".into(), ctx_size.to_string(),
                "--host".into(), "127.0.0.1".into(),
                "--port".into(), port.to_string(),
            ];
            if caps.supports_flash_attn {
                mcp_args.extend_from_slice(&["--flash-attn".into(), "on".into()]);
            }
            if caps.supports_cont_batching {
                mcp_args.push("--cont-batching".into());
            }

            // Draft model (speculative decoding)
            let draft_params = LaunchParams {
                model_draft: arguments.get("model_draft").and_then(|v| v.as_str()).map(String::from),
                gpu_layers_draft: arguments.get("gpu_layers_draft").and_then(|v| v.as_i64()).map(|v| v as i32),
                draft_max: arguments.get("draft_max").and_then(|v| v.as_i64()).map(|v| v as i32),
                draft_min: arguments.get("draft_min").and_then(|v| v.as_i64()).map(|v| v as i32),
                draft_p_min: arguments.get("draft_p_min").and_then(|v| v.as_f64()).map(|v| v as f32),
                ctx_size_draft: arguments.get("ctx_size_draft").and_then(|v| v.as_i64()).map(|v| v as i32),
                spec_type: arguments.get("spec_type").and_then(|v| v.as_str()).map(String::from),
                ..LaunchParams::default()
            };
            append_draft_args(&mut mcp_args, &draft_params, &caps);

            cmd.args(&mcp_args);

            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }

            let spawned = cmd.spawn().map_err(|e| format!("启动失败: {}", e))?;
            let pid = spawned.id();
            *child.lock().map_err(|e| e.to_string())? = Some(spawned);

            Ok(json!({
                "status": "started",
                "pid": pid,
                "model": model_path,
                "port": port,
                "url": format!("http://127.0.0.1:{}", port),
                "llama_dir": llama_dir,
                "llama_dir_auto_detected": auto_detected
            }).to_string())
        }

        "llamaui_stop" => {
            let mut guard = child.lock().map_err(|e| e.to_string())?;
            match guard.take() {
                Some(mut c) => {
                    c.kill().map_err(|e| format!("终止失败: {}", e))?;
                    let _ = c.wait();
                    Ok(json!({ "status": "stopped" }).to_string())
                }
                None => Err("没有正在运行的模型进程".into()),
            }
        }

        _ => Err(format!("未知工具: {}", name)),
    }
}

fn collect_all_models(config_store: &ConfigStore) -> Vec<ModelInfo> {
    let config = config_store.load_config();
    let mut all = Vec::new();
    for dir in &config.model_dirs {
        if let Ok(mut models) = model_scanner::scan_directory(dir) {
            all.append(&mut models);
        }
    }
    all
}
