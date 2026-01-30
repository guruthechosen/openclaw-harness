# @moltbot/harness-guard

Clawdbot plugin that intercepts `exec` tool calls and checks them against [MoltBot Harness](https://github.com/moltbot/harness) rules before execution.

## How It Works

1. Hooks into Clawdbot's `before_tool_call` lifecycle
2. Fetches rules from MoltBot Harness Web API (cached)
3. Matches the exec command against each rule's regex pattern
4. Based on rule action:
   - **CriticalAlert / PauseAndAsk** → blocks execution, returns error
   - **Alert / LogOnly** → logs warning, allows execution
5. Optionally sends Telegram notifications on matches

## Install

```bash
clawdbot plugins install /path/to/moltbot-harness/clawdbot-plugin
```

Or link for development:

```bash
clawdbot plugins install -l /path/to/moltbot-harness/clawdbot-plugin
```

## Configure

In your Clawdbot config (`~/.clawdbot/config.json`):

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
          cacheTtlSeconds: 30,
          // Optional Telegram notifications
          telegramBotToken: "123456:ABC...",
          telegramChatId: "-100123456789"
        }
      }
    }
  }
}
```

### Config Options

| Option | Default | Description |
|--------|---------|-------------|
| `enabled` | `true` | Enable/disable the guard |
| `apiUrl` | `http://localhost:8380` | MoltBot Harness API URL |
| `blockDangerous` | `true` | Block commands matching CriticalAlert/PauseAndAsk rules |
| `alertOnly` | `false` | Log only, never block (overrides blockDangerous) |
| `cacheTtlSeconds` | `30` | How long to cache rules before re-fetching |
| `telegramBotToken` | - | Telegram bot token for alerts |
| `telegramChatId` | - | Telegram chat ID for alerts |

## Requirements

- MoltBot Harness running with Web UI at the configured `apiUrl`
- Rules configured in MoltBot Harness dashboard

## How Rules Work

Rules are managed in MoltBot Harness Web Dashboard. Each rule has:
- **Pattern**: regex to match against exec commands
- **Action**: CriticalAlert, PauseAndAsk, Alert, or LogOnly
- **Risk Level**: severity classification

When you add/edit rules in the dashboard, the plugin picks them up automatically (within cache TTL).
