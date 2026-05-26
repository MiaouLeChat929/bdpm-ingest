# BDMP Final Consolidated Action List

**Date:** 2026-05-26
**Sources:** 13 external review artifacts (6 audits, 6 comparisons, 1 raw data verification, 1 MD catalog)
**Status:** Complete synthesis — ready for execution

---

## BLOCKING — Fix before any code is written

The fetcher, parser, or DB will break without these items.

---

### B1 — Add Windows-1252 encoding support (7 files)

**Files to change:** `01-01-PLAN.md`, `src/download/manifest.rs` (Encoding enum), `BRIEF.md`

**What to add/adjust:**
```rust
// Current (broken):
enum Encoding { Latin1, Utf8 }

// Fix: Add Windows1252 variant
enum Encoding { Windows1252, Latin1, Utf8 }
```

**Map 7 files to Windows-1252:**
- CIS_bdpm.txt
- CIS_COMPO_bdpm.txt
- CIS_CPD_bdpm.txt
- CIS_GENER_bdpm.txt
- CIS_HAS_SMR_bdpm.txt
- CIS_HAS_ASMR_bdpm.txt
- CIS_MITM.txt

**Verification:** `encoding_rs::WINDOWS_1252` produces U+2019 for byte 0x92 (not U+0092 control char). Confirm with unit test: decode sample bytes from CIS_HAS_SMR containing 0x92, assert output contains `'` (U+0027 after normalization).

---

### B2 — Smart quote normalization (U+2019 → U+0027)

**Files to change:** `01-04-PLAN.md` (Normalization pipeline), normalization module

**What to add:**
After Windows-1252 decoding of HAS files, add explicit normalization step:
```rust
// Normalize curly apostrophe to straight apostrophe
text.replace('\u{2019}', '\'')
```

**Verification:** Parse CIS_HAS_ASMR_bdpm.txt sample, assert output contains `'` (U+0027), not `'` (U+2019) or invisible control character.

---

### B3 — Fix Dispo CHECK constraint (1,2,3,4 not 1,4)

**Files to change:** `BRIEF.md` (schema), `01-05-PLAN.md` (availability import)

**What to change:**
```sql
-- Current (drops 56.9% of availability records):
CHECK (dispo_status_code IN (1, 4))

-- Fix:
CHECK (dispo_status_code IN (1, 2, 3, 4))
```

Status codes confirmed in raw data:
- 1: Rupture de stock (66 rows)
- 2: Tension d'approvisionnement (421 rows)
- 3: Arrêt de commercialisation (15 rows)
- 4: Remise à disposition (264 rows)

**Verification:** Query `CIS_CIP_Dispo_Spec.txt` — row count with code 2 + code 3 > 0; run import with CHECK (1,2,3,4) and assert all 766 rows load.

---

### B4 — Fix CIS_CIP_Dispo_Spec.txt encoding (Latin-1, not UTF-8)

**Files to change:** `BRIEF.md`, `01-01-PLAN.md`, `manifest.rs`

**What to change:**
```rust
// Current (causes decode error at byte 0xe0):
// CIS_CIP_Dispo_Spec.txt → Utf8

// Fix:
CIS_CIP_Dispo_Spec.txt → Latin1
```

**Verification:** UTF-8 decode fails on byte 0xe0 at position 207; Latin-1 decodes cleanly. Test parse with Latin-1, assert no decode errors, row count == 766.

---

### B5 — Relax FK constraints for orphaned CIS imports

**Files to change:** `01-05-PLAN.md` (import orchestration)

**What to add:**
```rust
// For drugs import: Use upsert (INSERT OR REPLACE), not DELETE + INSERT
// For SMR/ASMR/GENER imports: Either:
//   (a) PRAGMA foreign_keys=OFF during import phase, then re-enable
//   (b) Use nullable FK references for SMR/ASMR/GENER
```

**Raw data confirmed orphan rates:**
- CIS_HAS_SMR: 2,806 orphans (18.4%) — insert anyway
- CIS_HAS_ASMR: 1,567 orphans (15.8%) — insert anyway
- CIS_GENER: 2,503 orphans (23.5%) — insert anyway
- CIS_COMPO: 0 orphans — FK enforcement OK
- CIS_MITM: 0 orphans — FK enforcement OK

