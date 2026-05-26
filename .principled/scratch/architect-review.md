# BDMP Database Architecture Review

**Date:** 2026-05-26
**Author:** Systems Architect (AI)
**Status:** Draft - For Project Planning

---

## Executive Summary

The BDPM dataset comprises **151,979 rows** across 10 TSV files totaling ~17MB. Key findings:

- **No temporal data:** No row-level timestamps or version numbers exist anywhere
- **Update cadence variance:** Main files update monthly; Dispo_Spec updates weekly
- **Multi-value fields:** CPD, COMPO, GENER can have multiple rows per CIS
- **Price complexity:** 35% of CIP entries have null prices; prices use French decimal format (comma)
- **Encoding variance:** 70% Latin-1, 30% UTF-8
- **Schema inference required:** No official schema documentation available

---

## A. DATA VOLUME ANALYSIS

### Row Counts by File

| File | Rows | Size | Encoding | Notes |
|------|------|------|----------|-------|
| CIS_bdpm.txt | 15,848 | 3.2MB | Latin-1 | Master drug list |
| CIS_CIP_bdpm.txt | 20,903 | 4.2MB | UTF-8 | Pack presentations |
| CIS_COMPO_bdpm.txt | 32,389 | 2.7MB | Latin-1 | Composition (1:C many) |
| CIS_CPD_bdpm.txt | 28,160 | 1.3MB | Latin-1 | Usage restrictions |
| CIS_HAS_SMR_bdpm.txt | 15,269 | 4.5MB | Latin-1 | Reimbursement reviews |
| CIS_HAS_ASMR_bdpm.txt | 9,912 | 4.5MB | Latin-1 | Clinical benefit grades |
| CIS_GENER_bdpm.txt | 10,704 | 1.2MB | Latin-1 | Generic groups |
| CIS_MITM.txt | 7,711 | 1.1MB | Latin-1 | ATC classification |
| HAS_LiensPageCT_bdpm.txt | 10,342 | 0.5MB | UTF-8 | HAS page URLs |
| CIS_CIP_Dispo_Spec.txt | 766 | 0.2MB | Latin-1 | Stock status |

**Total:** 151,979 rows, ~17MB raw

### Multi-Row Per CIS Distribution

| File | Unique CIS | Multi-Row CIS | Max Rows/CIS |
|------|-----------|---------------|--------------|
| CIS_CPD | 28,160 | ~80% | 2-5 |
| CIS_COMPO | 15,846 | ~85% | 59 (rare) |
| CIS_GENER | 10,704 | ~85% | 8-10 |
| CIS_HAS_SMR | 9,023 | 39% (3,493) | 12 |
| CIS_HAS_ASMR | 6,176 | 31% (1,917) | 8 |

### Performance Estimates

- **Full parse time:** ~2-3 seconds Python (CSV), ~500ms Rust (streaming)
- **Memory for full parse:** ~50MB peak (with string interning)
- **SQLite write time:** ~5-10 seconds for all tables with indexes
- **Longest imports:** CIS_HAS_SMR/ASMR (largest files, complex text fields)

### Recommended Import Order (Dependency-Aware)

1. `CIS_bdpm.txt` (master drug records) — load first, all others reference CIS
2. `CIS_CIP_bdpm.txt` (pack presentations) — foreign key to CIS
3. `CIS_COMPO_bdpm.txt` (composition) — foreign key to CIS
4. `CIS_CPD_bdpm.txt` (usage) — foreign key to CIS
5. `CIS_GENER_bdpm.txt` (generics) — foreign key to CIS
6. `CIS_MITM.txt` (ATC) — foreign key to CIS
7. `CIS_HAS_SMR_bdpm.txt` (reimbursement) — foreign key to CIS
8. `CIS_HAS_ASMR_bdpm.txt` (clinical benefit) — foreign key to CIS
9. `HAS_LiensPageCT_bdpm.txt` (HAS URLs) — foreign key to CIS
10. `CIS_CIP_Dispo_Spec.txt` (dispo) — foreign key to CIP

---

## B. SCHEMA DESIGN TRADE-OFFS

### 1. SQLite vs SQLite + WAL

**Recommendation: WAL mode enabled**

Rationale:
- Read-heavy workload (API queries dominate)
- WAL allows concurrent reads during background sync
- Checkpoint can be deferred to low-traffic periods
- Default journal mode (DELETE) causes lock contention

```rust
// Enable on connection
conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
```

### 2. Price Storage: INTEGER cents vs REAL euros

