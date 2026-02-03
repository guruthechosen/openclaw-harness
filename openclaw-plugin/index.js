/**
 * OpenClaw Harness Guard Plugin for OpenClaw
 *
 * Intercepts tool calls via the `before_tool_call` plugin hook:
 * - exec: checks commands against rules
 * - write/edit/Write/Edit: checks file paths against self-protection rules
 *
 * Self-protection paths are HARDCODED — not dependent on API.
 * Fallback rules activate when the daemon API is unreachable.
 *
 * Compatible with OpenClaw 2026.1.29+ and legacy Clawdbot.
 */

// ---------------------------------------------------------------------------
// Hardcoded self-protection paths (API-independent, cannot be overridden)
// ---------------------------------------------------------------------------

const SELF_PROTECT_PATHS = [
  "safebot/config/",
  "safebot/config\\",
  "openclaw-harness/config/",
  "openclaw-harness/config\\",
  "safebot/src/",
  "safebot/src\\",
  "openclaw-harness/src/",
  "openclaw-harness/src\\",
  "safebot/target/",
  "safebot/target\\",
  "openclaw-harness/target/",
  "openclaw-harness/target\\",
  "safebot/openclaw-plugin/",
  "safebot/openclaw-plugin\\",
  "safebot/clawdbot-plugin/",
  "safebot/clawdbot-plugin\\",
  "openclaw-harness/openclaw-plugin/",
  "openclaw-harness/openclaw-plugin\\",
  "openclaw-harness/clawdbot-plugin/",
  "openclaw-harness/clawdbot-plugin\\",
  // Specific config files (match anywhere in path)
  "rules.yaml",
  "safebot.yaml",
  "openclaw-harness.yaml",
];

// Also block edits to openclaw.json / clawdbot.json that reference harness-guard
const CONFIG_JSON_PATTERN = /openclaw\.json|clawdbot\.json/;
const HARNESS_GUARD_CONTENT_PATTERN = /harness-guard|safebot-guard/i;

// ---------------------------------------------------------------------------
// Fallback rules (used when daemon API is unreachable)
// ---------------------------------------------------------------------------

const FALLBACK_RULES = [
  // Critical exec rules
  { name: "dangerous_rm", pattern: "rm\\s+(-rf?|--force|--recursive)\\s+[~/]", action: "CriticalAlert", risk_level: "critical", description: "Dangerous recursive delete", enabled: true },
  { name: "ssh_key_access", pattern: "\\.ssh/(id_rsa|id_ed25519|id_ecdsa)", action: "CriticalAlert", risk_level: "critical", description: "SSH private key access", enabled: true },
  { name: "wallet_access", pattern: "(\\.wallet|seed\\s*phrase|mnemonic|private\\s*key)", action: "CriticalAlert", risk_level: "critical", description: "Wallet/seed phrase access", enabled: true },
  { name: "api_key_exposure", pattern: "(api[_-]?key|secret|token|password)\\s*[=:]\\s*['\"][a-zA-Z0-9]{20,}", action: "CriticalAlert", risk_level: "critical", description: "API key exposure", enabled: true },
  // Self-protection exec rules
  { name: "self_protect_process", pattern: "(kill|pkill|killall)\\s+.*(openclaw|moltbot|safebot|harness)", action: "CriticalAlert", risk_level: "critical", description: "Block killing harness process", enabled: true },
  { name: "self_protect_stop", pattern: "(openclaw-harness|moltbot-harness|safebot)\\s+stop", action: "CriticalAlert", risk_level: "critical", description: "Block stopping harness via CLI", enabled: true },
  { name: "self_protect_api", pattern: "(curl|http|fetch|wget)\\s+.*(localhost|127\\.0\\.0\\.1):8380.*(rules|disable|delete)", action: "CriticalAlert", risk_level: "critical", description: "Block disabling rules via API", enabled: true },
  { name: "self_protect_patch_revert", pattern: "patch (openclaw|clawdbot).*(--revert|-r)|bash-tools\\.exec\\.js\\.orig", action: "CriticalAlert", risk_level: "critical", description: "Block reverting OpenClaw patch", enabled: true },
];

