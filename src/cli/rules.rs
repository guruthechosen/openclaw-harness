//! Rules management commands

use openclaw_harness::rules::{
    default_rules, all_templates, self_protection_rules, Rule, KeywordMatch, TemplateParams, RuleAction, MatchType,
    load_rules_from_file,
};
use openclaw_harness::RiskLevel;

pub async fn list() -> anyhow::Result<()> {
    println!("📜 Configured Rules");
    println!("───────────────────");

    // Try loading from config file first, fallback to defaults
    let config_path = std::path::Path::new("config/rules.yaml");
    let rules = if config_path.exists() {
        match load_rules_from_file(config_path) {
            Ok(r) => r,
            Err(_) => default_rules(),
        }
    } else {
        default_rules()
    };

    for rule in &rules {
        let status = if rule.enabled { "✅" } else { "❌" };
        let match_type = match rule.match_type {
            MatchType::Regex => "regex",
            MatchType::Keyword => "keyword",
            MatchType::Template => "template",
        };
        let lock = if rule.protected { " 🔒" } else { "" };
        println!(
            "{} [{}] {} [{:?}]{} - {}",
            status, match_type, rule.name, rule.risk_level, lock, rule.description
        );
    }

    println!("\nTotal: {} rules", rules.len());
    Ok(())
}

pub async fn templates() -> anyhow::Result<()> {
    println!("📋 Available Rule Templates");
    println!("═══════════════════════════\n");

    let templates = all_templates();
    let mut current_category = "";

    for t in &templates {
        if t.category != current_category {
            current_category = t.category;
            println!("── {} ──", current_category);
        }
        println!("  📌 {}", t.name);
        println!("     {}", t.description);
        if !t.required_params.is_empty() {
            println!("     Required: {}", t.required_params.join(", "));
        }
        if !t.optional_params.is_empty() {
            println!("     Optional: {}", t.optional_params.join(", "));
        }
        println!();
    }

    println!("Total: {} templates", templates.len());
    println!("\nUsage:");
    println!("  openclaw-harness rules add --template protect_path --path \"/etc\" --operations \"read,write\"");
    println!("  openclaw-harness rules add --keyword-contains \"rm -rf\" --risk critical --action block");
    Ok(())
}

pub async fn add_template(
    name: &str,
    template: &str,
    path: Option<&str>,
    operations: Option<&str>,
    commands: Option<&str>,
    risk: Option<&str>,
    rule_action: Option<&str>,
) -> anyhow::Result<()> {
    let params = TemplateParams {
        path: path.map(|s| s.to_string()),
        paths: vec![],
        operations: operations
            .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
            .unwrap_or_default(),
        commands: commands
            .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
            .unwrap_or_default(),
        patterns: vec![],
        extra: Default::default(),
    };

    let risk_level = match risk.unwrap_or("warning") {
        "critical" => RiskLevel::Critical,
        "info" => RiskLevel::Info,
        _ => RiskLevel::Warning,
    };

    let action = match rule_action.unwrap_or("block") {
        "log_only" => RuleAction::LogOnly,
        "alert" => RuleAction::Alert,
        "pause_and_ask" => RuleAction::PauseAndAsk,
        "critical_alert" => RuleAction::CriticalAlert,
        _ => RuleAction::Block,
    };

    let rule = Rule::new_template(name, template, params, risk_level, action);

    println!("✅ Created template rule:");
    println!("   Name: {}", rule.name);
    println!("   Template: {}", template);
    println!("   Risk: {:?}", rule.risk_level);
    println!("   Action: {:?}", rule.action);
    println!("   Description: {}", rule.description);
    println!("\n💡 Add to config/rules.yaml to persist.");

    Ok(())
}

pub async fn add_keyword(
    name: &str,
    contains: Option<&str>,
    starts_with: Option<&str>,
    any_of: Option<&str>,
    risk: Option<&str>,
    rule_action: Option<&str>,
) -> anyhow::Result<()> {
    let keyword = KeywordMatch {
        contains: contains
            .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
            .unwrap_or_default(),
        starts_with: starts_with
            .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
            .unwrap_or_default(),
        ends_with: vec![],
        glob: vec![],
        any_of: any_of
            .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
            .unwrap_or_default(),
    };

    let risk_level = match risk.unwrap_or("warning") {
        "critical" => RiskLevel::Critical,
        "info" => RiskLevel::Info,
        _ => RiskLevel::Warning,
    };

    let action = match rule_action.unwrap_or("block") {
        "log_only" => RuleAction::LogOnly,
        "alert" => RuleAction::Alert,
        "pause_and_ask" => RuleAction::PauseAndAsk,
        "critical_alert" => RuleAction::CriticalAlert,
        _ => RuleAction::Block,
    };

    let rule = Rule::new_keyword(name, "User keyword rule", keyword, risk_level, action);

    println!("✅ Created keyword rule:");
    println!("   Name: {}", rule.name);
    println!("   Risk: {:?}", rule.risk_level);
    println!("   Action: {:?}", rule.action);
    println!("\n💡 Add to config/rules.yaml to persist.");

    Ok(())
}

pub async fn enable(name: &str) -> anyhow::Result<()> {
    // Check if this is a self-protection rule
    let sp_rules = self_protection_rules();
    if sp_rules.iter().any(|r| r.name == name) {
        println!("✅ Rule '{}' is a self-protection rule and is always enabled.", name);
        return Ok(());
    }
    println!("Enabling rule: {}", name);
    // TODO: Update rule in config/database
    Ok(())
}

pub async fn disable(name: &str) -> anyhow::Result<()> {
    // Block disabling self-protection rules
    let sp_rules = self_protection_rules();
    if sp_rules.iter().any(|r| r.name == name) {
        println!("🔒 DENIED: Rule '{}' is a self-protection rule and cannot be disabled.", name);
        println!("   Self-protection rules are hardcoded and prevent the AI agent from");
        println!("   tampering with the security harness. Only a human can modify the source code.");
        return Ok(());
    }
    println!("Disabling rule: {}", name);
    // TODO: Update rule in config/database
    Ok(())
}

pub async fn show(name: &str) -> anyhow::Result<()> {
    let rules = default_rules();

    if let Some(rule) = rules.iter().find(|r| r.name == name) {
        println!("Rule: {}", rule.name);
        println!("Description: {}", rule.description);
        println!("Match Type: {:?}", rule.match_type);
        println!("Pattern: {}", rule.pattern);
        println!("Risk Level: {:?}", rule.risk_level);
        println!("Action: {:?}", rule.action);
        println!("Enabled: {}", rule.enabled);
    } else {
        // Check templates
        let templates = all_templates();
        if let Some(t) = templates.iter().find(|t| t.name == name) {
            println!("Template: {}", t.name);
            println!("Description: {}", t.description);
            println!("Category: {}", t.category);
            println!("Required params: {}", t.required_params.join(", "));
            println!("Optional params: {}", t.optional_params.join(", "));
        } else {
            println!("Rule or template not found: {}", name);
        }
    }

    Ok(())
}

pub async fn reload() -> anyhow::Result<()> {
    println!("Reloading rules from config...");
    let config_path = std::path::Path::new("config/rules.yaml");
    if config_path.exists() {
        let rules = load_rules_from_file(config_path)?;
        println!("✅ Loaded {} rules from config/rules.yaml", rules.len());
    } else {
        println!("⚠️  config/rules.yaml not found, using default rules");
    }
    Ok(())
}
