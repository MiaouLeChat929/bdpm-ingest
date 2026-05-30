# CIS_CPD_bdpm.txt Import Failure Analysis

## Summary

The import of CIS_CPD_bdpm.txt fails due to a **foreign key constraint violation** when inserting into `prescription_flags`. The root cause is that 20 CIS codes in the raw CPD file do not exist in the `drugs` table.

## Root Cause

**Location**: `/home/devadmin/Desktop/BDMP_DB/src/import/mod.rs` lines 343-355

```rust
BDPMFile::CIS_CPD_bdpm => {
    let v = &row.values;
    let cis = v[0].as_deref().unwrap_or("");
    let rule_text = v[1].as_deref().unwrap_or("");
    stmt.execute(rusqlite::params![
        cis, rule_text,
    ])?;  // <-- INSERT OR IGNORE into prescription_rules (OK)
    let flags = CpdFlags::from_rule(rule_text);
    tx.execute(
        "INSERT OR REPLACE INTO prescription_flags(cis, ...) VALUES (?1, ?2, ...)",
        //     ^^^^^^^^^^^^^^^^^^^ <-- FAILS on FK violation
        ...
    )
}
```

**Problem**:
- `prescription_rules` uses `INSERT OR IGNORE` via prepared statement - correctly skips FK violations
- `prescription_flags` uses `INSERT OR REPLACE` - **fails on FK violation** because the REPLACE action requires the FK to be satisfied

## Orphaned CIS Codes

20 CIS codes exist in `CIS_CPD_bdpm.txt` but NOT in the `drugs` table:

```
60862404
61876862
63300452
63621440
63887327
64163476
64181296
64743778
65329132
65647882
66202611
66308543
66578108
66829076
67159166
67183854
67401121
69200149
69302990
```

## Why These CIS Codes Are Missing

These are drugs that:
1. Have prescription/dispensing rules in BDPM's CPD file
2. Do NOT exist in the core `drugs` table (likely withdrawn, deprecated, or filtered as homeopathic)

This is a **data quality issue in BDPM** - the CPD file contains entries for drugs that no longer exist in the core database.

## Evidence

| Metric | Value |
|--------|-------|
| Raw CPD rows | 28,153 |
| Orphaned CIS | 20 |
| prescription_rules rows | 0 |
| prescription_flags rows | 0 |
| FK pragma | ON (1) |

## Relationship to External Audit Finding

The external audit noted "not all CIS have CPD entries" - this finding is about the **reverse**: some CIS have CPD entries but the drug no longer exists. These are:
- Drugs withdrawn from the market
- Drugs with deprecated CIS codes
- Homeopathic drugs filtered at import time

## Recommended Fix

Change `INSERT OR REPLACE` to `INSERT OR IGNORE` for `prescription_flags`:

```rust
// Before (line 351):
"INSERT OR REPLACE INTO prescription_flags(...) VALUES (...)

// After:
"INSERT OR IGNORE INTO prescription_flags(...) VALUES (...)
```

This will:
1. Skip rows where the CIS doesn't exist in drugs
2. Not attempt REPLACE (which fails FK check)
3. Match the behavior of `prescription_rules` insertion

## Verification After Fix

After the fix, verify:
```sql
SELECT COUNT(*) FROM prescription_rules;  -- Should be ~28,133 (28,153 - 20 orphans)
SELECT COUNT(*) FROM prescription_flags; -- Should be ~28,133
```
