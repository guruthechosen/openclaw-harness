//! Action analyzer and rule engine
//!
//! Analyzes incoming actions against configured rules
//! and produces risk assessments.

pub mod advanced_threats;
pub mod risk_scorer;
pub mod rule_engine;

use super::rules::Rule;
use super::{AgentAction, AnalysisResult, Recommendation, RiskLevel};
use advanced_threats::{scan_advanced_threats, ExecutionContext, RiskLevel as ThreatRisk};

/// The main analyzer that processes actions
pub struct Analyzer {
    rules: Vec<Rule>,
}

impl Analyzer {
    pub fn new(rules: Vec<Rule>) -> Self {
        Self { rules }
    }

    /// Analyze an action and return the result
    pub fn analyze(&self, action: &AgentAction) -> AnalysisResult {
        let mut matched_rules = Vec::new();
        let mut highest_risk = RiskLevel::Info;
        let mut recommendation = Recommendation::LogOnly;
        let mut explanations = Vec::new();

        // 1. 기존 규칙 검사
        for rule in &self.rules {
            if rule.matches(action) {
                matched_rules.push(rule.name.clone());

                if rule.risk_level > highest_risk {
                    highest_risk = rule.risk_level;
                }

                match rule.action {
                    crate::rules::RuleAction::CriticalAlert => {
                        recommendation = Recommendation::CriticalAlert;
                    }
                    crate::rules::RuleAction::Block
                        if recommendation != Recommendation::CriticalAlert =>
                    {
                        recommendation = Recommendation::CriticalAlert;
                    }
                    crate::rules::RuleAction::PauseAndAsk
                        if recommendation != Recommendation::CriticalAlert =>
                    {
                        recommendation = Recommendation::PauseAndAsk;
                    }
                    crate::rules::RuleAction::Alert
                        if recommendation == Recommendation::LogOnly =>
                    {
                        recommendation = Recommendation::Alert;
                    }
                    _ => {}
                }

                explanations.push(format!(
                    "Matched rule: {} - {}",
                    rule.name, rule.description
                ));
            }
        }

        // 2. 고급 위협 탐지 (Snyk-style)
        let context = ExecutionContext::new(
            action.target.clone().unwrap_or_default(),
            action.agent.to_string(),
        );
        let threat_scan = scan_advanced_threats(&action.content, &context);
        
        if !threat_scan.threats.is_empty() {
            for threat in &threat_scan.threats {
                matched_rules.push(format!("ADVANCED:{:?}", threat.threat_type));
                explanations.push(format!(
                    "🔒 Advanced threat: {} - {}",
                    threat.description, threat.mitigation
                ));
            }
            
            // 위협 위험도로 risk level 업데이트
            let threat_risk = match threat_scan.risk_level {
                ThreatRisk::Critical => RiskLevel::Critical,
                ThreatRisk::Warning => RiskLevel::Warning,
                ThreatRisk::Safe => RiskLevel::Info,
            };
            
            if threat_risk > highest_risk {
                highest_risk = threat_risk;
                recommendation = Recommendation::CriticalAlert;
            }
        }

        let explanation = if explanations.is_empty() {
            "No rules matched".to_string()
        } else {
            explanations.join("; ")
        };

        AnalysisResult {
            action: action.clone(),
            matched_rules,
            risk_level: highest_risk,
            recommendation,
            explanation,
        }
    }

    /// Reload rules
    pub fn reload_rules(&mut self, rules: Vec<Rule>) {
        self.rules = rules;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActionType, AgentType};
    use chrono::Utc;

    #[test]
    fn test_analyzer_no_rules() {
        let analyzer = Analyzer::new(vec![]);
        let action = AgentAction {
            id: "test".to_string(),
            timestamp: Utc::now(),
            agent: AgentType::OpenClaw,
            action_type: ActionType::Exec,
            content: "ls -la".to_string(),
            target: None,
            session_id: None,
            metadata: None,
        };

        let result = analyzer.analyze(&action);
        assert_eq!(result.risk_level, RiskLevel::Info);
        assert_eq!(result.recommendation, Recommendation::LogOnly);
    }
}
