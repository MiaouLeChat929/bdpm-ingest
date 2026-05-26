# ROADMAP — BDPM Rust Project

## Phase Structure

```
Phase 0:  Data Profiling       (DONE — findings baked into BRIEF.md)
Phase 1:  Foundation          [01-01 → 01-02 → 01-03 → 01-04 → 01-05 → 01-06 → 01-07]
Phase 2:  API                 [02-01 → 02-02 → 02-03 → 02-04]
Phase 3:  Sync Engine         [03-01 → 03-02 → 03-03 → 03-04]
Phase 3.5: Safety Data        [CIS_InfoImportantes — deferred]
Phase 4:  Polish              [04-01 → 04-02 → 04-03]
```

## Phase 0: Data Profiling ✅ DONE

Analysis of all 10 files completed. Findings baked into BRIEF.md:
- All 10 edge-case files analyzed, 15+ parsing gotchas documented
- avis field HTML characterization (4,031 rows, VARCHAR(2048) safe)
- Price thousands-separator pattern discovered (466 rows)
- CIS_GENER type field values 2/4 discovered
- Tab-split malformed rows in SMR/ASMR discovered
- Exact duplicate rows in CIS_COMPO discovered
- Seeds all Phase 1 integration tests

## Phase 1: Foundation

| Plan | Goal | Key Deliverable |
|------|------|-----------------|
| 01-01 | Project scaffold + BDPMFile manifest + fetcher + state store | `bdpm-ingest check` reports unchanged files |
| 01-02 | Tab parser with per-file encoding, CRLF, field-count guard | `parse_file()` for all 10 files |
| 01-03 | Data profiling integration + FileSchema validation | Field-count regression tests fire correctly |
| 01-04 | Normalization pipeline — price (cents), whitespace strip, CIP strip | `normalize_price("1,466,29") == 146629` |
| 01-05 | Database init + migration + import orchestrator | All 10 files imported, orphan FK relaxed, CHECK constraints enforced |

**Note on Phase 1 integration tests:** The test suite (row counts, field counts, referential integrity) is authored in 01-03. Phase 4's CI regression suite wires these tests into GitHub Actions — the tests themselves exist from day one of Phase 1.
| 01-06 | ID/code normalization — CIS TEXT, ATCD codes, generic type enum | `normalize_generic_type("4") == "sustained-release"` |
| 01-07 | Database init + staging + migration | `bdpm-ingest import` creates all tables, imports data |

**Critical ordering within Phase 1:**
- 01-01 → 01-02 → 01-03 (parser → validation)
- 01-03 → 01-04/01-05/01-06 (can run in parallel once validation is stable)
- 01-04+01-05+01-06 → 01-07 (all normalizers → DB init)

## Phase 2: API

| Plan | Goal |
|------|------|
| 02-01 | FTS5 drug name search (axum API) |
| 02-02 | Drug detail + presentations + compositions (axum) |
| 02-03 | Generic groups + ATC browse + availability (axum) |
| 02-04 | OpenAPI spec + health endpoint |

## Phase 3: Sync Engine

| Plan | Goal |
|------|------|
| 03-01 | BLAKE3 change detection + Content-Length optimization documented |
| 03-02 | Full-table truncate+reload (NOT row-level delta) |
| 03-03 | Availability file weekly sync (independent cadence) |
| 03-04 | Import log viewer + health dashboard |

**Naming clarity: "file-level refresh" not "incremental sync"**

## Phase 3.5: Safety Data (Deferred)

- CIS_InfoImportantes: 6-hour TTL cache, fallback to stale data with freshness indicator
- Safety alert API endpoint

## Phase 4: Polish + CI/CD

| Plan | Goal |
|------|------|
| 04-01 | CI regression suite (row counts, field counts, referential integrity, normalization) |
| 04-02 | GitHub Actions release workflow (build, test, publish `.db` as release asset) |
| 04-03 | Schema change response procedure + operational runbook |

**GitHub Actions workflows (produced in Phase 4):**
- `monthly-sync.yml` — scheduled cron + `workflow_dispatch`, fetches BDPM files, builds SQLite, publishes release
- `weekly-dispo.yml` — scheduled cron + `workflow_dispatch`, updates Dispo file only
- `ci.yml` — runs on push/PR: `cargo test`, integration tests, clippy

## Tracking

| Phase | Status |
|-------|--------|
| 00-data-profiling | ✅ DONE |
| 01-foundation | ⏳ pending |
| 02-api | ⏳ pending |
| 03-sync | ⏳ pending |
| 03.5-safety | ⏳ pending (deferred) |
| 04-cicd | ⏳ pending |
