//! Streaming (SSE) interception for Anthropic API responses.
//!
//! Buffers tool_use blocks until complete, then checks against rules.
//! Text blocks and other events pass through immediately.

use crate::rules::Rule;
use super::interceptor::{check_tool_use, InterceptResult, ApiProvider};
use crate::rules::RuleAction;
use serde_json::Value;
use tracing::{info, warn};

/// A parsed SSE event
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event_type: String,
    pub data: String,
}

impl SseEvent {
    /// Serialize back to SSE wire format (with trailing blank line)
    pub fn to_sse_bytes(&self) -> Vec<u8> {
        format!("event: {}\ndata: {}\n\n", self.event_type, self.data).into_bytes()
    }
}

/// Streaming interceptor state machine (multi-provider)
pub struct StreamInterceptor {
    rules: Vec<Rule>,
    enforce: bool,
    provider: Option<ApiProvider>,
    /// Index of the tool_use block currently being buffered (Anthropic)
    buffering_index: Option<usize>,
    /// Buffered SSE events for the current tool_use block
    buffer: Vec<SseEvent>,
    /// Tool name from content_block_start
    tool_name: String,
    /// Tool ID from content_block_start
    tool_id: String,
    /// Accumulated JSON fragments
    input_json_parts: Vec<String>,
    /// Collected intercept results for alerting
    pub intercepts: Vec<InterceptResult>,
    // --- OpenAI streaming state ---
    /// OpenAI: accumulated tool_calls by index
    openai_tool_calls: std::collections::HashMap<usize, OpenAiToolCallAccum>,
    /// OpenAI: buffered events while tool_calls are accumulating
    openai_buffer: Vec<SseEvent>,
    /// OpenAI: whether we're currently buffering tool_call deltas
    openai_buffering: bool,
    /// OpenAI: last seen chunk id for generating replacement events
    openai_chunk_id: String,
}

/// Accumulated OpenAI streaming tool call
#[derive(Debug, Clone, Default)]
struct OpenAiToolCallAccum {
    id: String,
    name: String,
    arguments: String,
}

impl StreamInterceptor {
    pub fn new(rules: Vec<Rule>, enforce: bool) -> Self {
        Self {
            rules,
            enforce,
            provider: None,
            buffering_index: None,
            buffer: Vec::new(),
            tool_name: String::new(),
            tool_id: String::new(),
            input_json_parts: Vec::new(),
            intercepts: Vec::new(),
            openai_tool_calls: std::collections::HashMap::new(),
            openai_buffer: Vec::new(),
            openai_buffering: false,
            openai_chunk_id: String::new(),
        }
    }

    /// Detect provider from the first meaningful SSE event
    fn detect_provider(&mut self, event: &SseEvent) {
        if self.provider.is_some() {
            return;
        }
        // OpenAI: data is JSON with "choices" or data is "[DONE]"
        if event.data == "[DONE]" {
            self.provider = Some(ApiProvider::OpenAI);
            return;
        }
        if let Ok(parsed) = serde_json::from_str::<Value>(&event.data) {
            if parsed.get("choices").is_some() {
                self.provider = Some(ApiProvider::OpenAI);
            } else if parsed.get("candidates").is_some() {
                self.provider = Some(ApiProvider::Gemini);
            } else if parsed.get("type").and_then(|t| t.as_str()).is_some() {
                // Anthropic uses event types like message_start, content_block_start
                self.provider = Some(ApiProvider::Anthropic);
            }
        }
        // Also detect from event_type field (Anthropic uses named events)
        if self.provider.is_none() {
            match event.event_type.as_str() {
                "message_start" | "content_block_start" | "content_block_delta" | "content_block_stop" | "message_delta" | "message_stop" => {
                    self.provider = Some(ApiProvider::Anthropic);
                }
                _ => {}
            }
        }
    }

    /// Process one SSE event. Returns events to send to the client.
    pub fn process_event(&mut self, event: SseEvent) -> Vec<SseEvent> {
        self.detect_provider(&event);

        match self.provider {
            Some(ApiProvider::OpenAI) => self.process_openai_event(event),
            Some(ApiProvider::Gemini) => self.process_gemini_event(event),
            _ => self.process_anthropic_event(event), // Default to Anthropic
        }
    }

