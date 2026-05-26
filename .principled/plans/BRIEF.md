# BRIEF.md — BDPM Rust Project

## Vision

Build a reliable, self-updating Rust-powered local database and API from the French public drug database (BDPM), transforming raw government TSV files into a clean, queryable SQLite database with incremental sync, robust data profiling, and a production-grade sync pipeline.

## Context

The BDPM publishes 11 plaintext TSV files at `base-donnees-publique.medicaments.gouv.fr/telechargement` monthly. These files contain every licensed drug in France: names, compositions, prices, reimbursement rates, generic groups, HTA decisions (SMR/ASMR), stock status, and prescription rules. The BDPM updates monthly as a batch; CIS_CIP_Dispo_Spec.txt updates weekly, independently. The goal is to ingest all stable files into SQLite, with a production-grade update pipeline and a clean API surface.

### Download URL Pattern

**Correct pattern:** `/download/file/{filename}`

The old `/telechargement?fich=` pattern is broken and returns invalid responses. Use the `/download/file/` path for all downloads.

---

## Source State (Validated by Analysis)

### 11 Files, Monthly Cadence, Zero HTTP Caching

| File | Encoding | Lines | Fields | Key Observation |
|------|----------|-------|--------|----------------|
| `CIS_bdpm.txt` | Windows-1252 | 15,848 | 12 | **Fields 8–9 structurally empty** (F8 has 2,254 rows carrying data — unexpected); 1 malformed EU/1 in F9; F10 leading space on 15,847 rows; name max 255+ chars |
| `CIS_CIP_bdpm.txt` | **UTF-8** | 20,903 | 13 | Prices as `24,34`; **critical: 466 rows with values >1000 use comma as thousands separator** (`1,466,29` → must remove both commas, not replace); 7,357 NULL prices (~35%); 872 rows pre-1990 |
| `CIS_COMPO_bdpm.txt` | Windows-1252 | 32,389 | 8 | **4,780 exact duplicate rows** (same CIS+substance_code+dosage key; external reviews missed this). Dedup yields 27,609 unique rows; 451 rows embed dosage in form_label field; `_FT_` (not SA) covers 17% of entries — valid; substance names need whitespace strip |
| `CIS_HAS_SMR_bdpm.txt` | Windows-1252 | 15,257 | 6 | avis max 2,018 chars, avg 231; dates YYYYMMDD integer; SMR rows validated as field_count=6 (0 malformed rows in current snapshot) |
| `CIS_HAS_ASMR_bdpm.txt` | Windows-1252 | 9,906 | 6 | avis max 2,019 chars, avg 400; 41 conditional non-standard level variants |
| `CIS_GENER_bdpm.txt` | Windows-1252 | 10,704 | 5 | **Field 3 has values 0/1/2/4** — type is not binary; 63 CIS reused across 2 groups; 910 rows double-space in name |
| `CIS_CPD_bdpm.txt` | Windows-1252 | 28,160 | 2 | **15.5% of CIS have multiple CPD rows** (up to 6); 165 unique rule values |
| `CIS_CIP_bdpm.txt` | **UTF-8** | 20,903 | 13 | Prices as `24,34`; **critical: 466 rows with values >1000 use comma as thousands separator** (`1,466,29` → must remove both commas, not replace); 7,357 NULL prices (~35%); 872 rows pre-1990; **100% of lines have trailing tab** — strip phantom empty 14th field on parse |
| `CIS_CIP_Dispo_Spec.txt` | **Latin-1** | 766 | 8 | Fields 2 (CIP13) and 7 (comeback date) are intentionally empty, creating `[tab][tab]` mid-field patterns — NOT trailing tabs. CIP field 7 can be empty string; weekly update cadence, tracked separately |
| `CIS_MITM.txt` | Windows-1252 | 7,711 | 4 | ATC codes: 1,223 seven-char + 32 five-char; no other lengths |
| `HAS_LiensPageCT_bdpm.txt` | **UTF-8** | 10,342 | 2 | Pure ASCII reference links (valid UTF-8 subset); static file |
| `CIS_InfoImportantes.txt` | — | — | — | On-demand generation; **excluded from v1** — safety-critical, requires dedicated scraping with TTL cache |

### Confirmed Data Characteristics

