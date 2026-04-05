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
                    let content = convert_message_content(item.get("content"));
                    messages.push(json!({ "role": role, "content": content }));
                }
                Some("function_call") => {
                    // assistant 发出的工具调用 → assistant message with tool_use block
                    let id = item.get("id").or_else(|| item.get("call_id"))
                        .and_then(|v| v.as_str()).unwrap_or("unknown");
                    let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let input_obj: Value = item.get("arguments")
                        .and_then(|v| v.as_str())
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or(json!({}));

                    // 收集同一批次的所有 function_call（并行工具调用）
                    let mut tool_use_blocks = vec![json!({
                        "type": "tool_use",
                        "id": id,
                        "name": name,
                        "input": input_obj
                    })];
                    // 向前看，合并连续的 function_call
                    while i + 1 < input.len() {
                        let next = &input[i + 1];
                        if next.get("type").and_then(|v| v.as_str()) == Some("function_call") {
                            i += 1;
                            let nid = next.get("id").or_else(|| next.get("call_id"))
                                .and_then(|v| v.as_str()).unwrap_or("unknown");
                            let nname = next.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            let ninput: Value = next.get("arguments")
                                .and_then(|v| v.as_str())
                                .and_then(|s| serde_json::from_str(s).ok())
                                .unwrap_or(json!({}));
                            tool_use_blocks.push(json!({
                                "type": "tool_use",
                                "id": nid,
                                "name": nname,
                                "input": ninput
                            }));
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

                    // 收集同一批次的所有 function_call_output
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

    // 转换工具定义：parameters → input_schema
    let tools: Vec<Value> = req.get("tools")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(convert_tool_def).collect())
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

fn convert_message_content(content: Option<&Value>) -> Value {
    match content {
        None => json!(""),
        Some(Value::String(s)) => json!(s),
        Some(Value::Array(arr)) => {
            // content 数组：[{type:"input_text", text:"..."}, {type:"input_image",...}]
            let blocks: Vec<Value> = arr.iter().filter_map(|block| {
                match block.get("type").and_then(|v| v.as_str()) {
                    Some("input_text") | Some("text") => {
                        let text = block.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        Some(json!({ "type": "text", "text": text }))
                    }
                    Some("output_text") => {
                        let text = block.get("text").and_then(|v| v.as_str()).unwrap_or("");
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
            json!(blocks)
        }
        Some(other) => other.clone(),
    }
}

/// 将 Codex function_call_output.output 转换为 Anthropic tool_result content
/// output 可以是字符串或数组 [{type:"input_text",text:"..."}, ...]
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
    // Codex 用 "parameters"，Anthropic 用 "input_schema"
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
