//! `moltbot-harness patch` subcommand

use anyhow::{bail, Result};
use tracing::info;

use crate::patcher::clawdbot;

#[derive(Debug, Clone, Copy)]
pub enum PatchMode {
    Apply,
    Revert,
    Check,
}

pub async fn run(target: &str, mode: PatchMode) -> Result<()> {
    match target {
        "clawdbot" => run_clawdbot(mode),
        _ => bail!("Unknown patch target: '{}'. Supported: clawdbot", target),
    }
}

fn run_clawdbot(mode: PatchMode) -> Result<()> {
    info!("Locating Clawdbot installation...");
    let dist = clawdbot::find_clawdbot_dist()?;
    println!("📍 Found Clawdbot dist: {}", dist.display());

    match mode {
        PatchMode::Check => {
            // Show version info
            if let Some(version) = clawdbot::detect_clawdbot_version() {
                println!("📌 Clawdbot version: {}", version);
            }
            let patched = clawdbot::is_patched(&dist)?;
            if patched {
                println!("✅ Clawdbot is patched (before_tool_call hook active)");
            } else {
                println!("❌ Clawdbot is NOT patched (before_tool_call hook not wired)");
            }
        }
        PatchMode::Apply => {
            println!("🔧 Applying before_tool_call hook patch...");
            clawdbot::apply_patch(&dist)?;
            println!("\n🎉 Patch applied! Restart Clawdbot gateway for changes to take effect:");
            println!("   clawdbot gateway restart");
        }
        PatchMode::Revert => {
            println!("↩️  Reverting patch...");
            clawdbot::revert_patch(&dist)?;
            println!("\n🎉 Patch reverted! Restart Clawdbot gateway for changes to take effect:");
            println!("   clawdbot gateway restart");
        }
    }

    Ok(())
}