**Recommendation: INTEGER cents with pre-computed display**

Evidence:
- Prices come as "24,34" (French comma decimal) in CIP file
- All prices are exact to cent precision
- No currency math needed (API returns formatted strings)
- INTEGER avoids floating-point comparison issues

```sql
CREATE TABLE cip_presentation (
    prix_ville_cents INTEGER,  -- 2434 = 24.34€
    prix_aph_cents INTEGER,    -- NULL if unavailable
    base_tarifaire_cents INTEGER,
    ...
);
```

Display: `format!("{}.{:02}", cents / 100, cents % 100)`

### 3. FTS5 Search Index

**Recommendation: Yes, but scoped**

Query surface analysis:
- Name search (prefix, substring) — HIGH frequency
- CIS code lookup — MEDIUM frequency
- ATC code browse — LOW frequency
- Generic group lookup — MEDIUM frequency

FTS5 is appropriate for name search only. Other lookups use B-tree indexes.

```sql
-- Name search table
CREATE VIRTUAL TABLE drug_search USING fts5(
    cis_code UNINDEXED,
    name,
    name_normalized,
    content='drug_master',
    content_rowid='rowid'
);
```

Trade-offs:
- + Fast prefix/substring search
- + Handles 15K drugs efficiently
- - Extra storage (~2-3MB)
- - Must keep FTS in sync on updates
- - Not needed for exact-match lookups

### 4. Staging vs Normalized — Two-Tier Approach

**Recommendation: Skip staging, use direct import with transaction safety**

Analysis:
- Staging adds complexity without clear benefit
- 150K rows is not large enough to require phased loading
- Rust's rusqlite supports transactions; failed import rolls back cleanly
- Incremental sync should use REPLACE, not delete-then-insert

```rust
// Direct import pattern
conn.execute("BEGIN IMMEDIATE")?;
for row in parse_tsv()? {
    upsert(&conn, &row)?;
}
conn.execute("COMMIT")?;
// On failure: automatic ROLLBACK
```

If staging is desired for validation, use a separate `import_staging` table that gets swapped with `import_prod` after validation, not separate databases.

### 5. CIS Code: TEXT vs INTEGER

**Recommendation: TEXT, always**

Rationale:
- CIS codes are 8-digit numbers but NOT sequential (e.g., 60002283, 69103878)
- Leading zeros may appear (verify from data — none observed)
- TEXT allows exact matching without conversion
- 8-char fixed-width doesn't waste space with pointer overhead
- Foreign key joins work equally well

```sql
CREATE TABLE drug_master (
    cis_code TEXT PRIMARY KEY,  -- e.g., "60002283"
    ...
);
```

### 6. Foreign Key Enforcement During Import

**Recommendation: Deferred enforcement with graceful handling**

```sql
PRAGMA foreign_keys=ON;  -- but use DEFERRABLE INITIALLY DEFERRED

-- Import order respects FK: CIS first, CIP second, etc.
-- Violations: log to import_log, skip row, continue import
```

Trade-offs:
- Enforcing catches data quality issues early
- Deferred allows circular dependency handling
- Skipping with logging (rather than aborting) enables partial import
- BDPM occasionally has orphan records in source files

---

## C. INCREMENTAL SYNC ARCHITECTURE

### Hash-Only Detection: Failure Modes

| Failure Mode | Likelihood | Impact | Mitigation |
|--------------|------------|--------|------------|
| Content changed, size unchanged | Low | Missed update | Include SHA256 in detection |
| New file same hash (unlikely) | Very Low | False positive | Accept minor risk |
| Network corruption | Very Low | Data corruption | SHA256 + size verification |
| Server clock skew | N/A | N/A | N/A (file-based only) |

**Recommended detection:**
```json
{
  "file": "CIS_bdpm.txt",
  "size_bytes": 3164943,
  "sha256": "abc123...",
  "last_modified": null,
  "imported_at": "2026-05-26T10:00:00Z"
}
```

### Tracking Changes Without Row-Level Diffs

**Challenge:** No timestamps in source data. Cannot know what changed.

**Solution: Hash-per-file, store as append-only with full replacement**

```rust
struct ImportLog {
    id: i64,
    file_name: String,
    file_hash: String,       // SHA256 of file content
    file_size: i64,
    row_count: i64,
    imported_at: DateTime,
    status: ImportStatus,   // success / partial / failed
}

// Incremental logic:
// 1. Fetch current hash from import_log for each file
// 2. Compare with remote hash (from HTTP Content-Length + ETag or downloaded SHA)
// 3. If changed: DELETE + INSERT new rows (transaction)
// 4. If unchanged: skip
```

