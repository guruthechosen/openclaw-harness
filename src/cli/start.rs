//! Start command - launches the OpenClaw Harness daemon

use openclaw_harness::collectors::{Collector, openclaw::OpenclawCollector};
use openclaw_harness::analyzer::Analyzer;
use openclaw_harness::enforcer::alerter::Alerter;
use openclaw_harness::rules::{default_rules, load_rules_from_file};
use openclaw_harness::web::{self, WebEvent};
use openclaw_harness::{AgentAction, RiskLevel, Recommendation, AlertConfig, TelegramConfig};
use std::fs;
use std::process::Command;
use tokio::sync::{mpsc, broadcast};
use sha2::{Sha256, Digest};
use tracing::{info, warn, error};

const PID_FILE: &str = "/tmp/openclaw-harness.pid";
const CONFIG_HASH_FILE: &str = "/tmp/openclaw-harness-config.hash";

/// Compute SHA256 hash of a file
fn compute_config_hash(path: &std::path::Path) -> Option<String> {
    let data = fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Some(format!("{:x}", hasher.finalize()))
}

pub async fn run(foreground: bool) -> anyhow::Result<()> {
    // Check if already running
    if is_running() {
        println!("⚠️  OpenClaw Harness is already running!");
        return Ok(());
    }

    if foreground {
        info!("Running in foreground mode");
        run_daemon().await
    } else {
        info!("Daemonizing...");
        daemonize().await
    }
}

fn is_running() -> bool {
    if let Ok(pid_str) = fs::read_to_string(PID_FILE) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            unsafe {
                if libc::kill(pid, 0) == 0 {
                    return true;
                }
            }
        }
    }
    false
}

fn write_pid() -> anyhow::Result<()> {
    let pid = std::process::id();
    fs::write(PID_FILE, pid.to_string())?;
    Ok(())
}

fn remove_pid() {
    let _ = fs::remove_file(PID_FILE);
}

async fn daemonize() -> anyhow::Result<()> {
    println!("🛡️  Starting OpenClaw Harness daemon...");
    run_daemon().await
}

/// Load Telegram config from environment variables
fn load_telegram_config() -> Option<TelegramConfig> {
    let bot_token = std::env::var("OPENCLAW_HARNESS_TELEGRAM_BOT_TOKEN")
        .or_else(|_| std::env::var("SAFEBOT_TELEGRAM_BOT_TOKEN"))
        .ok()?;
    let chat_id = std::env::var("OPENCLAW_HARNESS_TELEGRAM_CHAT_ID")
        .or_else(|_| std::env::var("SAFEBOT_TELEGRAM_CHAT_ID"))
        .ok()?;
    
    if bot_token.is_empty() || chat_id.is_empty() {
        return None;
    }
    
    Some(TelegramConfig { bot_token, chat_id })
}

/// Attempt to interrupt Clawdbot
async fn block_action(action: &AgentAction) -> anyhow::Result<()> {
    info!("🛑 Attempting to block action...");
    
    // Find OpenClaw gateway process and send interrupt
    // This is a best-effort approach - OpenClaw may have already executed the action
    
    // Method 1: Send SIGINT to the OpenClaw process if we can find it
    if let Some(session_id) = &action.session_id {
        // Try to find the session and interrupt it
        // Try openclaw first, then clawdbot for backward compat
        let result = Command::new("pkill")
            .args(["-INT", "-f", "openclaw.*gateway|clawdbot.*gateway"])
            .output();
        
        match result {
            Ok(output) => {
                if output.status.success() {
                    info!("🛑 Sent interrupt signal to OpenClaw");
                } else {
                    warn!("⚠️  Could not interrupt OpenClaw (may not be running or already finished)");
                }
            }
            Err(e) => {
                warn!("⚠️  Failed to send interrupt: {}", e);
            }
        }
    }
    
    // Method 2: Write a block marker that Clawdbot could check
    // (Would require Clawdbot integration)
    let block_file = "/tmp/openclaw-harness_block";
    let block_info = serde_json::json!({
        "blocked_at": chrono::Utc::now().to_rfc3339(),
        "action_id": action.id,
        "reason": "OpenClaw Harness security block",
    });
    fs::write(block_file, block_info.to_string())?;
    
    Ok(())
}

