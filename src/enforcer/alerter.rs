//! Alert sending to various channels

use super::super::{AnalysisResult, AlertConfig, TelegramConfig, SlackConfig, DiscordConfig};
use reqwest::Client;
use serde_json::json;
use tracing::{info, error};

pub struct Alerter {
    client: Client,
    telegram: Option<TelegramConfig>,
    slack: Option<SlackConfig>,
    discord: Option<DiscordConfig>,
}

impl Alerter {
    pub fn new(config: AlertConfig) -> Self {
        Self {
            client: Client::new(),
            telegram: config.telegram,
            slack: config.slack,
            discord: config.discord,
        }
    }

    /// Send an alert to all configured channels
    pub async fn send_alert(&self, result: &AnalysisResult) -> anyhow::Result<()> {
        let message = self.format_message(result);

        // Send to all configured channels concurrently
        let mut handles = vec![];

        if let Some(ref tg) = self.telegram {
            let msg = message.clone();
            let client = self.client.clone();
            let config = tg.clone();
            handles.push(tokio::spawn(async move {
                send_telegram(&client, &config, &msg).await
            }));
        }

        if let Some(ref slack) = self.slack {
            let msg = message.clone();
            let client = self.client.clone();
            let config = slack.clone();
            handles.push(tokio::spawn(async move {
                send_slack(&client, &config, &msg).await
            }));
        }

        if let Some(ref discord) = self.discord {
            let msg = message.clone();
            let client = self.client.clone();
            let config = discord.clone();
            handles.push(tokio::spawn(async move {
                send_discord(&client, &config, &msg).await
            }));
        }

        // Wait for all to complete
        for handle in handles {
            if let Err(e) = handle.await? {
                error!("Failed to send alert: {}", e);
            }
        }

        Ok(())
    }

    fn format_message(&self, result: &AnalysisResult) -> String {
        format!(
            "🛡️ *OpenClaw Harness Alert*\n\n\
            *Risk Level:* {}\n\
            *Agent:* {}\n\
            *Action:* {:?}\n\
            *Content:* `{}`\n\n\
            *Matched Rules:* {}\n\
            *Explanation:* {}",
            result.risk_level,
            result.action.agent,
            result.action.action_type,
            truncate(&result.action.content, 100),
            result.matched_rules.join(", "),
            result.explanation,
        )
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        let mut end = max_len;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    } else {
        s.to_string()
    }
}

async fn send_telegram(client: &Client, config: &TelegramConfig, message: &str) -> anyhow::Result<()> {
    let url = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        config.bot_token
    );

    client
        .post(&url)
        .json(&json!({
            "chat_id": config.chat_id,
            "text": message,
            "parse_mode": "Markdown"
        }))
        .send()
        .await?;

    info!("Sent Telegram alert");
    Ok(())
}

async fn send_slack(client: &Client, config: &SlackConfig, message: &str) -> anyhow::Result<()> {
    client
        .post(&config.webhook_url)
        .json(&json!({
            "text": message
        }))
        .send()
        .await?;

    info!("Sent Slack alert");
    Ok(())
}

async fn send_discord(client: &Client, config: &DiscordConfig, message: &str) -> anyhow::Result<()> {
    client
        .post(&config.webhook_url)
        .json(&json!({
            "content": message
        }))
        .send()
        .await?;

    info!("Sent Discord alert");
    Ok(())
}