- **Line endings**: Mixed: CIS_CIP_bdpm and CIS_InfoImportantes use LF; CIS_CPD has mixed `\r\r\n`; rest use CRLF. Parser strips `\r` before split.
- **Encoding**: 7 files Windows-1252 (CIS_bdpm, CIS_COMPO, CIS_CPD, CIS_GENER, CIS_HAS_SMR, CIS_HAS_ASMR, CIS_MITM); 1 file Latin-1 (CIS_CIP_Dispo_Spec); 2 files UTF-8 (CIS_CIP_bdpm, HAS_LiensPageCT)
- **No HTTP caching**: server returns no `ETag`, no `Last-Modified` — BLAKE3 hash is the only authoritative change signal
- **No header rows**: files contain raw data directly
- **CID codes**: 8-digit numbers but treated as TEXT (not sequential integers)
- **CIP uniqueness**: zero duplicate CIP codes across all 20,903 rows — valid primary key
- **EAN**: 100% start with `34009` (French national code prefix) — can normalize/validate at ingest
- **Smart apostrophe normalization**: U+2019 → U+0027 applies after Windows-1252 decode — 52,168 occurrences (SMR: 22,253; ASMR: 29,704; remaining files: 211) across all CP1252 files
- **Orphan handling**: SMR/ASMR/GENER tables contain orphan CIS references to withdrawn drugs. FK constraints relaxed during import. Explicit `is_orphan=1` flag set for orphan rows (2,806 SMR, 1,567 ASMR, 2,503 GENER). Presentations (CIP): 4 timing-artifact orphan CIS (drugs authorized after CIS_bdpm snapshot date). These are not errors — they will resolve on next CIS_bdpm update.

### Avis Field Characterization (SMR/ASMR)

Discovered by deep analysis after initial feasibility scan — not known at BRIEF time:
- SMR avis max 2,018 chars, median 153, avg 231; fits `VARCHAR(2048)`
- ASMR avis max 2,019 chars, median 212, avg 400; fits `VARCHAR(2048)`
- **4,031 rows contain HTML `<br>` tags** in avis field (13% SMR, 21% ASMR)
- Decision: strip HTML on store, preserve text content for API output

---

## Confirmed Edge Cases (Hardened by Analysis)

### Parsing Gotchas by File

**CIS_bdpm.txt:**
1. **Field 2**: 1,134 rows have literal leading space (`' comprimé...'`). Strip on ingest.
2. **Field 8**: Not always empty (assumption in initial study was wrong). 2,254 rows carry data — value unknown, tentatively "warning_type". Keep as nullable TEXT.
3. **Field 9**: 1 malformed EU number `EU/1/17/1235/` with trailing slash — strip slash on ingest.
4. **Field 10**: 15,847/15,848 rows have literal leading space. Strip on ingest.
5. **Name max length**: INFANRIX hexa hits 255 chars — use `VARCHAR(300)` or TEXT, not `VARCHAR(255)`.

**CIS_CIP_bdpm.txt:**
6. **Price format**: `replace(',', '.')` naive approach breaks on values >1000 (French thousands separator). Pattern: detect 2 commas → remove both commas entirely. e.g., `1,466,29` → `1466.29` → parse.
7. **Far-future date**: CIS `66338445` has date `29/11/2924` — likely typo for 2024. Validate date range on ingest, flag outliers.
8. **Reimbursement rate**: `65%`, `65 %` — same value, different format. Normalize to `f32` (0.65) at ingest.

**CIS_COMPO_bdpm.txt:**
9. **4,780 exact duplicates**: same `(CIS, substance_code, dosage)` — 32,389 total → 27,609 unique. External reviews missed this (they used wrong dedup key CIS+substance+Nature). Dedup via HashSet on ingest.
10. **Pharm code `FT`**: 5,497 rows valid (not `SA`). Valid set = `{SA, FT}` for this file only.
11. **Form label embed**: 451 rows embed dosage in form_label field. Parse correctly, don't treat as corruption.

**CIS_GENER_bdpm.txt:**
12. **Type field**: values `2` (36 rows) and `4` (61 rows) alongside `0` and `1`. Enum: `0=reference, 1=generic, 2=cross-group-link, 4=sustained-release`.
13. **CIS cross-group reuse**: 63 CIS appear in 2 groups. Type resolved per `(group_id, CIS)` pair, not globally per CIS.
14. **910 double-space names**: strip/normalize on ingest.

**CIS_HAS_SMR/ASMR:**
15. **18 tab-split malformed rows**: field_count ≠ 6. Filter at import: `WHERE len(fields) == 6`.
16. **HTML in avis**: 4,031 rows contain `<br>` tags. Strip on store, preserve text.

