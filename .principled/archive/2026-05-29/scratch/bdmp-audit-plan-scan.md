# BDMP_DB Project Audit — Plan Scan

**Generated:** 2026-05-29
**Working Directory:** `/home/devadmin/Desktop/BDMP_DB`

---

## 1. `.principled/plans/` Structure

### Root Plan Files
| File | Purpose |
|------|---------|
| `BRIEF.md` | Vision, data profiling, architectural decisions, SQLite schema, edge cases |
| `ROADMAP.md` | Phase structure (00-04), tracking status, delivery notes |

### Phase Plans (`.principled/plans/phases/`)
```
phases/
└── 02-api/
    ├── 02-01-SUMMARY.md   (FTS5 + axum scaffold — DONE)
    ├── 02-02-SUMMARY.md  (Drug detail + presentations API — DONE)
    └── 02-03-SUMMARY.md  (Generic groups + ATC + availability — DONE)

    # Phase 03 (sync) and 04 (CI/CD) plans exist in archive only
    # Phase 01 (foundation) plans exist in archive only
```

### Archive Structure
```
.principled/archive/2026-05-26/plans/
├── 01-foundation/    # 01-01 through 01-07 PLAN + SUMMARY files
├── 02-api/            # 02-01 through 02-03 PLAN + SUMMARY files
├── 03-sync/           # 03-01, 03-02 PLAN files
└── 04-cicd/           # 04-01 PLAN + SUMMARY files
```

### Scratch Directory
**50 research/audit files** in `.principled/scratch/`:
- Architecture reviews, edge-case analysis, dependency audits
- External research (BDPM format, GitHub Actions, feasibility studies)
- Verification artifacts, critique documents

---

## 2. Phase Plan Content Summary

### 02-01: FTS5 + axum Scaffold
**Status: DONE**
- Created `src/db/fts.rs` — FTS5 virtual table + sync triggers
- Created `src/api/mod.rs` — AppState, run_server, routes wired
- Created `src/api/search.rs` — `GET /drugs` FTS5 search endpoint
- Verification: 15,848 FTS rows, all 24 tests pass, health + search endpoints work

### 02-02: Drug Detail API
**Status: DONE**
- Created `src/api/drugs.rs` — `GET /drugs/:cis` endpoint
- DrugDetail with presentations + compositions
- ApiError enum with NotFound/Internal variants
- spawn_blocking pattern for rusqlite calls

### 02-03: Generic Groups + ATC + Availability
**Status: DONE**
- `src/api/groups.rs` — `/generic-groups` and `/generic-groups/:group_id`
- `src/api/atc.rs` — `/atc` and `/atc/:code` with hierarchy
- `src/api/availability.rs` — `/availability` with cis/status filters

### ROADMAP Status
| Phase | Status | Date |
|-------|--------|------|
| 00-data-profiling | DONE | — |
| 01-foundation | DONE | 2026-05-26 |
| 02-api | DONE | 2026-05-26 |
| 03-sync | DONE | 2026-05-26 |
| 03.5-safety | STUB | 2026-05-26 |
| 04-cicd | DONE | 2026-05-26 |

---

## 3. Git Status

```
 M Cargo.lock
 M Cargo.toml
 M src/api/openapi.yaml
 M src/db/schema.sql
 M src/download/manifest.rs
 M src/import/mod.rs
 M src/normalize/html.rs
 M src/normalize/mod.rs
 M src/parse/tab.rs
?? data_old/
```

**Key Modified Files:**
- `Cargo.toml` / `Cargo.lock` — dependency changes pending
- `src/normalize/mod.rs` — large file (1,704 lines), major changes
- `src/import/mod.rs` — import orchestration changes
- `src/db/schema.sql` — schema changes
- `src/parse/tab.rs` — parsing changes
- `src/api/openapi.yaml` — API spec changes

---

## 4. Source Directory Structure

```
src/
├── api/              # HTTP API layer (axum)
│   ├── mod.rs
│   ├── atc.rs
│   ├── availability.rs
│   ├── drugs.rs
│   ├── groups.rs
│   ├── openapi.rs
│   ├── openapi.yaml
│   ├── safety.rs
│   └── search.rs
├── db/               # Database layer
│   ├── mod.rs
│   ├── fts.rs        # FTS5 virtual table
│   └── schema.sql    # 253 lines
├── download/         # HTTP fetcher
│   ├── mod.rs
│   ├── fetcher.rs
│   ├── listing.rs
│   └── manifest.rs
├── import/           # Import orchestration
│   └── mod.rs        # 824 lines
├── normalize/        # Data normalization
│   ├── mod.rs        # 1,704 lines — LARGEST FILE
│   ├── date.rs       # 92 lines
│   ├── dedup.rs      # 140 lines
│   ├── fields.rs     # 80 lines
│   ├── html.rs       # 194 lines
│   └── price.rs      # 222 lines
├── parse/            # TSV parsing
│   ├── mod.rs
│   └── tab.rs        # TabParser
├── lib.rs
└── main.rs
```

