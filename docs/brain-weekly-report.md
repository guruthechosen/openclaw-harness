# Brain Weekly Report (MVP)

## What was implemented

### API
- `GET /api/reports/weekly?week=YYYY-Www`
- `POST /api/reports/weekly/generate`

### Generated outputs
- `data/reports/weekly/<report_id>.md`
- `data/reports/weekly/<report_id>.json`
- `data/ontology/nodes.jsonl`
- `data/ontology/edges.jsonl`

## Report contents
- Headline
- Activity summary (total events, project activity, top tools)
- Risk summary (critical/warning/info)
- Repeated pattern candidates
- Suggested next actions

## Ontology (minimal)
Nodes:
- `workspace:<id>`
- `report:<week-id>`
- `project:<project-id>`

Edges:
- `workspace -> has_report -> report`
- `report -> contains_project_activity -> project`

## Notes
- Week range parsing uses ISO week (`YYYY-Www`) with KST week boundaries.
- `generate` endpoint persists report + ontology outputs to disk.
- Current ontology output is intentionally minimal for MVP and can be expanded to full event-level graphs later.
