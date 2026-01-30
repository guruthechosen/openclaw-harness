//! Claude Code log collector
//!
//! Monitors:
//! - ~/.claude/logs/*.jsonl (session logs)
//! - Process activity via dtrace/ptrace (optional)

use super::super::{AgentAction, AgentType, ActionType};
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{info, warn};

pub struct ClaudeCodeCollector {
    log_dir: PathBuf,
}

impl ClaudeCodeCollector {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        Self {
            log_dir: home.join(".claude/logs"),
        }
    }
}

#[async_trait]
impl super::Collector for ClaudeCodeCollector {
    fn name(&self) -> &'static str {
        "claude_code"
    }

    async fn start(&self, _tx: mpsc::Sender<AgentAction>) -> anyhow::Result<()> {
        info!("Starting Claude Code collector, watching: {:?}", self.log_dir);

        if !self.log_dir.exists() {
            warn!("Claude Code log directory not found: {:?}", self.log_dir);
            return Ok(());
        }

        // TODO: Implement log watching similar to Moltbot
        // Claude Code logs are in JSONL format with tool_use events
        
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        info!("Stopping Claude Code collector");
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.log_dir.exists()
    }
}
