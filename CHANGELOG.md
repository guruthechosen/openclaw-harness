# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] — 2026-02-02

### Added
- 25 pre-built rule templates for common security scenarios
- 3 rule types: Regex, Keyword, and Template
- 8 hardcoded self-protection rules (tamper-proof)
- OpenClaw native plugin (`openclaw-plugin/`) with `before_tool_call` hook
- API proxy mode for Anthropic/OpenAI/Gemini streams
- Web dashboard with live event stream and rule management (port 8380)
- Real-time alerts: Telegram, Slack, Discord
- Auto-patcher for OpenClaw's exec tool (`openclaw-harness patch openclaw`)
- SQLite audit trail for all inspected actions
- Docker support with `docker-compose.yml`
- React + Vite web UI (`ui/`)
- Default configuration file (`config/default.yaml`)
- GitHub Actions CI (Linux + macOS)

### Changed
- Rebranded from `safebot` / `moltbot-harness` to `openclaw-harness`
- License changed to BSL-1.1

### Fixed
- Remaining legacy name references in display strings and plugin code

## [0.1.0] — 2026-01-24

### Added
- Initial release as `safebot`
- Basic regex rule matching
- File system watcher for config changes
- Telegram alert integration
- CLI with `start`, `stop`, `test` commands
