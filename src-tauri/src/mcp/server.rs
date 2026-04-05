use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::process::Child;
use std::sync::{Arc, Mutex};

use crate::services::config_store::ConfigStore;
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
                    "gpu_layers": { "type": "integer", "description": "GPU 层数", "default": 99 },
                    "ctx_size": { "type": "integer", "description": "上下文长度", "default": 4096 },
                    "port": { "type": "integer", "description": "Server 端口", "default": 8000 }
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

    // Clean up child process when stdin closes (MCP host disconnected)
    if let Ok(mut guard) = child.lock() {
        if let Some(ref mut c) = *guard {
            let _ = c.kill();
        }
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
            let config = config_store.load_config();
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
            let llama_dir = config.llama_dir.ok_or("未配置 llama.cpp 路径")?;
            let gpu_layers = arguments.get("gpu_layers").and_then(|v| v.as_i64()).unwrap_or(99);
            let ctx_size = arguments.get("ctx_size").and_then(|v| v.as_i64()).unwrap_or(4096);
            let port = arguments.get("port").and_then(|v| v.as_i64()).unwrap_or(8000);

            let bin_path = crate::services::llama_detector::get_binary_path(&llama_dir, "llama-server");
            if !bin_path.exists() {
                return Err(format!("找不到 llama-server: {}", bin_path.display()));
            }

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
            cmd.args([
                "-m", &model_path,
                "-ngl", &gpu_layers.to_string(),
                "-c", &ctx_size.to_string(),
                "--host", "127.0.0.1",
                "--port", &port.to_string(),
            ]);

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
                "url": format!("http://127.0.0.1:{}", port)
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
