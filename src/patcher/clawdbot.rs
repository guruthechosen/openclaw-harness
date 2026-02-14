//! OpenClaw patcher — injects before_tool_call hook into exec, write, and edit tools
//!
//! Patches:
//!   - `dist/agents/bash-tools.exec.js` — exec tool hook (v1)
//!   - `dist/agents/pi-tools.js` — write/edit tool hooks (v2)
//!
//! Supports both OpenClaw (2026.1.29+, including 2026.1.30) and legacy Clawdbot (2026.1.24-3).

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ============================================================
// V1 Patch — exec tool (bash-tools.exec.js)
// ============================================================

const PATCH_MARKER: &str = "// OPENCLAW_HARNESS_PATCH_v1";
#[allow(dead_code)]
const BACKUP_EXT: &str = ".orig";

/// The anchor text we search for in bash-tools.exec.js to find the injection point.
const ANCHOR_TEXT: &str = r#"if (!params.command) {
                throw new Error("Provide a command to start.");
            }"#;

/// The code to inject after the anchor for exec tool.
const PATCH_CODE: &str = r#"
            // OPENCLAW_HARNESS_PATCH_v1 — before_tool_call hook for exec
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
            // END OPENCLAW_HARNESS_PATCH_v1"#;

// ============================================================
// V2 Patch — write/edit tools (pi-tools.js)
// ============================================================

const PATCH_V2_MARKER: &str = "// OPENCLAW_HARNESS_PATCH_v2";

/// The anchor text we search for in pi-tools.js — the original write/edit tool creation.
const WRITE_EDIT_ANCHOR: &str = r#"if (tool.name === "write") {
            if (sandboxRoot)
                return [];
            // Wrap with param normalization for Claude Code compatibility
            return [
                wrapToolParamNormalization(createWriteTool(workspaceRoot), CLAUDE_PARAM_GROUPS.write),
            ];
        }
        if (tool.name === "edit") {
            if (sandboxRoot)
                return [];
            // Wrap with param normalization for Claude Code compatibility
            return [wrapToolParamNormalization(createEditTool(workspaceRoot), CLAUDE_PARAM_GROUPS.edit)];
        }"#;

/// Replacement code that wraps write/edit with before_tool_call hooks.
const WRITE_EDIT_REPLACEMENT: &str = r#"if (tool.name === "write") {
            if (sandboxRoot)
                return [];
            // Wrap with param normalization for Claude Code compatibility
            const _writeTool = wrapToolParamNormalization(createWriteTool(workspaceRoot), CLAUDE_PARAM_GROUPS.write);
            // OPENCLAW_HARNESS_PATCH_v2 — before_tool_call hook for write
            const _origWriteExec = _writeTool.execute;
            _writeTool.execute = async (toolCallId, params, signal, onUpdate) => {
                const { getGlobalHookRunner } = await import("../plugins/hook-runner-global.js");
                const _hookRunner = getGlobalHookRunner();
                if (_hookRunner) {
                    const _normalized = params && typeof params === "object" ? params : {};
                    const _hookResult = await _hookRunner.runBeforeToolCall({
                        toolName: "write",
                        params: { path: _normalized.path || _normalized.file_path, content: _normalized.content },
                    }, {});
                    if (_hookResult?.block) {
                        throw new Error(_hookResult.blockReason || "Blocked by before_tool_call hook");
                    }
                }
                return _origWriteExec(toolCallId, params, signal, onUpdate);
            };
            // END OPENCLAW_HARNESS_PATCH_v2
            return [_writeTool];
        }
        if (tool.name === "edit") {
            if (sandboxRoot)
                return [];
            // Wrap with param normalization for Claude Code compatibility
            const _editTool = wrapToolParamNormalization(createEditTool(workspaceRoot), CLAUDE_PARAM_GROUPS.edit);
            // OPENCLAW_HARNESS_PATCH_v2 — before_tool_call hook for edit
            const _origEditExec = _editTool.execute;
            _editTool.execute = async (toolCallId, params, signal, onUpdate) => {
                const { getGlobalHookRunner } = await import("../plugins/hook-runner-global.js");
                const _hookRunner = getGlobalHookRunner();
                if (_hookRunner) {
                    const _normalized = params && typeof params === "object" ? params : {};
                    const _hookResult = await _hookRunner.runBeforeToolCall({
                        toolName: "edit",
                        params: { path: _normalized.path || _normalized.file_path, oldText: _normalized.oldText || _normalized.old_string, newText: _normalized.newText || _normalized.new_string },
                    }, {});
                    if (_hookResult?.block) {
                        throw new Error(_hookResult.blockReason || "Blocked by before_tool_call hook");
                    }
                }
                return _origEditExec(toolCallId, params, signal, onUpdate);
            };
            // END OPENCLAW_HARNESS_PATCH_v2
            return [_editTool];
        }"#;

// ============================================================
// Dist directory discovery
// ============================================================

