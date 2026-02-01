# @openclaw/harness-guard

OpenClaw plugin that intercepts tool calls and blocks dangerous commands using [OpenClaw Harness](https://github.com/sparkishy/openclaw-harness) rules.

## How It Works

```
Agent calls exec("rm -rf /")
       │
       ▼
  before_tool_call hook
       │
       ▼
  Fetch rules from Harness API (cached)
       │
       ▼
  Match against 35+ rules
       │
       ├─ MATCH (critical/block) → ❌ Block execution, return error
       └─ NO MATCH → ✅ Allow execution
```

## Install

```bash
# Install from local path
openclaw plugins install /path/to/openclaw-harness/openclaw-plugin

# Or symlink for development
openclaw plugins install -l /path/to/openclaw-harness/openclaw-plugin
```

## Prerequisites

OpenClaw Harness daemon must be running:

```bash
openclaw-harness start --foreground
# Dashboard available at http://localhost:8380
```

## Configuration

Add to your OpenClaw config (`~/.openclaw/config.json`):

```json5
{
  plugins: {
    entries: {
      "harness-guard": {
        enabled: true,
        config: {
          enabled: true,
          apiUrl: "http://localhost:8380",
          blockDangerous: true,
          alertOnly: false,
          cacheTtlSeconds: 30
        }
      }
    }
  }
}
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `enabled` | `true` | Enable/disable the guard |
| `apiUrl` | `http://localhost:8380` | Harness API URL |
| `blockDangerous` | `true` | Block commands matching critical rules |
| `alertOnly` | `false` | Log only, never block |
| `cacheTtlSeconds` | `30` | Rule cache TTL |
| `telegramBotToken` | — | Optional: Telegram bot token for direct alerts |
| `telegramChatId` | — | Optional: Telegram chat ID |

## Verify It Works

```bash
# With the harness running, try a blocked command in OpenClaw:
# > exec("rm -rf /")
# ❌ Blocked by OpenClaw Harness: dangerous_rm (Critical)
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Plugin not loading | Check `openclaw plugins list` — is `harness-guard` listed? |
| Commands not blocked | Is the harness daemon running? (`curl http://localhost:8380/api/health`) |
| Stale rules | Reduce `cacheTtlSeconds` or restart the plugin |
