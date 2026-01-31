//! REST API routes

use super::AppState;
use crate::rules::{default_rules, Rule, RuleAction};
use crate::RiskLevel;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ============================================================================
// Status & Stats
// ============================================================================

#[derive(Serialize)]
pub struct StatusResponse {
    pub running: bool,
    pub version: String,
    pub uptime_seconds: u64,
    pub monitoring: Vec<String>,
}

pub async fn get_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let uptime = chrono::Utc::now()
        .signed_duration_since(state.started_at)
        .num_seconds()
        .max(0) as u64;

    Json(StatusResponse {
        running: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        monitoring: vec!["openclaw".to_string()],
    })
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub total_events: u64,
    pub critical_count: u64,
    pub warning_count: u64,
    pub info_count: u64,
    pub today_events: u64,
    pub rules_count: usize,
    pub blocked_count: u64,
    pub passed_count: u64,
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> Json<StatsResponse> {
    let rules = state.rules.read().await;
    let counters = state.counters.read().await;

    Json(StatsResponse {
        total_events: counters.total_requests,
        critical_count: counters.blocked_count,
        warning_count: counters.warning_count,
        info_count: counters.passed_count,
        today_events: counters.total_requests,
        rules_count: rules.len(),
        blocked_count: counters.blocked_count,
        passed_count: counters.passed_count,
    })
}

#[derive(Serialize)]
pub struct ProviderStats {
    pub provider: String,
    pub request_count: u64,
}

pub async fn get_stats_by_provider(State(state): State<Arc<AppState>>) -> Json<Vec<ProviderStats>> {
    let counters = state.counters.read().await;
    let stats: Vec<ProviderStats> = counters
        .by_provider
        .iter()
        .map(|(k, v)| ProviderStats {
            provider: k.clone(),
            request_count: *v,
        })
        .collect();
    Json(stats)
}

// ============================================================================
// Events
// ============================================================================

#[derive(Deserialize)]
pub struct EventsQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub risk_level: Option<String>,
    pub agent: Option<String>,
    pub provider: Option<String>,
    pub status: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct EventResponse {
    pub id: String,
    pub timestamp: String,
    pub agent: String,
    pub action_type: String,
    pub content: String,
    pub target: Option<String>,
    pub risk_level: Option<String>,
    pub matched_rules: Vec<String>,
    pub provider: Option<String>,
    pub status: Option<String>,
}

#[derive(Serialize)]
pub struct EventsResponse {
    pub events: Vec<EventResponse>,
    pub total: u64,
}

pub async fn get_events(
    State(_state): State<Arc<AppState>>,
    Query(_query): Query<EventsQuery>,
) -> Json<EventsResponse> {
    // TODO: Get from database with filters
    Json(EventsResponse {
        events: vec![],
        total: 0,
    })
}

pub async fn get_recent_events(State(_state): State<Arc<AppState>>) -> Json<Vec<EventResponse>> {
    // TODO: Get last 20 events from database
    Json(vec![])
}

pub async fn get_event(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<String>,
) -> Result<Json<EventResponse>, StatusCode> {
    Err(StatusCode::NOT_FOUND)
}

// ============================================================================
// Rules
// ============================================================================

#[derive(Serialize, Clone)]
pub struct RuleResponse {
    pub name: String,
    pub description: String,
    pub pattern: String,
    pub risk_level: String,
    pub action: String,
    pub enabled: bool,
    pub is_preset: bool,
}

impl RuleResponse {
    fn from_rule(rule: &Rule, preset_names: &[&str]) -> Self {
        RuleResponse {
            name: rule.name.clone(),
            description: rule.description.clone(),
            pattern: rule.pattern.clone(),
            risk_level: format!("{:?}", rule.risk_level),
            action: format!("{:?}", rule.action),
            enabled: rule.enabled,
            is_preset: preset_names.contains(&rule.name.as_str()),
        }
    }
}

const PRESET_RULE_NAMES: &[&str] = &[
    "dangerous_rm",
    "api_key_exposure",
    "ssh_key_access",
    "wallet_access",
    "mass_delete",
    "system_config",
    "sudo_command",
    "git_push",
    "npm_install",
];

pub async fn get_rules(State(state): State<Arc<AppState>>) -> Json<Vec<RuleResponse>> {
    let rules = state.rules.read().await;
    Json(
        rules
            .iter()
            .map(|r| RuleResponse::from_rule(r, PRESET_RULE_NAMES))
            .collect(),
    )
}

