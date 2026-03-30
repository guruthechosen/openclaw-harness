//! Cost-Aware Agent Router
//! 
//! harness-kit의 Cost-aware Delegation에서 영감을 받음
//! Brain의 사용자 패턴 학습을 바탕으로 최적의 AI Agent 선택

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AI Provider 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Provider {
    Anthropic,    // Claude
    OpenAI,       // GPT/Codex
    Google,       // Gemini
    Moonshot,     // Kimi
    Local,        // Ollama 등 로컬 모델
}

impl Provider {
    pub fn name(&self) -> &'static str {
        match self {
            Provider::Anthropic => "claude",
            Provider::OpenAI => "openai",
            Provider::Google => "gemini",
            Provider::Moonshot => "kimi",
            Provider::Local => "local",
        }
    }
    
    /// 대략적인 1K 토큰당 비용 (USD, input)
    pub fn cost_per_1k_input(&self) -> f64 {
        match self {
            Provider::Anthropic => 0.003,    // Claude 3.5 Sonnet
            Provider::OpenAI => 0.0025,      // GPT-4o
            Provider::Google => 0.00125,     // Gemini Pro
            Provider::Moonshot => 0.0015,    // Kimi K2.5
            Provider::Local => 0.0,          // 물질적 비용 없음
        }
    }
    
    /// 능력 점수 (1-10, 복잡한 태스크 수행 능력)
    pub fn capability_score(&self) -> u8 {
        match self {
            Provider::Anthropic => 9,    // 복잡한 추론 우수
            Provider::OpenAI => 8,       // 전반적으로 우수
            Provider::Google => 7,       // 멀티모달 강점
            Provider::Moonshot => 8,     // 긴 컨텍스트
            Provider::Local => 5,        // 제한적이지만 프라이버시
        }
    }
}

/// Agent 쿼ota 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQuota {
    pub agent_id: String,
    pub provider: Provider,
    pub remaining_tokens: u64,
    pub used_tokens_today: u64,
    pub daily_limit: u64,
    pub cost_per_1k_tokens: f64,
    pub capability_score: u8,
    pub is_available: bool,
}

impl AgentQuota {
    pub fn new(agent_id: impl Into<String>, provider: Provider) -> Self {
        Self {
            agent_id: agent_id.into(),
            provider,
            remaining_tokens: 1_000_000,
            used_tokens_today: 0,
            daily_limit: 1_000_000,
            cost_per_1k_tokens: provider.cost_per_1k_input(),
            capability_score: provider.capability_score(),
            is_available: true,
        }
    }
    
    /// 남은 쿼ota 비율
    pub fn quota_remaining_ratio(&self) -> f64 {
        if self.daily_limit == 0 {
            return 1.0;
        }
        (self.daily_limit - self.used_tokens_today) as f64 / self.daily_limit as f64
    }
    
    /// 예상 비용 계산
    pub fn estimate_cost(&self, estimated_tokens: u64) -> f64 {
        (estimated_tokens as f64 / 1000.0) * self.cost_per_1k_tokens
    }
}

/// 태스크 요구사항
#[derive(Debug, Clone)]
pub struct TaskRequirement {
    pub task_type: TaskType,
    pub estimated_tokens: u64,
    pub required_capability: u8,  // 최소 필요 능력 점수
    pub complexity: TaskComplexity,
    pub deadline: Option<std::time::Duration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskType {
    CodeGeneration,
    CodeReview,
    Refactoring,
    Debugging,
    Documentation,
    Architecture,
    Testing,
    Research,
}

impl TaskType {
    /// 해당 태스크 타입에 대한 기본 능력 요구치
    pub fn default_capability_requirement(&self) -> u8 {
        match self {
            TaskType::Architecture => 9,
            TaskType::CodeGeneration => 7,
            TaskType::Debugging => 8,
            TaskType::CodeReview => 6,
            TaskType::Refactoring => 7,
            TaskType::Testing => 5,
            TaskType::Documentation => 4,
            TaskType::Research => 6,
        }
    }
    