// Self-protection exec patterns that are ALWAYS enforced (even when API works)
const ALWAYS_ENFORCE_EXEC = [
  { name: "self_protect_process", pattern: /(kill|pkill|killall)\s+.*(openclaw|moltbot|safebot|harness)/i },
  { name: "self_protect_stop", pattern: /(openclaw-harness|moltbot-harness|safebot)\s+stop/i },
  { name: "self_protect_api", pattern: /(curl|http|fetch|wget)\s+.*(localhost|127\.0\.0\.1):8380.*(rules|disable|delete)/i },
  { name: "self_protect_patch_revert", pattern: /patch (openclaw|clawdbot).*(--revert|-r)|bash-tools\.exec\.js\.orig/i },
  { name: "self_protect_config_exec", pattern: /(cat\s*>|tee|sed\s+-i|vi|vim|nano|echo\s+.*>)\s+.*(rules\.yaml|safebot\.yaml|openclaw-harness\.yaml)/i },
  { name: "self_protect_chmod", pattern: /chmod\s+.*\/(safebot|openclaw-harness|moltbot-harness)\//i },
  { name: "self_protect_chown", pattern: /chown\s+.*\/(safebot|openclaw-harness|moltbot-harness)\//i },
  { name: "self_protect_mv_rm", pattern: /(mv|rm|cp)\s+.*\/(safebot|openclaw-harness|moltbot-harness)\/(config|src|target|openclaw-plugin|clawdbot-plugin)/i },
];

// ---------------------------------------------------------------------------
// Rule cache
// ---------------------------------------------------------------------------

let cachedRules = null;
let cacheTimestamp = 0;
let daemonDown = false;

async function fetchRules(apiUrl, logger) {
  try {
    const res = await fetch(`${apiUrl}/api/rules`);
    if (!res.ok) {
      logger?.warn?.(`[harness-guard] Failed to fetch rules: HTTP ${res.status}`);
      return null;
    }
    const rules = await res.json();
    daemonDown = false;
    return rules.filter((r) => r.enabled);
  } catch (err) {
    logger?.warn?.(`[harness-guard] Daemon unreachable: ${err.message || err}`);
    daemonDown = true;
    return null;
  }
}

async function getRules(apiUrl, cacheTtl, logger) {
  const now = Date.now();
  if (cachedRules && now - cacheTimestamp < cacheTtl * 1000) {
    return cachedRules;
  }
  const rules = await fetchRules(apiUrl, logger);
  if (rules) {
    cachedRules = rules;
    cacheTimestamp = now;
    return cachedRules;
  }
  // Daemon unreachable: use fallback
  if (!cachedRules) {
    logger?.warn?.(`[harness-guard] ⚠️ DAEMON DOWN — using FALLBACK rules (${FALLBACK_RULES.length} rules)`);
    return FALLBACK_RULES;
  }
  // Have stale cache, use it
  logger?.warn?.(`[harness-guard] ⚠️ DAEMON DOWN — using stale cached rules`);
  return cachedRules;
}

// ---------------------------------------------------------------------------
// Telegram notification (optional)
// ---------------------------------------------------------------------------

async function sendTelegramAlert(token, chatId, text, logger) {
  if (!token || !chatId) return;
  try {
    await fetch(`https://api.telegram.org/bot${token}/sendMessage`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ chat_id: chatId, text, parse_mode: "HTML" }),
    });
  } catch (err) {
    logger?.warn?.(`[harness-guard] Telegram alert failed: ${err}`);
  }
}

// ---------------------------------------------------------------------------
// Command matching (for exec tool)
// ---------------------------------------------------------------------------

function matchCommand(command, rules) {
  const matches = [];
  for (const rule of rules) {
    try {
      const re = new RegExp(rule.pattern, "i");
      if (re.test(command)) {
        matches.push(rule);
      }
    } catch {
      // invalid regex — skip
    }
  }
  return matches;
}

