# Edge Case Analysis: CIS_GENER_bdpm.txt

## File Overview

- File: `/home/devadmin/Desktop/BDMP_DB/raw/CIS_GENER_bdpm.txt`
- Encoding: ISO-8859-1 / Windows-1252 (Western Latin-1)
- Delimiter: Tab
- Header row: None
- Rows: 10,704
- File size: 1,215,963 bytes (~113.6 bytes/row)
- Line termination: CRLF (`\r\n`)
- All rows have exactly 5 fields (100% uniform — not a single short or long row found)
- Zero empty rows, zero all-blank rows, zero null bytes in any field

---

## Field Inventory

| # | Name | Content | Expected | Status |
|---|------|---------|----------|--------|
| 0 | `group_id` | Integer group key | Integer | **GOOD** |
| 1 | `name` | Drug group name | String | **GOOD** |
| 2 | `cis` | 8-digit CIS code | Integer | **GOOD** |
| 3 | `type` | Reference/generic classification | 0 or 1 | **ANOMALY — see below** |
| 4 | `sort_order` | 1-indexed intra-group rank | Integer | **GOOD** |

---

## Structural Findings

### All 10,704 rows are exactly 5 fields — STRUCTURAL RIGIDITY
No short rows, no long rows, no malformed lines. Parsers expecting exactly 5 fields will never encounter misalignment.

### 1,659 unique groups; group sizes span 1–36
- Size 1: 111 groups
- Size 2: 363 groups
- Size 3: 224 groups
- Size 4–5: 256 groups
- Size 6–10: 349 groups
- Size >10: 356 groups (largest: gid=968 with 36 members)

### No duplicate CIS codes within the same group

---

## Field-by-Field Findings

### field_0 (`group_id`) — Clean
- 10,704 total, 1,659 unique (matches group count)
- Every value is a pure integer string (`^\d+$`)
- Zero empty values
- **No action needed**

### field_1 (`name`) — Double-Space Contamination (HIGH PREVALENCE)
- 910 rows contain double-spaces (`  `) within the name string
- Not a structural corruption but a data-quality issue that will propagate into search indexes, caches, and UI labels
- Also appears in group names (shared across all rows of a group), so fix should be applied at deduplication time, not per-row
- No leading/trailing space contamination
- No null bytes

### field_2 (`cis`) — Clean and Highly Precise
- 10,704 values, 10,628 unique (76 CIS codes appear in exactly 2 groups)
- Every value is exactly 8-digit (`^\d{8}$`)
- Zero empty values
- **Important semantic pattern — see cross-field findings for details**

### field_3 (`type`) — SEMANTIC OVERLOAD (MAJOR FINDING)

Expected domain: `0` (reference brand) or `1` (generic).

**Actual distribution:**

| Value | Count | Semantic |
|-------|-------|----------|
| `0` | 1,781 | Reference brand |
| `1` | 8,826 | Generic |
| `2` | 36 | Reused across groups |
| `4` | 61 | Sustained-release companion |

Non-standard values: `2` and `4`. Both must be handled explicitly.

**Value `2` (36 rows across 28 groups):**
- A CIS code that appears in **two different groups** simultaneously
- The same drug (same CIS) is classified once at one dosage level (group) and reused at another
- Pattern: these CISs connect cross-group entries — they allow the same drug to participate in two therapeutic groups at once
- Examples:
  - `69448283` → gid=31 (METFORMINE 500mg, type=1) AND gid=33 (METFORMINE 1000mg, type=2)
  - `67709496` → gid=102 (SIMVASTATINE 20mg, type=2) AND gid=349 (SIMVASTATINE 10mg, type=1)