    // --- Anthropic processing ---
    fn process_anthropic_event(&mut self, event: SseEvent) -> Vec<SseEvent> {
        match event.event_type.as_str() {
            "content_block_start" => self.handle_block_start(event),
            "content_block_delta" => self.handle_block_delta(event),
            "content_block_stop" => self.handle_block_stop(event),
            _ => vec![event],
        }
    }

    // --- OpenAI processing ---
    fn process_openai_event(&mut self, event: SseEvent) -> Vec<SseEvent> {
        if event.data.trim() == "[DONE]" {
            // Finalize: check accumulated tool calls
            let mut result_events = self.finalize_openai_tool_calls();
            result_events.push(event);
            return result_events;
        }

        let parsed: Value = match serde_json::from_str(&event.data) {
            Ok(v) => v,
            Err(_) => return vec![event],
        };

        if let Some(id) = parsed.get("id").and_then(|i| i.as_str()) {
            self.openai_chunk_id = id.to_string();
        }

        // Check if this chunk has tool_calls deltas
        let has_tool_calls = parsed.pointer("/choices/0/delta/tool_calls").and_then(|t| t.as_array()).is_some();
        let finish_reason = parsed.pointer("/choices/0/finish_reason").and_then(|f| f.as_str());

        if has_tool_calls {
            self.openai_buffering = true;
            // Accumulate tool call fragments
            if let Some(tool_calls) = parsed.pointer("/choices/0/delta/tool_calls").and_then(|t| t.as_array()) {
                for tc in tool_calls {
                    let index = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                    let entry = self.openai_tool_calls.entry(index).or_default();

                    if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                        entry.id = id.to_string();
                    }
                    if let Some(name) = tc.pointer("/function/name").and_then(|n| n.as_str()) {
                        entry.name = name.to_string();
                    }
                    if let Some(args) = tc.pointer("/function/arguments").and_then(|a| a.as_str()) {
                        entry.arguments.push_str(args);
                    }
                }
            }
            self.openai_buffer.push(event);
            return vec![];
        }

        if finish_reason == Some("tool_calls") {
            self.openai_buffer.push(event);
            return self.finalize_openai_tool_calls();
        }

