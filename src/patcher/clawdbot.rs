//! Clawdbot patcher — injects before_tool_call hook into exec tool
//!
//! Patches `dist/agents/bash-tools.exec.js` to call `runBeforeToolCall`
//! before executing any shell command.

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PATCH_MARKER: &str = "// MOLTBOT_HARNESS_PATCH_v1";
const BACKUP_EXT: &str = ".orig";

/// The anchor text we search for in bash-tools.exec.js to find the injection point.
/// This is the command validation check inside the execute() callback.
const ANCHOR_TEXT: &str = r#"if (!params.command) {
                throw new Error("Provide a command to start.");
            }"#;

/// The code to inject after the anchor. This:
/// 1. Imports getGlobalHookRunner (lazy, at top of execute)
/// 2. Calls runBeforeToolCall with the exec params
/// 3. Blocks execution if hook returns { block: true }
const PATCH_CODE: &str = r#"
            // MOLTBOT_HARNESS_PATCH_v1 — before_tool_call hook for exec
            {
                const { getGlobalHookRunner } = await import("../plugins/hook-runner-global.js");
                const _hookRunner = getGlobalHookRunner();
                if (_hookRunner) {
                    const _hookResult = await _hookRunner.runBeforeToolCall({
                        toolName: "exec",
                        params: { command: params.command, workdir: params.workdir, env: params.env },
                    }, {});
                    if (_hookResult?.block) {
                        throw new Error(_hookResult.blockReason || "Blocked by before_tool_call hook");
                    }
                    if (_hookResult?.params) {
                        if (_hookResult.params.command) params.command = _hookResult.params.command;
                    }
                }
            }
            // END MOLTBOT_HARNESS_PATCH_v1"#;

/// Locate the Clawdbot dist directory by resolving `which clawdbot`.
pub fn find_clawdbot_dist() -> Result<PathBuf> {
    let output = Command::new("which")
        .arg("clawdbot")
        .output()
        .context("Failed to run `which clawdbot`")?;

    if !output.status.success() {
        bail!("clawdbot not found in PATH. Is it installed?");
    }

    let bin_path_str = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in `which` output")?
        .trim()
        .to_string();

    // Resolve symlinks
    let resolved = fs::canonicalize(&bin_path_str)
        .with_context(|| format!("Cannot resolve symlink for {}", bin_path_str))?;

    // clawdbot binary is typically at <prefix>/lib/node_modules/clawdbot/dist/cli/index.js
    // or the bin shim points to it. We need to find the dist/ directory.
    // Walk up from the resolved path to find node_modules/clawdbot/dist/
    let mut current = resolved.as_path();
    loop {
        if current.ends_with("clawdbot") || current.ends_with("clawdbot/") {
            let dist = current.join("dist");
            if dist.is_dir() {
                return Ok(dist);
            }
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    // Fallback: try common nvm path pattern
    let nvm_base = dirs::home_dir()
        .map(|h| h.join(".nvm/versions/node"));
    if let Some(nvm_base) = nvm_base {
        if nvm_base.is_dir() {
            // Find any node version with clawdbot
            if let Ok(entries) = fs::read_dir(&nvm_base) {
                for entry in entries.flatten() {
                    let dist = entry.path().join("lib/node_modules/clawdbot/dist");
                    if dist.is_dir() {
                        return Ok(dist);
                    }
                }
            }
        }
    }

    bail!(
        "Could not find Clawdbot dist/ directory. Resolved binary: {}",
        resolved.display()
    );
}

/// Get the path to the exec tool file.
fn exec_file(dist: &Path) -> PathBuf {
    dist.join("agents/bash-tools.exec.js")
}

/// Check if the file is already patched.
pub fn is_patched(dist: &Path) -> Result<bool> {
    let file = exec_file(dist);
    if !file.exists() {
        bail!("Exec tool file not found: {}", file.display());
    }
    let content = fs::read_to_string(&file)
        .with_context(|| format!("Cannot read {}", file.display()))?;
    Ok(content.contains(PATCH_MARKER))
}

/// Supported Clawdbot versions for this patch.
const SUPPORTED_VERSIONS: &[&str] = &["2026.1.24-3"];

/// Detect the installed Clawdbot version.
pub fn detect_clawdbot_version() -> Option<String> {
    let output = Command::new("clawdbot")
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Apply the patch.
pub fn apply_patch(dist: &Path) -> Result<()> {
    let file = exec_file(dist);
    if !file.exists() {
        bail!("Exec tool file not found: {}", file.display());
    }

    // Version compatibility check
    if let Some(version) = detect_clawdbot_version() {
        println!("📌 Detected Clawdbot version: {}", version);
        if SUPPORTED_VERSIONS.contains(&version.as_str()) {
            println!("✅ Version {} is supported", version);
        } else {
            println!("⚠️  Version {} is NOT in the tested list: {:?}", version, SUPPORTED_VERSIONS);
            println!("   The patch may still work if the internal structure hasn't changed.");
            println!("   Proceeding with anchor check...");
        }
    } else {
        println!("⚠️  Could not detect Clawdbot version (clawdbot --version failed)");
    }

    let content = fs::read_to_string(&file)
        .with_context(|| format!("Cannot read {}", file.display()))?;

    // Check for double-patch
    if content.contains(PATCH_MARKER) {
        println!("✅ Already patched.");
        return Ok(());
    }

    // Verify anchor exists
    if !content.contains(ANCHOR_TEXT) {
        bail!(
            "Cannot find injection anchor in {}. \
             Clawdbot version may be incompatible. \
             Supported versions: {:?}",
            file.display(),
            SUPPORTED_VERSIONS,
        );
    }

    // Backup original
    let backup = file.with_extension("js.orig");
    if !backup.exists() {
        fs::copy(&file, &backup)
            .with_context(|| format!("Cannot backup to {}", backup.display()))?;
        println!("📦 Backed up original to {}", backup.display());
    }

    // Apply patch: insert PATCH_CODE after ANCHOR_TEXT
    let patched = content.replacen(
        ANCHOR_TEXT,
        &format!("{}{}", ANCHOR_TEXT, PATCH_CODE),
        1,
    );

    fs::write(&file, &patched)
        .with_context(|| format!("Cannot write patched file {}", file.display()))?;

    println!("✅ Patched {}", file.display());
    Ok(())
}

/// Revert the patch by restoring the .orig backup.
pub fn revert_patch(dist: &Path) -> Result<()> {
    let file = exec_file(dist);
    let backup = file.with_extension("js.orig");

    if !backup.exists() {
        // No backup — try removing patch markers manually
        if !file.exists() {
            bail!("Exec tool file not found: {}", file.display());
        }
        let content = fs::read_to_string(&file)?;
        if !content.contains(PATCH_MARKER) {
            println!("✅ Not patched, nothing to revert.");
            return Ok(());
        }
        bail!(
            "No backup file found at {}. Cannot safely revert. \
             Please reinstall Clawdbot or manually remove the patch.",
            backup.display()
        );
    }

    fs::copy(&backup, &file)
        .with_context(|| format!("Cannot restore from {}", backup.display()))?;
    fs::remove_file(&backup)?;
    println!("✅ Reverted to original. Backup removed.");
    Ok(())
}
