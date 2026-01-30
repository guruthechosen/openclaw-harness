//! Test command - test a rule against sample input

use moltbot_harness::rules::{default_rules, load_rules_from_file};
use moltbot_harness::{AgentAction, AgentType, ActionType};

pub async fn run(rule_name: &str, input: &str) -> anyhow::Result<()> {
    println!("Testing rule '{}' against input: {}", rule_name, input);
    println!("────────────────────────────────────");
    
    // Try loading from config first, fall back to defaults
    let config_path = std::path::Path::new("config/rules.yaml");
    let rules = if config_path.exists() {
        match load_rules_from_file(config_path) {
            Ok(r) => r,
            Err(_) => default_rules(),
        }
    } else {
        default_rules()
    };
    
    if let Some(mut rule) = rules.into_iter().find(|r| r.name == rule_name) {
        rule.compile()?;
        
        // Create test action
        let action = AgentAction {
            id: "test".to_string(),
            timestamp: chrono::Utc::now(),
            agent: AgentType::Moltbot,
            action_type: ActionType::Exec,
            content: input.to_string(),
            target: None,
            session_id: None,
            metadata: None,
        };
        
        if rule.matches(&action) {
            println!("✅ MATCH");
            println!("Risk Level: {:?}", rule.risk_level);
            println!("Action: {:?}", rule.action);
        } else {
            println!("❌ NO MATCH");
        }
    } else {
        println!("Rule not found: {}", rule_name);
        println!("\nAvailable rules:");
        let all_rules = if config_path.exists() {
            load_rules_from_file(config_path).unwrap_or_else(|_| default_rules())
        } else {
            default_rules()
        };
        for rule in all_rules {
            let type_tag = match rule.match_type {
                moltbot_harness::rules::MatchType::Regex => "regex",
                moltbot_harness::rules::MatchType::Keyword => "keyword",
                moltbot_harness::rules::MatchType::Template => "template",
            };
            println!("  - {} [{}]", rule.name, type_tag);
        }
    }
    
    Ok(())
}