        // No tool_calls: passthrough
        vec![event]
    }

    fn finalize_openai_tool_calls(&mut self) -> Vec<SseEvent> {
        if self.openai_tool_calls.is_empty() {
            let events = std::mem::take(&mut self.openai_buffer);
            return events;
        }

        let mut blocked_indices = std::collections::HashSet::new();

        // Check each accumulated tool call
        let mut sorted_indices: Vec<usize> = self.openai_tool_calls.keys().cloned().collect();
        sorted_indices.sort();

        for &idx in &sorted_indices {
            let tc = &self.openai_tool_calls[&idx];
            let input: Value = serde_json::from_str(&tc.arguments).unwrap_or(Value::Object(Default::default()));
            if let Some(result) = check_tool_use(idx, &tc.name, &input, &self.rules) {
                let should_block = matches!(result.action, RuleAction::CriticalAlert | RuleAction::PauseAndAsk);
                self.intercepts.push(result);
                if should_block {
                    blocked_indices.insert(idx);
                }
            }
        }

        if blocked_indices.is_empty() || !self.enforce {
            // Flush all buffered events
            let events = std::mem::take(&mut self.openai_buffer);
            self.openai_tool_calls.clear();
            return events;
        }

        // Generate replacement events: drop all buffered tool_call events, emit content message
        let block_msgs: Vec<String> = self.intercepts.iter()
            .filter(|i| matches!(i.action, RuleAction::CriticalAlert | RuleAction::PauseAndAsk))
            .map(|i| format!("🛡️ MoltBot Harness blocked this action: [{}] {} (rule: {})", i.tool_name, i.reason, i.rule_name))
            .collect();

        let replacement = serde_json::json!({
            "id": self.openai_chunk_id,
            "object": "chat.completion.chunk",
            "choices": [{"index": 0, "delta": {"content": block_msgs.join("\n")}, "finish_reason": null}]
        });
        let finish = serde_json::json!({
            "id": self.openai_chunk_id,
            "object": "chat.completion.chunk",
            "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}]
        });

        self.openai_buffer.clear();
        self.openai_tool_calls.clear();

        vec![
            SseEvent { event_type: "message".into(), data: replacement.to_string() },
            SseEvent { event_type: "message".into(), data: finish.to_string() },
        ]
    }

    // --- Gemini processing ---
    fn process_gemini_event(&mut self, event: SseEvent) -> Vec<SseEvent> {
        let parsed: Value = match serde_json::from_str(&event.data) {
            Ok(v) => v,
            Err(_) => return vec![event],
        };

        // Check for functionCall in parts
        let candidates = match parsed.get("candidates").and_then(|c| c.as_array()) {
            Some(arr) => arr,
            None => return vec![event],
        };

        let mut has_blocked = false;
        let mut modified = parsed.clone();

        for (ci, candidate) in candidates.iter().enumerate() {
            let parts = match candidate.pointer("/content/parts").and_then(|p| p.as_array()) {
                Some(arr) => arr,
                None => continue,
            };

            for (pi, part) in parts.iter().enumerate() {
                let fc = match part.get("functionCall") {
                    Some(fc) => fc,
                    None => continue,
                };
                let name = fc.get("name").and_then(|n| n.as_str()).unwrap_or_default();
                let args = fc.get("args").cloned().unwrap_or(Value::Object(Default::default()));

                let block_index = ci * 1000 + pi;
                if let Some(result) = check_tool_use(block_index, name, &args, &self.rules) {
                    let should_block = matches!(result.action, RuleAction::CriticalAlert | RuleAction::PauseAndAsk);
                    self.intercepts.push(result.clone());

                    if should_block && self.enforce {
                        has_blocked = true;
                        let block_msg = format!(
                            "🛡️ MoltBot Harness blocked this action: [{}] {} (rule: {})",
                            result.tool_name, result.reason, result.rule_name
                        );
                        modified.as_object_mut().unwrap()
                            .get_mut("candidates").unwrap()
                            .as_array_mut().unwrap()[ci]
                            .pointer_mut("/content/parts").unwrap()
                            .as_array_mut().unwrap()[pi] = serde_json::json!({"text": block_msg});
                    }
                }
            }
        }

        if has_blocked {
            vec![SseEvent {
                event_type: event.event_type,
                data: modified.to_string(),
            }]
        } else {
            vec![event]
        }
    }

    fn handle_block_start(&mut self, event: SseEvent) -> Vec<SseEvent> {
        // Parse to check if this is a tool_use block
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&event.data) {
            let block_type = parsed
                .pointer("/content_block/type")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            if block_type == "tool_use" {
                let index = parsed.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let name = parsed
                    .pointer("/content_block/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let id = parsed
                    .pointer("/content_block/id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                self.buffering_index = Some(index);
                self.buffer.clear();
                self.buffer.push(event);
                self.tool_name = name;
                self.tool_id = id;
                self.input_json_parts.clear();
                return vec![];
            }
        }
        // Not tool_use → passthrough
        vec![event]
    }

    fn handle_block_delta(&mut self, event: SseEvent) -> Vec<SseEvent> {
        if self.buffering_index.is_some() {
            // Accumulate JSON fragment
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&event.data) {
                if let Some(partial) = parsed
                    .pointer("/delta/partial_json")
                    .and_then(|v| v.as_str())
                {
                    self.input_json_parts.push(partial.to_string());
                }
            }
            self.buffer.push(event);
            vec![]
        } else {
            vec![event]
        }
    }

    fn handle_block_stop(&mut self, event: SseEvent) -> Vec<SseEvent> {
        if let Some(index) = self.buffering_index.take() {
            self.buffer.push(event);

            // Assemble full input JSON
            let full_json_str: String = self.input_json_parts.drain(..).collect();
            let input_value: serde_json::Value = serde_json::from_str(&full_json_str)
                .unwrap_or(serde_json::Value::Object(Default::default()));

            // Check against rules
            let result = check_tool_use(index, &self.tool_name, &input_value, &self.rules);

            let should_block = match &result {
                Some(r) => matches!(r.action, RuleAction::CriticalAlert | RuleAction::PauseAndAsk),
                None => false,
            };

            if let Some(r) = result {
                self.intercepts.push(r);
            }

            if should_block && self.enforce {
                let intercept = self.intercepts.last().unwrap();
                let block_msg = format!(
                    "🛡️ MoltBot Harness blocked this action: [{}] {} (rule: {})",
                    intercept.tool_name, intercept.reason, intercept.rule_name
                );

                // Return replacement text block events with same index
                let start_data = serde_json::json!({
                    "type": "content_block_start",
                    "index": index,
                    "content_block": {"type": "text", "text": ""}
                });
                let delta_data = serde_json::json!({
                    "type": "content_block_delta",
                    "index": index,
                    "delta": {"type": "text_delta", "text": block_msg}
                });
                let stop_data = serde_json::json!({
                    "type": "content_block_stop",
                    "index": index
                });

                self.buffer.clear();
                vec![
                    SseEvent { event_type: "content_block_start".into(), data: start_data.to_string() },
                    SseEvent { event_type: "content_block_delta".into(), data: delta_data.to_string() },
                    SseEvent { event_type: "content_block_stop".into(), data: stop_data.to_string() },
                ]
            } else {
                // Safe or monitor mode → flush buffer
                let events = std::mem::take(&mut self.buffer);
                events
            }
        } else {
            vec![event]
        }
    }
}