#[derive(Deserialize)]
pub struct CreateRuleRequest {
    pub name: String,
    pub description: String,
    pub pattern: String,
    pub risk_level: String,
    pub action: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

fn parse_risk_level(s: &str) -> RiskLevel {
    match s.to_lowercase().as_str() {
        "critical" => RiskLevel::Critical,
        "warning" => RiskLevel::Warning,
        _ => RiskLevel::Info,
    }
}

fn parse_action(s: &str) -> RuleAction {
    match s.to_lowercase().as_str() {
        "criticalalert" | "critical_alert" => RuleAction::CriticalAlert,
        "pauseandask" | "pause_and_ask" => RuleAction::PauseAndAsk,
        "alert" => RuleAction::Alert,
        _ => RuleAction::LogOnly,
    }
}

pub async fn create_rule(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateRuleRequest>,
) -> Result<Json<RuleResponse>, StatusCode> {
    // Validate regex
    if regex::Regex::new(&body.pattern).is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut rule = Rule::new(
        &body.name,
        &body.description,
        &body.pattern,
        parse_risk_level(&body.risk_level),
        parse_action(&body.action),
    );
    rule.enabled = body.enabled;
    if rule.compile().is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let resp = RuleResponse::from_rule(&rule, PRESET_RULE_NAMES);

    let mut rules = state.rules.write().await;
    // Check duplicate
    if rules.iter().any(|r| r.name == body.name) {
        return Err(StatusCode::CONFLICT);
    }
    rules.push(rule);

    Ok(Json(resp))
}

#[derive(Deserialize)]
pub struct UpdateRuleRequest {
    pub description: Option<String>,
    pub pattern: Option<String>,
    pub risk_level: Option<String>,
    pub action: Option<String>,
    pub enabled: Option<bool>,
}

pub async fn update_rule(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<UpdateRuleRequest>,
) -> Result<Json<RuleResponse>, StatusCode> {
    let mut rules = state.rules.write().await;
    let rule = rules
        .iter_mut()
        .find(|r| r.name == name)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Block modification of protected (self-protection) rules
    if rule.protected {
        return Err(StatusCode::FORBIDDEN);
    }

    if let Some(desc) = body.description {
        rule.description = desc;
    }
    if let Some(pattern) = body.pattern {
        if regex::Regex::new(&pattern).is_err() {
            return Err(StatusCode::BAD_REQUEST);
        }
        rule.pattern = pattern;
        rule.compile().map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    if let Some(rl) = body.risk_level {
        rule.risk_level = parse_risk_level(&rl);
    }
    if let Some(act) = body.action {
        rule.action = parse_action(&act);
    }
    if let Some(en) = body.enabled {
        rule.enabled = en;
    }

    let resp = RuleResponse::from_rule(rule, PRESET_RULE_NAMES);
    Ok(Json(resp))
}

pub async fn delete_rule(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> StatusCode {
    // Prevent deleting preset or protected rules
    if PRESET_RULE_NAMES.contains(&name.as_str()) {
        return StatusCode::FORBIDDEN;
    }
    {
        let rules = state.rules.read().await;
        if rules.iter().any(|r| r.name == name && r.protected) {
            return StatusCode::FORBIDDEN;
        }
    }

    let mut rules = state.rules.write().await;
    let len_before = rules.len();
    rules.retain(|r| r.name != name);
    if rules.len() == len_before {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::NO_CONTENT
    }
}

#[derive(Deserialize)]
pub struct TestRuleRequest {
    pub pattern: String,
    pub input: String,
}

#[derive(Serialize)]
pub struct TestRuleResponse {
    pub matches: bool,
    pub matched_text: Option<String>,
}

pub async fn test_rule(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<TestRuleRequest>,
) -> Result<Json<TestRuleResponse>, StatusCode> {
    match regex::Regex::new(&body.pattern) {
        Ok(re) => {
            if let Some(m) = re.find(&body.input) {
                Ok(Json(TestRuleResponse {
                    matches: true,
                    matched_text: Some(m.as_str().to_string()),
                }))
            } else {
                Ok(Json(TestRuleResponse {
                    matches: false,
                    matched_text: None,
                }))
            }
        }
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

// ============================================================================
// Proxy Status & Config
// ============================================================================

#[derive(Serialize)]
pub struct ProxyStatusResponse {
    pub running: bool,
    pub mode: String,
    pub listen: String,
    pub target: String,
    pub uptime_seconds: u64,
}

pub async fn get_proxy_status(State(state): State<Arc<AppState>>) -> Json<ProxyStatusResponse> {
    let config = state.proxy_config.read().await;
    let uptime = chrono::Utc::now()
        .signed_duration_since(state.started_at)
        .num_seconds()
        .max(0) as u64;

    Json(ProxyStatusResponse {
        running: config.enabled,
        mode: format!("{:?}", config.mode).to_lowercase(),
        listen: config.listen.clone(),
        target: config.target.clone(),
        uptime_seconds: uptime,
    })
}

#[derive(Deserialize)]
pub struct UpdateProxyConfigRequest {
    pub mode: Option<String>,
    pub enabled: Option<bool>,
}

pub async fn update_proxy_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpdateProxyConfigRequest>,
) -> Json<ProxyStatusResponse> {
    let mut config = state.proxy_config.write().await;
    if let Some(mode) = body.mode {
        config.mode = match mode.to_lowercase().as_str() {
            "enforce" => crate::proxy::config::ProxyMode::Enforce,
            _ => crate::proxy::config::ProxyMode::Monitor,
        };
    }
    if let Some(enabled) = body.enabled {
        config.enabled = enabled;
    }

    let uptime = chrono::Utc::now()
        .signed_duration_since(state.started_at)
        .num_seconds()
        .max(0) as u64;

    Json(ProxyStatusResponse {
        running: config.enabled,
        mode: format!("{:?}", config.mode).to_lowercase(),
        listen: config.listen.clone(),
        target: config.target.clone(),
        uptime_seconds: uptime,
    })
}

// ============================================================================
// Providers
// ============================================================================

#[derive(Serialize)]
pub struct ProviderResponse {
    pub name: String,
    pub enabled: bool,
    pub target_url: String,
}

pub async fn get_providers(State(_state): State<Arc<AppState>>) -> Json<Vec<ProviderResponse>> {
    Json(vec![
        ProviderResponse {
            name: "Anthropic".to_string(),
            enabled: true,
            target_url: "https://api.anthropic.com".to_string(),
        },
        ProviderResponse {
            name: "OpenAI".to_string(),
            enabled: false,
            target_url: "https://api.openai.com".to_string(),
        },
        ProviderResponse {
            name: "Gemini".to_string(),
            enabled: false,
            target_url: "https://generativelanguage.googleapis.com".to_string(),
        },
    ])
}

// ============================================================================
// Alert Config
// ============================================================================

#[derive(Serialize, Deserialize, Clone)]
pub struct AlertConfigResponse {
    pub telegram_enabled: bool,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub slack_enabled: bool,
    pub slack_webhook: Option<String>,
    pub discord_enabled: bool,
    pub discord_webhook: Option<String>,
    pub notify_on_critical: bool,
    pub notify_on_warning: bool,
    pub notify_on_info: bool,
}

pub async fn get_alert_config(State(_state): State<Arc<AppState>>) -> Json<AlertConfigResponse> {
    // Check env vars first, then config file

    let token = std::env::var("OPENCLAW_HARNESS_TELEGRAM_BOT_TOKEN")
        .or_else(|_| std::env::var("SAFEBOT_TELEGRAM_BOT_TOKEN"))
        .ok();
    let chat_id = std::env::var("OPENCLAW_HARNESS_TELEGRAM_CHAT_ID")
        .or_else(|_| std::env::var("SAFEBOT_TELEGRAM_CHAT_ID"))
        .ok();
    // Try loading from config file
    let file_config = load_alert_config_from_file();

    let tg_token = token.or(file_config
        .as_ref()
        .and_then(|c| c.telegram_bot_token.clone()));
    let tg_chat = chat_id.or(file_config
        .as_ref()
        .and_then(|c| c.telegram_chat_id.clone()));

    Json(AlertConfigResponse {
        telegram_enabled: tg_token.is_some(),
        telegram_bot_token: tg_token.map(|t| mask_token(&t)),
        telegram_chat_id: tg_chat,
        slack_enabled: file_config
            .as_ref()
            .map(|c| c.slack_enabled)
            .unwrap_or(false),
        slack_webhook: file_config.as_ref().and_then(|c| c.slack_webhook.clone()),
        discord_enabled: file_config
            .as_ref()
            .map(|c| c.discord_enabled)
            .unwrap_or(false),
        discord_webhook: file_config.as_ref().and_then(|c| c.discord_webhook.clone()),
        notify_on_critical: true,
        notify_on_warning: true,
        notify_on_info: false,
    })
}

pub async fn update_alert_config(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<AlertConfigResponse>,
) -> StatusCode {
    // Save to config file
    if let Err(e) = save_alert_config_to_file(&body) {
        tracing::error!("Failed to save alert config: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    // Also set env vars for current process (so proxy picks them up)
    if let Some(ref token) = body.telegram_bot_token {
        if !token.contains("****") {
            std::env::set_var("OPENCLAW_HARNESS_TELEGRAM_BOT_TOKEN", token);
        }
    }
    if let Some(ref chat_id) = body.telegram_chat_id {
        std::env::set_var("OPENCLAW_HARNESS_TELEGRAM_CHAT_ID", chat_id);
    }

    StatusCode::OK
}

fn mask_token(token: &str) -> String {
    if token.len() <= 8 {
        "****".to_string()
    } else {
        format!("{}****{}", &token[..4], &token[token.len() - 4..])
    }
}

fn load_alert_config_from_file() -> Option<AlertConfigResponse> {
    let path = std::path::Path::new("config/alerts.json");
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_alert_config_to_file(config: &AlertConfigResponse) -> anyhow::Result<()> {
    std::fs::create_dir_all("config")?;
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write("config/alerts.json", content)?;
    Ok(())
}
