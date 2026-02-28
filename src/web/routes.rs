//! REST API routes

use super::AppState;
use crate::brain::{
    build_ontology_from_db, build_ontology_v2_from_db, persist_ontology, persist_ontology_v2,
    BrainInsights, OntologyBuildSummary,
};
use crate::campaign::{CampaignConstraints, CampaignEngine, LlmAiPlanner, MissionPlan};
use crate::rules::{Rule, RuleAction};
use crate::RiskLevel;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path as StdPath;
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

// ============================================================================
// Adaptive Campaign (AI-driven dynamic mission)
// ============================================================================

#[derive(Deserialize)]
pub struct AdaptiveCampaignRequest {
    pub user_id: String,
    pub max_points_per_mission: u32,
    pub min_completion_probability: Option<f32>,
    pub max_expected_hours: Option<f32>,
}

#[derive(Serialize)]
pub struct AdaptiveCampaignResponse {
    pub ok: bool,
    pub mission: MissionPlan,
}

pub async fn generate_adaptive_campaign(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AdaptiveCampaignRequest>,
) -> Result<Json<AdaptiveCampaignResponse>, StatusCode> {
    let constraints = CampaignConstraints {
        max_points_per_mission: body.max_points_per_mission,
        min_completion_probability: body.min_completion_probability.unwrap_or(0.35),
        max_expected_hours: body.max_expected_hours.unwrap_or(3.0),
    };

    let conn = rusqlite::Connection::open(&state.db_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let planner = LlmAiPlanner::from_env().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let engine = CampaignEngine::new(planner);
    let mission = engine
        .generate_mission(&conn, &body.user_id, &constraints)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(AdaptiveCampaignResponse { ok: true, mission }))
}

#[derive(Serialize)]
pub struct BuildOntologyResponse {
    pub ok: bool,
    pub summary: OntologyBuildSummary,
}

#[derive(Serialize)]
pub struct BuildOntologyV2Response {
    pub ok: bool,
    pub summary: OntologyBuildSummary,
    pub insights: BrainInsights,
}

