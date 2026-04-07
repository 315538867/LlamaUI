use serde_json::{json, Value};
use uuid::Uuid;

/// Anthropic SSE → Codex SSE 状态机
pub struct SseConverter {
    response_id: String,
    state: State,
    output_index: u32,
    text_buf: String,
}

enum State {
    Idle,
    InText { index: u32 },
    InToolCall { index: u32, id: String, name: String, args_buf: String },
    InThinking,
}

impl SseConverter {
    pub fn new() -> Self {
        Self {
            response_id: format!("resp_{}", Uuid::new_v4().simple()),
            state: State::Idle,
            output_index: 0,
            text_buf: String::new(),
        }
    }

    /// 返回流开始时需要发送的 response.created 事件
    pub fn created_event(&self) -> String {
        let data = json!({
            "type": "response.created",
            "response": {
                "id": self.response_id,
                "object": "realtime.response",
                "status": "in_progress",
                "output": []
            }
        });
        format!("event: response.created\ndata: {}\n\n", data)
    }

    /// 处理一行 Anthropic SSE 数据，返回零或多个 Codex SSE 事件字符串
    pub fn feed_line(&mut self, line: &str) -> Vec<String> {
        // 只处理 "data: {...}" 行
        let json_str = if let Some(s) = line.strip_prefix("data: ") {
            s.trim()
        } else {
            return vec![];
        };

        let event: Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match event_type {
            "content_block_start" => self.handle_block_start(&event),
            "content_block_delta" => self.handle_block_delta(&event),
            "content_block_stop" => self.handle_block_stop(&event),
            "message_stop" => self.handle_message_stop(),
            _ => vec![],
        }
    }

    fn handle_block_start(&mut self, event: &Value) -> Vec<String> {
        let block = match event.get("content_block") {
            Some(b) => b,
            None => return vec![],
        };
        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let index = self.output_index;
        self.output_index += 1;

        match block_type {
            "text" => {
                self.state = State::InText { index };
                self.text_buf.clear();
                let data = json!({
                    "type": "response.output_item.added",
                    "output_index": index,
                    "item": {
                        "type": "message",
                        "id": format!("msg_{}", index),
                        "role": "assistant",
                        "content": []
                    }
                });
                vec![format!("event: response.output_item.added\ndata: {}\n\n", data)]
            }
            "tool_use" => {
                let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();

                // 发送 response.output_item.added（空 arguments）
                let data = json!({
                    "type": "response.output_item.added",
                    "output_index": index,
                    "item": {
                        "type": "function_call",
                        "id": id,
                        "call_id": id,
                        "name": name,
                        "arguments": ""
                    }
                });
                self.state = State::InToolCall { index, id, name, args_buf: String::new() };
                vec![format!("event: response.output_item.added\ndata: {}\n\n", data)]
            }
            "thinking" => {
                self.state = State::InThinking;
                // 发一个 SSE comment 作为 keepalive，防止 LAN NAT 超时
                vec![": thinking\n\n".to_string()]
            }
            _ => vec![],
        }
    }

    fn handle_block_delta(&mut self, event: &Value) -> Vec<String> {
        let delta = match event.get("delta") {
            Some(d) => d,
            None => return vec![],
        };
        let delta_type = delta.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match delta_type {
            "text_delta" => {
                let text = delta.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let index = if let State::InText { index } = &self.state { *index } else { 0 };
                self.text_buf.push_str(text);
                let data = json!({
                    "type": "response.output_text.delta",
                    "output_index": index,
                    "content_index": 0,
                    "delta": text
                });
                vec![format!("event: response.output_text.delta\ndata: {}\n\n", data)]
            }
            "input_json_delta" => {
                let partial = delta.get("partial_json").and_then(|v| v.as_str()).unwrap_or("");
                if let State::InToolCall { args_buf, .. } = &mut self.state {
                    args_buf.push_str(partial);
                }
                vec![]
            }
            "thinking_delta" => {
                // thinking 内容不转发给 Codex，但发 SSE comment 保持连接活跃
                vec![": thinking\n\n".to_string()]
            }
            _ => vec![],
        }
    }

    fn handle_block_stop(&mut self, _event: &Value) -> Vec<String> {
        let prev = std::mem::replace(&mut self.state, State::Idle);
        match prev {
            State::InText { index } => {
                let full_text = std::mem::take(&mut self.text_buf);
                let done_text = json!({
                    "type": "response.output_text.done",
                    "output_index": index,
                    "content_index": 0,
                    "text": &full_text
                });
                let done_item = json!({
                    "type": "response.output_item.done",
                    "output_index": index,
                    "item": {
                        "type": "message",
                        "id": format!("msg_{}", index),
                        "role": "assistant",
                        "content": [{
                            "type": "output_text",
                            "text": &full_text
                        }]
                    }
                });
                vec![
                    format!("event: response.output_text.done\ndata: {}\n\n", done_text),
                    format!("event: response.output_item.done\ndata: {}\n\n", done_item),
                ]
            }
            State::InToolCall { index, id, name, args_buf } => {
                let data = json!({
                    "type": "response.output_item.done",
                    "output_index": index,
                    "item": {
                        "type": "function_call",
                        "id": id,
                        "call_id": id,
                        "name": name,
                        "arguments": args_buf
                    }
                });
                vec![format!("event: response.output_item.done\ndata: {}\n\n", data)]
            }
            _ => vec![],
        }
    }

    fn handle_message_stop(&self) -> Vec<String> {
        let data = json!({
            "type": "response.completed",
            "response": {
                "id": self.response_id,
                "object": "realtime.response",
                "status": "completed",
                "output": []
            }
        });
        vec![format!("event: response.completed\ndata: {}\n\n", data)]
    }
}
