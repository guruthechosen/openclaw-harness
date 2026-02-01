<div align="center">

# 🦞 OpenClaw Harness

**Security harness for AI coding agents — inspect, block, and audit every tool call before it executes.**

[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://rustup.rs/)
[![Node](https://img.shields.io/badge/node-20+-green.svg)](https://nodejs.org/)
[![License](https://img.shields.io/badge/license-BSL_1.1-blue.svg)](LICENSE)

[Quick Start](#-quick-start) · [Docker](#-docker) · [How It Works](#-how-it-works) · [Rule Types](#-3-rule-types) · [Self-Protection](#-self-protection) · [Templates](#available-templates-25) · [API Reference](#-api-reference)

<img src="demo/demo.gif" alt="OpenClaw Harness Demo" width="800" />

</div>

---

## What is OpenClaw Harness?

OpenClaw Harness is a security layer for AI coding agents. It intercepts dangerous tool calls — destructive shell commands, SSH key access, API key exposure — and **blocks them before they execute**.

It works in two complementary ways:

1. **Plugin Hook (recommended)** — Patches the agent's exec tool to call a `before_tool_call` hook. Commands are checked against rules and blocked **before execution**.
2. **API Proxy** — Sits between the agent and the AI provider, inspecting `tool_use` responses in real-time and stripping dangerous calls from the stream.

### Key Numbers

| Metric | Count |
|--------|-------|
| Total Rules | **35** (17 regex + 3 keyword + 7 template + 8 self-protection) |
| Rule Types | **3** (Regex, Keyword, Template) |
| Templates | **25** pre-built security scenarios |
| Self-Protection | **8** hardcoded tamper-proof rules |
| Alert Channels | Telegram, Slack, Discord |

Think of it as a firewall for AI agents.

## Architecture

```
┌─────────────────┐         ┌──────────────────────────────┐
│   AI Agent      │         │     OpenClaw Harness           │
│                 │         │                              │
│  OpenClaw       │─────────│  ┌─────────┐  ┌──────────┐  │
│  Claude Code    │         │  │ Plugin   │  │  Daemon  │  │
│  Any Agent      │         │  │ Hook     │  │  + API   │  │
│                 │         │  │ (block)  │  │  (8380)  │  │
└─────────────────┘         │  └────┬────┘  └────┬─────┘  │
                            │       │            │         │
                            │  ┌────▼────────────▼──────┐  │
                            │  │   Rule Engine (35)     │  │
                            │  │   3 Types + Templates  │  │
                            │  │   + Self-Protection    │  │
                            │  │   Block / Alert / Log  │  │
                            │  └────┬───────────┬──────┘  │
                            └───────┼───────────┼─────────┘
                                    │           │
                            ┌───────▼─────┐ ┌───▼──────────┐
                            │  Alerts     │ │ Web Dashboard │
                            │  Telegram   │ │ localhost:3000│
                            │  Slack      │ │ Events, Rules │
                            │  Discord    │ │ Stats         │
                            └─────────────┘ └──────────────┘
```

## ✨ Features

- **Pre-execution Blocking** — Blocks dangerous commands _before_ they run via `before_tool_call` hooks
- **Auto-Patcher** — One command to patch OpenClaw's exec tool: `openclaw-harness patch openclaw`
- **35 Built-in Rules** — Blocks `rm -rf /`, SSH key theft, API key exposure, crypto wallet access, and more
- **3 Rule Types** — Regex (power), Keyword (simple), Template (recommended) — choose what fits
- **25 Pre-built Templates** — Just pick a template, fill in params, done
- **🔒 Multi-Layer Self-Protection** — 6 defense layers: file permissions, plugin hardcoded rules, path protection, fallback rules, config integrity monitoring, and Rust hardcoded rules
- **Custom Rules** — Add rules via YAML, REST API, CLI, or Web UI
- **Two Operating Modes** — **Enforce** (block) or **Monitor** (log only)
- **API Proxy** — Optional transparent proxy for Anthropic/OpenAI/Gemini APIs
- **Real-time Alerts** — Telegram, Slack, and Discord notifications on critical events
- **Web Dashboard** — Live event stream, rule management, statistics
- **Audit Trail** — SQLite storage of every inspected action

---

## 🐳 Docker

The fastest way to try OpenClaw Harness:

```bash
# One command — builds and runs everything
docker compose up --build

# Or manually
docker build -t openclaw-harness .
docker run -p 8380:8380 openclaw-harness
```

Dashboard at [http://localhost:8380](http://localhost:8380). Edit `config/rules.yaml` to customize rules.

```bash
# Test a rule
docker exec <container> openclaw-harness test dangerous_rm "rm -rf /"
# ✅ MATCH — Risk Level: Critical
```

---

## 🚀 Quick Start

### Prerequisites

| Requirement | Version | Purpose |
|-------------|---------|---------|
| [Rust](https://rustup.rs/) | 1.75+ | Backend & rule engine |
| [Node.js](https://nodejs.org/) | 20+ | Web Dashboard (optional) |
| [OpenClaw](https://github.com/openclaw/openclaw) | 2026.1.29–2026.1.30 | AI agent to protect (see compatibility below) |

### ⚠️ OpenClaw Version Compatibility

OpenClaw Harness patches OpenClaw's `bash-tools.exec.js` to inject the `before_tool_call` hook. This patch depends on the internal code structure of OpenClaw, which may change between versions.

> **Note:** The project formerly known as "Clawdbot" was renamed to "OpenClaw" starting with version 2026.1.29. OpenClaw Harness supports both names — the patcher auto-detects which is installed.

| OpenClaw Harness Version | Compatible Versions | Patch Target | Status |
|-----------------|------------------------------|--------------|--------|
| 0.3.x | **OpenClaw 2026.1.29–2026.1.30** | `bash-tools.exec.js` + `pi-tools.js` (exec/write/edit) | ✅ Current |
| 0.2.x | **OpenClaw 2026.1.29+** | `bash-tools.exec.js` (exec tool only) | ⚠️ Legacy |
| 0.1.x | Clawdbot 2026.1.24-3 (legacy) | `bash-tools.exec.js` (exec tool) | ⚠️ Legacy |

**Supported OpenClaw versions (tested):**
- ✅ **2026.1.30** — Fully tested, anchor intact, hook-runner-global.js at same path
- ✅ **2026.1.29** — Fully tested, anchor intact, hook-runner-global.js at same path
- ⚠️ **2026.1.24-3** (Clawdbot) — Legacy support via backward-compatible detection

**How to check compatibility:**

```bash
# Check your OpenClaw version
openclaw --version

# Verify patch can be applied
openclaw-harness patch openclaw --check
```

**What happens when OpenClaw updates:**

The patch searches for a specific anchor in `bash-tools.exec.js`:
```javascript
if (!params.command) {
    throw new Error("Provide a command to start.");
}
```

If OpenClaw changes this code structure, the patch will **fail safely** with an error:
```
Cannot find injection anchor in bash-tools.exec.js.
OpenClaw version may be incompatible.
```

**After an OpenClaw update:**
1. Run `openclaw-harness patch openclaw --check` to verify patch status
2. If the patch was removed (OpenClaw updated its files), re-apply: `openclaw-harness patch openclaw`
3. If the patch fails, check for a OpenClaw Harness update that supports the new OpenClaw version
4. File an issue at the OpenClaw Harness repo if no compatible version is available

> **Note:** OpenClaw Harness fallback rules and file permission protections work **regardless of the patch status**. Starting with v0.3.x, the patch covers exec, write, AND edit tools. File permissions (`chmod 444`) serve as an additional defense layer on top of the write/edit hooks.

### 1. Build

```bash
git clone https://github.com/sparkishy/openclaw-harness.git
cd openclaw-harness
cargo build --release
```

The binary is at `./target/release/openclaw-harness`.

### 2. Patch OpenClaw

This injects `before_tool_call` hooks into OpenClaw's exec, write, and edit tools, enabling pre-execution blocking for commands and file operations.

```bash
# Apply the patch (creates .orig backups automatically)
# Patches two files:
#   - bash-tools.exec.js  (exec tool hook — v1)
#   - pi-tools.js          (write/edit tool hooks — v2)
./target/release/openclaw-harness patch openclaw

# Restart OpenClaw to load the patched code
openclaw gateway restart
```

> **Note:** `openclaw gateway restart` does a full process restart, which clears the ESM module cache. SIGUSR1-based config reloads (e.g., from `openclaw config.patch`) do NOT reload patched files — use `restart` for that.

### 3. Install the Plugin

The harness-guard plugin connects OpenClaw's hook system to OpenClaw Harness rules.

```bash
# Install from the included plugin directory
openclaw plugins install --path ./openclaw-plugin

# Or manually: copy to OpenClaw's plugin load path
# and add to openclaw.json:
#   "plugins.load.paths": ["./openclaw-plugin"]
#   "plugins.entries.harness-guard.enabled": true
```

### 4. Start the Daemon

```bash
# Start OpenClaw Harness daemon (provides rule API on port 8380)
./target/release/openclaw-harness start --foreground

# Or run in background
./target/release/openclaw-harness start
```

### 5. Verify

```bash
# Check patch status (should show v1 + v2)
openclaw-harness patch openclaw --check
# ✅ OpenClaw is fully patched (exec + write/edit hooks active)

# Check daemon status
openclaw-harness status
# Or via API:
curl http://127.0.0.1:8380/api/status
# {"running":true,"version":"0.1.0",...}

# Test a rule
openclaw-harness test dangerous_rm "rm -rf /"
# ✅ MATCH - Risk: Critical, Action: Block

openclaw-harness test dangerous_rm "ls -la"
# ❌ NO MATCH
```

That's it! Any dangerous command your AI agent tries to run will now be blocked before execution.

### Example: Blocked Command

When an AI agent tries to run `rm -rf ~/Documents`:

```
🛡️ Blocked by OpenClaw Harness Guard
Rule: dangerous_rm
Description: Dangerous recursive delete commands
Risk Level: Critical
```

The command never executes. The agent receives an error and can choose a safer approach.

---

## 🚀 Auto-Start on Boot

The daemon should run continuously so the plugin can check commands in real-time. You have two options:

**Option A: Manual start** — Just run the daemon yourself when you need it:

```bash
openclaw-harness start              # Background mode
openclaw-harness start --foreground  # Foreground (Ctrl+C to stop)
```

> When the daemon is not running, the plugin falls back to hardcoded self-protection rules only. Safe commands always pass through.

**Option B: System service** — Auto-start on boot (recommended for always-on protection):

### macOS (launchd)

**Step 1: Create a launcher script** (recommended — avoids permission issues with external drives):

```bash
mkdir -p ~/.local/bin
cat > ~/.local/bin/openclaw-harness-launcher.sh << 'EOF'
#!/bin/bash
cd /path/to/openclaw-harness
exec ./target/release/openclaw-harness start --foreground
EOF
chmod +x ~/.local/bin/openclaw-harness-launcher.sh
```

**Step 2: Create the launchd plist:**

```bash
cat > ~/Library/LaunchAgents/com.openclaw.harness.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.openclaw.harness</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>/Users/YOUR_USER/.local/bin/openclaw-harness-launcher.sh</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>OPENCLAW_HARNESS_TELEGRAM_BOT_TOKEN</key>
        <string>your_bot_token</string>
        <key>OPENCLAW_HARNESS_TELEGRAM_CHAT_ID</key>
        <string>your_chat_id</string>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>/tmp/openclaw-harness.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/openclaw-harness.err</string>
    <key>ThrottleInterval</key>
    <integer>5</integer>
</dict>
</plist>
EOF
```

**Step 3: Load the service:**

```bash
launchctl load ~/Library/LaunchAgents/com.openclaw.harness.plist
```

**Manage the daemon:**

```bash
# Check status
curl -s http://127.0.0.1:8380/api/status

# Stop the daemon (stays off until manually started or reboot)
launchctl unload ~/Library/LaunchAgents/com.openclaw.harness.plist

# Start the daemon
launchctl load ~/Library/LaunchAgents/com.openclaw.harness.plist

# View logs
tail -f /tmp/openclaw-harness.log
tail -f /tmp/openclaw-harness.err
```

**Optional: Shell aliases** (add to `~/.zshrc` or `~/.bashrc`):

```bash
alias harness-start="launchctl load ~/Library/LaunchAgents/com.openclaw.harness.plist"
alias harness-stop="launchctl unload ~/Library/LaunchAgents/com.openclaw.harness.plist"
alias harness-status="curl -s http://127.0.0.1:8380/api/status | python3 -m json.tool 2>/dev/null || echo '🔴 Stopped'"
alias harness-log="tail -f /tmp/openclaw-harness.log"
```

Then just use: `harness-start`, `harness-stop`, `harness-status`, `harness-log`

> **⚠️ Important:** If the daemon is registered as a launchd service, do **not** use `openclaw-harness stop` to shut it down — launchd will automatically restart it. Use `harness-stop` (or `launchctl unload ...`) instead, which stops the service and prevents auto-restart.

> **Note:** If the binary is on an external drive, use a launcher script on the local disk (`~/.local/bin/`) to avoid "Operation not permitted" errors from launchd.

### Linux (systemd)

```bash
sudo cat > /etc/systemd/system/openclaw-harness.service << 'EOF'
[Unit]
Description=OpenClaw Harness Security Daemon
After=network.target

[Service]
Type=simple
WorkingDirectory=/path/to/openclaw-harness
ExecStart=/path/to/openclaw-harness/target/release/openclaw-harness start --foreground
Restart=on-failure
RestartSec=5
Environment=OPENCLAW_HARNESS_TELEGRAM_BOT_TOKEN=your_token
Environment=OPENCLAW_HARNESS_TELEGRAM_CHAT_ID=your_chat_id

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now openclaw-harness
```

**Manage the daemon:**

```bash
# Check status
sudo systemctl status openclaw-harness

# Stop (stays off until manually started or reboot)
sudo systemctl stop openclaw-harness

# Start
sudo systemctl start openclaw-harness

# Disable auto-start on boot
sudo systemctl disable openclaw-harness
```

### Uninstall the service

If you prefer not to run the daemon as a system service:

**macOS:**
```bash
launchctl unload ~/Library/LaunchAgents/com.openclaw.harness.plist
rm ~/Library/LaunchAgents/com.openclaw.harness.plist
rm ~/.local/bin/openclaw-harness-launcher.sh  # if created
```

**Linux:**
```bash
sudo systemctl disable --now openclaw-harness
sudo rm /etc/systemd/system/openclaw-harness.service
sudo systemctl daemon-reload
```

You can still run the daemon manually anytime with `openclaw-harness start`.

---

## 🔧 How It Works

### Plugin Hook Flow (Recommended)

```
Agent wants to run "rm -rf /" 
    → OpenClaw exec tool (patched)
    → before_tool_call hook fires
    → harness-guard plugin checks rules via OpenClaw Harness API
    → Rule "dangerous_rm" matches
    → Plugin returns { block: true, blockReason: "..." }
    → OpenClaw throws error — command NEVER executes
    → Agent sees the error and adjusts
```

### API Proxy Flow (Alternative)

```
Agent sends API request
    → OpenClaw Harness Proxy (port 9090) forwards to provider
    → Provider returns response with tool_use
    → Proxy inspects tool calls against rules  
    → Dangerous calls stripped from response
    → Agent receives sanitized response
```

---

## 🛡️ 3 Rule Types

OpenClaw Harness supports **3 rule types** for maximum flexibility. Choose based on your needs:

| Type | Complexity | Best For | Regex Knowledge |
|------|-----------|----------|-----------------|
| **Regex** | Advanced | Precise pattern matching | Required |
| **Keyword** | Simple | Quick string-based rules | Not needed |
| **Template** | Easiest | Common security scenarios | Not needed |

---

### 1. Regex Rules (Traditional)

Standard regex pattern matching. Full power, full control.

**Pros:** Precise pattern matching, complex logic, negative lookaheads
**Cons:** Regex syntax knowledge required, harder to read

```yaml
- name: dangerous_rm
  description: "Dangerous recursive delete commands"
  match_type: regex
  pattern: 'rm\s+(-rf?|--force|--recursive)\s+[~/]'
  applies_to: [exec]
  risk_level: critical
  action: block
  enabled: true
```

More examples from the default ruleset:

```yaml
# SSH key access detection
- name: ssh_key_access
  match_type: regex
  pattern: '\.ssh/(id_rsa|id_ed25519|id_ecdsa)($|[^.])'
  risk_level: critical
  action: block

# API key exposure
- name: api_key_exposure
  match_type: regex
  pattern: '(api[_-]?key|secret|token|password)\s*[=:]\s*[''"][a-zA-Z0-9]{20,}'
  risk_level: critical
  action: block

# Git force push
- name: git_force_push
  match_type: regex
  pattern: 'git\s+push\s+.*(-f|--force)'
  risk_level: warning
  action: pause_and_ask
```

---

### 2. Keyword Rules (Simple) 🆕

No regex knowledge needed. Simple string matching with intuitive operators.

#### Match Operators

| Operator | Logic | Description | Example |
|----------|-------|-------------|---------|
| `contains` | **AND** | ALL strings must be present | `["curl", "--data"]` — matches only if both `curl` AND `--data` appear |
| `any_of` | **OR** | At least ONE string must be present | `["format", "mkfs"]` — matches if either appears |
| `starts_with` | — | Command starts with one of these | `["sudo ", "doas "]` |
| `ends_with` | — | Command ends with one of these | `[".conf", ".yaml"]` |
| `glob` | — | Glob/wildcard pattern matching | `["*.env", "/etc/*"]` |

You can combine multiple operators — all specified conditions must pass.

#### YAML Examples

```yaml
# AND logic: both "curl" AND "--data" must be present
- name: block_curl_upload
  description: "Block curl data upload"
  match_type: keyword
  keyword:
    contains: ["curl", "--data"]
  risk_level: warning
  action: block
  enabled: true

# OR logic: any destructive keyword triggers the rule
- name: block_destructive_keywords
  description: "Block commands with destructive keywords"
  match_type: keyword
  keyword:
    any_of: ["format", "mkfs", "wipefs", "fdisk"]
  risk_level: critical
  action: block
  enabled: true

# starts_with: block commands starting with sudo
- name: block_sudo_keyword
  description: "Block commands starting with sudo"
  match_type: keyword
  keyword:
    starts_with: ["sudo ", "doas "]
  risk_level: warning
  action: pause_and_ask
  enabled: true
```

#### CLI Examples

```bash
# Add a keyword rule with AND logic (contains)
openclaw-harness rules add \
  --name block_curl_upload \
  --keyword-contains "curl,--data" \
  --risk critical \
  --action block

# Add a keyword rule with OR logic (any_of)
openclaw-harness rules add \
  --name block_destructive \
  --keyword-any-of "format,mkfs,wipefs,fdisk" \
  --risk critical \
  --action block

# Add a keyword rule with starts_with
openclaw-harness rules add \
  --name block_sudo \
  --keyword-starts-with "sudo ,doas " \
  --risk warning \
  --action block
```

---

### 3. Template Rules (Recommended) 🆕

Pre-built security scenarios. Just pick a template and fill in parameters. **25 templates** covering all common threat vectors across 6 categories.

#### Quick Example

```yaml
- name: protect_my_docs
  match_type: template
  template: protect_path
  params:
    path: "/Users/archone/Documents"
    operations: [read, write, delete]
  risk_level: critical
  action: block
```

#### CLI Usage

```bash
# List all available templates
openclaw-harness rules templates

# Add a template rule with parameters
openclaw-harness rules add --template protect_path --path "/etc" --operations "read,write"
openclaw-harness rules add --template block_sudo --risk critical --action block
openclaw-harness rules add --template block_docker --name my_docker_rule
openclaw-harness rules add --template block_command --commands "telnet,ftp"
```

---

### Available Templates (25)

#### 📁 File/Folder Protection (4)

| Template | Description | Required Params | Optional Params |
|----------|-------------|-----------------|-----------------|
| `protect_path` | Block access to specific paths (read/write/delete) | `path` | `operations` |
| `prevent_delete` | Prevent file/folder deletion | `path` | — |
| `prevent_overwrite` | Prevent overwriting important files | `path` | — |
| `block_hidden_files` | Block .env, .ssh, .aws, .kube, .docker, etc. | — | — |

<details>
<summary>📁 YAML Examples</summary>

```yaml
# Protect a path from all operations
- name: protect_etc
  match_type: template
  template: protect_path
  params:
    path: "/etc"
    operations: [read, write, delete]
  risk_level: critical
  action: block

# Prevent deletion of specific files
- name: prevent_delete_docs
  match_type: template
  template: prevent_delete
  params:
    path: "/Users/archone/Documents"
  risk_level: critical
  action: block

# Block all hidden/secret file access
- name: block_dotfiles
  match_type: template
  template: block_hidden_files
  params: {}
  risk_level: critical
  action: block
```

</details>

#### 🚫 Command Restriction (6)

| Template | Description | Required Params | Optional Params |
|----------|-------------|-----------------|-----------------|
| `block_command` | Block specific commands | `commands` | — |
| `block_sudo` | Block sudo/su/doas/pkexec | — | — |
| `block_package_install` | Block apt, brew, pip, npm, cargo, gem, etc. | — | — |
| `block_service_control` | Block systemctl, launchctl, service, initctl | — | — |
| `block_network_tools` | Block curl, wget, nc, nmap, masscan | — | — |
| `block_compiler` | Block gcc, rustc, javac, make, cmake | — | — |

<details>
<summary>🚫 YAML Examples</summary>

```yaml
# Block specific commands
- name: block_telnet
  match_type: template
  template: block_command
  params:
    commands: [telnet, ftp, rsh]
  risk_level: warning
  action: block

# Block all sudo/privilege escalation
- name: no_sudo
  match_type: template
  template: block_sudo
  params: {}
  risk_level: critical
  action: block

# Block package managers
- name: no_installs
  match_type: template
  template: block_package_install
  params: {}
  risk_level: warning
  action: pause_and_ask
```

</details>

#### 🔐 Data Protection (4)

| Template | Description | Required Params | Optional Params |
|----------|-------------|-----------------|-----------------|
| `prevent_exfiltration` | Block POST, scp, rsync, sftp outbound | — | — |
| `protect_secrets` | Block API key/token/password exposure | — | — |
| `protect_database` | Block DROP, TRUNCATE, mass DELETE, FLUSHALL | — | — |
| `protect_git` | Block force push, branch delete, hard reset, clean -fd | — | — |

<details>
<summary>🔐 YAML Examples</summary>

```yaml
# Prevent data exfiltration
- name: no_exfil
  match_type: template
  template: prevent_exfiltration
  params: {}
  risk_level: critical
  action: block

# Protect git from destructive operations
- name: safe_git
  match_type: template
  template: protect_git
  params: {}
  risk_level: warning
  action: pause_and_ask
```

</details>

#### 🖥️ System Protection (5)

| Template | Description | Required Params | Optional Params |
|----------|-------------|-----------------|-----------------|
| `protect_system_config` | Block /etc/ edits, shell rc file changes | — | — |
| `block_disk_operations` | Block mkfs, fdisk, dd, wipefs, diskutil | — | — |
| `block_user_management` | Block useradd, userdel, passwd, dscl | — | — |
| `block_cron_modification` | Block crontab edits, /etc/cron, at, timers | — | — |
| `block_firewall_changes` | Block iptables, ufw, pf, nftables, firewall-cmd | — | — |

<details>
<summary>🖥️ YAML Examples</summary>

```yaml
# Protect system config
- name: no_system_edit
  match_type: template
  template: protect_system_config
  params: {}
  risk_level: critical
  action: block

# Block disk operations
- name: no_disk
  match_type: template
  template: block_disk_operations
  params: {}
  risk_level: critical
  action: block
```

</details>

#### ⚙️ App/Process Restriction (3)

| Template | Description | Required Params | Optional Params |
|----------|-------------|-----------------|-----------------|
| `block_app` | Block specific apps/processes | `commands` | — |
| `block_docker` | Block docker rm, kill, stop, prune, system prune | — | — |
| `block_kill_process` | Block kill -9, killall, pkill, xkill | — | — |

<details>
<summary>⚙️ YAML Examples</summary>

```yaml
# Block dangerous Docker commands
- name: safe_docker
  match_type: template
  template: block_docker
  params: {}
  risk_level: warning
  action: block

# Block process killing
- name: no_kill
  match_type: template
  template: block_kill_process
  params: {}
  risk_level: warning
  action: block
```

</details>

#### 🌐 Network (3)

| Template | Description | Required Params | Optional Params |
|----------|-------------|-----------------|-----------------|
| `block_port_open` | Block listeners (nc -l), tunnels (ngrok, ssh -R) | — | — |
| `block_ssh_connection` | Block SSH connections (ssh, sshpass, ssh-copy-id) | — | — |
| `block_dns_change` | Block resolv.conf edits, networksetup DNS changes | — | — |

<details>
<summary>🌐 YAML Examples</summary>

```yaml
# Block SSH connections
- name: no_ssh
  match_type: template
  template: block_ssh_connection
  params: {}
  risk_level: critical
  action: block

# Block DNS changes
- name: no_dns
  match_type: template
  template: block_dns_change
  params: {}
  risk_level: critical
  action: block
```

</details>

---

## 🔒 Self-Protection

> **This is OpenClaw Harness's most important security feature.**

AI agents are smart. A sufficiently capable agent might try to **disable the security harness** to bypass restrictions. OpenClaw Harness prevents this with **6 layers of defense** and **8 hardcoded self-protection rules** that cannot be removed, disabled, or overridden.

### Multi-Layer Defense

OpenClaw Harness uses defense-in-depth with 6 independent layers:

| Layer | Mechanism | What It Does |
|-------|-----------|-------------|
| **1** | File permissions (`chmod 444`) | Config files are read-only; blocks OpenClaw's Write/Edit tools |
| **2** | Plugin hardcoded rules | `ALWAYS_ENFORCE_EXEC` patterns block `chmod`/`chown`/`mv`/`rm`/`sed`/`kill`/`pkill` on harness paths |
| **3** | Plugin path protection + Write/Edit hook | Write/Edit tools intercepted via `before_tool_call` hook (v2 patch on `pi-tools.js`) and checked against `SELF_PROTECT_PATHS` (hardcoded, API-independent) |
| **4** | Plugin fallback rules | 8 critical rules work even when the daemon is down |
| **5** | Config integrity monitoring | SHA256 hash check every 30s, Telegram alert on tampering |
| **6** | Rust hardcoded rules | 8 rules with `protected` flag, cannot be disabled via CLI/API |

Each layer operates independently — compromising one layer does not compromise the others.

### Setup: File Permissions (Layer 1)

```bash
# Protect config files (read-only for all users)
chmod 444 config/rules.yaml config/safebot.yaml config/openclaw-harness.yaml

# To edit configs, temporarily unlock:
chmod 644 config/rules.yaml
# ... make changes ...
chmod 444 config/rules.yaml
```

### The 8 Self-Protection Rules

| # | Rule | What It Blocks |
|---|------|---------------|
| 1 | `self_protect_config` | Modifying OpenClaw Harness config files (`rules.yaml`, `safebot.yaml`, etc.) |
| 2 | `self_protect_source` | Modifying OpenClaw Harness source code (`.rs`, `.toml` files) |
| 3 | `self_protect_process` | Killing the OpenClaw Harness process (`kill`, `pkill`, `killall`) |
| 4 | `self_protect_stop` | Stopping OpenClaw Harness via CLI (`openclaw-harness stop`, `safebot stop`) |
| 5 | `self_protect_plugin` | Modifying the harness-guard OpenClaw plugin |
| 6 | `self_protect_binary` | Replacing the compiled binary (`target/release/`, `target/debug/`) |
| 7 | `self_protect_api` | Using the API to delete/disable rules (`curl ... :8380 ... rules ... delete`) |
| 8 | `self_protect_patch` | Reverting the OpenClaw security patch (`--revert`, `.orig` file access) |

### How It Works

```
AI Agent: "Let me just disable that pesky security rule..."
    → Tries: edit config/rules.yaml       → ❌ Blocked by self_protect_config
    → Tries: modify src/rules/mod.rs      → ❌ Blocked by self_protect_source
    → Tries: kill -9 $(pgrep safebot)     → ❌ Blocked by self_protect_process
    → Tries: openclaw-harness stop         → ❌ Blocked by self_protect_stop
    → Tries: edit openclaw-plugin/        → ❌ Blocked by self_protect_plugin
    → Tries: cargo build --release        → ❌ Blocked by self_protect_source
    → Tries: curl DELETE :8380/api/rules  → ❌ Blocked by self_protect_api
    → Tries: patch openclaw --revert      → ❌ Blocked by self_protect_patch
    → Gives up. 🛡️
```

### Protection Mechanisms

| Mechanism | Description |
|-----------|-------------|
| **Hardcoded in Rust** | Rules are defined in `self_protection_rules()` in source code, not in config files. Removing them from `rules.yaml` has no effect. |
| **`protected: true` flag** | CLI `rules disable` command refuses to disable protected rules. |
| **API rejection (HTTP 403)** | Web API rejects PUT/DELETE requests on self-protection rules. |
| **Config override prevention** | If someone adds a rule with the same name in `rules.yaml`, `load_rules_from_file()` strips it and uses the hardcoded version instead. |
| **Circular protection** | To modify the source code → you'd need to bypass `self_protect_source`. To compile → `self_protect_source` also blocks writes to `/src/`. |

### CLI Behavior

```bash
# Trying to disable a self-protection rule
$ openclaw-harness rules disable self_protect_config
🔒 DENIED: Rule 'self_protect_config' is a self-protection rule and cannot be disabled.
   Self-protection rules are hardcoded and prevent the AI agent from
   tampering with the security harness. Only a human can modify the source code.

# Self-protection rules always show as enabled
$ openclaw-harness rules enable self_protect_config
✅ Rule 'self_protect_config' is a self-protection rule and is always enabled.
```

> **Only a human with direct access to the source code, a Rust compiler, and ability to replace the running binary can modify self-protection rules.** And even that is blocked if OpenClaw Harness is running — `self_protect_source` and `self_protect_binary` prevent it.

### Attack Scenarios & Defenses

| Attack Vector | Defense Layer |
|---|---|
| AI uses Edit/Write tool on config | File permissions (444) |
| AI uses `exec` + `sed`/`vim` on config | Plugin `ALWAYS_ENFORCE_EXEC` |
| AI uses `chmod` to unlock files | Plugin `self_protect_chmod` pattern |
| AI kills harness process | Plugin `self_protect_process` + Rust rule |
| AI stops daemon via CLI | Plugin `self_protect_stop` + Rust rule |
| AI modifies via REST API | Rust `protected` flag + HTTP 403 |
| AI reverts OpenClaw patch | Plugin `self_protect_patch_revert` |
| External tampering of config | SHA256 integrity check + Telegram alert |
| Daemon crashes/stops | Plugin fallback rules (8 hardcoded) |

### Known Limitations

- If the AI agent runs as **root**, file permissions won't help (but v2 hook protection still works)
- A human with direct source access and a Rust compiler can modify the hardcoded rules
- SIGUSR1 config reloads do NOT pick up patched files — use `openclaw gateway restart`

---

## 📋 Rule Management

### 3 Ways to Add Rules

#### 1. YAML Config (`config/rules.yaml`)

Edit the file directly. Rules are loaded on daemon start or `rules reload`.

```yaml
# Full rules.yaml format
- name: my_rule              # Required: unique identifier
  description: "My rule"     # Optional: human-readable description
  match_type: regex          # regex | keyword | template (default: regex)
  pattern: 'dangerous.*'     # For regex type
  keyword:                   # For keyword type
    contains: ["a", "b"]
    any_of: ["c", "d"]
    starts_with: ["e"]
    ends_with: ["f"]
    glob: ["*.env"]
  template: protect_path     # For template type
  params:                    # For template type
    path: "/etc"
    operations: [read, write]
    commands: [rm, mv]
  applies_to: [exec]         # Optional: exec, file_read, file_write, file_delete, http_request, git_operation
  risk_level: critical        # info | warning | critical
  action: block               # log_only | alert | pause_and_ask | block | critical_alert
  enabled: true               # true | false
```

#### 2. CLI

```bash
# Template rule
openclaw-harness rules add --template protect_path --path "/etc" --operations "read,write"

# Keyword rule
openclaw-harness rules add --keyword-contains "curl,--data" --risk critical --action block

# Keyword with any_of (OR logic)
openclaw-harness rules add --keyword-any-of "format,mkfs,wipefs" --risk critical --action block
```

> **Note:** CLI-added rules are in-memory only. Add to `config/rules.yaml` to persist.

#### 3. REST API

```bash
# Add a keyword rule via API
curl -X POST http://127.0.0.1:8380/api/rules \
  -H "Content-Type: application/json" \
  -d '{
    "name": "block_curl_upload",
    "description": "Block outbound data exfiltration via curl",
    "match_type": "keyword",
    "keyword": {"contains": ["curl", "--data"]},
    "risk_level": "critical",
    "action": "block",
    "enabled": true
  }'

# Add a template rule via API
curl -X POST http://127.0.0.1:8380/api/rules \
  -H "Content-Type: application/json" \
  -d '{
    "name": "protect_etc",
    "match_type": "template",
    "template": "protect_path",
    "params": {"path": "/etc", "operations": ["read", "write"]},
    "risk_level": "critical",
    "action": "block"
  }'
```

### Manage Existing Rules

```bash
# List all rules (shows type, status, protection flag)
openclaw-harness rules list

# Show rule details
openclaw-harness rules show dangerous_rm

# Enable/disable
openclaw-harness rules enable dangerous_rm
openclaw-harness rules disable dangerous_rm
# ⚠️ Self-protection rules cannot be disabled

# Reload from config file
openclaw-harness rules reload
```

### Testing Rules

```bash
# Test a specific rule against sample input
openclaw-harness test dangerous_rm "rm -rf /"
# ✅ MATCH - Risk: Critical, Action: Block

openclaw-harness test ssh_key_access "cat ~/.ssh/id_rsa"
# ✅ MATCH - Risk: Critical

openclaw-harness test dangerous_rm "ls -la"
# ❌ NO MATCH
```

---

## 🩹 OpenClaw Integration

### Patching

OpenClaw Harness patches two OpenClaw files to wire up `before_tool_call` hooks:
- **`bash-tools.exec.js`** — intercepts `exec` tool calls (v1 patch)
- **`pi-tools.js`** — intercepts `write` and `edit` tool calls (v2 patch)

```bash
# Check if already patched
openclaw-harness patch openclaw --check

# Apply patch (backs up original as .orig)
openclaw-harness patch openclaw

# Revert patch (restores original)
openclaw-harness patch openclaw --revert
```

After patching, **restart the OpenClaw gateway**:

```bash
openclaw gateway restart
```

### Plugin Configuration

The harness-guard plugin is configured in `openclaw.json`:

```json
{
  "plugins": {
    "load": {
      "paths": ["/path/to/openclaw-harness/openclaw-plugin"]
    },
    "entries": {
      "harness-guard": {
        "enabled": true,
        "config": {
          "enabled": true,
          "apiUrl": "http://127.0.0.1:8380",
          "blockDangerous": true,
          "alertOnly": false,
          "cacheTtlSeconds": 30,
          "telegramBotToken": "YOUR_BOT_TOKEN",
          "telegramChatId": "YOUR_CHAT_ID"
        }
      }
    }
  }
}
```

| Option | Default | Description |
|--------|---------|-------------|
| `enabled` | `true` | Enable/disable the guard |
| `apiUrl` | `http://127.0.0.1:8380` | OpenClaw Harness daemon API URL. Use `127.0.0.1` instead of `localhost` to avoid DNS resolution delays. |
| `blockDangerous` | `true` | Actually block (false = log only) |
| `alertOnly` | `false` | Only send alerts, don't block |
| `cacheTtlSeconds` | `30` | How long to cache rules from the API |
| `telegramBotToken` | — | Telegram bot token for block notifications |
| `telegramChatId` | — | Telegram chat ID for block notifications |

### Plugin Development Note

The harness-guard plugin uses OpenClaw's **typed hook** system:

```javascript
// ✅ Correct — typed hook via api.on()
api.on("before_tool_call", async (event, ctx) => {
  // inspect event.toolName, event.params
  return { block: true, blockReason: "..." };
}, { priority: 100 });

// ❌ Wrong — api.registerHook() is for external webhook/event hooks
api.registerHook("before_tool_call", handler);
```

---

## 🔔 Alert Configuration

### Telegram

OpenClaw Harness supports **two environment variable naming conventions** (both work):

```bash
# Option 1: OPENCLAW_HARNESS_* prefix
export OPENCLAW_HARNESS_TELEGRAM_BOT_TOKEN="123456:ABC-DEF..."
export OPENCLAW_HARNESS_TELEGRAM_CHAT_ID="987654321"

# Option 2: SAFEBOT_* prefix
export SAFEBOT_TELEGRAM_BOT_TOKEN="123456:ABC-DEF..."
export SAFEBOT_TELEGRAM_CHAT_ID="987654321"
```

Or configure in `config/safebot.yaml`:

```yaml
alerts:
  telegram:
    enabled: true
    bot_token: "YOUR_BOT_TOKEN"
    chat_id: "YOUR_CHAT_ID"
  slack:
    enabled: false
    webhook_url: "https://hooks.slack.com/services/XXX/YYY/ZZZ"
  discord:
    enabled: false
    webhook_url: "https://discord.com/api/webhooks/XXX/YYY"
```

**Getting Telegram credentials:**

1. Message [@BotFather](https://t.me/BotFather) → `/newbot` → copy the token
2. Send any message to your bot, then get your chat ID:
   ```
   https://api.telegram.org/bot<TOKEN>/getUpdates
   ```

### Plugin-Level Alerts

The harness-guard plugin also sends Telegram alerts independently. Configure in `openclaw.json`:

```json
{
  "plugins.entries.harness-guard.config.telegramBotToken": "123456:ABC-DEF...",
  "plugins.entries.harness-guard.config.telegramChatId": "987654321"
}
```

---

## 🌐 Web Dashboard

### Starting

```bash
# Terminal 1: Backend daemon (provides API on port 8380)
openclaw-harness start --foreground

# Terminal 2: Frontend dev server
cd ui
npm install   # first time only
npm run dev
# Open http://localhost:3000
```

### Pages

| Page | Description |
|------|-------------|
| **Dashboard** | Real-time stats: events, blocked/passed counts, recent activity |
| **Rules** | View/edit/create rules, enable/disable, test patterns |
| **Events** | Full event history with filters (risk level, agent, date) |
| **Settings** | Alert config (Telegram/Slack/Discord), proxy settings |

Live updates via WebSocket (`ws://127.0.0.1:8380/ws/events`).

---

## 🔌 API Proxy (Optional)

For agents that don't support plugin hooks, OpenClaw Harness can act as a transparent API proxy.

```bash
# Start proxy (intercepts tool_use in API responses)
openclaw-harness proxy start --mode enforce

# Point your agent at the proxy
export ANTHROPIC_BASE_URL=http://127.0.0.1:9090
```

Supports JSON and SSE streaming responses. Works with Anthropic, OpenAI, and Gemini.

| Mode | Behavior |
|------|----------|
| `enforce` | Strips dangerous tool calls from responses |
| `monitor` | Logs everything, passes through unchanged |

---

## 📡 API Reference

Base URL: `http://127.0.0.1:8380`

### Status & Stats

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/status` | System status (running, version, uptime) |
| `GET` | `/api/stats` | Aggregate statistics |

### Rules

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/rules` | List all rules |
| `POST` | `/api/rules` | Create custom rule |
| `PUT` | `/api/rules/:name` | Update rule (⚠️ 403 for self-protection rules) |
| `DELETE` | `/api/rules/:name` | Delete custom rule (⚠️ 403 for self-protection rules) |
| `POST` | `/api/rules/test` | Test pattern against input |

### Events

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/events` | List events (`?limit=`, `?level=`) |
| `GET` | `/api/events/:id` | Event details |

### WebSocket

| Endpoint | Description |
|----------|-------------|
| `ws://127.0.0.1:8380/ws/events` | Real-time event stream |

---

## 🖥️ CLI Reference

```
openclaw-harness [OPTIONS] <COMMAND>

Commands:
  start    Start the daemon (web API + log collector)
  stop     Stop the running daemon
  status   Show daemon status
  rules    Manage rules (list/show/enable/disable/reload/templates/add)
  test     Test a rule against sample input
  patch    Patch external tools (e.g., OpenClaw) to wire up hooks
  proxy    API Proxy — intercept AI provider responses
  logs     View recent activity logs
  tui      Interactive TUI dashboard

Options:
  -v, --verbose  Enable verbose logging (use -vv for trace)
  -h, --help     Print help
  -V, --version  Print version
```

### Rules Subcommands

```bash
# List all rules (type, status, protection)
openclaw-harness rules list

# Show rule or template details
openclaw-harness rules show <name>

# Enable/disable rules
openclaw-harness rules enable <name>
openclaw-harness rules disable <name>

# Reload rules from config/rules.yaml
openclaw-harness rules reload

# List all 25 templates with descriptions
openclaw-harness rules templates

# Add template rule
openclaw-harness rules add --template <template> [--name <name>] [--path <path>] [--operations <ops>] [--commands <cmds>] [--risk <level>] [--action <action>]

# Add keyword rule
openclaw-harness rules add [--name <name>] --keyword-contains <csv> [--risk <level>] [--action <action>]
openclaw-harness rules add [--name <name>] --keyword-any-of <csv> [--risk <level>] [--action <action>]
openclaw-harness rules add [--name <name>] --keyword-starts-with <csv> [--risk <level>] [--action <action>]
```

### Patch Commands

```bash
openclaw-harness patch openclaw            # Apply patch
openclaw-harness patch openclaw --check    # Check status
openclaw-harness patch openclaw --revert   # Revert patch
```

---

## 📁 Project Structure

```
openclaw-harness/
├── src/
│   ├── main.rs              # CLI entry (clap)
│   ├── rules/
│   │   └── mod.rs           # Rule engine (3 types + templates + self-protection)
│   ├── proxy/               # API Proxy (Axum)
│   ├── patcher/             # Auto-patcher for OpenClaw
│   │   └── clawdbot.rs      # Injects before_tool_call hook
│   ├── cli/                 # CLI commands
│   │   └── rules.rs         # rules list/templates/add/enable/disable
│   ├── analyzer/            # Rule engine
│   ├── enforcer/            # Alert system
│   ├── collectors/          # Log collectors
│   ├── db/                  # SQLite storage
│   └── web/                 # Web API + WebSocket
├── openclaw-plugin/         # OpenClaw harness-guard plugin
│   ├── index.js             # Plugin entry (before_tool_call hook)
│   ├── openclaw.plugin.json # Plugin manifest
│   └── package.json
├── ui/                      # React + TypeScript dashboard
├── config/
│   ├── safebot.yaml         # Main config (alerts, settings)
│   └── rules.yaml           # Custom rules (35 rules included)
├── Cargo.toml
└── README.md
```

---

## 🔍 Troubleshooting

### Hook not blocking commands

1. **Check patch status:**
   ```bash
   openclaw-harness patch openclaw --check
   ```

2. **Check plugin is loaded:** Look for `[harness-guard] Registering before_tool_call hook via api.on()` in OpenClaw gateway logs.

3. **Check daemon is running:**
   ```bash
   curl http://127.0.0.1:8380/api/status
   ```

4. **Restart required after patching:** Use `openclaw gateway restart` for a full process restart that clears ESM module cache. Note: SIGUSR1 config reloads do NOT reload patched files.
   ```bash
   openclaw gateway restart
   ```

### `ssh_key_access` rule and regex lookahead

The `ssh_key_access` rule uses `($|[^.])` instead of lookahead (`(?!...)`) because the Rust `regex` crate does **not support lookahead/lookbehind**. If you write custom regex rules, avoid `(?=...)`, `(?!...)`, `(?<=...)`, `(?<!...)` — they will fail to compile.

### Write/Edit not being blocked

If write/edit operations bypass the guard but exec works:

1. **Check v2 patch is applied:** The v2 patch targets `pi-tools.js` (in addition to v1's `bash-tools.exec.js`):
   ```bash
   grep "OPENCLAW_HARNESS_PATCH_v2" "$(dirname $(which openclaw))/../lib/node_modules/openclaw/dist/agents/pi-tools.js"
   ```
   If not found, re-run the patcher.

2. **Full restart after patching:**
   ```bash
   openclaw gateway restart
   ```

### Daemon binary name

The project was renamed from `safebot` → `openclaw-harness`. The correct binary is:
```bash
./target/debug/openclaw-harness start --foreground
# NOT: ./target/debug/safebot start --foreground
```

### Daemon needs Telegram env vars

The daemon reads Telegram credentials from environment variables at startup:
```bash
SAFEBOT_TELEGRAM_BOT_TOKEN="your-token" \
SAFEBOT_TELEGRAM_CHAT_ID="your-chat-id" \
./target/debug/openclaw-harness start --foreground
```

Or use `nohup` for background:
```bash
SAFEBOT_TELEGRAM_BOT_TOKEN="..." SAFEBOT_TELEGRAM_CHAT_ID="..." \
nohup ./target/debug/openclaw-harness start --foreground > /tmp/openclaw-harness.log 2>&1 &
```

### Config file load failure

If `config/rules.yaml` fails to parse (syntax error, invalid YAML), OpenClaw Harness falls back to the **default rules** (9 built-in regex rules + 8 self-protection rules). Check logs for the parse error and fix the YAML.

```bash
# Verify YAML syntax
python3 -c "import yaml; yaml.safe_load(open('config/rules.yaml'))"
```

### Plugin API: `api.on()` vs `api.registerHook()`

If you're writing a custom plugin for OpenClaw's `before_tool_call`:

- Use **`api.on("before_tool_call", handler)`** for typed hooks (tool interception)
- **`api.registerHook()`** is for external webhook/event registrations — it will NOT intercept tool calls

### UTF-8 truncate panic (multibyte characters)

Fixed: `truncate()` previously panicked on multibyte characters (Korean, Chinese, emoji) when the cut point fell inside a character boundary. All 3 truncate functions now use char boundary checking to prevent this.

### Daemon won't start on port 8380

Check if another process is using the port:
```bash
lsof -i :8380
```

### Rules not applying

The harness-guard plugin caches rules for 30 seconds. After adding rules via the API, wait up to 30s or restart the gateway.

---

## 🗺️ Roadmap

### ✅ Completed

- [x] Pre-execution blocking via `before_tool_call` hook
- [x] Auto-patcher for OpenClaw (`openclaw-harness patch openclaw`)
- [x] OpenClaw harness-guard plugin with rule caching
- [x] API Proxy with tool_use inspection (JSON + SSE)
- [x] **3 Rule Types** — Regex, Keyword, Template
- [x] **25 Pre-built Templates** across 6 categories
- [x] **8 Self-Protection Rules** (hardcoded, tamper-proof)
- [x] 35 built-in security rules (4 severity tiers)
- [x] Enforce mode (block) and Monitor mode (log only)
- [x] Telegram/Slack/Discord alerts (dual env var naming: `OPENCLAW_HARNESS_*` / `SAFEBOT_*`)
- [x] Web Dashboard with live event streaming
- [x] SQLite event storage
- [x] Custom rule support (YAML + REST API + CLI)
- [x] CLI with rule management, templates, and testing
- [x] Write/Edit tool interception via `before_tool_call` hook (v2 patch on `pi-tools.js`)

### 🔲 Planned

- [ ] Claude Code native integration
- [ ] Cursor native integration
- [ ] Custom rule builder in Web UI
- [ ] Multi-agent support
- [ ] AI-assisted risk analysis (local LLM)
- [ ] Metrics & Grafana integration
- [ ] Rule import/export (share rulesets)

---

## 📄 License

**Business Source License 1.1 (BSL 1.1)**

- ✅ Free for personal use, academic research, non-commercial projects
- 💼 Commercial use requires a separate license
- 📅 After 4 years, each release converts to Apache 2.0

See [LICENSE](LICENSE) for full text.

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Write tests: `cargo test`
4. Commit: `git commit -m 'Add my feature'`
5. Open a Pull Request

## 🙏 Acknowledgments

Built with [Rust](https://www.rust-lang.org/), [Axum](https://github.com/tokio-rs/axum), and [React](https://react.dev/). Inspired by the need for AI agent safety and guardrails.