**Verification:** Import SMR/ASMR/GENER with FK relaxed, assert orphan count matches: SMR=2806, ASMR=1567, GENER=2503. FK on COMPO and MITM must pass (0 orphans).

---

### B6 — Upsert drugs table (not truncate)

**Files to change:** `01-05-PLAN.md` (drugs import function)

**What to change:**
```rust
// Current: DELETE FROM drugs; INSERT INTO drugs VALUES ...
// Fix: INSERT OR REPLACE INTO drugs VALUES ...
```

**Rationale:** Withdrawn drugs remain in CIS_bdpm.txt archive but disappear from current CIS file. Full truncate on `drugs` would lose historical HAS references. Upsert preserves withdrawn-drug rows while updating changed ones.

**Note:** Truncate+reload is OK for: presentations, compositions, generic_groups (all track current CIS only).

**Verification:** Import twice (simulate monthly update), assert withdrawn drug rows persist in `drugs` table after second import.

---

### B7 — URL pattern: `/download/file/` not `/telechargement?fich=`

**Files to change:** `01-01-PLAN.md` (fetcher base URL), `src/fetch/mod.rs`

**What to change:**
```rust
// Current (returns HTML, NOT data file):
BASE_URL = "https://base-donnees-publique.medicaments.gouv.fr/telechargement?fich={filename}"

// Fix:
BASE_URL = "https://base-donnees-publique.medicaments.gouv.fr"
// All 10 stable files: /download/file/{filename}
// CIS_InfoImportantes: /download/file/CIS_InfoImportantes.txt (preferred)
```

**Verification:** HEAD request to `/download/file/HAS_LiensPageCT_bdpm.txt` returns `Content-Type: application/octet-stream`. HEAD request to `/telechargement?fich=...` returns `Content-Type: text/html` (wrong).

---

## HIGH — Fix in the first implementation pass

Will cause data quality issues otherwise.

---

### H1 — ADD CHECK constraints for all enumeration columns

**Files to change:** `BRIEF.md` (schema section), `01-05-PLAN.md` (CREATE TABLE statements)

**What to add (before parser implementation):**
```sql
-- On compositions:
CHECK (pharm_code IN ('SA', 'FT'))

-- On generic_groups:
CHECK (generic_type IN (0, 1, 2, 4))

-- On smr:
CHECK (level IN ('Important', 'Modéré', 'Modérée', 'Modéré', 'Faible', 'Insuffisant',
                 'Insuffisant à HAS', 'Pas d''avis disponible', 'Légèrement important'))

-- On asmr:
CHECK (level IN ('I', 'II', 'III', 'IV', 'V', 'III bis', 'IV bis', 'V bis'))

-- On availability:
CHECK (status_type IN (1, 2, 3, 4))
```

**Verification:** Attempt to insert invalid enum value, assert SQLite CHECK constraint fires with meaningful error.

---

### H2 — Multi-line record handling

**Files to change:** `01-02-PLAN.md` (RowIterator), tab parser module

**What to add:**
```rust
// Continuation-line algorithm for CIS_HAS_SMR/CIS_HAS_ASMR avis fields:
loop {
    let line = match lines.next() {
        Some(l) => l?,
        None => break,
    };
    if line.strip().is_empty() {
        continue; // empty skip
    } else if line.chars().take(9).all(|c| c.is_ascii_digit()) {
        // Starts with 9-digit CIS — new record
        emit_record(&mut current_record);
        current_record = parse_cis_line(line);
    } else {
        // Continuation of previous record — append with space
        current_record.avis.push(' ');
        current_record.avis.push_str(line.trim());
    }
}
// emit final record
```

**Verification:** Parse CIS_HAS_SMR with embedded `\n` in avis field, assert single record with combined text (not split into multiple records).

---

### H3 — 60-second timeout for large files

**Files to change:** `01-01-PLAN.md` (fetcher), `src/fetch/mod.rs`

**What to add:**
```rust
// ureq: set_timeout(Duration::from_secs(60))
// Files up to 26 MB need generous timeout; current code has no timeout
```

**Rationale:** External analysis confirms 60s timeout covers largest files. Largest file is CIS_HAS_SMR at ~4.5 MB, but full download chain needs headroom.

**Verification:** Test download of largest file, assert completes without timeout; test with artificially short timeout, assert error is timeout (not generic network error).

---

### H4 — Max backoff cap (1 hour)

