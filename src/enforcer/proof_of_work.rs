//! Proof of Work Module
//! 
//! harness-kit의 Proof of Work 패턴 적용
//! 모든 실행에 근거(증거) 첨부 - 왜 block/allow 했는지

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 실행 증거 (Proof of Work)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfWork {
    /// 고유 ID
    pub id: String,
    
    /// 타임스탬프
    pub timestamp: DateTime<Utc>,
    
    /// 세션 정보
    pub session_id: String,
    pub user_id: String,
    
    /// 실행된 액션
    pub action: ActionEvidence,
    
    /// 분석 결과
    pub analysis: AnalysisEvidence,
    
    /// 최종 결정
    pub decision: DecisionEvidence,
    
    /// 관련 규칙들
    pub rules_triggered: Vec<RuleEvidence>,
    
    /// 실행 시간/비용 (있는 경우)
    pub execution_metrics: Option<ExecutionMetrics>,
    
    /// Chain of thought / reasoning
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionEvidence {
    pub tool_type: String,
    pub raw_command: String,
    pub normalized_command: String,
    pub target_path: Option<String>,
    pub working_directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisEvidence {
    pub risk_level: RiskLevel,
    pub threat_count: usize,
    pub pattern_matches: Vec<String>,
    pub semantic_context: SemanticContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticContext {
    pub related_projects: Vec<String>,
    pub historical_similarity: f64,  // 과거 유사 명령과의 유사도
    pub user_skill_level: String,    // 해당 도구에 대한 사용자 숙련도
    pub time_since_last_similar: Option<std::time::Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEvidence {
    pub action: DecisionAction,
    pub confidence: f64,
    pub human_override: Option<HumanOverride>,
    pub auto_blocked: bool,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionAction {
    Allow,
    Block,
    PauseAsk,
    AlertOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanOverride {
    pub overridden_by: String,
    pub timestamp: DateTime<Utc>,
    pub original_decision: DecisionAction,
    pub new_decision: DecisionAction,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEvidence {
    pub rule_id: String,
    pub rule_name: String,
    pub rule_type: String,
    pub severity: String,
    pub matched_pattern: String,
    pub matched_content: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub duration_ms: u64,
    pub tokens_used: Option<u64>,
    pub cost_usd: Option<f64>,
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Safe,
    Info,
    Warning,
    Critical,
}

impl ProofOfWork {
    /// 새로운 Proof of Work 생성
    pub fn builder(session_id: impl Into<String>, user_id: impl Into<String>) -> ProofOfWorkBuilder {
        ProofOfWorkBuilder::new(session_id, user_id)
    }
    
    /// 요약 문자열 생성
    pub fn summary(&self) -> String {
        format!(
            "[{}] {} | {} | Risk: {:?} | Action: {:?} | Confidence: {:.1}%",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.action.tool_type,
            truncate(&self.action.normalized_command, 50),
            self.analysis.risk_level,
            self.decision.action,
            self.decision.confidence * 100.0
        )
    }
    
    /// Markdown 보고서 생성
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str(&format!("# Proof of Work: {}\n\n", self.id));
        md.push_str(&format!("**Timestamp:** {}\n\n", self.timestamp.to_rfc3339()));
        md.push_str(&format!("**Session:** {} | **User:** {}\n\n", self.session_id, self.user_id));
        
        md.push_str("## Action\n\n");
        md.push_str(&format!("- **Tool:** `{}`\n", self.action.tool_type));
        md.push_str(&format!("- **Command:** `{}`\n", self.action.raw_command));
        md.push_str(&format!("- **Working Directory:** `{}`\n", self.action.working_directory));
        if let Some(ref target) = self.action.target_path {
            md.push_str(&format!("- **Target:** `{}`\n", target));
        }
        md.push('\n');
        
        md.push_str("## Analysis\n\n");
        md.push_str(&format!("- **Risk Level:** {:?}\n", self.analysis.risk_level));
        md.push_str(&format!("- **Threats Detected:** {}\n", self.analysis.threat_count));
        if !self.analysis.pattern_matches.is_empty() {
            md.push_str(&format!("- **Patterns:** {}\n", self.analysis.pattern_matches.join(", ")));
        }
        md.push('\n');
        
        if !self.rules_triggered.is_empty() {
            md.push_str("## Rules Triggered\n\n");
            for rule in &self.rules_triggered {
                md.push_str(&format!("### {} (`{}`)\n", rule.rule_name, rule.rule_id));
                md.push_str(&format!("- **Type:** {}\n", rule.rule_type));
                md.push_str(&format!("- **Severity:** {}\n", rule.severity));
                md.push_str(&format!("- **Matched:** `{}`\n", rule.matched_content));
                md.push_str(&format!("- **Confidence:** {:.1}%\n\n", rule.confidence * 100.0));
            }
        }
        
        md.push_str("## Decision\n\n");
        md.push_str(&format!("- **Action:** {:?}\n", self.decision.action));
        md.push_str(&format!("- **Confidence:** {:.1}%\n", self.decision.confidence * 100.0));
        md.push_str(&format!("- **Auto-blocked:** {}\n", if self.decision.auto_blocked { "Yes" } else { "No" }));
        md.push_str(&format!("- **Rationale:** {}\n", self.decision.rationale));
        
        if let Some(ref metrics) = self.execution_metrics {
            md.push('\n');
            md.push_str("## Execution Metrics\n\n");
            md.push_str(&format!("- **Duration:** {}ms\n", metrics.duration_ms));
            if let Some(tokens) = metrics.tokens_used {
                md.push_str(&format!("- **Tokens:** {}\n", tokens));
            }
            if let Some(cost) = metrics.cost_usd {
                md.push_str(&format!("- **Cost:** ${:.4}\n", cost));
            }
            if let Some(ref agent) = metrics.agent_id {
                md.push_str(&format!("- **Agent:** {}\n", agent));
            }
        }
        
        md.push('\n');
        md.push_str("## Reasoning\n\n");
        md.push_str(&self.reasoning);
        md.push('\n');
        
        md
    }
}

/// Proof of Work 빌더
pub struct ProofOfWorkBuilder {
    pow: ProofOfWork,
}

impl ProofOfWorkBuilder {
    pub fn new(session_id: impl Into<String>, user_id: impl Into<String>) -> Self {
        Self {
            pow: ProofOfWork {
                id: generate_id(),
                timestamp: Utc::now(),
                session_id: session_id.into(),
                user_id: user_id.into(),
                action: ActionEvidence {
                    tool_type: String::new(),
                    raw_command: String::new(),
                    normalized_command: String::new(),
                    target_path: None,
                    working_directory: String::new(),
                },
                analysis: AnalysisEvidence {
                    risk_level: RiskLevel::Safe,
                    threat_count: 0,
                    pattern_matches: Vec::new(),
                    semantic_context: SemanticContext {
                        related_projects: Vec::new(),
                        historical_similarity: 0.0,
                        user_skill_level: "unknown".to_string(),
                        time_since_last_similar: None,
                    },
                },
                decision: DecisionEvidence {
                    action: DecisionAction::Allow,
                    confidence: 0.0,
                    human_override: None,
                    auto_blocked: false,
                    rationale: String::new(),
                },
                rules_triggered: Vec::new(),
                execution_metrics: None,
                reasoning: String::new(),
            },
        }
    }
    
    pub fn action(mut self, tool: impl Into<String>, command: impl Into<String>) -> Self {
        let cmd = command.into();
        self.pow.action.tool_type = tool.into();
        self.pow.action.raw_command = cmd.clone();
        self.pow.action.normalized_command = normalize_command(&cmd);
        self
    }
    
    pub fn working_dir(mut self, dir: impl Into<String>) -> Self {
        self.pow.action.working_directory = dir.into();
        self
    }
    
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.pow.action.target_path = Some(target.into());
        self
    }
    
    pub fn risk_level(mut self, level: RiskLevel) -> Self {
        self.pow.analysis.risk_level = level;
        self
    }
    
    pub fn threats(mut self, count: usize) -> Self {
        self.pow.analysis.threat_count = count;
        self
    }
    
    pub fn decision(mut self, action: DecisionAction, confidence: f64) -> Self {
        self.pow.decision.action = action;
        self.pow.decision.confidence = confidence;
        self
    }
    
    pub fn rationale(mut self, rationale: impl Into<String>) -> Self {
        self.pow.decision.rationale = rationale.into();
        self
    }
    
    pub fn reasoning(mut self, reasoning: impl Into<String>) -> Self {
        self.pow.reasoning = reasoning.into();
        self
    }
    
    pub fn rule(mut self, rule: RuleEvidence) -> Self {
        self.pow.rules_triggered.push(rule);
        self
    }
    
    pub fn metrics(mut self, metrics: ExecutionMetrics) -> Self {
        self.pow.execution_metrics = Some(metrics);
        self
    }
    
    pub fn build(self) -> ProofOfWork {
        self.pow
    }
}

/// Proof of Work 저장소
pub struct ProofOfWorkRepository {
    proofs: Vec<ProofOfWork>,
    index_by_session: HashMap<String, Vec<usize>>,
    index_by_risk: HashMap<String, Vec<usize>>,
}

impl ProofOfWorkRepository {
    pub fn new() -> Self {
        Self {
            proofs: Vec::new(),
            index_by_session: HashMap::new(),
            index_by_risk: HashMap::new(),
        }
    }
    
    pub fn store(&mut self, pow: ProofOfWork) {
        let idx = self.proofs.len();
        
        // 세션 인덱스
        self.index_by_session
            .entry(pow.session_id.clone())
            .or_insert_with(Vec::new)
            .push(idx);
        
        // 리스크 인덱스
        let risk_key = format!("{:?}", pow.analysis.risk_level);
        self.index_by_risk
            .entry(risk_key)
            .or_insert_with(Vec::new)
            .push(idx);
        
        self.proofs.push(pow);
    }
    
    pub fn get_by_session(&self, session_id: &str) -> Vec<&ProofOfWork> {
        self.index_by_session
            .get(session_id)
            .map(|indices| indices.iter().map(|&i| &self.proofs[i]).collect())
            .unwrap_or_default()
    }
    
    pub fn get_critical(&self) -> Vec<&ProofOfWork> {
        self.index_by_risk
            .get("Critical")
            .map(|indices| indices.iter().map(|&i| &self.proofs[i]).collect())
            .unwrap_or_default()
    }
    
    pub fn get_stats(&self) -> PoWStats {
        let total = self.proofs.len();
        let blocked = self.proofs.iter().filter(|p| matches!(p.decision.action, DecisionAction::Block)).count();
        let allowed = self.proofs.iter().filter(|p| matches!(p.decision.action, DecisionAction::Allow)).count();
        let with_override = self.proofs.iter().filter(|p| p.decision.human_override.is_some()).count();
        
        let total_duration: u64 = self.proofs
            .iter()
            .filter_map(|p| p.execution_metrics.as_ref())
            .map(|m| m.duration_ms)
            .sum();
        
        let total_cost: f64 = self.proofs
            .iter()
            .filter_map(|p| p.execution_metrics.as_ref())
            .filter_map(|m| m.cost_usd)
            .sum();
        
        PoWStats {
            total_proofs: total,
            blocked,
            allowed,
            human_overrides: with_override,
            avg_duration_ms: if total > 0 { total_duration / total as u64 } else { 0 },
            total_cost_usd: total_cost,
        }
    }
    
    /// 일일 보고서 생성
    pub fn generate_daily_report(&self, date: chrono::NaiveDate) -> String {
        let day_proofs: Vec<_> = self.proofs
            .iter()
            .filter(|p| p.timestamp.date_naive() == date)
            .collect();
        
        let mut report = format!("# Daily Security Report: {}\n\n", date);
        
        report.push_str(&format!("**Total Actions:** {}\n", day_proofs.len()));
        report.push_str(&format!("**Blocked:** {}\n", day_proofs.iter().filter(|p| matches!(p.decision.action, DecisionAction::Block)).count()));
        report.push_str(&format!("**Allowed:** {}\n", day_proofs.iter().filter(|p| matches!(p.decision.action, DecisionAction::Allow)).count()));
        report.push('\n');
        
        if !day_proofs.is_empty() {
            report.push_str("## Critical Events\n\n");
            for pow in day_proofs.iter().filter(|p| matches!(p.analysis.risk_level, RiskLevel::Critical)) {
                report.push_str(&format!("- {}\n", pow.summary()));
            }
        }
        
        report
    }
}

#[derive(Debug, Clone)]
pub struct PoWStats {
    pub total_proofs: usize,
    pub blocked: usize,
    pub allowed: usize,
    pub human_overrides: usize,
    pub avg_duration_ms: u64,
    pub total_cost_usd: f64,
}

// 헬퍼 함수
fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("pow-{}", timestamp)
}

fn normalize_command(cmd: &str) -> String {
    // 명령어 정규화: extra whitespace 제거, 소문자 변환
    cmd.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_of_work_builder() {
        let pow = ProofOfWork::builder("session-1", "user-1")
            .action("Exec", "rm -rf /tmp/test")
            .working_dir("/home/user")
            .risk_level(RiskLevel::Warning)
            .threats(1)
            .decision(DecisionAction::PauseAsk, 0.75)
            .rationale("Destructive command requires confirmation")
            .build();
        
        assert_eq!(pow.session_id, "session-1");
        assert_eq!(pow.action.tool_type, "Exec");
        assert!(matches!(pow.decision.action, DecisionAction::PauseAsk));
    }

    #[test]
    fn test_repository() {
        let mut repo = ProofOfWorkRepository::new();
        
        let pow = ProofOfWork::builder("s1", "u1")
            .action("Exec", "ls")
            .decision(DecisionAction::Allow, 0.99)
            .build();
        
        repo.store(pow);
        
        let stats = repo.get_stats();
        assert_eq!(stats.total_proofs, 1);
        assert_eq!(stats.allowed, 1);
    }
}