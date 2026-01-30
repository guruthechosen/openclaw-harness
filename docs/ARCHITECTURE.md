# MoltBot Harness Architecture

## Overview

MoltBot Harness is a security daemon that monitors AI agents (OpenClaw, Claude Code, Cursor) for potentially dangerous actions.

## Design Principles

1. **Deterministic Rules First**: Rule engine is the primary decision maker, AI is supplementary
2. **Read-Only Observer**: MoltBot Harness never modifies the system, only monitors and alerts
3. **Local First**: All data stays on the user's machine by default
4. **Low Overhead**: Minimal CPU/memory footprint

## Components

### 1. Collectors (`src/collectors/`)

Platform-specific modules that watch for agent activity:

```
collectors/
├── mod.rs           # Collector trait definition
├── openclaw.rs       # OpenClaw/Clawdbot log parser
├── claude_code.rs   # Claude Code log parser
└── cursor.rs        # Cursor IDE integration
```

**Data Flow:**
- File system watchers monitor log directories
- New entries are parsed into `AgentAction` structs
- Actions are sent to the Analyzer via async channel

### 2. Analyzer (`src/analyzer/`)

Rule matching and risk assessment:

```
analyzer/
├── mod.rs           # Main Analyzer struct
├── rule_engine.rs   # Pattern matching logic
└── risk_scorer.rs   # Risk calculation
```

**Rule Matching:**
- Regex-based pattern matching
- Action type filtering
- Risk level aggregation

### 3. Enforcer (`src/enforcer/`)

Takes action based on analysis results:

```
enforcer/
├── mod.rs           # Main Enforcer struct
└── alerter.rs       # Multi-channel alert sender
```

**Actions:**
- `LogOnly`: Just record the action
- `Alert`: Send notification to configured channels
- `PauseAndAsk`: Request user approval (future)
- `Block`: Attempt to prevent the action (future)

### 4. Database (`src/db/`)

SQLite-based persistence:

```
db/
└── mod.rs           # Database operations
```

**Tables:**
- `actions`: All observed agent actions
- `analysis_results`: Rule match results

### 5. CLI (`src/cli/`)

Command-line interface:

```
cli/
├── mod.rs
├── start.rs         # Start daemon
├── stop.rs          # Stop daemon
├── status.rs        # Show status
├── logs.rs          # View logs
├── rules.rs         # Manage rules
├── test.rs          # Test rules
└── tui.rs           # TUI dashboard
```

## Data Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Collectors │ ──▶ │   Analyzer  │ ──▶ │  Enforcer   │ ──▶ │   Alerts    │
│ (log watch) │     │ (rule match)│     │ (action)    │     │ (Telegram)  │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
       │                   │                   │
       │                   │                   │
       └───────────────────┴───────────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  Database   │
                    │  (SQLite)   │
                    └─────────────┘
```

## Configuration

Configuration files are stored in `~/.openclaw-harness/`:

```
~/.openclaw-harness/
├── config.yaml      # Main configuration
├── rules.yaml       # Custom rules
├── openclaw-harness.db       # SQLite database
└── openclaw-harness.log      # Log file
```

## Security Considerations

1. **No Elevated Privileges**: MoltBot Harness runs as the user, not root
2. **Local Data**: All data stays local unless cloud sync is enabled
3. **Read-Only Access**: Only reads logs, doesn't modify agent behavior
4. **Secure Alerts**: Alert channels use authenticated APIs

## Future Enhancements

### Phase 2
- System tray application (Tauri)
- Web dashboard
- AI-assisted risk analysis

### Phase 3
- Cloud sync for teams
- API for integrations
- More agent support (Ralph, Copilot, etc.)

## Performance Targets

- CPU: < 1% during idle, < 5% during active monitoring
- Memory: < 50 MB resident
- Disk: < 100 MB for 30-day logs
- Latency: < 100ms from action to alert
