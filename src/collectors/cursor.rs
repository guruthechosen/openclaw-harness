//! Cursor IDE log collector
//!
//! Monitors:
//! - ~/.cursor/logs/ (Cursor logs)
//! - Workspace file changes
//! - Terminal command execution

use super::super::{AgentAction, AgentType, ActionType};
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{info, warn};

pub struct CursorCollector {
    log_dir: PathBuf,
}

impl CursorCollector {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        Self {
            log_dir: home.join(".cursor/logs"),
        }
    }
}

#[async_trait]
impl super::Collector for CursorCollector {
    fn name(&self) -> &'static str {
        "cursor"
    }

    async fn start(&self, _tx: mpsc::Sender<AgentAction>) -> anyhow::Result<()> {
        info!("Starting Cursor collector, watching: {:?}", self.log_dir);

        if !self.log_dir.exists() {
            warn!("Cursor log directory not found: {:?}", self.log_dir);
            return Ok(());
        }

        // TODO: Implement Cursor-specific log parsing
        // May need VSCode Extension API integration
        
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        info!("Stopping Cursor collector");
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.log_dir.exists()
    }
}