- 21 of 36 type-2 codes are reused across groups; 15 are single-group
- Type 2 CISs maintain `sort_order` at the original position (not necessarily at the group's sort boundary)

**Value `4` (61 rows across 10 groups):**
- Sustained-release (LP/retard) formulations paired with their reference brand
- Clusters across a small number of large groups:

| Group | Count | Drug |
|-------|-------|------|
| 968 | 21 | ESOMEPRAZOLE MAGNESIUM TRIHYDRATE (largest, 36 members) |
| 969 | 19 | ESOMEPRAZOLE MAGNESIUM TRIHYDRATE (?) 40mg |
| 698 | 5 | TRAMADOL 100mg LP |
| 699 | 5 | TRAMADOL 150mg LP |
| 700 | 4 | TRAMADOL 200mg LP |
| 146 | 3 | TAMSULOSINE |
| others | 1-2 | ROPINIROLE, ALFUZOSINE |

- Type 4 co-exists with types 0 and 1 in the same group in 10 groups; type 4 rows appear interleaved (not clustered)
- Within gid=968, type 4 fills positions 2,4,6–11,14,16–18,22–23,25–26,28–30 in sequence alongside types 0 and 1

**Parser design implication:** Field 3 is semantically overloaded. A naive boolean interpretation (`type == 0 ? "reference" : "generic"`) will silently classify type-2 and type-4 rows as generics, which is semantically incorrect.
- Recommended enum: 0=reference, 1=generic, 2=cross-group-link, 4=sustained-release

### field_4 (`sort_order`) — Perfect 1-to-N Continuity
- Every single row has a sort_order value (0 empty rows)
- All 1,659 groups use a perfect 1-to-N sequence (no gaps, no overflow, no gaps)
- Range: 1–36 (matches exactly the largest group size)
- All values are positive integers within [1, group_size]
- **No action needed — no parser guard required**

---

## Cross-Field Anomalies

### CIS Cross-Group Reuse (63 CIS codes, MEDIUM PRIORITY)
- 76 unique CIS codes (of 10,628 total) appear in 2 different groups
- This means the same drug product participates in multiple therapeutic groupings
- In every verified case: different doses, same active ingredient, different group contexts
- This is semantically valid but means a join on (group_id, CIS) cannot be unique
- Parser behavior when encountering this: should NOT log as an error — it is intentional
- Canonical lookup: the same CIS code may have different `type` values in different groups

**Resolution:** When resolving "is this CIS a generic?" — type must be checked per (group_id, CIS) pair, not globally per CIS.

### Groups with Zero Reference Brands (11 groups — edge case, INFO)
- Group contains only generics (type=1) or type-2 entries
- Examples: gid=141 (DOXYCYCLINE HYCLATE), gid=181 (ERYTHROMYCINE), gid=957 (IBANDRONATE)
- Not a data error — reflects drugs with only generic market entries at that dosage

### Groups with >1 Reference Brand (130 groups — INFO)
- Multiple drugs within a group carry type=0 (e.g., two reference brands for the same compound)
- Examples: gid=7, 8, 9 (RANITIDINE groups each have 2 reference brands)

### Groups with Zero Generics (106 groups — edge case, INFO)
- Orphan groups where only the reference brand exists (type=0 only)
- Example: gid=3 (CIMETIDINE 400mg), gid=18 (MEBEVERINE)

---

## Encoding and Line Structure

| Check | Result |
|-------|--------|
| Line termination | CRLF (`\r\n`) — standard Windows |
| Null bytes | Zero |
| BOM (UTF-8) | Would cause Latin-1 decode errors — file is clean |
| Short rows | Zero |
| Empty rows | Zero |

---

## Parsing Gotcha Summary

| Priority | Finding | Implication |
|----------|---------|-------------|
| **HIGH** | Field 3 values `2` and `4` exist | Type enum must support 4 values, not boolean |
| **HIGH** | CIS reused across groups (63 codes) | Type must be resolved per (group, CIS) pair |
| **MEDIUM** | 910 rows with double-spaces in name | Strip/normalize names on ingest |
| **LOW** | Groups with only generics or only references | Group-level aggregation must handle empty subsets |
| **NONE** | NULL bytes, malformed fields, short rows | None — file is structurally clean |

---

## Verified Assertions

- All rows pass `len(row) == 5` test
- `group_id` is never empty or non-integer
- `cis` is never empty or non-8-digit
- `sort_order` never negative, never exceeds group size, always fills the 1-to-N sequence completely per group
- No file corruption indicators (null bytes, truncation, encoding errors)
