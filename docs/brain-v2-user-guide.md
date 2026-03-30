# User Brain v2 - Detailed Usage Guide

This guide explains how to use the OpenClaw User Brain system (v2), what it builds, and what users can get from it.

## 1) What Brain v2 does

Brain v2 converts OpenClaw action history into a structured ontology and semantic insights.

### Input data
- `actions` table (OpenClaw tool actions)
- `analysis_results` table (risk matches / incidents)

### Outputs
- `data/ontology/v2/nodes.jsonl`
- `data/ontology/v2/edges.jsonl`
- `data/ontology/v2/insights.json`
- `data/ontology/v2/summary.json`

## 2) API endpoints

### Build ontology v1 (structural)
`POST /api/brain/ontology/build`

### Build ontology v2 (semantic)
`POST /api/brain/ontology/v2/build`

### Query brain insights
`POST /api/brain/query`

Request body examples:
```json
{ "query_type": "top_bottlenecks", "limit": 5 }
```
```json
{ "query_type": "top_patterns", "limit": 10 }
```
```json
{ "query_type": "skills", "limit": 10 }
```
```json
{ "query_type": "decisions", "limit": 10 }
```
```json
{ "query_type": "automation_opportunities", "limit": 10 }
```
```json
{ "query_type": "recommendations", "limit": 5 }
```

Example response (v2):
```json
{
  "ok": true,
  "summary": { "nodes": 421, "edges": 862 },
  "insights": {
    "repeated_patterns": 14,
    "decisions_detected": 27,
    "bottlenecks_detected": 6,
    "skills_inferred": 5,
    "automation_opportunities": 3
  }
}
```

## 3) Ontology model

### Core entities
- User
- Session
- Tool
- Command
- File
- Project
- Incident

### v2 semantic entities
- TaskPattern (repeated command habits)
- Decision (intent-heavy action patterns)
- Bottleneck (incident-heavy command clusters)
- Skill (tool proficiency estimation)
- AutomationOpportunity (반복 + 리스크 기반 자동화 후보)

### Core relations
- `did`
- `used_tool`
- `ran_command`
- `touched_file`
- `worked_on`
- `triggered_incident`
- `incident_on_command`

### v2 semantic relations
- `pattern_of`
- `derived_from`
- `caused_by`
- `has_skill`
- `suggests_automation_for`

## 4) What users can get (practical value)

### A. Personal operating map
Users can see:
- Which projects they actually spend time on
- Which tools they rely on most
- Which commands dominate their workflow

### B. Repetition intelligence (automation candidates)
`TaskPattern` identifies repetitive command loops.
This supports:
- shell script suggestions
- CI helper generation
- macro/automation proposals

### C. Decision trail
`Decision` entities capture intent-like actions (fix/refactor/implement/deploy patterns).
This helps:
- reconstructing why changes happened
- improving team handoff context
- building postmortem timelines

### D. Bottleneck/risk map
`Bottleneck` entities aggregate commands frequently tied to incidents.
This helps:
- reduce repeated mistakes
- tighten policies around fragile workflows
- prioritize hardening efforts

### E. Skill profile
`Skill` inference estimates strengths by tool usage adjusted by incident burden.
This helps:
- personalized coaching
- routing tasks to strengths
- detecting capability growth over time

## 5) Recommended operating flow

1. Run v2 build at least daily (or after major sessions).
2. Store generated files in versioned storage.
3. Generate weekly reports from `insights.json` + ontology graph deltas.
4. Track trend lines:
   - repeated_patterns (down is usually better after automation)
   - bottlenecks_detected (down = risk improvement)
   - skills_inferred (quality-adjusted growth)

## 6) Integrating into reports

In weekly report sections, include:
- Top 5 task patterns
- Top 3 bottlenecks + mitigation suggestions
- New decisions created this week
- Skill trend comparison (week-over-week)

## 7) Guardrails and caveats

- Brain v2 is deterministic pattern extraction (not perfect semantics).
- Decision detection is heuristic keyword-based in v2; tune keywords over time.
- Skill score is proxy-based and should be treated as directional.
- For high-stakes use, add human review and stronger semantic models in v3.

## 8) Next-step ideas (v3)

- LLM-backed semantic labeling over v2 graph
- confidence scores for each semantic node
- timeline-aware causality chains
- user-level recommendation API (automation/risk/learning plans)