/// Parse a raw SSE text chunk into events.
/// SSE events are separated by blank lines. Each event has optional `event:` and `data:` lines.
pub fn parse_sse_events(raw: &str) -> Vec<SseEvent> {
    let mut events = Vec::new();
    let mut event_type = String::new();
    let mut data = String::new();

    for line in raw.lines() {
        if line.is_empty() {
            // Blank line = end of event
            if !event_type.is_empty() || !data.is_empty() {
                events.push(SseEvent {
                    event_type: if event_type.is_empty() { "message".into() } else { event_type },
                    data,
                });
                event_type = String::new();
                data = String::new();
            }
        } else if let Some(val) = line.strip_prefix("event: ").or_else(|| line.strip_prefix("event:")) {
            event_type = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("data: ").or_else(|| line.strip_prefix("data:")) {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(val);
        }
    }

    // Handle trailing event without final blank line
    if !event_type.is_empty() || !data.is_empty() {
        events.push(SseEvent {
            event_type: if event_type.is_empty() { "message".into() } else { event_type },
            data,
        });
    }

    events
}

/// Line buffer for accumulating SSE chunks across network boundaries.
/// Yields complete SSE event text blocks (delimited by blank lines).
pub struct SseLineBuffer {
    buf: String,
}

impl SseLineBuffer {
    pub fn new() -> Self {
        Self { buf: String::new() }
    }

    /// Feed a chunk of bytes. Returns complete SSE event blocks ready for parsing.
    pub fn feed(&mut self, chunk: &str) -> Vec<String> {
        self.buf.push_str(chunk);
        let mut results = Vec::new();

        // Split on double newline (SSE event boundary)
        while let Some(pos) = self.buf.find("\n\n") {
            let event_block = self.buf[..pos].to_string();
            self.buf = self.buf[pos + 2..].to_string();
            if !event_block.trim().is_empty() {
                // Re-add the trailing \n\n so parse_sse_events sees blank line
                results.push(format!("{}\n\n", event_block));
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::default_rules;

    fn get_rules() -> Vec<Rule> {
        let mut rules = default_rules();
        for r in &mut rules {
            let _ = r.compile();
        }
        rules
    }

    fn make_event(event_type: &str, data: &str) -> SseEvent {
        SseEvent { event_type: event_type.to_string(), data: data.to_string() }
    }

    #[test]
    fn test_text_only_passthrough() {
        let rules = get_rules();
        let mut interceptor = StreamInterceptor::new(rules, true);

        let events = vec![
            make_event("message_start", r#"{"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-20250514","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}"#),
            make_event("content_block_start", r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#),
            make_event("content_block_delta", r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello world"}}"#),
            make_event("content_block_stop", r#"{"type":"content_block_stop","index":0}"#),
            make_event("message_delta", r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":5}}"#),
            make_event("message_stop", r#"{"type":"message_stop"}"#),
        ];

        let mut output = Vec::new();
        for e in events {
            output.extend(interceptor.process_event(e));
        }

        assert_eq!(output.len(), 6);
        assert_eq!(output[0].event_type, "message_start");
        assert_eq!(output[1].event_type, "content_block_start");
        assert!(interceptor.intercepts.is_empty());
    }

    #[test]
    fn test_safe_tool_use_passthrough() {
        let rules = get_rules();
        let mut interceptor = StreamInterceptor::new(rules, true);

        let events = vec![
            make_event("message_start", r#"{"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-20250514","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}"#),
            make_event("content_block_start", r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"exec"}}"#),
            make_event("content_block_delta", r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"command\": \"ls -la\"}"}}"#),
            make_event("content_block_stop", r#"{"type":"content_block_stop","index":0}"#),
            make_event("message_stop", r#"{"type":"message_stop"}"#),
        ];

        let mut output = Vec::new();
        for e in events {
            output.extend(interceptor.process_event(e));
        }

        // message_start + 3 buffered (flushed) + message_stop = 5
        assert_eq!(output.len(), 5);
        assert_eq!(output[1].event_type, "content_block_start");
    }

    #[test]
    fn test_dangerous_tool_use_blocked() {
        let rules = get_rules();
        let mut interceptor = StreamInterceptor::new(rules, true);

        let events = vec![
            make_event("message_start", r#"{"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-20250514","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}"#),
            make_event("content_block_start", r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"exec"}}"#),
            make_event("content_block_delta", r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"com"}}"#),
            make_event("content_block_delta", r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"mand\": \"rm -rf /\"}"}}"#),
            make_event("content_block_stop", r#"{"type":"content_block_stop","index":0}"#),
            make_event("message_stop", r#"{"type":"message_stop"}"#),
        ];

        let mut output = Vec::new();
        for e in events {
            output.extend(interceptor.process_event(e));
        }

        // message_start + 3 replacement events + message_stop = 5
        assert_eq!(output.len(), 5);
        assert_eq!(output[1].event_type, "content_block_start");
        // Check that the replacement is a text block
        let start_data: serde_json::Value = serde_json::from_str(&output[1].data).unwrap();
        assert_eq!(start_data.pointer("/content_block/type").unwrap(), "text");
        // Check delta has blocked message
        let delta_data: serde_json::Value = serde_json::from_str(&output[2].data).unwrap();
        let text = delta_data.pointer("/delta/text").unwrap().as_str().unwrap();
        assert!(text.contains("MoltBot Harness blocked"));
        assert!(!interceptor.intercepts.is_empty());
    }

    #[test]
    fn test_mixed_blocks() {
        let rules = get_rules();
        let mut interceptor = StreamInterceptor::new(rules, true);

        let events = vec![
            make_event("message_start", r#"{"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-20250514","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}"#),
            // Text block (index 0)
            make_event("content_block_start", r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#),
            make_event("content_block_delta", r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Let me help"}}"#),
            make_event("content_block_stop", r#"{"type":"content_block_stop","index":0}"#),
            // Dangerous tool_use (index 1)
            make_event("content_block_start", r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_1","name":"exec"}}"#),
            make_event("content_block_delta", r#"{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"command\": \"rm -rf /\"}"}}"#),
            make_event("content_block_stop", r#"{"type":"content_block_stop","index":1}"#),
            // Safe tool_use (index 2)
            make_event("content_block_start", r#"{"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"toolu_2","name":"exec"}}"#),
            make_event("content_block_delta", r#"{"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"command\": \"ls -la\"}"}}"#),
            make_event("content_block_stop", r#"{"type":"content_block_stop","index":2}"#),
            make_event("message_stop", r#"{"type":"message_stop"}"#),
        ];

        let mut output = Vec::new();
        for e in events {
            output.extend(interceptor.process_event(e));
        }

        // message_start(1) + text block(3) + blocked replacement(3) + safe tool(3) + message_stop(1) = 11
        assert_eq!(output.len(), 11);

        // Text block passes through
        assert_eq!(output[1].event_type, "content_block_start");

        // Dangerous block replaced
        let blocked_start: serde_json::Value = serde_json::from_str(&output[4].data).unwrap();
        assert_eq!(blocked_start.pointer("/content_block/type").unwrap(), "text");

        // Safe tool passes through
        let safe_start: serde_json::Value = serde_json::from_str(&output[7].data).unwrap();
        assert_eq!(safe_start.pointer("/content_block/type").unwrap(), "tool_use");

        assert_eq!(interceptor.intercepts.len(), 1);
    }

    #[test]
    fn test_parse_sse_events() {
        let raw = "event: message_start\ndata: {\"type\":\"message_start\"}\n\nevent: content_block_start\ndata: {\"type\":\"content_block_start\"}\n\n";
        let events = parse_sse_events(raw);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "message_start");
        assert_eq!(events[1].event_type, "content_block_start");
    }

    #[test]
    fn test_sse_line_buffer() {
        let mut buf = SseLineBuffer::new();

        // Partial chunk
        let r1 = buf.feed("event: message_start\ndata: {\"type\"");
        assert!(r1.is_empty());

        // Complete the event
        let r2 = buf.feed(":\"message_start\"}\n\nevent: ping\ndata: {}\n\n");
        assert_eq!(r2.len(), 2);
    }

    // --- OpenAI streaming tests ---

    #[test]
    fn test_openai_streaming_block() {
        let rules = get_rules();
        let mut interceptor = StreamInterceptor::new(rules, true);

        let events = vec![
            make_event("message", r#"{"id":"chatcmpl-1","choices":[{"delta":{"role":"assistant"},"index":0}]}"#),
            make_event("message", r#"{"id":"chatcmpl-1","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"exec","arguments":""}}]},"index":0}]}"#),
            make_event("message", r#"{"id":"chatcmpl-1","choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"com"}}]},"index":0}]}"#),
            make_event("message", r#"{"id":"chatcmpl-1","choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"mand\": \"rm -rf /\"}"}}]},"index":0}]}"#),
            make_event("message", r#"{"id":"chatcmpl-1","choices":[{"delta":{},"index":0,"finish_reason":"tool_calls"}]}"#),
        ];

        let mut output = Vec::new();
        for e in events {
            output.extend(interceptor.process_event(e));
        }

        // First event (role) passes through, tool_call events buffered then replaced
        assert!(!interceptor.intercepts.is_empty());
        // Should have replacement content events instead of tool_call events
        let has_blocked = output.iter().any(|e| e.data.contains("MoltBot Harness blocked"));
        assert!(has_blocked, "Should contain block message, got: {:?}", output.iter().map(|e| &e.data).collect::<Vec<_>>());
    }

    #[test]
    fn test_openai_streaming_passthrough() {
        let rules = get_rules();
        let mut interceptor = StreamInterceptor::new(rules, true);

        let events = vec![
            make_event("message", r#"{"id":"chatcmpl-1","choices":[{"delta":{"role":"assistant"},"index":0}]}"#),
            make_event("message", r#"{"id":"chatcmpl-1","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"exec","arguments":""}}]},"index":0}]}"#),
            make_event("message", r#"{"id":"chatcmpl-1","choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"command\": \"ls -la\"}"}}]},"index":0}]}"#),
            make_event("message", r#"{"id":"chatcmpl-1","choices":[{"delta":{},"index":0,"finish_reason":"tool_calls"}]}"#),
        ];

        let mut output = Vec::new();
        for e in events {
            output.extend(interceptor.process_event(e));
        }

        assert!(interceptor.intercepts.is_empty());
        // All buffered events flushed
        let has_blocked = output.iter().any(|e| e.data.contains("MoltBot Harness blocked"));
        assert!(!has_blocked);
    }

    // --- Gemini streaming tests ---

    #[test]
    fn test_gemini_streaming_block() {
        let rules = get_rules();
        let mut interceptor = StreamInterceptor::new(rules, true);

        let event = make_event("message", r#"{"candidates":[{"content":{"parts":[{"functionCall":{"name":"exec","args":{"command":"rm -rf /"}}}]},"finishReason":"STOP"}]}"#);

        let output = interceptor.process_event(event);
        assert!(!interceptor.intercepts.is_empty());
        assert!(output[0].data.contains("MoltBot Harness blocked"));
    }

    #[test]
    fn test_gemini_streaming_passthrough() {
        let rules = get_rules();
        let mut interceptor = StreamInterceptor::new(rules, true);

        let event = make_event("message", r#"{"candidates":[{"content":{"parts":[{"functionCall":{"name":"exec","args":{"command":"ls -la"}}}]},"finishReason":"STOP"}]}"#);

        let output = interceptor.process_event(event);
        assert!(interceptor.intercepts.is_empty());
        assert!(!output[0].data.contains("MoltBot Harness blocked"));
    }

    #[test]
    fn test_monitor_mode_no_block() {
        let rules = get_rules();
        let mut interceptor = StreamInterceptor::new(rules, false); // enforce=false

        let events = vec![
            make_event("content_block_start", r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"exec"}}"#),
            make_event("content_block_delta", r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"command\": \"rm -rf /\"}"}}"#),
            make_event("content_block_stop", r#"{"type":"content_block_stop","index":0}"#),
        ];

        let mut output = Vec::new();
        for e in events {
            output.extend(interceptor.process_event(e));
        }

        // Monitor mode: all 3 original events flushed (not replaced)
        assert_eq!(output.len(), 3);
        let start_data: serde_json::Value = serde_json::from_str(&output[0].data).unwrap();
        assert_eq!(start_data.pointer("/content_block/type").unwrap(), "tool_use");
        // But intercept is still recorded
        assert_eq!(interceptor.intercepts.len(), 1);
    }
}
