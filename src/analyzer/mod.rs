//! Action analyzer and rule engine
//!
//! Analyzes incoming actions against configured rules
//! and produces risk assessments.

pub mod rule_engine;
pub mod risk_scorer;

use super::{AgentAction, AnalysisResult, RiskLevel, Recommendation};
use super::rules::Rule;

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
                    crate::rules::RuleAction::Block if recommendation != Recommendation::CriticalAlert => {
                        recommendation = Recommendation::CriticalAlert;
                    }
                    crate::rules::RuleAction::PauseAndAsk if recommendation != Recommendation::CriticalAlert => {
                        recommendation = Recommendation::PauseAndAsk;
                    }
                    crate::rules::RuleAction::Alert if recommendation == Recommendation::LogOnly => {
                        recommendation = Recommendation::Alert;
                    }
                    _ => {}
                }

                explanations.push(format!("Matched rule: {} - {}", rule.name, rule.description));
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
    use crate::{AgentType, ActionType};
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