function shouldBlock(action) {
  const normalized = (action || "").replace(/[^a-zA-Z]/g, "").toLowerCase();
  return normalized === "criticalalert" || normalized === "pauseandask" || normalized === "block";
}

// ---------------------------------------------------------------------------
// Self-protection: file path checking (for write/edit tools)
// ---------------------------------------------------------------------------

function isProtectedPath(filePath) {
  if (!filePath || typeof filePath !== "string") return false;
  const normalized = filePath.replace(/\\/g, "/");
  for (const protPath of SELF_PROTECT_PATHS) {
    const normProt = protPath.replace(/\\/g, "/");
    if (normalized.includes(normProt)) {
      return { blocked: true, reason: `Protected path: ${normProt}` };
    }
  }
  return false;
}

function isProtectedClawdbotJsonEdit(filePath, params) {
  if (!filePath || !CONFIG_JSON_PATTERN.test(filePath)) return false;
  // Check if the content being written references harness-guard
  const content = params?.content || params?.newText || params?.new_string || "";
  const oldText = params?.oldText || params?.old_string || "";
  if (HARNESS_GUARD_CONTENT_PATTERN.test(content) || HARNESS_GUARD_CONTENT_PATTERN.test(oldText)) {
    return { blocked: true, reason: "Modifying harness-guard config in openclaw.json/clawdbot.json" };
  }
  return false;
}

