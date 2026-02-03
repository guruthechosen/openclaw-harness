//! `openclaw-harness patch` subcommand

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
        "openclaw" | "clawdbot" => run_openclaw(mode),
        _ => bail!(
            "Unknown patch target: '{}'. Supported: openclaw (or clawdbot)",
            target
        ),
    }
}

fn run_openclaw(mode: PatchMode) -> Result<()> {
    info!("Locating OpenClaw installation...");
    let dist = clawdbot::find_clawdbot_dist()?;
    println!("📍 Found OpenClaw dist: {}", dist.display());

    match mode {
        PatchMode::Check => {
            if let Some(version) = clawdbot::detect_clawdbot_version() {
                println!("📌 OpenClaw version: {}", version);
            }
            let v1 = clawdbot::is_patched(&dist)?;
            let v2 = clawdbot::is_v2_patched(&dist).unwrap_or(false);
            if v1 && v2 {
                println!("✅ OpenClaw is fully patched (exec + write/edit hooks active)");
            } else if v1 {
                println!("⚠️  OpenClaw is partially patched (exec hook active, write/edit hooks missing)");
                println!("   Run: openclaw-harness patch openclaw");
            } else if v2 {
                println!("⚠️  OpenClaw is partially patched (write/edit hooks active, exec hook missing)");
                println!("   Run: openclaw-harness patch openclaw");
            } else {
                println!("❌ OpenClaw is NOT patched (no hooks wired)");
                println!("   Run: openclaw-harness patch openclaw");
            }
        }
        PatchMode::Apply => {
            println!("🔧 Applying before_tool_call hook patches...");
            clawdbot::apply_patch(&dist)?;
        }
        PatchMode::Revert => {
            println!("↩️  Reverting patches...");
            clawdbot::revert_patch(&dist)?;
            println!("\n🎉 Patches reverted! Restart OpenClaw gateway:");
            println!("   openclaw gateway restart");
        }
    }

    Ok(())
}
