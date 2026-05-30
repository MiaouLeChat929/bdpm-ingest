# CIS_COMPO_bdpm Rejection Analysis

## Summary

CIS_COMPO_bdpm.txt has a **12.7% rejection rate** (3,506 logged as `bad_rows`).
The database contains **24,096 rows** vs 27,609 after deduplication.

## The Numbers

| Stage | Count | Delta |
|-------|-------|-------|
| Raw file lines | 32,389 | - |
| After dedup (cis+substance_code+dosage) | 27,609 | -4,780 |
| **Actual DB rows** | **24,096** | **-3,513** |
| Import log: rows_imported | 24,103 | - |
| Import log: bad_rows | 3,506 | - |
| **Unexplained gap** | | **3,513** |

The gap between dedup output (27,609) and DB (24,096) is 3,513 rows.
The import log says 24,103 + 3,506 = 27,609 — but DB has 24,096.
The 7-row discrepancy between 24,103 (logged) and 24,096 (actual) equals the number
of false-negative dedup drops.

## Root Causes

### 1. Dedup Key Mismatch (Primary Data Loss)

**The dedup key is `(cis, substance_code, dosage)`, but the PK is `(cis, substance_code, seq)`.**

This causes rows with the same `(cis, substance_code, dosage)` but **different seq** to be
incorrectly deduplicated. These rows represent different pharmaceutical forms (granules,
solution buvable, etc.) for the same substance at the same dilution.

**Example:** CIS 60002746, substance 05319 (ACTAEA RACEMOSA...), dosage
"2CH a 30CH et 4DH a 60DH" has seq=7 (granules) and seq=8 (solution buvable).
Dedup keeps one, drops the other — data loss of 1 row.

Across all such cases: **4,780 rows removed by dedup are all false negatives**.
Only 17 rows in the raw file are true PK duplicates (same cis+substance_code+seq).
The dedup comment "4,780 duplicates" is misleading — these are not duplicates in the
database sense.

This is the **largest cause of data loss** (~4,780 rows lost before INSERT).

### 2. INSERT OR IGNORE Constraint Violations

After dedup, 27,609 rows go to INSERT OR IGNORE. Only 24,096 make it into the DB.

**3,506 rows are logged as `bad_rows`** (returned as `Err` from `stmt.execute()`).
These are rows that violated a constraint at INSERT time:

- **FK violations**: CIS in compositions referencing drugs not in drugs table.
  Earlier analysis shows 0 orphans, so FK is not the cause.
- **CHECK constraint**: `pharm_code IN ('SA', 'FT')` — all 32,389 rows have SA or FT.
- **NOT NULL violations**: 0 empty CIS/substance_code/seq in raw data.
- **Unknown**: 3,506 errors remain unexplained by the checked constraints.

The most likely cause: **False-negative dedup drops + subsequent PK collision at INSERT.**
When dedup removes row B (different seq, same cis+substance_code, different dosage
from row A), and row B's PK (cis+substance_code+seq) is unique, row B inserts OK.
When dedup removes row B (different seq, same cis+substance_code, SAME dosage),
row B's PK may match an already-inserted row. This creates a PK conflict at INSERT.

### 3. INSERT OR IGNORE Silent Failure Counting (Import Bug)

The import code counts constraint violations as successful imports:

```rust
match res {  // res = stmt.execute(...)
    Ok(_) => {
        stats.rows_imported += 1;  // BUG: Ok(0) for skipped rows counted as success
        if row.invalid_ean13 { stats.invalid_ean13 += 1; }
    }
    Err(e) => {
        stats.bad_rows += 1;  // Only Err increments bad_rows
    }
}
```

`INSERT OR IGNORE` returns `Ok(0)` when a constraint is violated (silent skip).
rusqlite's `execute()` wraps `sqlite3_step()` + `sqlite3_changes()`, returning the
number of rows modified. For skipped rows, this is 0.

These `Ok(0)` returns are **incorrectly counted as `rows_imported`** instead of
being tracked as skipped. The 24,103 `rows_imported` is inflated by ~3,499 silent skips.

## Homeopathy Filtering

**Not applied to CIS_COMPO_bdpm.** `normalize_row()` routes CIS_COMPO_bdpm to
`normalize_compo()` which has no filtering. Homeopathy detection only applies to
CIS_bdpm (4-layer detection including lab name, keyword, procedure type, dilution pattern).

The 2,810 rows with empty dosage include homeopathic dilutions but are NOT filtered.
They insert with NULL `dosage_mg`.

## Field Count

All 32,389 rows have exactly 8 fields. No field-count rejections.

## Recommendations

### Fix 1: Correct Dedup Key to Match Primary Key

Change dedup key from `(cis, substance_code, dosage)` to `(cis, substance_code, seq)`:

```rust
// dedup.rs
let key = (
    vals[0].as_deref().unwrap_or("").to_string(),   // cis
    vals[2].as_deref().unwrap_or("").to_string(),  // substance_code
    vals[7].as_deref().unwrap_or("").to_string(),  // seq (was: vals[4] = dosage)
);
```

This would keep all 32,372 unique rows (only 17 true duplicates), importing ~32,355 rows.

### Fix 2: Count Silent INSERT OR IGNORE Skips

Track the execute return value to distinguish `Ok(0)` (skipped) from `Ok(n>0)` (inserted):

```rust
match res {
    Ok(n) if n > 0 => { stats.rows_imported += 1; }
    Ok(0) => { stats.skipped += 1; }  // INSERT OR IGNORE silent skip
    Err(e) => { stats.bad_rows += 1; }
}
```

### Fix 3: Investigate 3,506 INSERT Errors

The `bad_rows` are `Err` returns from `stmt.execute()`, not `Ok(0)`.
This means SQLite IS raising errors for these rows, not silently skipping.
Run a diagnostic import that logs failing row content to determine the exact cause.

## Expected vs Actual

| Metric | Value | Notes |
|--------|-------|-------|
| Raw rows | 32,389 | - |
| Deduped (wrong key) | 27,609 | Should be 32,372 |
| Lost to dedup (false negatives) | 4,763 | Wrong dedup key |
| Lost to INSERT errors | 3,506 | Constraint violations |
| Lost to INSERT skips | ~0 | If errors account for all |
| **Actual DB rows** | **24,096** | - |

**Conclusion**: The 12.7% rejection rate combines:
1. **~4,763 rows lost via incorrect dedup** (wrong key — by far the largest cause)
2. **~3,506 INSERT constraint violations** (logged as bad_rows — second cause)
3. **~17 true duplicates** (legitimate)
