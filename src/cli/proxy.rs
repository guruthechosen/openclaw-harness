//! CLI handler for the proxy subcommand

use openclaw_harness::proxy::config::{ProxyConfig, ProxyMode};
use openclaw_harness::proxy::start_proxy;
use openclaw_harness::{AlertConfig, TelegramConfig};
use tracing::info;

pub async fn start(port: Option<u16>, target: Option<String>, mode: Option<String>) -> anyhow::Result<()> {
    let mut config = ProxyConfig::default();

    if let Some(p) = port {
        config.listen = format!("127.0.0.1:{}", p);
    }
    if let Some(t) = target {
        config.target = t;
    }
    if let Some(m) = mode {
        config.mode = match m.as_str() {
            "monitor" => ProxyMode::Monitor,
            "enforce" => ProxyMode::Enforce,
            _ => {
                eprintln!("Unknown mode '{}', using enforce", m);
                ProxyMode::Enforce
            }
        };
    }

    // Try to load Telegram config from environment
    let alert_config = match (
        std::env::var("OPENCLAW_HARNESS_TELEGRAM_BOT_TOKEN"),
        std::env::var("OPENCLAW_HARNESS_TELEGRAM_CHAT_ID"),
    ) {
        (Ok(token), Ok(chat_id)) => {
            info!("Telegram alerts enabled");
            Some(AlertConfig {
                telegram: Some(TelegramConfig {
                    bot_token: token,
                    chat_id,
                }),
                slack: None,
                discord: None,
            })
        }
        _ => {
            info!("Telegram alerts not configured (set OPENCLAW_HARNESS_TELEGRAM_BOT_TOKEN and OPENCLAW_HARNESS_TELEGRAM_CHAT_ID)");
            None
        }
    };

    start_proxy(config, alert_config).await
}

pub async fn status() -> anyhow::Result<()> {
    // Simple status check — try to connect to the proxy port
    let client = reqwest::Client::new();
    match client.get("http://127.0.0.1:9090/health").send().await {
        Ok(_) => println!("✅ MoltBot Harness proxy is running on 127.0.0.1:9090"),
        Err(_) => println!("❌ MoltBot Harness proxy is not running (or not on default port 9090)"),
    }
    Ok(())
}
