# 04-01 — OpenAPI Spec + Operational Runbook

## Summary

OpenAPI 3.0.3 spec generated from axum handlers via utoipa, served as JSON and YAML endpoints. Operational runbook written for common incident procedures.

### Files Created
- `openapi.yaml` — 402-line OpenAPI 3.0.3 spec covering all 9 endpoints
- `docs/runbook.md` — 204-line operational runbook

### Files Modified
- `src/api/mod.rs` — Added `/openapi.json` and `/openapi.yaml` routes, `HealthResponse` with `utoipa::ToSchema`
- `src/api/search.rs` — `DrugSearchResult` + `SearchParams` with utoipa derives
- `src/api/drugs.rs` — `DrugDetail`, `Presentation`, `Composition` with utoipa derives
- `src/api/groups.rs` — `GenericGroupList`, `GenericGroupMember` with utoipa derives
- `src/api/atc.rs` — `AtcCode`, `AtcDetail` with utoipa derives
- `src/api/availability.rs` — `AvailabilityRow`, `AvailParams` with utoipa derives
- `src/main.rs` — Added `DumpOpenApi` subcommand
- `Cargo.toml` — Added `serde_yaml` dependency

### Endpoints Covered
`GET /health`, `GET /drugs`, `GET /drugs/{cis}`, `GET /generic-groups`, `GET /generic-groups/{id}`, `GET /atc`, `GET /atc/{code}`, `GET /availability`, `GET /openapi.json`, `GET /openapi.yaml`

### Runbook Contents (docs/runbook.md)
- Monitoring: import failure alerts, row count deviation (<90% threshold), schema drift detection
- Manual operations: force reimport, single-file sync, stats/logs health checks
- Schema change response: 7-step procedure (detect → audit → update → test → migrate → deploy → monitor)

### Verification
```
cargo build --release   ✓
./bdpm-ingest dump-open-api | head -5  ✓ Valid YAML output
openapi.yaml            ✓ 402 lines, openapi: 3.0.3
```