---

## Architectural Decisions

### Decision: Skip Staging, Direct Import with Transaction Safety

150K rows is not large enough to justify two-tier loading. Transaction-based import with rollback handles failures cleanly.

### Decision: Price → Integer Cents, Not Float Euros

Prices are stored as `24,34` → 2434 (cents). Integer arithmetic avoids floating-point errors. Display: `format!("{}.{:02}", cents / 100, cents % 100)`.

### Decision: CIS Codes as TEXT, Always

8-digit numbers but not sequential. TEXT enables exact matching without conversion. Foreign key joins work equally well.

### Decision: Dates Converted to ISO-8601 on Ingest

Store raw CSV in import_log. Normalized tables use ISO format (`YYYY-MM-DD`) for correct SQL date arithmetic and sorting. The `DecisionDate` field from SMR/ASMR files (YYYYMMDD integer) is also parsed to ISO.

### Decision: Content-Length is not a change signal, BLAKE3 is authority

Content-Length may be used as a cheap pre-filter (don't start HTTP GET if HEAD size matches stored size). BLAKE3 hash is the only authoritative change signal — computed post-download. First run always downloads (no stored state).

### Decision: Synchronous stack (no tokio)

SQLite is fundamentally synchronous. rusqlite is synchronous. The CLI downloads 10 files monthly and serves a read-only API over ~15K records. Async adds ~60s compile time + ~4MB binary size for zero benefit. Stack: `ureq` (sync HTTP) + `rusqlite` (sync SQLite) + `rouille` (sync HTTP API in Phase 2). No tokio, no async runtime anywhere.

### Decision: Full-Table Truncate+Reload, Not Row-Level Delta

No row-level timestamps exist in any BDPM file. Row-level delta is impossible. Full-table truncate + reload + optional audit log of what changed is the practical approach. For 32K-row tables this completes in seconds monthly. Name it what it is: **file-level change detection + full-table refresh**.

### Decision: Dual Sync Schedule

- Monthly: all stable BDPM files
- Weekly: CIS_CIP_Dispo_Spec.txt (independent cadence)

### Decision: `BDPMFile` Manifest Carries Full Schema Definition

Each file variant carries: `filename`, `base_url`, `encoding`, `field_count`, `required_fields`, `nullable_fields`, `date_format`, `numeric_fields`. Makes validation explicit and testable.

### Decision: WAL Mode + NORMAL Pragma on Every Connection

```rust
conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
```

WAL for concurrent reads during sync. NORMAL for monthly batch writes — safe without power-loss risk at this scale.

### Decision: `synchronous=OFF` During Bulk Insert (Performance)

For maximum write throughput during monthly import, temporarily set `PRAGMA synchronous=OFF` (skips disk flush per write) and `PRAGMA cache_size=-64000` (64MB page cache). After COMMIT, restore `PRAGMA synchronous=NORMAL` and `PRAGMA cache_size=-2000`. This halves import time with no durability risk for a single-user database.

### Decision: Three-Tier Sync Frequency

| Tier | Files | Cadence | Rationale |
|------|-------|---------|-----------|
| Standard | 9 files | Monthly | BDPM batch release schedule |
| Frequent | CIS_CIP_bdpm, CIS_CIP_Dispo | Weekly | Price/availability changes more often |
| Deferred | CIS_InfoImportantes | Phase 3.5 | Safety-critical, needs dedicated scraping |

### Decision: `axum` + `tokio` for Phase 2 API (not `rouille`)

For the read-only API (FTS5 search, drug detail, ATC browse, availability), `rouille` works but `axum` is better:
- `spawn_blocking` wraps `rusqlite` calls without blocking the async runtime
- tokio handles concurrent connections efficiently (vs rouille's thread-per-request)
- tower/tower-http middleware (tracing, timeouts, compression, rate-limiting) integrates cleanly
- Active maintenance (26K stars, daily commits) vs rouille's sparse updates
- MSRV 1.80 compatible with project toolchain

**Phase 1 import pipeline stays synchronous** — no tokio dependency needed. Only Phase 2 API adds async.

---

## Proposed SQLite Schema

```sql
-- Core drug records (CIS = unique identifier, TEXT for safety)
CREATE TABLE drugs (
    cis                     TEXT PRIMARY KEY,
    name                    TEXT NOT NULL,           -- max 255+ chars, use TEXT
    form                    TEXT,                    -- stripped leading/trailing space
    route                   TEXT,                    -- semicolon-separated
    auth_status             TEXT,                   -- "Autorisation active" etc.
    procedure_type         TEXT,                   -- "Procédure nationale" etc.
    comm_status            TEXT,                   -- renamed from drugs.comm_status
    auth_date              TEXT,                   -- ISO-8601: "1998-03-12"
    lab_name               TEXT,                   -- stripped leading space
    is_patent              INTEGER NOT NULL DEFAULT 0,  -- BOOLEAN: 0=Non, 1=Oui
    -- Warning/metadata from field 8
    alert_type             TEXT,                    -- nullable, 2254 rows carry data
    eu_number              TEXT,                    -- nullable, malformed slash stripped

    -- Generic group (informational, no FK enforcement)
    generic_group_id       TEXT,
    generic_sort           INTEGER,
    generic_type           TEXT,                   -- 0=ref,1=gen,2=cross,4=LP

    -- ATC: most specific ATC (7-char) from CIS_MITM — convenience column
    -- Full ATC list: query mitm table. Detail URL: mitm.detail_url
    atc_code               TEXT,                   -- convenience column, most common ATC per drug

    -- Auto-managed
    imported_at           DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_drugs_name ON drugs(name);
CREATE INDEX idx_drugs_atc ON drugs(atc_code);
CREATE INDEX idx_drugs_generic_group ON drugs(generic_group_id);

-- Presentations (one drug, many CIP codes)
CREATE TABLE presentations (
    cis                    TEXT REFERENCES drugs(cis),
    cip                    TEXT PRIMARY KEY,      -- 7-digit canonical (34009 stripped)
    cip_raw                TEXT,                   -- raw value from CSV
    labels                 TEXT,                   -- packaging description, may contain HTML
    pres_status            TEXT,                   -- renamed from pres.commercial_status
    comm_status            TEXT,
    comm_date              TEXT,                   -- ISO-8601
    ean13                  TEXT,                   -- 13-digit, starts 34009, CHECK constraint
    reimbursable           TEXT,                   -- "oui"/"non"
    reimb_rate             REAL,                   -- normalized: 0.65, 1.0 (not "65%")
    prix_ht_cents          INTEGER,                -- NULL if non-commercialisé
    prix_ville_cents       INTEGER,                -- NULL if non-commercialisé (NOT zero)
    prix_rate_cents         INTEGER,
    reimb_conditions       TEXT,                   -- free-text, may contain HTML
    PRIMARY KEY (cis, cip)
);

CREATE INDEX idx_presentations_cis ON presentations(cis);

-- Compositions
CREATE TABLE compositions (
    cis                    TEXT REFERENCES drugs(cis),
    form_label             TEXT,                    -- stripped
    substance_code         TEXT,                    -- "42215" — TEXT (leading zeros)
    substance_name          TEXT,
    dosage                 TEXT,                    -- "1,00 mg"
    per_unit               TEXT,
    pharm_code             TEXT,                    -- SA or FT (valid here only)
    seq                    INTEGER,
    UNIQUE (cis, substance_code, seq)
);

CREATE INDEX idx_compo_cis ON compositions(cis);

-- Generic groups
CREATE TABLE generic_groups (
    group_id               TEXT NOT NULL,          -- TEXT (e.g., "31", "968")
    group_name             TEXT,
    cis                    TEXT,                    -- CIS ∈ group
    type                   TEXT,                    -- "reference","generic","cross-group","LP"
    sort_order             INTEGER,
    is_orphan              INTEGER NOT NULL DEFAULT 0,  -- 1 if CIS not in drugs (withdrawn drug)
    PRIMARY KEY (group_id, cis)
);

CREATE INDEX idx_gengroup_groupid ON generic_groups(group_id);

-- Prescription rules
CREATE TABLE prescription_rules (
    cis                    TEXT REFERENCES drugs(cis),
    rule                   TEXT,
    PRIMARY KEY (cis, rule)
);

CREATE INDEX idx_rxrules_cis ON prescription_rules(cis);

-- HAS Ratings (SMR)
CREATE TABLE smr (
    cis                    TEXT REFERENCES drugs(cis),
    ct_id                  TEXT,
    decision_type          TEXT,
    decision_date          TEXT,                    -- ISO-8601 parsed from YYYYMMDD
    level                  TEXT,                    -- "Important"/"Modéré"/"Faible"/"Insuffisant"/conditional variants
    avis                   TEXT,                     -- stripped HTML, max VARCHAR(2048)
    is_orphan              INTEGER NOT NULL DEFAULT 0,  -- 1 if CIS not in drugs (withdrawn drug, 2806 rows expected)
    PRIMARY KEY (cis, ct_id)
);

CREATE INDEX idx_smr_cis ON smr(cis);

-- HAS Ratings (ASMR)
CREATE TABLE asmr (
    cis                    TEXT REFERENCES drugs(cis),
    ct_id                  TEXT,
    decision_type          TEXT,
    decision_date          TEXT,
    level                  TEXT,                    -- "I"–"V"
    avis                   TEXT,
    is_orphan              INTEGER NOT NULL DEFAULT 0,  -- 1 if CIS not in drugs (withdrawn drug, 1567 rows expected)
    PRIMARY KEY (cis, ct_id)
);

CREATE INDEX idx_asmr_cis ON asmr(cis);

-- Stock availability (live, weekly refresh)
CREATE TABLE availability (
    cis                    TEXT REFERENCES drugs(cis),
    cip                    TEXT,                    -- empty string valid here
    status_type            INTEGER,                -- CHECK IN (1, 2, 3, 4): 1=Rupture, 2=Tension, 3=Arrêt, 4=Remise
    status                 TEXT,
    date_start             TEXT,                    -- ISO-8601
    date_end               TEXT,                    -- ISO-8601 (nullable)
    date_remise            TEXT,                    -- ISO-8601 (nullable)
    source_url             TEXT,
    PRIMARY KEY (cis, status_type, date_start)
);

CREATE INDEX idx_avail_cis ON availability(cis);

-- ATC codes (WHO taxonomy — pure lookup, drug names come from drugs table)
CREATE TABLE atc_codes (
    atc_code               TEXT PRIMARY KEY,       -- 7-char (specific) or 5-char (group)
    parent_5_char          TEXT,                    -- 5-char parent
    parent_3_char          TEXT,                    -- 3-char parent
    parent_1_char          TEXT                     -- 1-char parent
);

-- MITM junction: CIS ↔ ATC mapping (1:N — drugs can have multiple ATC codes)
CREATE TABLE mitm (
    cis                    TEXT NOT NULL,            -- FK to drugs(cis)
    atc_code               TEXT NOT NULL REFERENCES atc_codes(atc_code),
    detail_url             TEXT,                    -- BDPM detail URL for this CIS-ATC pair
    PRIMARY KEY (cis, atc_code)
);

CREATE INDEX idx_mitm_cis ON mitm(cis);
CREATE INDEX idx_mitm_atc ON mitm(atc_code);

-- HAS external links
CREATE TABLE has_links (
    ct_id                  TEXT PRIMARY KEY,
    url                    TEXT
);

-- Import audit log
CREATE TABLE import_log (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    file_name              TEXT NOT NULL,
    file_hash              TEXT NOT NULL,           -- BLAKE3
    file_size              INTEGER NOT NULL,
    row_count              INTEGER NOT NULL,
    status                 TEXT NOT NULL,           -- success/partial/failed
    bad_rows               INTEGER DEFAULT 0,
    skipped_rows           INTEGER DEFAULT 0,
    imported_at            DATETIME DEFAULT CURRENT_TIMESTAMP,
    duration_ms            INTEGER
);

CREATE INDEX idx_import_log_file ON import_log(file_name, imported_at DESC);
```

---

## Deployment Target: GitHub Actions

The project is designed to run as a **GitHub Actions CI/CD pipeline**, not as a local daemon or Docker container.

**Sync pipeline:** Scheduled workflows replace local cron:
- Monthly workflow (`schedule: '0 2 1 * *'`): full BDPM sync, rebuild SQLite, publish as GitHub Release asset
- Weekly workflow (`schedule: '0 3 * * 0'`): Dispo file update only
- Manual trigger (`workflow_dispatch`): `--full` flag for forced rebuild

**State persistence across runs:** `import_state.json` stored as a workflow artifact (download previous → run sync → upload updated). Alternative: commit to a dedicated `state/` branch.

**Database distribution:** The built `bdmp.db` file is published as a **GitHub Release asset** (not workflow artifact — avoids 10GB shared quota). `softprops/action-gh-release@v2` handles tagging and upload. Consumers download the `.db` file directly from releases.

**WAL checkpoint before upload:** `sqlite3 data/bdmp.db "PRAGMA wal_checkpoint(TRUNCATE);"` runs before artifact upload to produce a single portable `.db` file without `-wal`/`-shm` sidecar files.

**Binary build:** `cargo build --release` with `Swatinem/rust-cache@v2` for Cargo caching. Optional musl static binary for portability. `rust-toolchain.toml` committed for reproducible CI.

**API deployment (future):** The Rust API binary can be deployed to Fly.io or Shuttle.rs with a mounted SQLite volume. No Docker needed — both platforms detect Rust and build from source.

---

## Update Detection Strategy

```
Monthly trigger: bdpm-sync --main
  → For each of 10 stable files:
      → HTTP HEAD with Content-Length (optimization only: skip GET if size matches stored)
      → HTTP GET download
      → BLAKE3 hash computed after download (AUTHORITATIVE SIGNAL)
      → If hash == stored hash: log "unchanged", skip
      → If hash differs or first run:
          → BEGIN IMMEDIATE transaction
          → DELETE FROM affected_table
          → INSERT all rows with normalization
          → COMMIT
          → Update import_log with hash, size, row_count, status, bad_rows

Weekly trigger: bdpm-sync --dispo
  → Same pattern for CIS_CIP_Dispo_Spec.txt only
  → Separate schedule, independent state tracking
```

**Change detection authority: BLAKE3 only.** Content-Length is an optimization for skipping the GET request, never the authoritative signal.

---

## CI Regression Tests (Defined Early, Not Polished Later)

Produced from profiling findings — embedded in the codebase from day 1:

```rust
// File-level invariants from profiling
const CIS_BDPM_EXPECTED_ROWS: usize = 15_848;
const CIS_CIP_EXPECTED_ROWS: usize = 20_903;
const CIS_COMPO_EXPECTED_ROWS: usize = 32_389;
// ... per file

// Per-file row count assertions
#[test] fn cis_bdpm_row_count() { assert_eq!(count_rows("CIS_bdpm.txt"), CIS_BDPM_EXPECTED_ROWS); }

// Field count assertions
#[test] fn cis_bdpm_field_count() { assert_eq!(field_count("CIS_bdpm.txt"), 12); }

// Referential integrity
#[test] fn cip_cis_exists_in_drugs() {
    let missing = missing_fk_refs("presentations", "cis", "drugs", "cis");
    assert_eq!(missing, 0, "CIP references {} non-existent CIS: {:?}", missing, &missing[..5]);
}

// Price normalization
#[test] fn price_normalization() {
    assert_eq!(parse_price_cents("24,34"), 2434);
    assert_eq!(parse_price_cents("1,466,29"), 146629); // thousands separator
    assert_eq!(parse_price_cents(""), None);
}

// Date parsing
#[test] fn date_parsing() {
    assert_eq!(parse_date_smrasmr("20260422"), "2026-04-22"); // YYYYMMDD
    assert_eq!(parse_date_ddmmYYYY("28/04/2026"), "2026-04-28");
}

// Field count regression (fails on schema drift)
#[test] fn schema_drift_detection() {
    let actual = field_count("CIS_bdpm.txt");
    assert_eq!(actual, 12, "CIS_bdpm field count changed: {} != 12", actual);
    // → CI fails and alerts before production re-import
}
```

---

## Priority Order for Rust Implementation

**Phase 0**: Data Profiling — already done via analysis. Seeds test assertions and edge case documentation.

**Phase 1** (Foundation):
1. Project scaffold + BDPMFile manifest with full schema metadata
2. HTTP fetcher with BLAKE3 change detection, rate limiting, first-run logic
3. Tab parser with per-file encoding + CRLF handling
4. Data profiling integration + FileSchema validation
5. Normalization pipeline: price, date, ID, whitespace, encoding
6. Staging layer + raw imports table (optional recovery)
7. Database init + migrations

**Phase 2**: FTS5 search, drug detail API, price lookup, generic groups, availability

**Phase 3**: File-level polling, full-table refresh, import log viewer

**Phase 3.5** (deferred): CIS_InfoImportantes with 6-hour TTL cache

**Phase 4**: CI regression suite, GitHub Actions release workflow, operational runbook

---

## Success Criteria

- All 10 stable files parseable with 0 silent data loss
- Row count assertions fire correctly when file structure changes
- SQLite opens in under 50ms for primary key lookups
- Full re-import completes in under 5 minutes
- `check --changed` completes in under 30 seconds (no re-download)
- Price normalization handles thousands-separator pattern correctly
- avis HTML stripping: `<br>` tags removed, text preserved
- API returns drug info with name search in under 100ms
