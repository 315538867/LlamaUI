use serde_json::{json, Value};

/// 将 Codex CLI (OpenAI Responses API) 请求转换为 Anthropic Messages API 请求
pub fn codex_to_anthropic(req: &Value) -> Value {
    let mut messages: Vec<Value> = Vec::new();

    // 转换 input[] → messages[]
    if let Some(input) = req.get("input").and_then(|v| v.as_array()) {
        let mut i = 0;
        while i < input.len() {
            let item = &input[i];
            match item.get("type").and_then(|v| v.as_str()) {
                Some("message") => {
                    let role = item.get("role").and_then(|v| v.as_str()).unwrap_or("user");
                    let mut content_blocks = convert_message_content_blocks(item.get("content"));

                    // assistant 消息：向前看并合并紧跟的 function_call，避免两条连续 assistant 消息
                    if role == "assistant" {
                        while i + 1 < input.len() {
                            let next = &input[i + 1];
                            if next.get("type").and_then(|v| v.as_str()) == Some("function_call") {
                                i += 1;
                                content_blocks.push(make_tool_use_block(next));
                            } else {
                                break;
                            }
                        }
                    }

                    let content: Value = if content_blocks.len() == 1 {
                        // 单个纯文本块时退化为字符串，保持简洁
                        if content_blocks[0].get("type").and_then(|v| v.as_str()) == Some("text") {
                            content_blocks[0]["text"].clone()
                        } else {
                            json!(content_blocks)
                        }
                    } else {
                        json!(content_blocks)
                    };

                    messages.push(json!({ "role": role, "content": content }));
                }
                Some("function_call") => {
                    // 孤立的 function_call（没有前置 assistant message）
                    let mut tool_use_blocks = vec![make_tool_use_block(item)];
                    while i + 1 < input.len() {
                        let next = &input[i + 1];
                        if next.get("type").and_then(|v| v.as_str()) == Some("function_call") {
                            i += 1;
                            tool_use_blocks.push(make_tool_use_block(next));
                        } else {
                            break;
                        }
                    }
                    messages.push(json!({ "role": "assistant", "content": tool_use_blocks }));
                }
                Some("function_call_output") => {
                    // 工具结果 → user message with tool_result block
                    let call_id = item.get("call_id").and_then(|v| v.as_str()).unwrap_or("");
                    let content = convert_tool_output(item.get("output"));

                    let mut result_blocks = vec![json!({
                        "type": "tool_result",
                        "tool_use_id": call_id,
                        "content": content
                    })];
                    while i + 1 < input.len() {
                        let next = &input[i + 1];
                        if next.get("type").and_then(|v| v.as_str()) == Some("function_call_output") {
                            i += 1;
                            let ncid = next.get("call_id").and_then(|v| v.as_str()).unwrap_or("");
                            let ncontent = convert_tool_output(next.get("output"));
                            result_blocks.push(json!({
                                "type": "tool_result",
                                "tool_use_id": ncid,
                                "content": ncontent
                            }));
                        } else {
                            break;
                        }
                    }
                    messages.push(json!({ "role": "user", "content": result_blocks }));
                }
                _ => {}
            }
            i += 1;
        }
    }

    // 转换工具定义：只保留 type="function" 的工具，parameters → input_schema
    let tools: Vec<Value> = req.get("tools")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter()
            .filter(|t| t.get("type").and_then(|v| v.as_str()) == Some("function"))
            .map(convert_tool_def)
            .collect())
        .unwrap_or_default();

    let mut anthropic_req = json!({
        "messages": messages,
        "stream": true,
        "max_tokens": req.get("max_output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(4096),
    });

    // model
    if let Some(model) = req.get("model").and_then(|v| v.as_str()) {
        anthropic_req["model"] = json!(model);
    }

    // instructions → system
    if let Some(sys) = req.get("instructions").and_then(|v| v.as_str()) {
        if !sys.is_empty() {
            anthropic_req["system"] = json!(sys);
        }
    }

    if !tools.is_empty() {
        anthropic_req["tools"] = json!(tools);
    }

    anthropic_req
}

/// 将一个 function_call item 转为 Anthropic tool_use block
fn make_tool_use_block(item: &Value) -> Value {
    let id = item.get("call_id").or_else(|| item.get("id"))
        .and_then(|v| v.as_str()).unwrap_or("unknown");
    let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let input_obj: Value = item.get("arguments")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(json!({}));
    json!({
        "type": "tool_use",
        "id": id,
        "name": name,
        "input": input_obj
    })
}

/// 将 message.content 转为 Anthropic content blocks Vec
fn convert_message_content_blocks(content: Option<&Value>) -> Vec<Value> {
    match content {
        None => vec![json!({ "type": "text", "text": "" })],
        Some(Value::String(s)) => vec![json!({ "type": "text", "text": s })],
        Some(Value::Array(arr)) => {
            let blocks: Vec<Value> = arr.iter().filter_map(|block| {
                match block.get("type").and_then(|v| v.as_str()) {
                    Some("input_text") | Some("text") | Some("output_text") => {
                        let text = block.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        Some(json!({ "type": "text", "text": text }))
                    }
                    _ => None,
                }
            }).collect();
            if blocks.is_empty() {
                vec![json!({ "type": "text", "text": "" })]
            } else {
                blocks
            }
        }
        Some(other) => vec![json!({ "type": "text", "text": other.as_str().unwrap_or("") })],
    }
}

/// 将 Codex function_call_output.output 转换为 Anthropic tool_result content
fn convert_tool_output(output: Option<&Value>) -> Value {
    match output {
        None => json!(""),
        Some(Value::String(s)) => json!(s),
        Some(Value::Array(arr)) => {
            let blocks: Vec<Value> = arr.iter().filter_map(|item| {
                match item.get("type").and_then(|v| v.as_str()) {
                    Some("input_text") | Some("text") | Some("output_text") => {
                        let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        Some(json!({ "type": "text", "text": text }))
                    }
                    _ => None,
                }
            }).collect();
            if blocks.len() == 1 {
                if let Some(text) = blocks[0].get("text").and_then(|v| v.as_str()) {
                    return json!(text);
                }
            }
            if blocks.is_empty() { json!("") } else { json!(blocks) }
        }
        Some(other) => other.clone(),
    }
}

fn convert_tool_def(tool: &Value) -> Value {
    let name = tool.get("name").cloned().unwrap_or(json!(""));
    let description = tool.get("description").cloned().unwrap_or(json!(""));
    let input_schema = tool.get("parameters")
        .or_else(|| tool.get("input_schema"))
        .cloned()
        .unwrap_or(json!({ "type": "object", "properties": {} }));

    json!({
        "name": name,
        "description": description,
        "input_schema": input_schema,
    })
}
