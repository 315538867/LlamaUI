use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::sync::Arc;

use crate::services::config_store::ConfigStore;
use crate::services::model_scanner;

/// MCP Server running over stdio (JSON-RPC 2.0)
/// This is intended to be launched as a separate process or CLI subcommand
/// for Claude Code / Codex tool integration.

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
            "description": "启动 llama.cpp 模型服务",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "model": { "type": "string", "description": "模型名称或路径" },
                    "mode": { "type": "string", "enum": ["server", "cli"], "default": "server" },
                    "gpu_layers": { "type": "integer", "description": "GPU 层数", "default": 99 },
                    "ctx_size": { "type": "integer", "description": "上下文长度", "default": 4096 },
                    "port": { "type": "integer", "description": "Server 端口", "default": 8080 }
                },
                "required": ["model"]
            }
        },
        {
            "name": "llamaui_stop",
            "description": "停止当前运行的 llama.cpp 模型"
        },
        {
            "name": "llamaui_status",
            "description": "查询 llama.cpp 模型运行状态"
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
                let _ = writeln!(&stdout, "{}", serde_json::to_string(&response).unwrap());
                continue;
            }
        };

        let id = request.id.unwrap_or(Value::Null);
        let response = handle_request(&request.method, request.params, &config_store, id.clone());

        let _ = writeln!(&stdout, "{}", serde_json::to_string(&response).unwrap());
        let _ = stdout.lock().flush();
    }
}

fn handle_request(
    method: &str,
    params: Option<Value>,
    config_store: &ConfigStore,
    id: Value,
) -> JsonRpcResponse {
    match method {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": SERVER_VERSION
                }
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

            let result = handle_tool_call(tool_name, &arguments, config_store);
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
) -> Result<String, String> {
    match name {
        "llamaui_status" => {
            // In stdio mode we can't access the ProcessManager directly
            // Return config-based info instead
            Ok(json!({
                "message": "Status query via MCP - use the GUI for real-time process status",
                "config": config_store.load_config()
            })
            .to_string())
        }

        "llamaui_list_models" => {
            let config = config_store.load_config();
            let mut all_models = Vec::new();
            for dir in &config.model_dirs {
                if let Ok(mut models) = model_scanner::scan_directory(dir) {
                    all_models.append(&mut models);
                }
            }
            Ok(serde_json::to_string_pretty(&all_models)
                .unwrap_or_else(|_| "[]".into()))
        }

        "llamaui_get_config" => {
            let config = config_store.load_config();
            Ok(serde_json::to_string_pretty(&config)
                .unwrap_or_else(|_| "{}".into()))
        }

        "llamaui_start" => {
            let model = arguments
                .get("model")
                .and_then(|v| v.as_str())
                .ok_or("缺少 model 参数")?;

            // In stdio MCP mode, we provide the command to run
            let config = config_store.load_config();
            let llama_dir = config.llama_dir.ok_or("未配置 llama.cpp 路径")?;
            let mode = arguments
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("server");
            let gpu_layers = arguments.get("gpu_layers").and_then(|v| v.as_i64()).unwrap_or(99);
            let ctx_size = arguments.get("ctx_size").and_then(|v| v.as_i64()).unwrap_or(4096);
            let port = arguments.get("port").and_then(|v| v.as_i64()).unwrap_or(8080);

            let binary = if mode == "cli" { "llama-cli" } else { "llama-server" };
            let bin_path = crate::services::llama_detector::get_binary_path(&llama_dir, binary);

            let mut cmd = format!(
                "{} -m {} -ngl {} -c {}",
                bin_path.display(),
                model,
                gpu_layers,
                ctx_size
            );
            if mode == "server" {
                cmd.push_str(&format!(" --host 127.0.0.1 --port {}", port));
            }

            Ok(json!({
                "message": format!("请在终端执行以下命令启动模型，或打开 LlamaUI GUI 操作"),
                "command": cmd,
                "model": model,
                "mode": mode,
                "port": if mode == "server" { Some(port) } else { None }
            })
            .to_string())
        }

        "llamaui_stop" => {
            Ok(json!({
                "message": "请在 LlamaUI GUI 中停止模型，或手动终止进程"
            })
            .to_string())
        }

        _ => Err(format!("未知工具: {}", name)),
    }
}