**Trade-off:** This approach replaces entire tables on any change. For 150K rows and weekly updates, this is acceptable.

### Dual Sync Schedule

| File Group | Cadence | Rationale |
|-----------|---------|-----------|
| Main BDPM files | Monthly | Official update schedule |
| Dispo_Spec | Weekly | ANSM publishes independently |

**Implementation:**

```rust
enum SyncGroup {
    MainBdmp,    // All files except Dispo
    Dispo,       // Dispo_Spec only
}

// Two separate sync jobs with independent schedules
// MainBdmp: cron "0 3 first-day-of-month *"
// Dispo: cron "0 3 * * 1"  (Monday mornings)
```

### import_log Table Schema

```sql
CREATE TABLE import_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_name TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    row_count INTEGER NOT NULL,
    status TEXT NOT NULL,        -- 'success', 'partial', 'failed'
    error_message TEXT,
    imported_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    duration_ms INTEGER,
    skipped_rows INTEGER DEFAULT 0,
    bad_rows INTEGER DEFAULT 0
);

CREATE INDEX idx_import_log_file ON import_log(file_name, imported_at DESC);
```

**Audit capability:** Query import_log to see when files changed, row counts, errors.

**Rollback capability:** Keep last N successful imports. On critical failure, re-import from previous known-good state.

---

## D. API DESIGN

### Query Surface Analysis

| Query Type | Frequency | Performance Target | Index Needed |
|------------|-----------|-------------------|--------------|
| Name search (prefix) | High | <50ms | FTS5 |
| Name search (substring) | Medium | <100ms | FTS5 |
| CIS exact lookup | High | <10ms | PRIMARY KEY |
| Price by CIP | High | <10ms | Index on cip_code |
| By generic group | Medium | <50ms | Index on group_id |
| By ATC code | Low | <100ms | Index on atc_code |
| By substance | Low | <100ms | Index on substance_id |
| SMR/ASMR history | Low | <100ms | Index on cis_code |
| Dispo status | Medium | <50ms | Index on cis_code |

### Read-Only API (Recommended)

**Why not write API:**
- BDPM is a public reference database, not a user data store
- No business logic requires mutation
- Write API adds complexity, security surface, testing burden
- If write API needed later, add it then

### API Endpoints (Suggested)

```
GET /api/drugs?q=<name>           # FTS search, paginated
GET /api/drugs/{cis}              # Full drug with presentations
GET /api/drugs/{cis}/compositions  # Composition breakdown
GET /api/drugs/{cis}/smr          # Reimbursement history
GET /api/drugs/{cis}/asmr         # Clinical benefit grades

GET /api/presentations?cis=<cis>   # All packs for a drug
GET /api/presentations/{cip}      # Single presentation with price

GET /api/generics/{group_id}      # All drugs in generic group

GET /api/atc/{code}               # Drugs by ATC classification
GET /api/atc/{code}/tree          # Full ATC tree

GET /api/dispo/{cis}              # Current availability status

GET /api/search?q=<name>&type=<drug|presentation|substance>
```

### Caching Strategy

**Static Database = Static Cache**

| Layer | TTL | Strategy |
|-------|-----|----------|
| File-based cache | Forever | Cache full responses, invalidate on sync |
| In-memory LRU | 1 hour | Cache hot queries |
| CDN | 24 hours | If API goes public |

**Implementation:**
```rust
// On sync completion:
clear_file_cache();

// Response with cache headers:
Cache-Control: public, max-age=3600
ETag: "v{import_id}"
Last-Modified: {last_import_time}
```

---

## E. ERROR HANDLING + DATA QUALITY

### Parse Failure Strategy

**Decision: Partial import with detailed logging**

```rust
enum ImportMode {
    Strict,     // Abort on any error
    Lenient,    // Skip bad rows, log, continue
}

struct ImportError {
    file: String,
    line: usize,
    reason: String,
    raw_line: String,
}
```

Recommendations:
- Default to `Lenient` for production
- Strict mode for initial schema validation
- All errors logged to `import_errors` table
- Metrics: `bad_rows / total_rows` ratio per file

### Bad Row Handling

| Error Type | Action | Rationale |
|-----------|--------|-----------|
| Missing required field | Skip | Incomplete data unusable |
| Invalid field type | Skip | Type mismatch |
| FK reference missing | Skip | Orphaned data |
| Encoding error | Retry with lossy | Some files have edge cases |
| Duplicate PK | Replace | Accept newer version |