async fn run_daemon() -> anyhow::Result<()> {
    // Write PID file
    write_pid()?;
    
    // Setup cleanup on exit
    let _guard = scopeguard::guard((), |_| {
        remove_pid();
    });

    info!("🛡️ OpenClaw Harness daemon starting (PID: {})...", std::process::id());
    
    // Load rules (config file first, fallback to defaults)
    let config_path = std::path::Path::new("config/rules.yaml");
    let rules = if config_path.exists() {
        match load_rules_from_file(config_path) {
            Ok(r) => {
                info!("📜 Loaded {} rules from config/rules.yaml", r.len());
                r
            }
            Err(e) => {
                warn!("⚠️ Failed to load config/rules.yaml: {}, using defaults", e);
                let r = default_rules();
                info!("📜 Loaded {} default rules", r.len());
                r
            }
        }
    } else {
        let r = default_rules();
        info!("📜 Loaded {} default rules", r.len());
        r
    };

    // Config integrity: compute and store initial hash
    let config_hash = if config_path.exists() {
        let hash = compute_config_hash(config_path);
        if let Some(ref h) = hash {
            let _ = fs::write(CONFIG_HASH_FILE, h);
            info!("🔐 Config integrity hash stored: {}...{}", &h[..8], &h[h.len()-8..]);
        }
        hash
    } else {
        None
    };
    let config_hash_ref = config_hash.clone();
    
    // Create broadcast channel for web events
    let (web_tx, _) = broadcast::channel::<WebEvent>(100);
    let web_tx_clone = web_tx.clone();
    
    // Start web server
    let web_port = std::env::var("OPENCLAW_HARNESS_WEB_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8380);
    
    let db_path = "~/.openclaw-harness/openclaw-harness.db".to_string();
    tokio::spawn(async move {
        if let Err(e) = web::start_server(web_port, web_tx_clone, db_path, None).await {
            error!("Web server error: {}", e);
        }
    });
    
    // Create analyzer
    let analyzer = Analyzer::new(rules);
    
    // Load alert config from environment
    let telegram_config = load_telegram_config();
    let alerter = if telegram_config.is_some() {
        info!("📱 Telegram alerts enabled");
        Some(Alerter::new(AlertConfig {
            telegram: telegram_config,
            slack: None,
            discord: None,
        }))
    } else {
        warn!("⚠️  No Telegram config found (set OPENCLAW_HARNESS_TELEGRAM_BOT_TOKEN and OPENCLAW_HARNESS_TELEGRAM_CHAT_ID)");
        None
    };
    
    // Create channel for actions
    let (tx, mut rx) = mpsc::channel::<AgentAction>(100);
    
    // Start OpenClaw collector
    let collector = OpenclawCollector::new();
    if collector.is_available() {
        info!("🦞 OpenClaw collector available");
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            if let Err(e) = collector.start(tx_clone).await {
                error!("OpenClaw collector error: {}", e);
            }
        });
    } else {
        warn!("⚠️  OpenClaw sessions directory not found");
    }
    
    info!("✅ OpenClaw Harness daemon started successfully");
    info!("👀 Monitoring for AI agent actions...");
    
    // Keep tx alive to prevent channel from closing
    let _tx_keepalive = tx;
    
    info!("🔄 Entering main event loop...");
    
    // Main event loop - process actions
    loop {
        tokio::select! {
            action_opt = rx.recv() => {
                match action_opt {
                    Some(action) => {
                        info!("📥 Received action: {} - {}", action.action_type, truncate(&action.content, 50));
                        
                        // Broadcast to web clients
                        let _ = web_tx.send(WebEvent::from(&action));
                        
                        // Analyze the action
                        let result = analyzer.analyze(&action);
                        
                        // Broadcast analysis result
                        let _ = web_tx.send(WebEvent::from(&result));
                        
                        // Handle based on result
                        if result.matched_rules.is_empty() {
                            continue;
                        }
                        
                        match result.risk_level {
                            RiskLevel::Critical => {
                                error!("🚨 CRITICAL: {} (rules: {:?})", 
                                    result.explanation, result.matched_rules);
                                
                                // Send alert
                                if let Some(ref alerter) = alerter {
                                    if let Err(e) = alerter.send_alert(&result).await {
                                        error!("Failed to send alert: {}", e);
                                    }
                                }
                                
                                match result.recommendation {
                                    Recommendation::CriticalAlert => {
                                        error!("🛑 ACTION BLOCKED");
                                        if let Err(e) = block_action(&action).await {
                                            error!("Failed to block: {}", e);
                                        }
                                    }
                                    Recommendation::PauseAndAsk => {
                                        warn!("⏸️  Requires user approval");
                                        // Send alert for approval
                                        if let Some(ref alerter) = alerter {
                                            let _ = alerter.send_alert(&result).await;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            RiskLevel::Warning => {
                                warn!("⚠️  WARNING: {} (rules: {:?})", 
                                    result.explanation, result.matched_rules);
                                
                                // Send alert for warnings too
                                if let Some(ref alerter) = alerter {
                                    if let Err(e) = alerter.send_alert(&result).await {
                                        error!("Failed to send alert: {}", e);
                                    }
                                }
                            }
                            RiskLevel::Info => {
                                info!("ℹ️  INFO: {}", result.explanation);
                                // Don't send alerts for info level
                            }
                        }
                    }
                    None => {
                        warn!("⚠️  Channel closed, all senders dropped");
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
            // Heartbeat + config integrity check every 30 seconds
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                info!("💓 Daemon heartbeat - still monitoring...");

                // Config integrity check
                if let Some(ref original_hash) = config_hash_ref {
                    if config_path.exists() {
                        if let Some(current_hash) = compute_config_hash(config_path) {
                            if &current_hash != original_hash {
                                error!("🚨 CONFIG TAMPERING DETECTED: rules.yaml was modified externally!");
                                error!("🚨 Expected: {}..., Got: {}...", &original_hash[..16], &current_hash[..16]);
                                error!("🚨 Ignoring tampered config — keeping original in-memory rules");

                                // Send Telegram alert
                                if let Some(ref alerter) = alerter {
                                    let tamper_action = AgentAction {
                                        id: format!("tamper-{}", chrono::Utc::now().timestamp()),
                                        timestamp: chrono::Utc::now(),
                                        agent: openclaw_harness::AgentType::Unknown,
                                        action_type: openclaw_harness::ActionType::FileWrite,
                                        content: "CONFIG TAMPERING: rules.yaml was modified externally".to_string(),
                                        target: Some("config/rules.yaml".to_string()),
                                        session_id: None,
                                        metadata: None,
                                    };
                                    let tamper_result = openclaw_harness::AnalysisResult {
                                        action: tamper_action,
                                        risk_level: RiskLevel::Critical,
                                        matched_rules: vec!["CONFIG_TAMPERING".to_string()],
                                        explanation: "⚠️ CONFIG TAMPERING DETECTED: rules.yaml was modified externally! Original rules kept in memory.".to_string(),
                                        recommendation: Recommendation::CriticalAlert,
                                    };
                                    if let Err(e) = alerter.send_alert(&tamper_result).await {
                                        error!("Failed to send tampering alert: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        // Find a valid char boundary at or before max
        let mut end = max;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    } else {
        s.to_string()
    }
}
