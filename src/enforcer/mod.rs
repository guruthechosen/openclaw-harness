//! Enforcement actions (alerts, blocking)
//!
//! Handles the actual response to risky actions.

pub mod alerter;

use super::{AlertConfig, AnalysisResult, Recommendation};
use tracing::{info, warn};

/// Enforcer handles actions based on analysis results
pub struct Enforcer {
    alerter: alerter::Alerter,
}

impl Enforcer {
    pub fn new(config: AlertConfig) -> Self {
        Self {
            alerter: alerter::Alerter::new(config),
        }
    }

    /// Enforce the recommendation from an analysis result
    pub async fn enforce(&self, result: &AnalysisResult) -> anyhow::Result<()> {
        match result.recommendation {
            Recommendation::LogOnly => {
                info!(
                    "[{}] {} - {}",
                    result.action.agent, result.action.action_type, result.action.content
                );
            }
            Recommendation::Alert => {
                info!("⚠️ Alert: {}", result.explanation);
                self.alerter.send_alert(result).await?;
            }
            Recommendation::PauseAndAsk => {
                warn!("⏸️ Pause required: {}", result.explanation);
                self.alerter.send_alert(result).await?;
                // TODO: Implement actual pause mechanism
                // This would require IPC with the agent
            }
            Recommendation::CriticalAlert => {
                warn!("🚨 BLOCKED: {}", result.explanation);
                self.alerter.send_alert(result).await?;
                // TODO: Implement actual blocking mechanism
                // This might involve killing processes or revoking permissions
            }
        }

        Ok(())
    }
}