    /// 평균 토큰 소모량 추정
    pub fn estimated_tokens(&self) -> u64 {
        match self {
            TaskType::Architecture => 8000,
            TaskType::CodeGeneration => 4000,
            TaskType::Debugging => 3000,
            TaskType::CodeReview => 2000,
            TaskType::Refactoring => 3500,
            TaskType::Testing => 2500,
            TaskType::Documentation => 1500,
            TaskType::Research => 5000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskComplexity {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Cost-Aware 라우터
pub struct CostAwareRouter {
    quotas: Vec<AgentQuota>,
    // TODO: brain: crate::brain::BrainConnector,
    selection_history: Vec<SelectionRecord>,
}

#[derive(Debug, Clone)]
struct SelectionRecord {
    timestamp: chrono::DateTime<chrono::Utc>,
    task_type: TaskType,
    selected_agent: String,
    estimated_cost: f64,
    actual_cost: Option<f64>,
    success: Option<bool>,
}

impl CostAwareRouter {
    pub fn new(quotas: Vec<AgentQuota>) -> Self {
        Self {
            quotas,
            selection_history: Vec::new(),
        }
    }
    
    /// 최적의 Agent 선택 (Cost + Capability + Brain 패턴)
    pub fn select_agent(&mut self, requirement: &TaskRequirement) -> Option<AgentSelection> {
        let candidates: Vec<_> = self.quotas
            .iter()
            .filter(|q| q.is_available)
            .filter(|q| q.remaining_tokens >= requirement.estimated_tokens)
            .filter(|q| q.capability_score >= requirement.required_capability)
            .collect();
        
        if candidates.is_empty() {
            return None;
        }
        
        // 1. Brain에서 유사 태스크의 과거 성공률 조회 (TODO: implement brain connector)
        let historical_scores: HashMap<String, f64> = candidates
            .iter()
            .map(|q| {
                let success_rate = 0.5; // 기본값 50%
                (q.agent_id.clone(), success_rate)
            })
            .collect();
        
        // 2. 점수 계산: Cost 효율성 + Capability + Brain 성공률
        let mut scored_candidates: Vec<_> = candidates
            .into_iter()
            .map(|q| {
                let cost_efficiency = 1.0 / (q.cost_per_1k_tokens + 0.0001); // 싸면 높은 점수
                let capability_match = q.capability_score as f64 / 10.0;
                let success_rate = *historical_scores.get(&q.agent_id).unwrap_or(&0.5);
                let quota_ratio = q.quota_remaining_ratio();
                
                // 가중치: Cost 30%, Capability 30%, Success Rate 30%, Quota 10%
                let score = cost_efficiency * 0.3 
                    + capability_match * 0.3 
                    + success_rate * 0.3 
                    + quota_ratio * 0.1;
                
                (q, score)
            })
            .collect();
        
        // 점수 높은 순 정렬
        scored_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // 최고 점수 선택
        let (selected, score) = scored_candidates.into_iter().next()?;
        
        let selection = AgentSelection {
            agent_id: selected.agent_id.clone(),
            provider: selected.provider,
            estimated_cost: selected.estimate_cost(requirement.estimated_tokens),
            estimated_tokens: requirement.estimated_tokens,
            confidence_score: score,
            reason: format!(
                "Selected based on: cost=${:.4}, capability={}, brain_success_rate={:.1}%", 
                selected.cost_per_1k_tokens * requirement.estimated_tokens as f64 / 1000.0,
                selected.capability_score,
                historical_scores.get(&selected.agent_id).unwrap_or(&0.5) * 100.0
            ),
        };
        
        // 기록
        self.selection_history.push(SelectionRecord {
            timestamp: chrono::Utc::now(),
            task_type: requirement.task_type,
            selected_agent: selected.agent_id.clone(),
            estimated_cost: selection.estimated_cost,
            actual_cost: None,
            success: None,
        });
        
        Some(selection)
    }
    
    /// 실행 결과 피드백 (Brain 학습용)
    pub fn report_execution_result(&mut self, agent_id: &str, success: bool, actual_cost: Option<f64>) {
        // 마지막 선택 기록 업데이트
        if let Some(record) = self.selection_history.last_mut() {
            if record.selected_agent == agent_id {
                record.success = Some(success);
                record.actual_cost = actual_cost;
            }
        }
        
        // TODO: Brain에 결과 저장
        // self.brain.record_task_result(agent_id, success);
    }
    
    /// 현재 라우팅 통계
    pub fn get_stats(&self) -> RouterStats {
        let total_selections = self.selection_history.len();
        let successful = self.selection_history.iter().filter(|r| r.success == Some(true)).count();
        let total_estimated_cost: f64 = self.selection_history.iter().map(|r| r.estimated_cost).sum();
        let total_actual_cost: f64 = self.selection_history
            .iter()
            .filter_map(|r| r.actual_cost)
            .sum();
        
        RouterStats {
            total_selections,
            success_rate: if total_selections > 0 { successful as f64 / total_selections as f64 } else { 0.0 },
            total_estimated_cost,
            total_actual_cost,
            cost_accuracy: if total_actual_cost > 0.0 { 
                1.0 - ((total_estimated_cost - total_actual_cost).abs() / total_actual_cost)
            } else { 1.0 },
            provider_usage: self.calculate_provider_usage(),
        }
    }
    
    fn calculate_provider_usage(&self) -> HashMap<Provider, usize> {
        let mut usage = HashMap::new();
        for record in &self.selection_history {
            if let Some(quota) = self.quotas.iter().find(|q| q.agent_id == record.selected_agent) {
                *usage.entry(quota.provider).or_insert(0) += 1;
            }
        }
        usage
    }
}

#[derive(Debug, Clone)]
pub struct AgentSelection {
    pub agent_id: String,
    pub provider: Provider,
    pub estimated_cost: f64,
    pub estimated_tokens: u64,
    pub confidence_score: f64,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct RouterStats {
    pub total_selections: usize,
    pub success_rate: f64,
    pub total_estimated_cost: f64,
    pub total_actual_cost: f64,
    pub cost_accuracy: f64,
    pub provider_usage: HashMap<Provider, usize>,
}

/// Brain과의 인터페이스 (Mock for now)
pub mod brain {
    use super::*;
    use std::collections::HashMap;
    
    pub struct BrainConnector {
        success_rates: HashMap<(TaskType, String), f64>,
    }
    
    impl BrainConnector {
        pub fn new() -> Self {
            Self {
                success_rates: HashMap::new(),
            }
        }
        
        pub fn get_task_success_rate(&self, task_type: &TaskType, agent_id: &str) -> Option<f64> {
            self.success_rates.get(&(*task_type, agent_id.to_string())).copied()
        }
        
        pub fn record_task_result(&mut self, _agent_id: &str, _success: bool) {
            // 실제 구현에서는 Ontology Brain에 저장
            // 현재는 메모리에만
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_calculation() {
        let quota = AgentQuota::new("claude-1", Provider::Anthropic);
        let cost = quota.estimate_cost(1000);
        assert_eq!(cost, 0.003);
    }

    #[test]
    fn test_provider_capability() {
        assert_eq!(Provider::Anthropic.capability_score(), 9);
        assert_eq!(Provider::Local.capability_score(), 5);
    }

    #[test]
    fn test_task_requirements() {
        let arch = TaskType::Architecture;
        assert_eq!(arch.default_capability_requirement(), 9);
        assert!(arch.estimated_tokens() > TaskType::Documentation.estimated_tokens());
    }
}