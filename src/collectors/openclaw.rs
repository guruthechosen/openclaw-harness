//! OpenClaw (formerly Clawdbot) log collector
//!
//! Monitors:
//! - ~/.clawdbot/agents/main/sessions/*.jsonl (session logs)

use super::super::{AgentAction, AgentType, ActionType};
// When compiled as part of lib, use super's parent
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, debug, warn, error};

/// Collector for OpenClaw/Clawdbot
pub struct OpenclawCollector {
    sessions_dir: PathBuf,
    /// Track file positions to only read new content
    file_positions: Arc<Mutex<std::collections::HashMap<PathBuf, u64>>>,
    /// Track seen action IDs to avoid duplicates
    seen_ids: Arc<Mutex<HashSet<String>>>,
}

impl OpenclawCollector {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        Self {
            sessions_dir: home.join(".clawdbot/agents/main/sessions"),
            file_positions: Arc::new(Mutex::new(std::collections::HashMap::new())),
            seen_ids: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Parse a JSONL session log line and extract tool calls
    fn parse_log_line(&self, line: &str) -> Vec<AgentAction> {
        let mut actions = Vec::new();
        
        let entry: OpenclawLogEntry = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(_) => return actions,
        };

        // Only process message entries
        if entry.entry_type != "message" {
            return actions;
        }

        let message = match entry.message {
            Some(m) => m,
            None => return actions,
        };

        // Only process assistant messages (which contain tool calls)
        if message.role != "assistant" {
            return actions;
        }

        // Extract tool calls from content
        for content in message.content {
            if content.content_type == "toolCall" {
                if let Some(tool_call) = content.into_tool_call() {
                    let action_type = match tool_call.name.as_str() {
                        "exec" => ActionType::Exec,
                        "Read" | "read" => ActionType::FileRead,
                        "Write" | "write" => ActionType::FileWrite,
                        "Edit" | "edit" => ActionType::FileWrite,
                        "web_fetch" | "web_search" => ActionType::HttpRequest,
                        "browser" => ActionType::BrowserAction,
                        "message" => ActionType::MessageSend,
                        _ => ActionType::Unknown,
                    };

                    // Extract relevant content from arguments
                    let (content, target) = extract_content_and_target(&tool_call);

                    actions.push(AgentAction {
                        id: tool_call.id,
                        timestamp: chrono::DateTime::parse_from_rfc3339(&entry.timestamp)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .unwrap_or_else(|_| chrono::Utc::now()),
                        agent: AgentType::OpenClaw,
                        action_type,
                        content,
                        target,
                        session_id: Some(entry.id.clone()),
                        metadata: tool_call.arguments,
                    });
                }
            }
        }

        actions
    }

    /// Read new lines from a file
    async fn read_new_lines(&self, path: &PathBuf) -> Vec<String> {
        let mut positions = self.file_positions.lock().await;
        let current_pos = positions.get(path).copied().unwrap_or(0);

        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to open log file {:?}: {}", path, e);
                return vec![];
            }
        };

        let metadata = match file.metadata() {
            Ok(m) => m,
            Err(_) => return vec![],
        };

        let file_size = metadata.len();
        
        // If file is smaller than our position, it was truncated/rotated
        let start_pos = if file_size < current_pos { 0 } else { current_pos };

        let mut reader = BufReader::new(file);
        if reader.seek(SeekFrom::Start(start_pos)).is_err() {
            return vec![];
        }

        let mut lines = Vec::new();
        let mut new_pos = start_pos;

        for line in reader.lines() {
            match line {
                Ok(l) => {
                    new_pos += l.len() as u64 + 1; // +1 for newline
                    if !l.is_empty() {
                        lines.push(l);
                    }
                }
                Err(_) => break,
            }
        }

        positions.insert(path.clone(), new_pos);
        lines
    }
    
    /// Get all JSONL files in sessions directory
    fn get_session_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.sessions_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "jsonl") {
                    files.push(path);
                }
            }
        }
        files
    }
}

