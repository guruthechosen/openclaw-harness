//! Web server for OpenClaw Harness Control Center
//!
//! Provides REST API and WebSocket endpoints for the UI.

pub mod routes;
pub mod ws;

use axum::{
    routing::{get, post, put, delete},
    Router,
};
use tower_http::cors::{CorsLayer, Any};
use tower_http::services::ServeDir;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::info;

use crate::{AgentAction, AnalysisResult};
use crate::rules::Rule;
use crate::proxy::config::{ProxyConfig, ProxyMode};

/// Shared state for the web server
pub struct AppState {
    /// Broadcast channel for real-time events
    pub event_tx: broadcast::Sender<WebEvent>,
    /// Database path
    pub db_path: String,
    /// Mutable rules list
    pub rules: RwLock<Vec<Rule>>,
    /// Proxy configuration
    pub proxy_config: RwLock<ProxyConfig>,
    /// Server start time
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Event counters
    pub counters: RwLock<EventCounters>,
}

/// Runtime event counters
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct EventCounters {
    pub total_requests: u64,
    pub blocked_count: u64,
    pub warning_count: u64,
    pub passed_count: u64,
    pub by_provider: std::collections::HashMap<String, u64>,
}

/// Events sent over WebSocket
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type")]
pub enum WebEvent {
    #[serde(rename = "action")]
    Action {
        id: String,
        timestamp: String,
        agent: String,
        action_type: String,
        content: String,
        target: Option<String>,
    },
    #[serde(rename = "analysis")]
    Analysis {
        action_id: String,
        risk_level: String,
        matched_rules: Vec<String>,
        recommendation: String,
        explanation: String,
    },
    #[serde(rename = "status")]
    Status {
        connected: bool,
        monitoring: Vec<String>,
    },
}

impl From<&AgentAction> for WebEvent {
    fn from(action: &AgentAction) -> Self {
        WebEvent::Action {
            id: action.id.clone(),
            timestamp: action.timestamp.to_rfc3339(),
            agent: action.agent.to_string(),
            action_type: action.action_type.to_string(),
            content: action.content.clone(),
            target: action.target.clone(),
        }
    }
}

impl From<&AnalysisResult> for WebEvent {
    fn from(result: &AnalysisResult) -> Self {
        WebEvent::Analysis {
            action_id: result.action.id.clone(),
            risk_level: result.risk_level.to_string(),
            matched_rules: result.matched_rules.clone(),
            recommendation: format!("{:?}", result.recommendation),
            explanation: result.explanation.clone(),
        }
    }
}

/// Start the web server
pub async fn start_server(
    port: u16,
    event_tx: broadcast::Sender<WebEvent>,
    db_path: String,
    static_dir: Option<String>,
) -> anyhow::Result<()> {
    let mut rules = crate::rules::default_rules();
    for r in &mut rules {
        r.compile()?;
    }

    let state = Arc::new(AppState {
        event_tx,
        db_path,
        rules: RwLock::new(rules),
        proxy_config: RwLock::new(ProxyConfig::default()),
        started_at: chrono::Utc::now(),
        counters: RwLock::new(EventCounters::default()),
    });

    // Build routes
    let mut app = Router::new()
        // API routes
        .route("/api/status", get(routes::get_status))
        .route("/api/stats", get(routes::get_stats))
        .route("/api/stats/by-provider", get(routes::get_stats_by_provider))
        .route("/api/events", get(routes::get_events))
        .route("/api/events/recent", get(routes::get_recent_events))
        .route("/api/events/:id", get(routes::get_event))
        .route("/api/rules", get(routes::get_rules).post(routes::create_rule))
        .route("/api/rules/:name", put(routes::update_rule).delete(routes::delete_rule))
        .route("/api/rules/test", post(routes::test_rule))
        .route("/api/proxy/status", get(routes::get_proxy_status))
        .route("/api/proxy/config", put(routes::update_proxy_config))
        .route("/api/providers", get(routes::get_providers))
        .route("/api/alerts/config", get(routes::get_alert_config).put(routes::update_alert_config))
        // WebSocket
        .route("/ws/events", get(ws::ws_handler))
        .with_state(state)
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any));

    // Serve static files if directory provided
    if let Some(dir) = static_dir {
        app = app.fallback_service(ServeDir::new(dir));
    }

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("🌐 Web server starting on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