---

## 5. `src/normalize/fields.rs` — EXISTS

**File:** `/home/devadmin/Desktop/BDMP_DB/src/normalize/fields.rs` (80 lines)

**Functions:**
- `strip_field()` — trim whitespace
- `normalize_spaces()` — collapse double-spaces (CIS_GENER)
- `strip_cip_ean()` — strip 34009 prefix from EAN-13 to CIP-7
- `normalize_generic_type()` — "0"→"reference", "1"→"generic", "2"→"cross-group", "4"→"sustained-release"

**Tests:** 9 unit tests covering all functions

---

## 6. `src/normalize/dedup.rs` — EXISTS

**File:** `/home/devadmin/Desktop/BDMP_DB/src/normalize/dedup.rs` (140 lines)

**Key Function:**
- `dedup_compo(rows)` — removes exact duplicates from CIS_COMPO
- Dedup key: `(cis, substance_code, dosage)` — excludes `per_unit`
- 4,780 duplicates in 32,389 rows → 27,609 unique
- Malformed rows (len < 5) preserved for logging

**Tests:** 6 unit tests (all unique, all dupes, mixed, empty, short rows, null dosage)

---

## 7. Cargo.toml — Current Dependencies

### Core Dependencies
```toml
rusqlite = "0.31"              # SQLite (bundled)
ureq = "2"                      # HTTP fetcher (native-tls)
clap = "4"                      # CLI parsing
serde = "1"                     # Serialization
serde_json = "1"
encoding_rs = "0.8"             # Windows-1252 decoding
anyhow = "1"                    # Error handling
blake3 = "1"                    # Hashing
tracing = "0.1"                 # Logging
tracing-subscriber = "0.3"
regex-lite = "0.1"             # HTML stripping
```

### API Dependencies
```toml
axum = "0.8"                   # HTTP framework
tokio = "1"                     # Async runtime (rt-multi-thread, macros)
utoipa = "4"                    # OpenAPI generation (yaml feature)
htmlize = "1.1.0"              # HTML unescape
```

### Dev Dependencies
```toml
hyper = "1"
rand = "0.8"
reqwest = "0.12"               # (json, rustls-tls)
tokio-test = "0.4"
tempfile = "3"
```

### Lint Configuration
```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
indexing_slicing = "allow"     # TSV parsing — intentional
unwrap_used = "allow"          # TSV parsing — intentional
# ... 12 additional clippy rules
```

---

## 8. Key File Sizes

| File | Lines | Purpose |
|------|-------|---------|
| `src/normalize/mod.rs` | 1,704 | Normalization pipeline (LARGEST) |
| `src/import/mod.rs` | 824 | Import orchestration |
| `src/db/schema.sql` | 253 | SQLite schema |
| `src/api/openapi.yaml` | ~380 | OpenAPI specification |
| `src/normalize/html.rs` | 194 | HTML entity decoding |
| `src/normalize/price.rs` | 222 | Price normalization |

---

## 9. Observations

### Phase Completeness
- All phases through 04-cicd marked DONE in ROADMAP
- Only 02-api SUMMARY files exist in active `plans/` directory
- Archive contains full history of 01-04 plan files from 2026-05-26

### Large Files of Concern
- `src/normalize/mod.rs` at 1,704 lines is significantly larger than other modules
- May benefit from further extraction of concerns

### Pending Changes
- 9 files modified but not committed
- `Cargo.lock` modified — dependency updates in progress
- `src/normalize/mod.rs` and `src/import/mod.rs` are largest changed files

### Module Coverage
- `fields.rs` — field normalization utilities
- `dedup.rs` — CIS_COMPO deduplication
- `date.rs` — date parsing
- `price.rs` — price normalization
- `html.rs` — HTML entity decoding

---

## 10. Recommendations for Planning

1. **Review pending changes** before starting new phase work
2. **normalize/mod.rs** may be a candidate for further decomposition
3. **Archive old plans** before starting new phase work to maintain clarity
4. **Verify Phase 04-cicd** deliverables actually exist before marking complete