fn extract_content_and_target(tool_call: &ToolCall) -> (String, Option<String>) {
    let args = match &tool_call.arguments {
        Some(a) => a,
        None => return (String::new(), None),
    };

    match tool_call.name.as_str() {
        "exec" => {
            let cmd = args.get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (cmd, None)
        }
        "Read" | "read" => {
            let path = args.get("path")
                .or_else(|| args.get("file_path"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (format!("read {}", path.as_deref().unwrap_or("")), path)
        }
        "Write" | "write" => {
            let path = args.get("path")
                .or_else(|| args.get("file_path"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (format!("write {}", path.as_deref().unwrap_or("")), path)
        }
        "Edit" | "edit" => {
            let path = args.get("path")
                .or_else(|| args.get("file_path"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (format!("edit {}", path.as_deref().unwrap_or("")), path)
        }
        "web_fetch" => {
            let url = args.get("url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (format!("fetch {}", url.as_deref().unwrap_or("")), url)
        }
        "web_search" => {
            let query = args.get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (format!("search: {}", query), None)
        }
        "browser" => {
            let action = args.get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let url = args.get("targetUrl")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (format!("browser:{}", action), url)
        }
        "message" => {
            let target = args.get("target")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let msg = args.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (msg, target)
        }
        _ => {
            (serde_json::to_string(args).unwrap_or_default(), None)
        }
    }
}

#[async_trait]
impl super::Collector for OpenclawCollector {
    fn name(&self) -> &'static str {
        "openclaw"
    }

    async fn start(&self, tx: mpsc::Sender<AgentAction>) -> anyhow::Result<()> {
        info!("🦞 Starting OpenClaw collector, watching: {:?}", self.sessions_dir);

        if !self.sessions_dir.exists() {
            warn!("OpenClaw sessions directory not found: {:?}", self.sessions_dir);
            return Ok(());
        }

        // Initialize file positions to end of existing files
        for path in self.get_session_files() {
            if let Ok(metadata) = std::fs::metadata(&path) {
                let mut positions = self.file_positions.lock().await;
                positions.insert(path, metadata.len());
            }
        }

        info!("OpenClaw collector started, monitoring for new tool calls...");

        // Polling-based monitoring (simpler than notify for now)
        let poll_interval = tokio::time::Duration::from_millis(500);
        
        loop {
            tokio::time::sleep(poll_interval).await;
            
            for path in self.get_session_files() {
                let lines = self.read_new_lines(&path).await;
                
                if lines.is_empty() {
                    continue;
                }
                
                debug!("Processing {} new lines from {:?}", lines.len(), path);
                
                let mut seen = self.seen_ids.lock().await;
                
                for line in lines {
                    let actions = self.parse_log_line(&line);
                    for action in actions {
                        // Avoid duplicates
                        if seen.contains(&action.id) {
                            continue;
                        }
                        seen.insert(action.id.clone());
                        
                        info!("📍 Detected: {} - {}", action.action_type, 
                              truncate(&action.content, 60));
                        
                        if tx.send(action).await.is_err() {
                            error!("Failed to send action to analyzer");
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    async fn stop(&self) -> anyhow::Result<()> {
        info!("Stopping OpenClaw collector");
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.sessions_dir.exists()
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        let mut end = max_len;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    } else {
        s.to_string()
    }
}

// ============================================
// Serde structures for parsing OpenClaw logs
// ============================================

#[derive(Debug, Deserialize)]
struct OpenclawLogEntry {
    #[serde(rename = "type")]
    entry_type: String,
    id: String,
    timestamp: String,
    message: Option<Message>,
}

#[derive(Debug, Deserialize)]
struct Message {
    role: String,
    #[serde(default)]
    content: Vec<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<serde_json::Value>,
}

impl Content {
    fn into_tool_call(self) -> Option<ToolCall> {
        if self.content_type != "toolCall" {
            return None;
        }
        Some(ToolCall {
            id: self.id.unwrap_or_default(),
            name: self.name.unwrap_or_default(),
            arguments: self.arguments,
        })
    }
}

#[derive(Debug)]
struct ToolCall {
    id: String,
    name: String,
    arguments: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exec_log() {
        let collector = OpenclawCollector::new();
        let line = r#"{"type":"message","id":"test123","parentId":"parent","timestamp":"2026-01-27T23:50:46.138Z","message":{"role":"assistant","content":[{"type":"toolCall","id":"tool1","name":"exec","arguments":{"command":"ls -la"}}]}}"#;
        
        let actions = collector.parse_log_line(line);
        assert_eq!(actions.len(), 1);
        
        let action = &actions[0];
        assert_eq!(action.action_type, ActionType::Exec);
        assert_eq!(action.agent, AgentType::OpenClaw);
        assert_eq!(action.content, "ls -la");
    }

    #[test]
    fn test_parse_write_log() {
        let collector = OpenclawCollector::new();
        let line = r#"{"type":"message","id":"test123","parentId":"parent","timestamp":"2026-01-27T23:50:46.138Z","message":{"role":"assistant","content":[{"type":"toolCall","id":"tool1","name":"Write","arguments":{"path":"/tmp/test.txt","content":"hello"}}]}}"#;
        
        let actions = collector.parse_log_line(line);
        assert_eq!(actions.len(), 1);
        
        let action = &actions[0];
        assert_eq!(action.action_type, ActionType::FileWrite);
        assert_eq!(action.target, Some("/tmp/test.txt".to_string()));
    }
}
