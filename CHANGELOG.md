# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- Brain weekly report APIs:
  - `GET /api/reports/weekly?week=YYYY-Www`
  - `POST /api/reports/weekly/generate`
- Weekly report persistence:
  - `data/reports/weekly/<week>.md`
  - `data/reports/weekly/<week>.json`
- Minimal ontology materialization outputs:
  - `data/ontology/nodes.jsonl`
  - `data/ontology/edges.jsonl`

- Adaptive campaign production planner (`LlmAiPlanner`) with:
  - strict mission JSON validation
  - retry/repair loop
  - SQLite audit logging (`mission_generation_audit`)
  - configurable provider env (`SAFEBOT_LLM_API_KEY`, `SAFEBOT_LLM_BASE_URL`, `SAFEBOT_LLM_MODEL`)
- Ontology v1 user-brain pipeline:
  - deterministic graph builder from OpenClaw action logs
  - persistence outputs: `data/ontology/v1/nodes.jsonl`, `edges.jsonl`, `summary.json`
  - API endpoint: `POST /api/brain/ontology/build`
- Ontology v2 semantic layer:
  - semantic entities: `TaskPattern`, `Decision`, `Bottleneck`, `Skill`
  - artifacts: `data/ontology/v2/nodes.jsonl`, `edges.jsonl`, `insights.json`, `summary.json`
  - API endpoint: `POST /api/brain/ontology/v2/build`
- Detailed user guide for Brain v2:
  - `docs/brain-v2-user-guide.md`
- Brain query API:
  - endpoint: `POST /api/brain/query`
  - semantic query modes: `top_bottlenecks`, `top_patterns`, `skills`, `decisions`

### Fixed
- UI quality gate issues in dashboard/rules/settings/websocket hooks (lint/build clean)

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