**Files to change:** `01-01-PLAN.md` (retry logic), `src/fetch/mod.rs`

**What to add/adjust:**
```rust
// Current: retry with 5s, 10s, 30s backoff — no cap
// Fix: cap at 1 hour (matches external consensus)
let backoff_secs = min(30 * 2_u64.pow(retry), 3600);
```

**Verification:** Simulate 5 consecutive 5xx responses, assert final backoff == 3600s (not > 3600).

---

### H5 — EAN13 UNIQUE constraint

**Files to change:** `BRIEF.md` (schema), `01-05-PLAN.md`

**What to add:**
```sql
-- In presentations table:
ean13 TEXT UNIQUE
```

**Rationale:** CIP13/EAN13 codes are globally unique identifiers. UNIQUE constraint enforces this DB-level invariant. All valid codes start with 34009 (French mandate) — add application-level pattern validation (SQLite CHECK on LIKE pattern may not be supported).

**Verification:** Attempt to insert duplicate ean13, assert UNIQUE constraint fires.

---

## MEDIUM — Fix before Phase 2 (API)

Improve robustness and maintainability.

---

### M1 — Fix "CRLF everywhere" claim in BRIEF.md

**Files to change:** `BRIEF.md` (file format section)

**What to change:**
```markdown
# Current (incorrect):
"CRLF everywhere: all files use Windows-style `\r\n` terminators"

# Fix:
"Line endings are mixed: CIS_CIP_bdpm.txt and CIS_InfoImportantes.txt use LF only.
 CIS_CPD_bdpm.txt has mixed \r\r\n sequences. Remaining files use CRLF.
 The parser strips `\r` then splits on `\n` — handles all variants."
```

**Verification:** N/A (documentation only).

---

### M2 — Document FTS5 content= anti-pattern

**Files to change:** `ROADMAP.md` (Phase 2 notes), `02-01-PLAN.md` (when created)

**What to add:**
```markdown
# Phase 2 FTS5 — CRITICAL anti-pattern to avoid:

## DO NOT USE:
CREATE VIRTUAL TABLE drugs_fts USING fts5(content='drugs', content_rowid='rowid');
// This pattern causes "database disk image is malformed" corruption during sync.

## USE INSTEAD:
CREATE VIRTUAL TABLE drugs_fts USING fts5(name, lab_name, form);
// Standalone FTS5 table with manual sync post-import:
-- INSERT INTO drugs_fts(name, lab_name, form) SELECT name, lab_name, form FROM drugs;
```

**Verification:** N/A (documentation + Phase 2 guardrail).

---

### M3 — CIS_InfoImportantes Phase 3.5 scope explicit

**Files to change:** `ROADMAP.md`, `BRIEF.md`

**What to add:**
```markdown
# Phase 3.5 — CIS_InfoImportantes (safety alerts)

## Implementation approach:
1. Parse BDPM download page HTML to extract current timestamped filename
2. Download via /download/file/CIS_InfoImportantes.txt (consistent pattern)
3. 6-hour TTL: store with timestamp, return cached if age < 6h
4. Re-scrape if age >= 6h (never proactive polling)
5. Safety alert API endpoint: flag staleness in response
```

**Verification:** N/A (scope clarification).

---

### M4 — Weekly cron shift to Thursday

**Files to change:** `ROADMAP.md` (GitHub Actions schedule), `.github/workflows/sync.yml`

**What to change:**
```yaml
# Current:
schedule: '0 3 * * 0'  # Sunday 03:00

# Fix:
schedule: '0 3 * * 4'  # Thursday 03:00 (closer to mid-week BDPM updates)
```

**Verification:** Verify cron expression is `4` not `0`.

---

### M5 — Extended import_log fields

**Files to change:** `BRIEF.md` (import_log table), `01-05-PLAN.md`

**What to add:**
```sql
ALTER TABLE import_log ADD COLUMN source_date TEXT;    -- BDPM's stated update date
ALTER TABLE import_log ADD COLUMN parser_version TEXT; -- git commit hash
```

**Rationale:** `source_date` enables tracking staleness without scraping. `parser_version` enables CI regression linking. External analyses recommended both; our plan had neither.

**Verification:** After import, query `import_log` and assert `source_date` and `parser_version` populated.

---

### M6 — Add Batch Size specification (1000 rows)

