//! Proxy configuration

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_target")]
    pub target: String,
    #[serde(default = "default_mode")]
    pub mode: ProxyMode,
    #[serde(default)]
    pub streaming: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    /// Log only, pass everything through
    Monitor,
    /// Actually block dangerous tool_use
    Enforce,
}

fn default_enabled() -> bool {
    true
}
fn default_listen() -> String {
    "127.0.0.1:9090".to_string()
}
fn default_target() -> String {
    "https://api.anthropic.com".to_string()
}
fn default_mode() -> ProxyMode {
    ProxyMode::Enforce
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            listen: default_listen(),
            target: default_target(),
            mode: default_mode(),
            streaming: false,
        }
    }
}
