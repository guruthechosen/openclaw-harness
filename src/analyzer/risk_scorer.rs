//! Risk scoring based on multiple factors

use super::{AgentAction, RiskLevel};

/// Calculate overall risk score for an action
pub fn calculate_risk(_action: &AgentAction, matched_rules: &[String]) -> RiskLevel {
    // Simple logic for now - take the highest risk from matched rules
    // In the future, this could incorporate:
    // - Historical context
    // - AI-based analysis
    // - User behavior patterns

    if matched_rules.is_empty() {
        RiskLevel::Info
    } else {
        // Default to Warning if any rules matched
        RiskLevel::Warning
    }
}
