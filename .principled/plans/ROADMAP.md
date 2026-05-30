# ROADMAP — BDPM Rust Project

## Phase Structure

```
Phase 0:  Data Profiling       (DONE — findings baked into BRIEF.md)
Phase 1:  Foundation          [01-01 → 01-02 → 01-03 → 01-04 → 01-05 → 01-06 → 01-07]
Phase 2:  API                 [02-01 → 02-02 → 02-03 → 02-04]
Phase 3:  Sync Engine         [03-01 → 03-02 → 03-03 → 03-04]
Phase 3.5: Safety Data        [CIS_InfoImportantes — deferred]
Phase 4:  Polish              [04-01 → 04-02 → 04-03]
Phase 05: Normalization       [05-01 → 05-02 → 05-03] (DONE)
Phase 06: Data Quality        [06-01]                        (DONE)
Phase 07: Pipeline Quality   [07-01 → 07-02 → 07-03 → 07-04] (DONE)
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

## Phase 1: Foundation ✅ DONE (2026-05-26)

| Plan | Goal | Key Deliverable |
|------|------|-----------------|
| 01-01 | Project scaffold + BDPMFile manifest + fetcher + state store | `bdpm-ingest check` reports unchanged files |
| 01-02 | Tab parser with per-file encoding, CRLF, field-count guard | `parse_file()` for all 10 files |
| 01-03 | Data profiling integration + FileSchema validation | Field-count regression tests fire correctly |
| 01-04 | Normalization pipeline — price (cents), whitespace strip, CIP strip | `normalize_price("1,466,29") == 146629` |
| 01-05 | Database init + migration + import orchestrator | All 10 files imported, orphan FK relaxed, CHECK constraints enforced |
| 01-06 | ID/code normalization — CIS TEXT, ATCD codes, generic type enum | `normalize_generic_type("4") == "sustained-release"` |
| 01-07 | Database init + staging + migration | `bdpm-ingest import` creates all tables, imports data |

**Post-Phase 1 findings (from research 2026-05-26):**
- BDPM sync: monthly cadence (~28th), twice-daily poll (06h/18h) sufficient for intra-month obligation
- No delta possible — full-table reload always, confirmed by reference implementation (medicaments-api.giygas.dev)
- Zero-byte anomaly on CIS_CIP_bdpm + Ruptures_stocks — null checks needed
- Encoding Phase 1.5: `std::fs::read()` + encoding_rs, keep sync, ~20 lines
- CI: rust-cache@v2, MSRV 1.80, clippy -D warnings, cargo-audit for security
- GitHub release: .db as release asset, NOT git-committed; cross-platform builds via cross

## Phase 2: API ✅ DONE

| Plan | Goal |
|------|------|
| 02-01 | FTS5 drug name search (axum API) |
| 02-02 | Drug detail + presentations + compositions (axum) |
| 02-03 | Generic groups + ATC browse + availability (axum) |
| 02-04 | OpenAPI spec + health endpoint |

**Phase 02 deliverables:**
- FTS5 full-text search with accent-insensitive matching
- Drug detail endpoint with presentations, compositions, generic groups, ATC codes
- Generic group browse by ID, ATC hierarchy traversal, availability query
- OpenAPI 3.0.3 spec (542 lines) served at `/openapi.json` and `/openapi.yaml`

## Phase 3: Sync Engine ✅ DONE (absorbed into Phase 01)

Sync engine functionality was delivered as part of Phase 01 Foundation — no separate Phase 03 build was needed:

| Plan | Goal | Status |
|------|------|--------|
| 03-01 | BLAKE3 change detection + file-level reload | ✅ Absorbed into 01-01 |
| 03-02 | Full-table truncate+reload (NOT row-level delta) | ✅ In 01-05/01-07 |
| 03-03 | Availability file weekly sync (independent cadence) | ✅ weekly-dispo.yml |
| 03-04 | Import log viewer + health dashboard | ✅ `logs` command in 01-07 |

**Why no separate Phase 03?** BDPM provides no timestamps or ETag headers. No row-level delta is possible — full-table reload per changed file is the correct pattern. The sync engine is the ingest pipeline itself with a state store, delivered in Phase 01.

**Naming clarity: "file-level refresh" not "incremental sync"**

## Phase 3.5: Safety Data (Deferred)

- CIS_InfoImportantes: 6-hour TTL cache, fallback to stale data with freshness indicator
- Safety alert API endpoint

## Phase 4: Polish + CI/CD ✅ DONE

| Plan | Goal | Status |
|------|------|--------|
| 04-01 | OpenAPI spec + operational runbook | ✅ DONE |
| 04-02 | GitHub Actions release workflows | ✅ DONE |
| 04-03 | Schema change response procedure + runbook | ✅ DONE |

**Phase 04 deliverables:**
- OpenAPI 3.0.3 spec covering all 9 endpoints, served at `/openapi.json` and `/openapi.yaml`
- `docs/runbook.md` — 204-line operational runbook with monitoring, manual ops, schema change procedure
- `.github/workflows/ci.yml` — fmt + test + clippy on every push/PR
- `.github/workflows/release.yml` — build + ingest + publish `.db` as release asset on every push to main
- `.github/workflows/monthly-db-release.yml` — scheduled monthly rebuild (1st of month, 02:00 UTC)
- `.github/workflows/weekly-dispo.yml` — scheduled weekly availability sync (Monday 03:00 UTC)

## Phase 05: Normalization Upgrade

| Plan | Goal | Status |
|------|------|--------|
| 05-01 | Tier 1: ATC hierarchy, NFD diacritics, 4-layer homeopathy, EAN-13 validation | ✅ DONE (2026-05-29) |
| 05-02 | Tier 2: Form/route canonicalization, lab families, CPD condition flags | ✅ DONE (2026-05-29) |
| 05-03 | Tier 3: Salt stripping, validation thresholds, FTS diacritics, parallel processing | ✅ DONE (2026-05-29) |

**Phase 05 deliverables:**
- ATC parent hierarchy (5/3/1 char) populated for all codes post-ingest
- NFD diacritic stripping (`strip_diacritics()`) + FTS tokenizer `unicode61 remove_diacritics 1`
- Four-layer homeopathy detection (lab names, keywords, procedure, dilution pattern)
- EAN-13 checksum validation (logged, not rejected) via gtin-validate crate
- Form canonicalization (FORM_CANONICAL phf_map, 20+ entries)
- Route canonicalization (ROUTE_CANONICAL phf_map, 16 entries)
- Lab family pattern matching (LAB_FAMILY_MAP phf_map, 20 entries + suffix stripping)
- CPD prescription condition flags (CpdFlags struct, 6 booleans, prescription_flags table)
- Salt prefix/suffix stripping for substance names (26 prefixes, 16 suffixes)
- Post-import validation thresholds (5 checks: ghost CIS, substance cardinality, princeps groups, name coverage, date coherence)
- Rayon parallelization for CIS_COMPO normalization (determinism tested)

## Phase 06: Data Quality Fixes

| Plan | Goal | Status |
|------|------|--------|
| 06-01 | Fix CPD FK, COMPO dedup key, dilution regex false positive, wire canonicalization, extend salt/route maps | DONE (2026-05-29) |

**Phase 06 deliverables:**
- CIS_CPD FK fix: `INSERT OR IGNORE` for prescription_flags
- COMPO dedup key: `(cis, substance_code, seq)` matches PK — ~4,763 recovered rows
- Dilution regex: `X` removed from alternation — 14 gene/cell therapy products restored
- Form/route/lab canonicalization wired into `normalize_cis_bdpm`
- Salt suffixes extended: sodique, calcique, trihydratée, mésylate, fumarate, etc.
- Route canonicalization extended: intraveineuse, sous-cutanée, ophtalmique, inhalée, etc.
- NFD diacritics stripping in `canonicalize_form` and `canonicalize_route`

## Phase 07: Pipeline Quality ✅ DONE (2026-05-29)

| Plan | Goal | Status |
|------|------|--------|
| 07-01 | PRAGMA optimize post-import + comprehensive orphan detection | ✅ DONE |
| 07-02 | Quarantine table + CIP7 CHECK constraint | ✅ DONE |
| 07-03 | FTS5 trigram tokenizer for autocomplete | ✅ DONE |
| 07-04 | Compile-time field count const assertions | ✅ DONE |

**Phase 07 research findings (2026-05-29):**
- ANALYZE via `PRAGMA optimize=0x10002` — force-scan-all-tables for fresh-import DB with no query history
- VACUUM: skip for full-table-reload pattern (pages already reused by inserts, no orphaned free pages)
- Trigram tokenizer: SQLite 3.48+ via rusqlite bundled feature, substring matching (3-char min), ~3-5x more tokens but negligible at 15K rows
- Trigram fallback: short queries (< 3 chars) need LIKE fallback
- Quarantine: single generic table with `target_table` + `error_type` + `raw_line`, NOT per-table
- FK bulk load: disable via `PRAGMA foreign_keys=OFF` + explicit NOT EXISTS orphan validation post-import
- CIP7 validation: schema CHECK constraint + application-level 7-digit check
- Orphan detection: NOT EXISTS outperforms LEFT JOIN WHERE IS NULL
- Typed structs: current runtime test pattern is fine for stable pipeline; const assertions give 90% of benefit at 5% of cost

## Tracking

| Phase | Status |
|-------|--------|
| 00-data-profiling | ✅ DONE |
| 01-foundation | ✅ DONE (2026-05-26) |
| 02-api | ✅ DONE (2026-05-26) |
| 03-sync | ✅ DONE (2026-05-26) |
| 03.5-safety | ✅ STUB (2026-05-26) — endpoint + TTL cache, scraping deferred |
| 04-cicd | ✅ DONE (2026-05-26) |
| 05-normalize | ✅ DONE (2026-05-29) |
| 06-data-quality | ✅ DONE (2026-05-29) |
| 07-pipeline-quality | ✅ DONE (2026-05-29) |