**Files to change:** `01-05-PLAN.md` (import functions)

**What to add:**
```rust
// In import functions:
// Batch 1000 rows per transaction for optimal WAL performance
// External analysis reports 50,000+ rows/sec achievable with this batch size
let batch_size = 1000;
```

**Verification:** N/A (performance tuning note).

---

## LOW — Consider for future phases

Nice-to-have improvements.

---

### L1 — Add encoding fallback path for U+00BF ¿ character

**Files to change:** `01-01-PLAN.md` (decoder module)

**What to add note:**
The `¿` character (U+00BF, 159 occurrences in CIS_CIP_bdpm.txt) confirms Latin-1 encoding for this file. Current per-file hardcoded encoding handles this. If fallback is needed: detect replacement characters (U+FFFD) at >0.5% threshold, log warning, attempt secondary encoding.

**Verification:** N/A (defensive note).

---

### L2 — Trailing tab awareness in CIS_CIP_bdpm.txt

**Files to change:** `01-02-PLAN.md` (parser notes)

**What to add note:**
CIS_CIP_bdpm.txt has 96.1% of lines with a trailing tab (creates phantom field 14). Parse by filtering empty trailing fields after tab split, preserving real data as fields 0-12 (13 fields).

**Verification:** N/A (parser awareness note).

---

## WHAT WE'RE CONFIRMED CORRECT

External analysis validates these decisions. Do not second-guess.

| Decision | Evidence | Source |
|----------|----------|--------|
| Tab-separated format (not fixed-width) | 10 files contain tab characters | consolidated-audit, compare-schemas, verification-raw-data |
| BLAKE3 hash (4-10x faster than SHA-256) | Parallelizable, modern, adequate for content identity | compare-monitoring, critique-architecture |
| INTEGER cents for prices (not float) | Fixed 2-decimal BDPM prices; integer arithmetic avoids fp errors | compare-schemas, critique-architecture |
| ISO-8601 date normalization | All sources confirm DD/MM/YYYY + YYYYMMDD coexist | all 6 external reviews |
| encoding_rs crate | Zero-copy, hardcoded per-file encoding | all 6 external reviews |
| rusqlite with bundled SQLite | De facto standard, WAL support | all 6 external reviews |
| HTML stripping for SMR/ASMR avis | 4,031 rows contain `<br>` tags (13-21% of records) | all 6 external reviews |
| Sync stack (ureq, not reqwest+tokio) | 120 downloads/year, no concurrency benefit, +60s compile time | all 6 external reviews |
| Single crate (not 5-crate workspace) | Solo dev, internal modules provide sufficient separation | all 6 external reviews |
| CIS as TEXT primary key | TEXT CIS PK confirmed across all external schemas | all 6 external reviews |
| Full-table truncate+reload | No row-level timestamps, 32K rows reload in seconds | all 6 external reviews |
| Price thousands-separator handling | 466 rows with `1,466,29` pattern correctly identified | all 6 external reviews |
| Trailing tabs in CIS_CIP_bdpm | 96.1% of lines confirmed in raw data | verification-raw-data |
| CIS_CIP_bdpm uses LF only (not CRLF) | 0 CRLF in CIS_CIP_bdpm | consolidated-audit, verification-raw-data |
| `/download/file/` for 10 stable files | `application/octet-stream` confirmed for all 10 | verify-live-urls |
| CIS_InfoImportantes works with `/file/` pattern | Both `/download/` and `/download/file/` confirmed | verify-live-urls |

---

## WHAT TO IGNORE

Findings from external reviews that don't apply to our project.

