//! OpenClaw Harness Library
//!
//! Core components for AI agent monitoring.

pub mod analyzer;
pub mod collectors;
pub mod db;
pub mod enforcer;
pub mod patcher;
pub mod proxy;
pub mod rules;
pub mod web;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a single action performed by an AI agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAction {
    /// Unique identifier
    pub id: String,
    /// Timestamp of the action
    pub timestamp: DateTime<Utc>,
    /// Source agent (openclaw, claude_code, cursor)
    pub agent: AgentType,
    /// Type of action
    pub action_type: ActionType,
    /// Raw command or content
    pub content: String,
    /// Target (file path, URL, etc.)
    pub target: Option<String>,
    /// Session ID if available
    pub session_id: Option<String>,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Supported AI agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    OpenClaw,
    ClaudeCode,
    Cursor,
    Ralph,
    Unknown,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::OpenClaw => write!(f, "openclaw"),
            AgentType::ClaudeCode => write!(f, "claude_code"),
            AgentType::Cursor => write!(f, "cursor"),
            AgentType::Ralph => write!(f, "ralph"),
            AgentType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Types of actions agents can perform
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Shell command execution
    Exec,
    /// File read
    FileRead,
    /// File write
    FileWrite,
    /// File delete
    FileDelete,
    /// HTTP request
    HttpRequest,
    /// Browser action
    BrowserAction,
    /// Message send
    MessageSend,
    /// Git operation
    GitOperation,
    /// Unknown action
    Unknown,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionType::Exec => write!(f, "exec"),
            ActionType::FileRead => write!(f, "read"),
            ActionType::FileWrite => write!(f, "write"),
            ActionType::FileDelete => write!(f, "delete"),
            ActionType::HttpRequest => write!(f, "http"),
            ActionType::BrowserAction => write!(f, "browser"),
            ActionType::MessageSend => write!(f, "message"),
            ActionType::GitOperation => write!(f, "git"),
            ActionType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Risk level of an action
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// Informational, just logged
    #[default]
    Info,
    /// Warning, may require attention
    Warning,
    /// Critical, should be blocked or require approval
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Info => write!(f, "INFO"),
            RiskLevel::Warning => write!(f, "WARNING"),
            RiskLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Result of analyzing an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// The analyzed action
    pub action: AgentAction,
    /// Matched rules
    pub matched_rules: Vec<String>,
    /// Overall risk level
    pub risk_level: RiskLevel,
    /// Recommended action
    pub recommendation: Recommendation,
    /// Human-readable explanation
    pub explanation: String,
}

/// What to do with a risky action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recommendation {
    /// Just log it
    LogOnly,
    /// Alert the user
    Alert,
    /// Pause and ask for approval
    PauseAndAsk,
    /// Critical alert + attempt interrupt (best-effort, action may have already executed)
    CriticalAlert,
}

/// Configuration for the OpenClaw Harness daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Enabled collectors
    pub collectors: CollectorConfig,
    /// Alert configuration
    pub alerts: AlertConfig,
    /// Database path
    pub db_path: String,
    /// Log retention days
    pub log_retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorConfig {
    pub openclaw: bool,
    pub claude_code: bool,
    pub cursor: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub telegram: Option<TelegramConfig>,
    pub slack: Option<SlackConfig>,
    pub discord: Option<DiscordConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub webhook_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub webhook_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            collectors: CollectorConfig {
                openclaw: true,
                claude_code: true,
                cursor: false,
            },
            alerts: AlertConfig {
                telegram: None,
                slack: None,
                discord: None,
            },
            db_path: "~/.openclaw-harness/openclaw-harness.db".to_string(),
            log_retention_days: 30,
        }
    }
}
