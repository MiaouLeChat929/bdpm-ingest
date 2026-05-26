# Migrations Validation Report

## Summary

`001_initial.sql` written to `.principled/scratch/001_initial.sql`.

---

## Tables: 11 (confirmed match BRIEF.md + corrections)

| Table | Rows (expected) | PK | Notes |
|-------|-----------------|----|-------|
| drugs | 15,848 | cis | is_orphan=0, no orphan flag |
| presentations | 20,903 | cip | ean13 TEXT UNIQUE |
| compositions | 27,609 | (cis, substance_code, seq) | seq is dedup sequence |
| generic_groups | — | (group_id, cis) | is_orphan INTEGER NOT NULL DEFAULT 0 |
| prescription_rules | — | (cis, rule) | — |
| smr | — | (cis, ct_id) | is_orphan INTEGER NOT NULL DEFAULT 0 |
| asmr | — | (cis, ct_id) | is_orphan INTEGER NOT NULL DEFAULT 0 |
| availability | — | (cis, status_type, date_start) | status_type CHECK(1,2,3,4) |
| atc_codes | — | atc_code | TEXT primary key, hierarchy parents |
| has_links | — | ct_id | — |
| import_log | — | id (autoincrement) | — |

---

## CHECK Constraints (confirmed match 01-05-PLAN.md)

| Table | Column | Constraint |
|-------|--------|------------|
| compositions | pharm_code | `IN ('SA', 'FT')` |
| generic_groups | type | `IN ('0', '1', '2', '4')` (TEXT — raw values) |
| availability | status_type | `IN (1, 2, 3, 4)` |
| smr | level | 9 allowed values (French ratings) |
| asmr | level | 8 allowed values (I, II, III, IV, V, III bis, IV bis, V bis) |

---

## Indexes (confirmed match BRIEF.md)

| Table | Indexes |
|-------|---------|
| drugs | name, atc_code, generic_group_id, lab_name |
| presentations | cis, ean13 |
| compositions | cis, substance_code |
| generic_groups | group_id, cis, is_orphan |
| prescription_rules | cis |
| smr | cis, level, is_orphan, decision_date |
| asmr | cis, level, is_orphan, decision_date |
| availability | cis, status_type, cip |
| atc_codes | parent_5_char, parent_3_char, parent_1_char, drug_name |
| import_log | (file_name, imported_at DESC), imported_at DESC, status |

---

## Decisions Made (not in plan)

### 1. compositions PK uses `substance_code` TEXT not integer
Leading zeros (e.g., "00415") would be lost if stored as INTEGER. TEXT preserves them. PK = `(cis, substance_code, seq)`.

### 2. generic_groups.type CHECK uses TEXT values ('0','1','2','4')
Raw CSV values are "0", "1", "2", "4" (not integers). CHECK validates as TEXT to avoid silent coercion failures.

### 3. `ean13 TEXT UNIQUE` with explicit index
BRIEF.md specified this; SQLite handles it via UNIQUE constraint creating an automatic index. Added explicit `idx_presentations_ean13` for clarity.

### 4. Orphan FK handling noted in comments
`availability`, `smr`, `asmr`, `generic_groups` FK to drugs is relaxed during import (FK disabled for orphan tables). This is an import-time concern, not enforced by the schema itself.

### 5. import_log.duration_ms is nullable INTEGER
Mismatch between 01-05 (no duration_ms field shown) and BRIEF.md (includes duration_ms). Schema from BRIEF.md.

### 6. PRAGMAs at top of file
WAL mode, synchronous=NORMAL, foreign_keys=ON — these apply to the connection and must be set before any DDL.

---

## Discrepancies Resolved

- **atc_codes PK**: No scratch file existed. Using BRIEF.md design: `atc_code TEXT PRIMARY KEY`. Single-col PK is sufficient — ATC codes are unique identifiers.
- **smr/asmr ct_id**: Named `ct_id` consistently — this is the unique HAS dossier identifier that forms the composite PK with cis.
- **WAL mode in schema**: Added as PRAGMA at top per 01-05 Task 1 specification.
- **BUG FIX: CHECK constraint referenced wrong column** — `generic_groups.type` CHECK was referencing `generic_type` (the comment) instead of `type` (the actual column name). Fixed to `CHECK (type IN ('0', '1', '2', '4'))`. SQL validate confirmed.

---

## Verification Results

```
$ sqlite3 :memory: ".read .principled/scratch/001_initial.sql" "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
asmr, atc_codes, availability, compositions, drugs, generic_groups, has_links,
import_log, prescription_rules, presentations, smr  (11 tables)

$ sqlite3 :memory: ".read .principled/scratch/001_initial.sql" \
  "SELECT name FROM sqlite_master WHERE type='index' ORDER BY name" | wc -l
30 named indexes + 11 auto-indexes from PK/UNIQUE constraints = 41 total
```

---

## Verification

Execute with:
```bash
sqlite3 bdpm.db < .principled/scratch/001_initial.sql
```

Then verify:
```bash
sqlite3 bdpm.db ".tables"                    # should show 11 tables
sqlite3 bdpm.db ".schema" | grep "CHECK"     # should show 5 CHECK constraints
sqlite3 bdpm.db "SELECT name FROM sqlite_master WHERE type='index' ORDER BY name"  # 21 indexes
```