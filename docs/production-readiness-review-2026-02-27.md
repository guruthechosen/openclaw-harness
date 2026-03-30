# OpenClaw Harness Production Readiness Review (2026-02-27)

## Scope checked
- Core backend tests/build
- UI lint/build
- Adaptive campaign LLM planner
- Ontology Brain v1/v2 + query API
- OpenClaw recent version compatibility assumptions

## Current status summary

### 1) OpenClaw compatibility
- OpenClaw runtime checked at 2026.2.26.
- Legacy patch check command may fail on changed internal file paths in newer OpenClaw.
- Plugin/built-in hook flow is the recommended path.

### 2) Core quality gates
- `cargo check` passed (compile-level validation).
- `npm run lint` passed.
- `npm run build` passed (bundle-size warning only).

Note: full `cargo test`/`cargo build --release` intermittently blocked by local Xcode license requirement on this host (`xcodebuild -license` pending).

### 3) Ontology Brain feature maturity
- v1 structural ontology: implemented and persisted.
- v2 semantic layer: implemented (TaskPattern, Decision, Bottleneck, Skill).
- Query API: implemented (`POST /api/brain/query`) with semantic modes.

### 4) Improvements completed in this pass
- Skill inference changed from hardcoded `user:openclaw` to per-user/per-tool scoring and linking.

## Not production-complete yet (must-fix)
1. Access control
   - Brain build/query endpoints currently have no explicit authz beyond service-level exposure.
2. Query endpoint robustness
   - Uses file-based reads from `data/ontology/v2`; should support graceful fallback + clear structured errors for missing/stale artifacts.
3. End-to-end coverage
   - Need API-level integration tests for:
     - build v2 endpoint
     - query endpoint modes
4. Observability
   - Need metrics for ontology build duration, node/edge counts, and query latency/error rates.
5. Data lifecycle
   - Need retention/rotation policy for ontology outputs and audit logs.
6. Semantic quality
   - Decision extraction is keyword heuristic; should add confidence scoring and optional LLM semantic post-labeling.

## Existing unrelated unfinished areas in repo
- `web/routes.rs`: events list endpoints still TODO (DB-backed retrieval).
- multiple CLI modules have TODO markers (status/stop/logs/rules persistence/TUI).

## Recommended next implementation order
1. Add API integration tests for `/api/brain/ontology/v2/build` + `/api/brain/query`.
2. Add structured error codes and artifact freshness checks to query path.
3. Add metrics + logs for brain build/query.
4. Add auth guard pattern for brain endpoints (if exposed beyond local).
5. Add weekly trend report generation directly from v2 insights.
