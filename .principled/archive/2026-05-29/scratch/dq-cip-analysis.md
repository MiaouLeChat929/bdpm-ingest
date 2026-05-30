# CIS_CIP_bdpm 320 Bad Rows Analysis

## Summary

The 320 "bad" rows are **foreign key constraint violations**. Rows with CIS codes not present in the `drugs` table fail the `REFERENCES drugs(cis)` constraint during `INSERT OR IGNORE`.

## Root Cause

**243 CIS codes** in `CIS_CIP_bdpm.txt` do not exist in the `drugs` table. These orphan CIS codes have **320 total rows** across the raw file (some CIS codes have multiple presentations/CIP codes). When the import attempted to insert these rows, the FK constraint failed.

## Key Evidence

| Check | Result |
|-------|--------|
| Total rows in raw file | 20,899 |
| Rows imported | 20,579 |
| Bad rows (FK failures) | 320 |
| 20,899 - 320 | 20,579 ✓ |
| Orphan CIS codes in raw | 243 |
| Total rows for orphan CIS | 320 |

## How INSERT OR IGNORE Handles FK Violations

Unlike UNIQUE/PRIMARY KEY violations (silently ignored), **foreign key constraint violations cause INSERT OR IGNORE to return an error**. This is SQLite behavior:

```
# With FK enforcement ON (set by init_db):
INSERT OR IGNORE → Err("FOREIGN KEY constraint failed")

# With FK enforcement OFF:
INSERT OR IGNORE → Ok(0) (silently allows)
```

See `src/db/mod.rs` line 28: `PRAGMA foreign_keys=ON` is set during database initialization.

## Why No Orphan CIS in Final DB

The FK constraint prevented insertion of all 320 rows for the 243 orphan CIS codes. Query confirms:
```sql
SELECT COUNT(*) FROM presentations p WHERE NOT EXISTS (SELECT 1 FROM drugs d WHERE d.cis = p.cis)
-- Returns: 0
```

## Example Orphan CIS

Sample CIS codes in raw file but missing from drugs table:
- `63529999`
- `60545343`
- `69919810`
- `64167478`
- `64917175`

## Secondary Issue: Phantom Trailing Tab

The raw file has inconsistent field structure:
- **20,085 rows**: 14 fields (13 content + 1 trailing empty) → after `strip_one_trailing_empty`: 13 fields
- **814 rows**: 14 fields (no trailing empty) → after strip: 14 fields

The `has_trailing_tab_fix: true` in the schema only removes one trailing empty if present. This causes misalignment where 814 rows pass through with 14 fields, but `normalize_cis_cip` only accesses indices 0-11 (13 fields expected), ignoring field 13. Data in this extra field (e.g., reimbursement notes like "Ce medicament peut etre pris en charge...") is silently discarded.

## Recommendation

The orphan CIS codes likely represent:
1. Withdrawn/delisted drugs present in CIS_CIP_bdpm but removed from CIS_bdpm
2. Data synchronization lag between BDPM files

Options:
1. **Accept现状** - FK enforcement is correct behavior; 320 rows properly rejected
2. **Insert with FK OFF** - Risky; would create orphan presentations with no drug reference
3. **Log and investigate** - Add tracing for FK violations to capture which CIS codes fail and why

The 320 "bad" rows are **intentional and correct** rejections, not a bug.
