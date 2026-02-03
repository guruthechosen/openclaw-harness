# 🛡️ Harness Guard Plugin

OpenClaw plugin for [OpenClaw Harness](https://github.com/sparkishy/openclaw-harness). Intercepts tool calls and blocks dangerous commands before execution.

## Install

```bash
# From the openclaw-harness project root:
openclaw plugins install -l ./openclaw-plugin
```

## Modes

- **Standalone** — Works with built-in rules only. No daemon needed.
- **Connected** — Full features when paired with the OpenClaw Harness daemon (port 8380).

The plugin auto-detects which mode to use.

## Configuration

Add to your `openclaw.json`:

```json
{
  "plugins": {
    "entries": {
      "harness-guard": {
        "enabled": true,
        "config": {
          "apiUrl": "http://127.0.0.1:8380",
          "blockDangerous": true,
          "telegramBotToken": "YOUR_TOKEN",
          "telegramChatId": "YOUR_CHAT_ID"
        }
      }
    }
  }
}
```

See the [main README](../README.md) for full documentation.