function checkAlwaysEnforceExec(command) {
  for (const rule of ALWAYS_ENFORCE_EXEC) {
    if (rule.pattern.test(command)) {
      return rule;
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// Extract file path from write/edit tool params
// ---------------------------------------------------------------------------

function extractFilePath(params) {
  if (!params) return null;
  return params.path || params.file_path || null;
}

function extractCommand(params) {
  if (!params) return null;
  if (typeof params.command === "string") return params.command;
  return null;
}

// ---------------------------------------------------------------------------
// Plugin entry
// ---------------------------------------------------------------------------

export default function register(api) {
  const cfg = api.config?.plugins?.entries?.["harness-guard"]?.config ?? {};
  const enabled = cfg.enabled !== false;
  const apiUrl = cfg.apiUrl || "http://localhost:8380";
  const blockDangerous = cfg.blockDangerous !== false;
  const alertOnly = cfg.alertOnly === true;
  const cacheTtl = cfg.cacheTtlSeconds ?? 30;
  const telegramToken = cfg.telegramBotToken || null;
  const telegramChatId = cfg.telegramChatId || null;

  if (!enabled) return;

  api.on(
    "before_tool_call",
    async (event, _ctx) => {
      const toolName = event?.toolName ?? event?.name;
      const params = event?.params ?? event?.input;
      // Minimal logging — only log when something is actually blocked

      // ===== WRITE/EDIT TOOL INTERCEPTION =====
      if (["write", "edit", "Write", "Edit"].includes(toolName)) {
        const filePath = extractFilePath(params);
        if (!filePath) return;

        // Check hardcoded self-protection paths
        const pathCheck = isProtectedPath(filePath);
        if (pathCheck) {
          api.logger?.warn?.(
            `[harness-guard] 🔒 BLOCKED ${toolName} on protected path: ${filePath} (${pathCheck.reason})`
          );

          // Telegram alert
          if (telegramToken && telegramChatId) {
            const alertText =
              `🔒 <b>SELF-PROTECTION BLOCK</b>\n` +
              `<b>Tool:</b> ${toolName}\n` +
              `<b>Path:</b> <code>${filePath.slice(0, 200)}</code>\n` +
              `<b>Reason:</b> ${pathCheck.reason}`;
            void sendTelegramAlert(telegramToken, telegramChatId, alertText, api.logger);
          }

          return {
            block: true,
            blockReason:
              `🔒 Blocked by OpenClaw Harness Guard (Self-Protection)\n` +
              `Tool: ${toolName}\n` +
              `Path: ${filePath}\n` +
              `Reason: ${pathCheck.reason}\n` +
              `This path is protected and cannot be modified.`,
          };
        }

        // Check clawdbot.json harness-guard edits
        const jsonCheck = isProtectedClawdbotJsonEdit(filePath, params);
        if (jsonCheck) {
          api.logger?.warn?.(
            `[harness-guard] 🔒 BLOCKED ${toolName} on clawdbot.json: ${jsonCheck.reason}`
          );

          if (telegramToken && telegramChatId) {
            const alertText =
              `🔒 <b>SELF-PROTECTION BLOCK</b>\n` +
              `<b>Tool:</b> ${toolName}\n` +
              `<b>Path:</b> <code>${filePath}</code>\n` +
              `<b>Reason:</b> ${jsonCheck.reason}`;
            void sendTelegramAlert(telegramToken, telegramChatId, alertText, api.logger);
          }

          return {
            block: true,
            blockReason:
              `🔒 Blocked by OpenClaw Harness Guard (Self-Protection)\n` +
              `Tool: ${toolName}\n` +
              `Reason: ${jsonCheck.reason}`,
          };
        }

        // write/edit that isn't a protected path — allow
        return;
      }

      // ===== EXEC TOOL INTERCEPTION =====
      if (toolName !== "exec") return;

      const command = extractCommand(params);
      if (!command) return;

      // Always-enforce self-protection (hardcoded, API-independent)
      const alwaysBlock = checkAlwaysEnforceExec(command);
      if (alwaysBlock) {
        api.logger?.warn?.(
          `[harness-guard] 🔒 ALWAYS-ENFORCE blocked: ${alwaysBlock.name} — ${command}`
        );

        if (telegramToken && telegramChatId) {
          const alertText =
            `🔒 <b>SELF-PROTECTION BLOCK</b>\n` +
            `<b>Rule:</b> ${alwaysBlock.name}\n` +
            `<b>Command:</b> <code>${command.slice(0, 200)}</code>`;
          void sendTelegramAlert(telegramToken, telegramChatId, alertText, api.logger);
        }

        return {
          block: true,
          blockReason:
            `🔒 Blocked by OpenClaw Harness Guard (Self-Protection)\n` +
            `Rule: ${alwaysBlock.name}\n` +
            `This action is permanently blocked.`,
        };
      }

      // Fetch rules from API (or use fallback)
      const rules = await getRules(apiUrl, cacheTtl, api.logger);
      if (!rules || rules.length === 0) return;

      const matched = matchCommand(command, rules);
      if (matched.length === 0) return;

      const blockingRule = matched.find((r) => shouldBlock(r.action));
      const alertRules = matched.filter((r) => !shouldBlock(r.action));

      for (const rule of matched) {
        api.logger?.warn?.(
          `[harness-guard] Rule "${rule.name}" matched command: ${command} (action: ${rule.action})`
        );
      }

      if (telegramToken && telegramChatId) {
        const ruleNames = matched.map((r) => r.name).join(", ");
        const alertText =
          `🚨 <b>OpenClaw Harness Guard</b>\n` +
          `<b>Command:</b> <code>${command.slice(0, 200)}</code>\n` +
          `<b>Rules:</b> ${ruleNames}\n` +
          `<b>Action:</b> ${blockingRule ? "BLOCKED" : "ALERT"}` +
          (daemonDown ? `\n⚠️ <i>Using fallback rules (daemon unreachable)</i>` : "");
        void sendTelegramAlert(telegramToken, telegramChatId, alertText, api.logger);
      }

      if (blockingRule && blockDangerous && !alertOnly) {
        const reason =
          `🛡️ Blocked by OpenClaw Harness Guard\n` +
          `Rule: ${blockingRule.name}\n` +
          `Description: ${blockingRule.description}\n` +
          `Risk Level: ${blockingRule.risk_level}` +
          (daemonDown ? `\n⚠️ (Fallback mode — daemon unreachable)` : "");

        return { block: true, blockReason: reason };
      }

      return undefined;
    },
    { priority: 100 }
  );
}