/// Locate the OpenClaw (or legacy Clawdbot) dist directory.
pub fn find_clawdbot_dist() -> Result<PathBuf> {
    for bin_name in &["openclaw", "clawdbot"] {
        if let Ok(dist) = find_dist_for_binary(bin_name) {
            return Ok(dist);
        }
    }

    // Fallback: try common nvm path pattern
    let nvm_base = dirs::home_dir().map(|h| h.join(".nvm/versions/node"));
    if let Some(nvm_base) = nvm_base {
        if nvm_base.is_dir() {
            if let Ok(entries) = fs::read_dir(&nvm_base) {
                for entry in entries.flatten() {
                    for pkg_name in &["openclaw", "clawdbot"] {
                        let dist = entry
                            .path()
                            .join(format!("lib/node_modules/{}/dist", pkg_name));
                        if dist.is_dir() {
                            return Ok(dist);
                        }
                    }
                }
            }
        }
    }

    bail!(
        "Could not find OpenClaw or Clawdbot dist/ directory. \
         Is openclaw (or clawdbot) installed?"
    );
}

fn find_dist_for_binary(bin_name: &str) -> Result<PathBuf> {
    let output = Command::new("which")
        .arg(bin_name)
        .output()
        .with_context(|| format!("Failed to run `which {}`", bin_name))?;

    if !output.status.success() {
        bail!("{} not found in PATH", bin_name);
    }

    let bin_path_str = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in `which` output")?
        .trim()
        .to_string();

    let resolved = fs::canonicalize(&bin_path_str)
        .with_context(|| format!("Cannot resolve symlink for {}", bin_path_str))?;

    let mut current = resolved.as_path();
    loop {
        if current.ends_with(bin_name) {
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

    bail!(
        "Could not find dist/ directory for {}. Resolved binary: {}",
        bin_name,
        resolved.display()
    );
}

// ============================================================
// File paths
// ============================================================

fn exec_file(dist: &Path) -> PathBuf {
    dist.join("agents/bash-tools.exec.js")
}

fn pi_tools_file(dist: &Path) -> PathBuf {
    dist.join("agents/pi-tools.js")
}

fn bundled_loader_file(dist: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dist).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("loader-") && name.ends_with(".js") {
                return Some(path);
            }
        }
    }
    None
}

pub fn has_builtin_before_tool_call(dist: &Path) -> Result<bool> {
    let Some(loader) = bundled_loader_file(dist) else {
        return Ok(false);
    };
    let content =
        fs::read_to_string(&loader).with_context(|| format!("Cannot read {}", loader.display()))?;
    Ok(content.contains("wrapToolWithBeforeToolCallHook") && content.contains("before_tool_call"))
}

// ============================================================
// Check patch status
// ============================================================

/// Check if v1 (exec) patch is applied.
pub fn is_patched(dist: &Path) -> Result<bool> {
    let file = exec_file(dist);
    if !file.exists() {
        // New bundled OpenClaw builds may not have agents/*.js
        if has_builtin_before_tool_call(dist)? {
            return Ok(true);
        }
        bail!("Exec tool file not found: {}", file.display());
    }
    let content =
        fs::read_to_string(&file).with_context(|| format!("Cannot read {}", file.display()))?;
    Ok(content.contains(PATCH_MARKER))
}

/// Check if v2 (write/edit) patch is applied.
pub fn is_v2_patched(dist: &Path) -> Result<bool> {
    let file = pi_tools_file(dist);
    if !file.exists() {
        // New bundled OpenClaw builds may not have agents/*.js
        if has_builtin_before_tool_call(dist)? {
            return Ok(true);
        }
        bail!("pi-tools.js not found: {}", file.display());
    }
    let content =
        fs::read_to_string(&file).with_context(|| format!("Cannot read {}", file.display()))?;
    Ok(content.contains(PATCH_V2_MARKER))
}

// ============================================================
// Version detection
// ============================================================

const SUPPORTED_VERSIONS: &[&str] = &[
    "2026.1.24-3",
    "2026.1.29",
    "2026.1.30",
    "2026.2.2-3",
    "2026.2.3-1",
    "2026.2.6-3",
    "2026.2.9",
    "2026.2.12",
];

pub fn detect_clawdbot_version() -> Option<String> {
    for bin_name in &["openclaw", "clawdbot"] {
        let output = Command::new(bin_name).arg("--version").output().ok()?;
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !version.is_empty() {
                return Some(version);
            }
        }
    }
    None
}

// ============================================================
// Apply patches
// ============================================================

/// Apply both v1 and v2 patches.
pub fn apply_patch(dist: &Path) -> Result<()> {
    // Version compatibility check
    if let Some(version) = detect_clawdbot_version() {
        println!("📌 Detected OpenClaw version: {}", version);
        if SUPPORTED_VERSIONS.contains(&version.as_str()) {
            println!("✅ Version {} is supported", version);
        } else {
            println!(
                "⚠️  Version {} is NOT in the tested list: {:?}",
                version, SUPPORTED_VERSIONS
            );
            println!("   The patch may still work if the internal structure hasn't changed.");
            println!("   Proceeding with anchor check...");
        }
    } else {
        println!("⚠️  Could not detect OpenClaw version");
    }

    // New bundled OpenClaw builds already wrap tools with before_tool_call
    if has_builtin_before_tool_call(dist)? {
        println!("✅ OpenClaw has built-in before_tool_call hooks (no patch needed)");
        return Ok(());
    }

    // === V1 Patch: exec tool ===
    apply_v1_patch(dist)?;

    // === V2 Patch: write/edit tools ===
    apply_v2_patch(dist)?;

    println!();
    println!("🎉 All patches applied! Restart OpenClaw to activate:");
    println!("   openclaw gateway restart");

    Ok(())
}