pub async fn build_ontology_v1(
    State(state): State<Arc<AppState>>,
) -> Result<Json<BuildOntologyResponse>, StatusCode> {
    let conn = rusqlite::Connection::open(&state.db_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let (nodes, edges) = build_ontology_from_db(&conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let summary = persist_ontology(StdPath::new("data"), &nodes, &edges)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(BuildOntologyResponse { ok: true, summary }))
}

pub async fn build_ontology_v2(
    State(state): State<Arc<AppState>>,
) -> Result<Json<BuildOntologyV2Response>, StatusCode> {
    let conn = rusqlite::Connection::open(&state.db_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let (nodes, edges, insights) = build_ontology_v2_from_db(&conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let summary = persist_ontology_v2(StdPath::new("data"), &nodes, &edges, &insights)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(BuildOntologyV2Response {
        ok: true,
        summary,
        insights,
    }))
}

#[derive(Deserialize)]
pub struct BrainQueryRequest {
    pub query_type: String,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct BrainQueryResponse {
    pub ok: bool,
    pub query_type: String,
    pub results: Vec<serde_json::Value>,
    pub insights: Option<serde_json::Value>,
}

pub async fn query_brain_v2(
    Json(body): Json<BrainQueryRequest>,
) -> Result<Json<BrainQueryResponse>, StatusCode> {
    let base = StdPath::new("data/ontology/v2");
    let nodes_path = base.join("nodes.jsonl");
    let insights_path = base.join("insights.json");

    let nodes_txt = fs::read_to_string(nodes_path).map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut rows: Vec<serde_json::Value> = vec![];
    for line in nodes_txt.lines() {
        if line.trim().is_empty() { continue; }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            rows.push(v);
        }
    }

    let limit = body.limit.unwrap_or(10);
    let results = match body.query_type.as_str() {
        "top_bottlenecks" => rows.into_iter().filter(|v| v["kind"] == "Bottleneck").take(limit).collect(),
        "top_patterns" => rows.into_iter().filter(|v| v["kind"] == "TaskPattern").take(limit).collect(),
        "skills" => rows.into_iter().filter(|v| v["kind"] == "Skill").take(limit).collect(),
        "decisions" => rows.into_iter().filter(|v| v["kind"] == "Decision").take(limit).collect(),
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let insights = fs::read_to_string(insights_path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());

    Ok(Json(BrainQueryResponse {
        ok: true,
        query_type: body.query_type,
        results,
        insights,
    }))
}

// ============================================================================
// Brain Reports (Weekly)
// ============================================================================

#[derive(Deserialize)]
pub struct WeeklyReportQuery {
    pub week: Option<String>, // YYYY-Www
}

#[derive(Deserialize)]
pub struct GenerateWeeklyReportRequest {
    pub workspace_id: Option<String>,
    pub week: Option<String>,
    pub timezone: Option<String>,
    pub force_regenerate: Option<bool>,
}

#[derive(Serialize)]
pub struct WeeklyProjectActivity {
    pub project_id: String,
    pub events: u64,
}

#[derive(Serialize)]
pub struct WeeklyToolCount {
    pub tool: String,
    pub count: u64,
}

#[derive(Serialize)]
pub struct WeeklyPattern {
    pub name: String,
    pub count: u64,
    pub suggestion: String,
}

#[derive(Serialize)]
pub struct WeeklyRisk {
    pub critical: u64,
    pub warning: u64,
    pub info: u64,
}

#[derive(Serialize)]
pub struct WeeklyActivity {
    pub total_events: u64,
    pub projects: Vec<WeeklyProjectActivity>,
    pub top_tools: Vec<WeeklyToolCount>,
}

#[derive(Serialize)]
pub struct WeeklyReportResponse {
    pub report_id: String,
    pub workspace_id: String,
    pub week_start: String,
    pub week_end: String,
    pub headline: String,
    pub activity: WeeklyActivity,
    pub risk: WeeklyRisk,
    pub patterns: Vec<WeeklyPattern>,
    pub next_actions: Vec<String>,
    pub markdown: String,
    pub created_at: String,
}

fn week_range_kst(week: Option<String>) -> anyhow::Result<(String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> {
    use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Weekday};

    let now_kst = chrono::Utc::now() + Duration::hours(9);
    let (year, iso_week) = if let Some(w) = week {
        let parts: Vec<&str> = w.split('-').collect();
        if parts.len() != 2 || !parts[1].starts_with('W') {
            anyhow::bail!("week must be YYYY-Www");
        }
        let year = parts[0].parse::<i32>()?;
        let iso_week = parts[1][1..].parse::<u32>()?;
        (year, iso_week)
    } else {
        (now_kst.year(), now_kst.iso_week().week())
    };

    let monday = NaiveDate::from_isoywd_opt(year, iso_week, Weekday::Mon)
        .ok_or_else(|| anyhow::anyhow!("invalid ISO week"))?;
    let sunday = monday + Duration::days(6);

    let start_kst = NaiveDateTime::new(monday, chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    let end_kst = NaiveDateTime::new(sunday, chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap());

    let start_utc = chrono::FixedOffset::east_opt(9 * 3600)
        .unwrap()
        .from_local_datetime(&start_kst)
        .single()
        .unwrap()
        .with_timezone(&chrono::Utc);
    let end_utc = chrono::FixedOffset::east_opt(9 * 3600)
        .unwrap()
        .from_local_datetime(&end_kst)
        .single()
        .unwrap()
        .with_timezone(&chrono::Utc);

    Ok((format!("{}-W{:02}", year, iso_week), start_utc, end_utc))
}

fn build_markdown(report: &WeeklyReportResponse) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Weekly Report {}\n\n", report.report_id));
    out.push_str(&format!("- Headline: {}\n", report.headline));
    out.push_str(&format!("- Range (UTC): {} ~ {}\n\n", report.week_start, report.week_end));
    out.push_str("## Activity\n");
    out.push_str(&format!("- Total events: {}\n", report.activity.total_events));
    for p in &report.activity.projects {
        out.push_str(&format!("- Project `{}`: {} events\n", p.project_id, p.events));
    }
    out.push_str("\n## Risk\n");
    out.push_str(&format!("- Critical: {}\n- Warning: {}\n- Info: {}\n", report.risk.critical, report.risk.warning, report.risk.info));
    out.push_str("\n## Patterns\n");
    for p in &report.patterns {
        out.push_str(&format!("- {} ({}): {}\n", p.name, p.count, p.suggestion));
    }
    out.push_str("\n## Next Actions\n");
    for a in &report.next_actions {
        out.push_str(&format!("- {}\n", a));
    }
    out
}

fn persist_weekly_outputs(base_dir: &StdPath, report: &WeeklyReportResponse) -> anyhow::Result<()> {
    let weekly_dir = base_dir.join("reports").join("weekly");
    fs::create_dir_all(&weekly_dir)?;

    let md_path = weekly_dir.join(format!("{}.md", report.report_id));
    let json_path = weekly_dir.join(format!("{}.json", report.report_id));

    fs::write(md_path, &report.markdown)?;
    fs::write(json_path, serde_json::to_string_pretty(report)?)?;

    Ok(())
}

fn materialize_ontology_minimal(base_dir: &StdPath, report: &WeeklyReportResponse) -> anyhow::Result<()> {
    let ontology_dir = base_dir.join("ontology");
    fs::create_dir_all(&ontology_dir)?;

    let nodes_path = ontology_dir.join("nodes.jsonl");
    let edges_path = ontology_dir.join("edges.jsonl");

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    nodes.push(serde_json::json!({
        "id": format!("workspace:{}", report.workspace_id),
        "kind": "Workspace",
        "title": report.workspace_id,
        "ts": report.created_at,
    }));

    nodes.push(serde_json::json!({
        "id": format!("report:{}", report.report_id),
        "kind": "WeeklyReport",
        "title": report.headline,
        "week_start": report.week_start,
        "week_end": report.week_end,
        "ts": report.created_at,
    }));

    edges.push(serde_json::json!({
        "from": format!("workspace:{}", report.workspace_id),
        "to": format!("report:{}", report.report_id),
        "rel": "has_report",
        "ts": report.created_at,
    }));

    for p in &report.activity.projects {
        let project_node = serde_json::json!({
            "id": format!("project:{}", p.project_id),
            "kind": "Project",
            "title": p.project_id,
            "events": p.events,
            "ts": report.created_at,
        });
        nodes.push(project_node);

        edges.push(serde_json::json!({
            "from": format!("report:{}", report.report_id),
            "to": format!("project:{}", p.project_id),
            "rel": "contains_project_activity",
            "weight": p.events,
            "ts": report.created_at,
        }));
    }

    let nodes_jsonl = nodes
        .into_iter()
        .map(|n| serde_json::to_string(&n))
        .collect::<Result<Vec<_>, _>>()?
        .join("\n")
        + "\n";
    let edges_jsonl = edges
        .into_iter()
        .map(|e| serde_json::to_string(&e))
        .collect::<Result<Vec<_>, _>>()?
        .join("\n")
        + "\n";

    fs::write(nodes_path, nodes_jsonl)?;
    fs::write(edges_path, edges_jsonl)?;

    Ok(())
}

fn compute_weekly_report(db_path: &str, week: Option<String>, workspace_id: Option<String>) -> anyhow::Result<WeeklyReportResponse> {
    use rusqlite::Connection;

    let (report_id, start_utc, end_utc) = week_range_kst(week)?;
    let workspace = workspace_id.unwrap_or_else(|| "default".to_string());
    let conn = Connection::open(db_path)?;

    let total_events: u64 = conn.query_row(
        "SELECT COUNT(*) FROM actions WHERE timestamp BETWEEN ?1 AND ?2",
        [start_utc.to_rfc3339(), end_utc.to_rfc3339()],
        |r| r.get::<_, i64>(0).map(|v| v as u64),
    )?;

    let mut projects_map: HashMap<String, u64> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT COALESCE(target, 'unknown'), COUNT(*) FROM actions WHERE timestamp BETWEEN ?1 AND ?2 GROUP BY COALESCE(target, 'unknown') ORDER BY COUNT(*) DESC LIMIT 5",
    )?;
    let rows = stmt.query_map([start_utc.to_rfc3339(), end_utc.to_rfc3339()], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
    })?;
    for row in rows {
        let (target, count) = row?;
        let project = if target.starts_with('/') {
            target.split('/').take(5).collect::<Vec<_>>().join("/")
        } else {
            target
        };
        *projects_map.entry(project).or_insert(0) += count;
    }
    let projects: Vec<WeeklyProjectActivity> = projects_map
        .into_iter()
        .map(|(project_id, events)| WeeklyProjectActivity { project_id, events })
        .collect();

    let mut top_tools_stmt = conn.prepare(
        "SELECT action_type, COUNT(*) FROM actions WHERE timestamp BETWEEN ?1 AND ?2 GROUP BY action_type ORDER BY COUNT(*) DESC LIMIT 5",
    )?;
    let top_tools = top_tools_stmt
        .query_map([start_utc.to_rfc3339(), end_utc.to_rfc3339()], |row| {
            Ok(WeeklyToolCount {
                tool: row.get::<_, String>(0)?,
                count: row.get::<_, i64>(1)? as u64,
            })
        })?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let critical: u64 = conn.query_row(
        "SELECT COUNT(*) FROM analysis_results WHERE timestamp BETWEEN ?1 AND ?2 AND risk_level='Critical'",
        [start_utc.to_rfc3339(), end_utc.to_rfc3339()],
        |r| r.get::<_, i64>(0).map(|v| v as u64),
    )?;
    let warning: u64 = conn.query_row(
        "SELECT COUNT(*) FROM analysis_results WHERE timestamp BETWEEN ?1 AND ?2 AND risk_level='Warning'",
        [start_utc.to_rfc3339(), end_utc.to_rfc3339()],
        |r| r.get::<_, i64>(0).map(|v| v as u64),
    )?;
    let info: u64 = conn.query_row(
        "SELECT COUNT(*) FROM analysis_results WHERE timestamp BETWEEN ?1 AND ?2 AND risk_level='Info'",
        [start_utc.to_rfc3339(), end_utc.to_rfc3339()],
        |r| r.get::<_, i64>(0).map(|v| v as u64),
    )?;

    let mut patterns = Vec::new();
    let mut patt_stmt = conn.prepare(
        "SELECT content, COUNT(*) as c FROM actions WHERE timestamp BETWEEN ?1 AND ?2 GROUP BY content HAVING c >= 3 ORDER BY c DESC LIMIT 3",
    )?;
    let patt_rows = patt_stmt.query_map([start_utc.to_rfc3339(), end_utc.to_rfc3339()], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
    })?;
    for row in patt_rows {
        let (name, count) = row?;
        patterns.push(WeeklyPattern {
            name: if name.len() > 70 { format!("{}…", &name[..70]) } else { name },
            count,
            suggestion: "반복 작업은 스크립트/자동화 후보로 검토".to_string(),
        });
    }

    let next_actions = vec![
        "상위 반복 작업 1개 자동화 스크립트로 전환".to_string(),
        "Warning 규칙 false-positive 1건 정밀 조정".to_string(),
        "주요 프로젝트별 decision note 자동 생성 활성화".to_string(),
    ];

    let headline = if critical > 0 {
        "Critical 이벤트가 감지되어 정책 강화가 필요함".to_string()
    } else if warning > 0 {
        "Warning 이벤트 중심으로 정책 튜닝이 필요한 주간".to_string()
    } else {
        "안정적인 주간 활동 (risk low)".to_string()
    };

    let mut report = WeeklyReportResponse {
        report_id,
        workspace_id: workspace,
        week_start: start_utc.to_rfc3339(),
        week_end: end_utc.to_rfc3339(),
        headline,
        activity: WeeklyActivity {
            total_events,
            projects,
            top_tools,
        },
        risk: WeeklyRisk { critical, warning, info },
        patterns,
        next_actions,
        markdown: String::new(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    report.markdown = build_markdown(&report);
    Ok(report)
}

pub async fn get_weekly_report(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WeeklyReportQuery>,
) -> Result<Json<WeeklyReportResponse>, StatusCode> {
    compute_weekly_report(&state.db_path, query.week, None)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn generate_weekly_report(
    State(state): State<Arc<AppState>>,
    Json(body): Json<GenerateWeeklyReportRequest>,
) -> Result<Json<WeeklyReportResponse>, StatusCode> {
    let _ = body.timezone;
    let _ = body.force_regenerate;
    let report = compute_weekly_report(&state.db_path, body.week, body.workspace_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let base_dir = StdPath::new("data");
    persist_weekly_outputs(base_dir, &report).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    materialize_ontology_minimal(base_dir, &report)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(report))
}

#[cfg(test)]
mod brain_report_tests {
    use super::*;

    #[test]
    fn test_week_range_kst_parses_iso_week() {
        let (report_id, start, end) = week_range_kst(Some("2026-W09".to_string())).unwrap();
        assert_eq!(report_id, "2026-W09");
        assert!(end > start);
    }

    #[test]
    fn test_persist_and_materialize_outputs() {
        let tmp = tempfile::tempdir().unwrap();
        let report = WeeklyReportResponse {
            report_id: "2026-W09".to_string(),
            workspace_id: "default".to_string(),
            week_start: "2026-02-23T00:00:00Z".to_string(),
            week_end: "2026-03-01T23:59:59Z".to_string(),
            headline: "test headline".to_string(),
            activity: WeeklyActivity {
                total_events: 10,
                projects: vec![WeeklyProjectActivity {
                    project_id: "proj:safebot".to_string(),
                    events: 7,
                }],
                top_tools: vec![WeeklyToolCount {
                    tool: "Exec".to_string(),
                    count: 5,
                }],
            },
            risk: WeeklyRisk {
                critical: 1,
                warning: 2,
                info: 3,
            },
            patterns: vec![],
            next_actions: vec!["do x".to_string()],
            markdown: "# test".to_string(),
            created_at: "2026-02-27T00:00:00Z".to_string(),
        };

        persist_weekly_outputs(tmp.path(), &report).unwrap();
        materialize_ontology_minimal(tmp.path(), &report).unwrap();

        assert!(tmp
            .path()
            .join("reports/weekly/2026-W09.md")
            .exists());
        assert!(tmp
            .path()
            .join("reports/weekly/2026-W09.json")
            .exists());
        assert!(tmp.path().join("ontology/nodes.jsonl").exists());
        assert!(tmp.path().join("ontology/edges.jsonl").exists());
    }
}
