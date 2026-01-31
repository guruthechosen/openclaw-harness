# OpenClaw Harness — Clawdbot → OpenClaw Migration Log

**Started:** 2026-01-31
**Completed:** 2026-01-31
**Goal:** Update all references from "Clawdbot" to "OpenClaw" for compatibility with OpenClaw 2026.1.29+

## Migration Steps

| Step | Description | Status |
|------|-------------|--------|
| 1 | Patcher: `clawdbot` → `openclaw` paths + version detection | ✅ Done |
| 2 | Plugin manifest + package.json | ✅ Done |
| 3 | Plugin index.js | ✅ Done |
| 4 | Rust source (remaining files) | ✅ Done |
| 5 | Config / environment variables | ✅ No changes needed |
| 6 | README.md | ✅ Done |
| 7 | Build + integration test | ✅ Done |

---

## Step 1: Patcher Update ✅

**File:** `src/patcher/clawdbot.rs`

**Changes:**
- `find_clawdbot_dist()` now tries `openclaw` first, falls back to `clawdbot`
- New helper `find_dist_for_binary()` for cleaner binary lookup
- `detect_clawdbot_version()` tries `openclaw --version` first, then `clawdbot`
- `SUPPORTED_VERSIONS` updated: added `"2026.1.29"`
- NVM fallback path checks `openclaw` before `clawdbot`
- Error messages updated to say "OpenClaw" instead of "Clawdbot"

**Verified:**
- ✅ `bash-tools.exec.js` exists at same relative path in OpenClaw
- ✅ Anchor text `if (!params.command)` unchanged (line 538)
- ✅ `hook-runner-global.js` at same path: `dist/plugins/hook-runner-global.js`
- ✅ `cargo build --release` passes (0 errors, 3 pre-existing warnings)

**Backward Compatible:** Yes — still works with Clawdbot if installed.

---

## Step 2: Plugin manifest + package.json ✅

**Changes:**
- `package.json`: `"clawdbot"` key → `"openclaw"` key; updated keywords
- Folder renamed: `clawdbot-plugin/` → `openclaw-plugin/`
- Removed legacy `clawdbot.plugin.json` (duplicate of `openclaw.plugin.json`)
- `openclaw.plugin.json` kept as-is (already correct)

**Remaining refs to update in later steps:**
- `src/rules/mod.rs` line 1169-1170: `"clawdbot-plugin"`, `"clawdbot.plugin.json"`

---

## Step 3: Plugin index.js ✅

**File:** `openclaw-plugin/index.js`

**Changes:**
- Header comment: "for Clawdbot" → "for OpenClaw"
- `SELF_PROTECT_PATHS`: added `openclaw-plugin/` paths (kept `clawdbot-plugin/` for backward compat)
- `CLAWDBOT_JSON_PATTERN` → `CONFIG_JSON_PATTERN`: now matches both `openclaw.json` and `clawdbot.json`
- `FALLBACK_RULES`: `self_protect_patch_revert` pattern now matches `patch openclaw` and `patch clawdbot`
- `ALWAYS_ENFORCE_EXEC`: same update for patch revert + `self_protect_mv_rm` includes `openclaw-plugin`
- Error messages updated to reference both config files

**Backward Compatible:** Yes — all patterns match both openclaw and clawdbot variants.

---

## Step 4: Rust Source (remaining files) ✅

**Files modified:**
- `src/collectors/openclaw.rs` — sessions dir: `.openclaw` first, `.clawdbot` fallback
- `src/cli/start.rs` — pkill pattern matches both `openclaw.*gateway` and `clawdbot.*gateway`
- `src/cli/patch.rs` — accepts both `openclaw` and `clawdbot` as patch targets; updated all messages
- `src/rules/mod.rs` — `self_protect_plugin`: added `openclaw-plugin`, `openclaw.plugin.json`; `self_protect_patch`: added `patch openclaw --revert`
- `src/main.rs` — CLI help: `"openclaw" or "clawdbot"`

**Build:** ✅ `cargo build --release` — 0 errors, 3 pre-existing warnings (33.54s)

---

## Step 5: Config / Environment Variables ✅

No changes needed — already using `OPENCLAW_HARNESS_*` / `SAFEBOT_*` conventions.
Removed `ANTHROPIC_BASE_URL=http://127.0.0.1:9090` (proxy mode not used, hook-only setup).

---

## Step 6: README.md ✅

Updated ~67 lines via sub-agent. All "Clawdbot" → "OpenClaw" in text, commands, paths, compatibility table.

---

## Step 7: Build + Integration Test ✅

**Results:**
- `cargo build --release` ✅ (0 errors)
- `openclaw-harness patch openclaw` ✅ — Found dist, version 2026.1.29, patch applied
- `openclaw-harness patch openclaw --check` ✅ — "OpenClaw is patched"
- Plugin registered in `openclaw.json` ✅
- `openclaw status` shows: `[harness-guard] Registering before_tool_call hook` ✅
- `ANTHROPIC_BASE_URL` proxy removed (hook-only mode)

---

## Migration Complete 🎉

**Date:** 2026-01-31
**All 7 steps passed. OpenClaw Harness is now fully compatible with OpenClaw 2026.1.29+.**
