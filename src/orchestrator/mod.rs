//! Agent Orchestration Module
//!
//! Cost-aware agent selection and task routing.
//! Harness-kit 스타일 orchestration 기능.

pub mod cost_router;

use crate::orchestrator::cost_router::{CostAwareRouter, AgentQuota, TaskRequirement};

/// Orchestrator manages multi-agent workflows
pub struct Orchestrator {
    router: CostAwareRouter,
}

impl Orchestrator {
    pub fn new(quotas: Vec<AgentQuota>) -> Self {
        Self {
            router: CostAwareRouter::new(quotas),
        }
    }
    
    /// Select the best agent for a task
    pub fn select_agent(&mut self, requirement: &TaskRequirement) -> Option<String> {
        self.router.select_agent(requirement).map(|s| s.agent_id)
    }
}