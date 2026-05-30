# 04-03 SUMMARY — Operational Runbook

## What was done

### Operational runbook (`docs/runbook.md`)
204-line runbook covering common incident procedures:

**Monitoring section:**
- Import failure alerts: detecting `status = 'failed'` in import_log
- Row count deviation: <90% threshold query with LAG window function
- Schema drift detection: CI fails on field_count assertion

**Manual operations:**
- Force full re-import: `bdpm-ingest ingest`
- Single-file re-import: fetch + ingest for one file
- Health checks: `bdpm-ingest stats` and `bdpm-ingest logs --limit N`

**Schema change response (7-step procedure):**
1. Detect: CI fails on field_count assertion
2. Audit: Download new file, compare to previous
3. Update: Modify FileSchema in `src/download/manifest.rs`
4. Test: Update integration tests in `tests/`
5. Migrate: Add migration if table structure changes
6. Deploy: PR with updated schema + migration
7. Monitor: Watch import log for new bad_rows count

## Verification
- `docs/runbook.md` exists, covers all three operational scenarios
- Referenced in ROADMAP Phase 04 deliverables

## Files created
- `docs/runbook.md`