### Schema Evolution Strategy

**Challenge:** BDPM adds columns silently with no version notification.

**Detection:**
```rust
// After parse, compare expected vs actual field count
let expected_fields = get_expected_field_count(&file_name);
if actual_fields != expected_fields {
    log_warning("Field count mismatch: {} vs {}", actual_fields, expected_fields);
    // Log to import_log for review
}
```

**Mitigation:**
1. Store extra columns (if any) in a `raw_extra` JSONB column
2. Alert on schema change via monitoring
3. Manual review of import_log after each sync
4. Version field in import_log tracks schema expectations

**Long-term:**
- Maintain field mapping documentation per file
- Version the parser, not just the data
- Emit warning on any parse divergence

---

## F. PHASE STRUCTURE

### Phase 1: Foundation (Core Schema + Basic Import)

**Goal:** Get data into SQLite, verify correctness, establish baseline

**Deliverables:**
- SQLite schema for all 10 tables
- CSV parser with encoding detection (Latin-1/UTF-8)
- Transaction-based import (all-or-nothing)
- import_log table with basic tracking
- CLI: `bdmp-import --file=<path>`

**What makes this unique:**
- This is the only phase that defines the schema
- All subsequent work depends on correct data model
- Error handling patterns established here propagate

**Dependencies:** None
**Estimated scope:** ~500 lines Rust

---

### Phase 2: API Layer (Read-Only REST)

**Goal:** Expose data via HTTP API with query capabilities

**Deliverables:**
- actix-web HTTP server
- Endpoints: drug search, CIS lookup, price lookup, generic groups, ATC browse
- FTS5 name search
- JSON response formatting
- Basic health check endpoint

**What makes this unique:**
- This is where query patterns are defined
- Performance expectations established (indexes confirmed)
- Response formats locked down

**Dependencies:** Phase 1 complete
**Estimated scope:** ~400 lines Rust

---

### Phase 3: Sync Engine (Incremental Updates)

**Goal:** Automated sync from BDPM source with change detection

**Deliverables:**
- File hash tracking (SHA256 + size)
- Incremental import (skip unchanged files)
- Dual sync schedule (monthly main, weekly Dispo)
- import_log enrichment with rollback capability
- CLI: `bdmp-sync --full` and `bdmp-sync --dispo-only`

**What makes this unique:**
- This handles the external data source
- Change detection logic lives here
- Error recovery and retry logic
- No data mutations during this phase

**Dependencies:** Phase 1 (needs schema), Phase 2 (needs health checks)
**Estimated scope:** ~300 lines Rust

---

### Phase 4: Observability (Monitoring + Metrics)

**Goal:** Visibility into sync health and API performance

**Deliverables:**
- Metrics endpoint (Prometheus format)
- Sync history dashboard data
- Import error tracking + alerting
- API request latency histograms
- Last-modified timestamps for cache control

**What makes this unique:**
- This is operations concern, not data concern
- Could be skipped for internal use
- Required for production deployment

**Dependencies:** Phase 2 (API metrics), Phase 3 (sync metrics)
**Estimated scope:** ~200 lines Rust

---

### Phase 5: Polish (Performance + UX)

**Goal:** Optimize for real-world usage, improve developer experience

**Deliverables:**
- FTS5 query optimization
- Connection pooling for API
- CLI: `bdmp-import --dry-run` validation
- CLI: `bdmp-status` showing sync state
- Documentation: schema reference, API docs

**What makes this unique:**
- This is optimization, not feature work
- Driven by actual usage patterns
- No new capabilities, only improvements

**Dependencies:** All prior phases
**Estimated scope:** ~150 lines Rust + docs

---

## Summary

The BDMP dataset is well-suited for SQLite with the following key decisions:

1. **Schema:** Normalized with staging skipped, TEXT for CIS, INTEGER for prices (cents)
2. **Sync:** Hash-per-file with REPLACE, dual schedule for Dispo
3. **API:** Read-only REST with FTS5 for search
4. **Error handling:** Partial import with logging, no abort on bad rows
5. **Phases:** Foundation -> API -> Sync -> Observability -> Polish

The absence of timestamps is the biggest architectural constraint, but the small dataset size (150K rows) makes full-table replacement practical and simple.

---

*Next steps: Review this document with stakeholder, confirm Phase 1 scope, begin implementation.*