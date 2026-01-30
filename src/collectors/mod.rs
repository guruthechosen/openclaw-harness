//! Log collectors for various AI agents
//!
//! Each collector is responsible for:
//! 1. Finding and watching log files
//! 2. Parsing log entries into `AgentAction`
//! 3. Emitting actions to the analyzer

pub mod openclaw;
pub mod claude_code;
pub mod cursor;

use super::{AgentAction, CollectorConfig};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Trait for log collectors
#[async_trait]
pub trait Collector: Send + Sync {
    /// Name of the collector
    fn name(&self) -> &'static str;

    /// Start collecting logs and send actions to the channel
    async fn start(&self, tx: mpsc::Sender<AgentAction>) -> anyhow::Result<()>;

    /// Stop the collector
    async fn stop(&self) -> anyhow::Result<()>;

    /// Check if the agent is installed/available
    fn is_available(&self) -> bool;
}

/// Create all enabled collectors
pub fn create_collectors(config: &CollectorConfig) -> Vec<Box<dyn Collector>> {
    let mut collectors: Vec<Box<dyn Collector>> = Vec::new();

    if config.openclaw {
        collectors.push(Box::new(openclaw::OpenclawCollector::new()));
    }

    if config.claude_code {
        collectors.push(Box::new(claude_code::ClaudeCodeCollector::new()));
    }

    if config.cursor {
        collectors.push(Box::new(cursor::CursorCollector::new()));
    }

    collectors
}
