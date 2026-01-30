//! MoltBot Harness - AI Agent Monitoring Daemon
//!
//! Monitors AI agents (OpenClaw, Claude Code, Cursor) for risky actions
//! and alerts/blocks based on configurable rules.

use clap::{Parser, Subcommand};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

// Re-export from library
pub use openclaw_harness::*;

mod cli;

/// MoltBot Harness - AI Agent Security Monitor
#[derive(Parser)]
#[command(name = "openclaw-harness")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MoltBot Harness daemon
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,
    },

    /// Stop the running daemon
    Stop,

    /// Show daemon status
    Status,

    /// View recent activity logs
    Logs {
        /// Number of recent entries to show
        #[arg(short, long, default_value = "20")]
        tail: usize,

        /// Filter by agent (openclaw, claude, cursor)
        #[arg(short, long)]
        agent: Option<String>,

        /// Filter by risk level (critical, warning, info)
        #[arg(short, long)]
        level: Option<String>,
    },

    /// Interactive TUI dashboard
    Tui,

    /// Manage rules
    Rules {
        #[command(subcommand)]
        action: RulesAction,
    },

    /// Test a specific rule against sample input
    Test {
        /// Rule name to test
        rule: String,
        /// Sample input to test against
        input: String,
    },

    /// API Proxy — intercept Anthropic API responses
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },

    /// Patch external tools to wire up hooks
    Patch {
        /// Target to patch (e.g., "clawdbot")
        target: String,
        /// Revert the patch
        #[arg(long)]
        revert: bool,
        /// Check patch status without modifying
        #[arg(long)]
        check: bool,
    },
}

#[derive(Subcommand)]
enum ProxyAction {
    /// Start the proxy server
    Start {
        /// Port to listen on
        #[arg(short, long)]
        port: Option<u16>,
        /// Target API URL
        #[arg(short, long)]
        target: Option<String>,
        /// Mode: monitor or enforce
        #[arg(short, long)]
        mode: Option<String>,
    },
    /// Check proxy status
    Status,
}

#[derive(Subcommand)]
enum RulesAction {
    /// List all rules
    List,
    /// Enable a rule
    Enable { name: String },
    /// Disable a rule
    Disable { name: String },
    /// Show rule details
    Show { name: String },
    /// Reload rules from config
    Reload,
    /// List available rule templates
    Templates,
    /// Add a new rule
    Add {
        /// Rule name
        #[arg(long)]
        name: Option<String>,

        /// Template to use (e.g., protect_path, block_sudo)
        #[arg(long)]
        template: Option<String>,

        /// Path parameter for templates
        #[arg(long)]
        path: Option<String>,

        /// Operations: read,write,delete
        #[arg(long)]
        operations: Option<String>,

        /// Commands to block
        #[arg(long)]
        commands: Option<String>,

        /// Keyword contains (comma-separated)
        #[arg(long)]
        keyword_contains: Option<String>,

        /// Keyword starts_with (comma-separated)
        #[arg(long)]
        keyword_starts_with: Option<String>,

        /// Keyword any_of (comma-separated)
        #[arg(long)]
        keyword_any_of: Option<String>,

        /// Risk level: info, warning, critical
        #[arg(long)]
        risk: Option<String>,

        /// Action: log_only, alert, pause_and_ask, block, critical_alert
        #[arg(long, name = "action")]
        rule_action: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let level = match cli.verbose {
        0 => Level::INFO,
        1 => Level::DEBUG,
        _ => Level::TRACE,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    match cli.command {
        Commands::Start { foreground } => {
            info!("🛡️ Starting MoltBot Harness daemon...");
            cli::start::run(foreground).await?;
        }
        Commands::Stop => {
            info!("Stopping MoltBot Harness daemon...");
            cli::stop::run().await?;
        }
        Commands::Status => {
            cli::status::run().await?;
        }
        Commands::Logs { tail, agent, level } => {
            cli::logs::run(tail, agent, level).await?;
        }
        Commands::Tui => {
            info!("Launching TUI dashboard...");
            cli::tui::run().await?;
        }
        Commands::Rules { action } => {
            match action {
                RulesAction::List => cli::rules::list().await?,
                RulesAction::Enable { name } => cli::rules::enable(&name).await?,
                RulesAction::Disable { name } => cli::rules::disable(&name).await?,
                RulesAction::Show { name } => cli::rules::show(&name).await?,
                RulesAction::Reload => cli::rules::reload().await?,
                RulesAction::Templates => cli::rules::templates().await?,
                RulesAction::Add {
                    name,
                    template,
                    path,
                    operations,
                    commands,
                    keyword_contains,
                    keyword_starts_with,
                    keyword_any_of,
                    risk,
                    rule_action,
                } => {
                    if let Some(ref tmpl) = template {
                        let rule_name = name.as_deref().unwrap_or(tmpl);
                        cli::rules::add_template(
                            rule_name,
                            tmpl,
                            path.as_deref(),
                            operations.as_deref(),
                            commands.as_deref(),
                            risk.as_deref(),
                            rule_action.as_deref(),
                        ).await?;
                    } else if keyword_contains.is_some() || keyword_starts_with.is_some() || keyword_any_of.is_some() {
                        let rule_name = name.as_deref().unwrap_or("custom_keyword_rule");
                        cli::rules::add_keyword(
                            rule_name,
                            keyword_contains.as_deref(),
                            keyword_starts_with.as_deref(),
                            keyword_any_of.as_deref(),
                            risk.as_deref(),
                            rule_action.as_deref(),
                        ).await?;
                    } else {
                        eprintln!("Error: Specify --template or --keyword-contains/--keyword-any-of");
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Test { rule, input } => {
            cli::test::run(&rule, &input).await?;
        }
        Commands::Patch { target, revert, check } => {
            let mode = if check {
                cli::patch::PatchMode::Check
            } else if revert {
                cli::patch::PatchMode::Revert
            } else {
                cli::patch::PatchMode::Apply
            };
            cli::patch::run(&target, mode).await?;
        }
        Commands::Proxy { action } => {
            match action {
                ProxyAction::Start { port, target, mode } => {
                    info!("🛡️ Starting MoltBot Harness API Proxy...");
                    cli::proxy::start(port, target, mode).await?;
                }
                ProxyAction::Status => {
                    cli::proxy::status().await?;
                }
            }
        }
    }

    Ok(())
}