| Finding | Why Ignore | External Source |
|---------|-----------|-----------------|
| 5-crate workspace | Over-engineering for solo dev; internal modules provide equivalent separation | analyse_technique, etude_faisabilite |
| tokio/reqwest async for Phase 1 | Zero concurrency requirement; 120 downloads/year; add +60s compile time | analyse_technique, etude_faisabilite |
| rust_decimal for prices | BDPM prices are fixed 2-decimal; integer cents avoids fp AND extra crate dependency | etude_faisabilite |
| `_import_id` in every data table | Full-table-truncate pipeline makes this redundant; import_log via row_count/status is sufficient | analyse_technique, etude_faisabilite, feasibility_body |
| `is_orphan` flag per row | Solved at import-time via test assertions; `bad_rows` counter in import_log | etude_faisabilite |
| `is_active` soft-delete column | Full-table truncate makes this unnecessary; historical data preserved in raw archive | analyse_technique, etude_faisabilite |
| AUTOINCREMENT for 8 tables | Natural composite keys (cis+ct_id, group_id+cis) are semantically superior to synthetic ROWIDs | analyse_technique, etude_faisabilite, feasibility_body |
| Three-layer monitoring (BLAKE3 + data.gouv.fr + HTML scrape) | BLAKE3 post-download is definitive; data.gouv.fr has reliability concerns for medical data; HTML scraping is fragile | analyse_technique |
| CIS_bdpm.txt retention policy of "5 years" | **WRONG.** Data confirmed: oldest AMM date 1974-03-11, 89.5% have AMM > 5 years old. Full history retained. | etude_faisabilite |
| format_doc fixed-width format claim | TSV confirmed by raw byte analysis: tab characters exist in all 10 files; format_doc is outlier | format_doc.pdf only |
| data.gouv.fr API as change detection gate | Do not integrate as dependency; API is unreliable for medical use per etude_faisabilite section 9 | bdpm_feasibility_body |
| Incremental import with soft-delete | Correct approach for incremental pipelines; inappropriate for full-truncate design without timestamps | bdpm_feasibility_body |

---

## SUMMARY MATRIX

| ID | Priority | Fix | Files | Estimated Impact |
|----|----------|-----|-------|------------------|
| B1 | BLOCKING | Windows-1252 for 7 files | 01-01, manifest.rs, BRIEF.md | 52,000+ garbled apostrophes |
| B2 | BLOCKING | Smart quote U+2019→U+0027 | 01-04, normalization.rs | Search broken on HAS data |
| B3 | BLOCKING | CHECK (1,2,3,4) not (1,4) | BRIEF.md, 01-05 | 56.9% availability records lost |
| B4 | BLOCKING | CIS_CIP_Dispo Latin-1 not UTF-8 | BRIEF.md, 01-01, manifest.rs | Decode crash at byte 0xe0 |
| B5 | BLOCKING | Relax FK for orphan imports | 01-05 | SMR/ASMR/GENER imports fail |
| B6 | BLOCKING | Upsert drugs, not truncate | 01-05 | Historical HAS references lost |
| B7 | BLOCKING | /download/file/ not /telechargement?fich= | 01-01, fetch/mod.rs | Fetcher downloads HTML |
| H1 | HIGH | CHECK constraints on enum columns | BRIEF.md, 01-05 | Invalid values enter DB |
| H2 | HIGH | Multi-line record handling | 01-02, tab parser | Record splits on long avis |
| H3 | HIGH | 60s fetcher timeout | 01-01, fetch/mod.rs | Large file downloads timeout |
| H4 | HIGH | Max backoff cap 1h | 01-01, fetch/mod.rs | Unbounded retry escalation |
| H5 | HIGH | EAN13 UNIQUE constraint | BRIEF.md, 01-05 | Duplicate CIP13 accepted |
| M1 | MEDIUM | Fix CRLF "everywhere" doc | BRIEF.md | Documentation only |
| M2 | MEDIUM | FTS5 content= anti-pattern doc | ROADMAP.md | DB corruption in Phase 2 |
| M3 | MEDIUM | InfoImportantes Phase 3.5 scope | ROADMAP.md | Scope ambiguity |
| M4 | MEDIUM | Weekly cron Thursday not Sunday | ROADMAP.md, sync.yml | Suboptimal timing |
| M5 | MEDIUM | import_log +source_date +parser_version | BRIEF.md, 01-05 | Audit gap |
| M6 | MEDIUM | Batch size 1000 rows | 01-05 | Performance tuning note |
| L1 | LOW | Encoding fallback for U+00BF | 01-01 (notes) | Defensive note only |
| L2 | LOW | Trailing tab awareness | 01-02 (notes) | Parser awareness note |

---

## EXECUTION ORDER

```
PHASE A (Before any code):
  B1 → B2 → B3 → B4 → B5 → B6 → B7

PHASE B (First implementation pass):
  H1 → H2 → H3 → H4 → H5

PHASE C (Post-implementation / Phase 2 prep):
  M1 → M2 → M3 → M4 → M5 → M6

PHASE D (Future):
  L1 → L2
```