fn apply_v1_patch(dist: &Path) -> Result<()> {
    let file = exec_file(dist);
    if !file.exists() {
        bail!("Exec tool file not found: {}", file.display());
    }

    let content =
        fs::read_to_string(&file).with_context(|| format!("Cannot read {}", file.display()))?;

    if content.contains(PATCH_MARKER) {
        println!("✅ [v1] exec hook already patched.");
        return Ok(());
    }

    if !content.contains(ANCHOR_TEXT) {
        bail!(
            "Cannot find injection anchor in {}. \
             OpenClaw version may be incompatible. \
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
        println!("📦 [v1] Backed up original to {}", backup.display());
    }

    let patched = content.replacen(ANCHOR_TEXT, &format!("{}{}", ANCHOR_TEXT, PATCH_CODE), 1);

    fs::write(&file, &patched)
        .with_context(|| format!("Cannot write patched file {}", file.display()))?;

    println!("✅ [v1] Patched exec hook: {}", file.display());
    Ok(())
}

fn apply_v2_patch(dist: &Path) -> Result<()> {
    let file = pi_tools_file(dist);
    if !file.exists() {
        println!(
            "⚠️  [v2] pi-tools.js not found: {}. Skipping write/edit patch.",
            file.display()
        );
        return Ok(());
    }

    let content =
        fs::read_to_string(&file).with_context(|| format!("Cannot read {}", file.display()))?;

    if content.contains(PATCH_V2_MARKER) {
        println!("✅ [v2] write/edit hooks already patched.");
        return Ok(());
    }

    if !content.contains(WRITE_EDIT_ANCHOR) {
        println!(
            "⚠️  [v2] Cannot find write/edit anchor in {}.",
            file.display()
        );
        println!("   OpenClaw version may have changed the write/edit tool structure.");
        println!("   Skipping v2 patch. Exec hook (v1) still works.");
        return Ok(());
    }

    // Backup original
    let backup = file.with_extension("js.orig");
    if !backup.exists() {
        fs::copy(&file, &backup)
            .with_context(|| format!("Cannot backup to {}", backup.display()))?;
        println!("📦 [v2] Backed up original to {}", backup.display());
    }

    // Replace the anchor with hooked version
    let patched = content.replacen(WRITE_EDIT_ANCHOR, WRITE_EDIT_REPLACEMENT, 1);

    fs::write(&file, &patched)
        .with_context(|| format!("Cannot write patched file {}", file.display()))?;

    println!("✅ [v2] Patched write/edit hooks: {}", file.display());
    Ok(())
}

// ============================================================
// Revert patches
// ============================================================

/// Revert both v1 and v2 patches.
pub fn revert_patch(dist: &Path) -> Result<()> {
    revert_v1_patch(dist)?;
    revert_v2_patch(dist)?;
    Ok(())
}

fn revert_v1_patch(dist: &Path) -> Result<()> {
    let file = exec_file(dist);
    let backup = file.with_extension("js.orig");

    if !backup.exists() {
        if !file.exists() {
            bail!("Exec tool file not found: {}", file.display());
        }
        let content = fs::read_to_string(&file)?;
        if !content.contains(PATCH_MARKER) {
            println!("✅ [v1] Not patched, nothing to revert.");
            return Ok(());
        }
        bail!(
            "No backup file found at {}. Cannot safely revert.",
            backup.display()
        );
    }

    fs::copy(&backup, &file)
        .with_context(|| format!("Cannot restore from {}", backup.display()))?;
    fs::remove_file(&backup)?;
    println!("✅ [v1] Reverted exec hook. Backup removed.");
    Ok(())
}

fn revert_v2_patch(dist: &Path) -> Result<()> {
    let file = pi_tools_file(dist);
    let backup = file.with_extension("js.orig");

    if !backup.exists() {
        if !file.exists() {
            println!("✅ [v2] pi-tools.js not found, nothing to revert.");
            return Ok(());
        }
        let content = fs::read_to_string(&file)?;
        if !content.contains(PATCH_V2_MARKER) {
            println!("✅ [v2] Not patched, nothing to revert.");
            return Ok(());
        }
        bail!(
            "No backup file found at {}. Cannot safely revert.",
            backup.display()
        );
    }

    fs::copy(&backup, &file)
        .with_context(|| format!("Cannot restore from {}", backup.display()))?;
    fs::remove_file(&backup)?;
    println!("✅ [v2] Reverted write/edit hooks. Backup removed.");
    Ok(())
}
