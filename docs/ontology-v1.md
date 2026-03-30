# Ontology v1 (OpenClaw User Brain)

## Goal
Convert OpenClaw action logs into a user-specific knowledge graph (brain) that supports deep behavior analysis.

## Node Types
- `User`
- `Session`
- `Project`
- `Command`
- `File`
- `Tool`
- `Incident`

## Edge Types
- `did` (User -> Action/Command)
- `used_tool` (Session -> Tool)
- `ran_command` (Session -> Command)
- `touched_file` (Session -> File)
- `worked_on` (Session -> Project)
- `triggered_incident` (Session -> Incident)
- `incident_on_command` (Incident -> Command)

## Mapping Rules (v1)
1. `actions.agent` + `actions.session_id` -> User/Session nodes
2. `actions.action_type` -> Tool node + `used_tool` edge
3. `actions.content` -> Command node (for `Exec`) + `ran_command`
4. `actions.target` path -> File/Project nodes
5. `analysis_results.risk_level in (Warning, Critical)` -> Incident node
6. Link incident to session and command when available

## Outputs
- `data/ontology/v1/nodes.jsonl`
- `data/ontology/v1/edges.jsonl`
- `data/ontology/v1/summary.json`

## Notes
- v1 is deterministic and reproducible from DB snapshots.
- Higher-level semantic entities (Decision, TaskPattern) can be layered in v2.
