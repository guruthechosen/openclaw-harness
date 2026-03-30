//! Enforcement actions (alerts, blocking)
//!
//! Handles the actual response to risky actions.

pub mod alerter;
pub mod proof_of_work;

use super::{AlertConfig, AnalysisResult, Recommendation, RiskLevel};
use proof_of_work::{ProofOfWork, ProofOfWorkRepository, DecisionAction, RiskLevel as PoWRiskLevel};
use tracing::{info, warn};
use std::sync::Mutex;

/// Enforcer handles actions based on analysis results
pub struct Enforcer {
    alerter: alerter::Alerter,
    pow_repository: Mutex<ProofOfWorkRepository>,
}

impl Enforcer {
    pub fn new(config: AlertConfig) -> Self {
        Self {
            alerter: alerter::Alerter::new(config),
            pow_repository: Mutex::new(ProofOfWorkRepository::new()),
        }
    }
    
    /// Get Proof of Work statistics
    pub fn get_pow_stats(&self) -> proof_of_work::PoWStats {
        self.pow_repository.lock().unwrap().get_stats()
    }
    
    /// Generate daily report
    pub fn generate_daily_report(&self, date: chrono::NaiveDate) -> String {
        self.pow_repository.lock().unwrap().generate_daily_report(date)
    }

    /// Enforce the recommendation from an analysis result
    pub async fn enforce(&self, result: &AnalysisResult) -> anyhow::Result<()> {
        // Proof of Work 생성
        let pow = self.create_proof_of_work(result);
        let pow_summary = pow.summary();
        
        // PoW 저장
        self.pow_repository.lock().unwrap().store(pow);
        
        match result.recommendation {
            Recommendation::LogOnly => {
                info!(
                    "[{}] {} - {}",
                    result.action.agent, result.action.action_type, result.action.content
                );
                info!("📝 {}", pow_summary);
            }
            Recommendation::Alert => {
                info!("⚠️ Alert: {}", result.explanation);
                info!("📝 {}", pow_summary);
                self.alerter.send_alert(result).await?;
            }
            Recommendation::PauseAndAsk => {
                warn!("⏸️ Pause required: {}", result.explanation);
                warn!("📝 {}", pow_summary);
                self.alerter.send_alert(result).await?;
                // TODO: Implement actual pause mechanism
            }
            Recommendation::CriticalAlert => {
                warn!("🚨 BLOCKED: {}", result.explanation);
                warn!("📝 {}", pow_summary);
                self.alerter.send_alert(result).await?;
                // TODO: Implement actual blocking mechanism
            }
        }

        Ok(())
    }
    
    /// Create Proof of Work from analysis result
    fn create_proof_of_work(&self, result: &AnalysisResult) -> ProofOfWork {
        use proof_of_work::RuleEvidence;
        
        let decision_action = match result.recommendation {
            Recommendation::LogOnly => DecisionAction::Allow,
            Recommendation::Alert => DecisionAction::AlertOnly,
            Recommendation::PauseAndAsk => DecisionAction::PauseAsk,
            Recommendation::CriticalAlert => DecisionAction::Block,
        };
        
        let risk_level = match result.risk_level {
            RiskLevel::Info => PoWRiskLevel::Info,
            RiskLevel::Warning => PoWRiskLevel::Warning,
            RiskLevel::Critical => PoWRiskLevel::Critical,
        };
        
        let mut builder = ProofOfWork::builder(
            result.action.session_id.clone().unwrap_or_default(),
            result.action.agent.to_string(),
        )
        .action(
            result.action.action_type.to_string(),
            result.action.content.clone(),
        )
        .working_dir(result.action.target.clone().unwrap_or_default())
        .risk_level(risk_level)
        .threats(result.matched_rules.len())
        .decision(decision_action, 0.85)
        .rationale(result.explanation.clone())
        .reasoning(format!(
            "Analysis completed at {}. Matched {} rules. Risk level: {:?}. Recommended action: {:?}.",
            chrono::Utc::now().to_rfc3339(),
            result.matched_rules.len(),
            result.risk_level,
            result.recommendation
        ));
        
        // 규칙들 추가
        for rule_name in &result.matched_rules {
            builder = builder.rule(RuleEvidence {
                rule_id: rule_name.clone(),
                rule_name: rule_name.clone(),
                rule_type: "unknown".to_string(),
                severity: format!("{:?}", result.risk_level),
                matched_pattern: result.action.content.clone(),
                matched_content: result.action.content.clone(),
                confidence: 0.85,
            });
        }
        
        builder.build()
    }
}
