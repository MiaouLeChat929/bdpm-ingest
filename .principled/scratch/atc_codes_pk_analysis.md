# atc_codes Table Primary Key Analysis

## Current Data Reality

**Verified from raw/CIS_MITM.txt:**

| Metric | Value | Implication |
|--------|-------|-------------|
| Total rows | 7,711 | All data, no header row |
| Unique CIS codes | 7,711 | 1:1 with rows — every CIS appears exactly once |
| Unique ATC codes | 1,255 | 81 five-char + 7,630 seven-char |
| Duplicate (CIS, ATC) pairs | 0 | Each (CIS, ATC) pair unique |
| ATC→CIS mapping | Many-to-one | Common ATC codes shared by 40-84 drugs (e.g., N02BF02: 84 drugs) |

**Field 3 (MITM status): NOT PRESENT in this file.**

The file has exactly 4 fields:
0. CIS code
1. ATC code (5-char or 7-char)
2. Drug name (brand name + dosage + form)
3. BDPM detail URL

There is no MITM status column. The "MITM" in the filename refers to the file being the source of MITM mapping data (drug → ATC), not a column in this file.

---

## Option Analysis

### Option A: Separate atc_codes + mitm (current BRIEF.md approach)

```
atc_codes(atc_code PK, drug_name, detail_url, parent_5, parent_3, parent_1)
mitm(cis, atc_code, mitm_status, PRIMARY KEY (cis, atc_code))
```

This was designed under the assumption that:
- `atc_codes` stores the WHO ATC taxonomy (lookup by code)
- `mitm` maps CIS → ATC with MITM status
- MITM status exists as a field

**Reality check:**
- There is NO separate `mitm_status` field in CIS_MITM.txt
- The CIS → ATC mapping is already 1:1 (each CIS has exactly 1 ATC in this file)
- BUT: 1,255 ATC codes are shared across 7,711 drugs — this tells us drugs can share ATC codes

**PROS:**
- Clean separation: WHO ATC taxonomy (stable, de-duplicated) vs CIS→ATC mapping
- ATC lookup by code is fast (single-table index scan)
- Mirrors how WHO actually publishes ATC data (codes are classification, not mappings)

**CONS:**
- Two tables to join for queries like "drugs under ATC A01AB02"
- Current BRIEF.md has `atc_code` column in `drugs` table — this conflicts with one-to-many mapping
- If WHO taxonomy changes, `atc_codes` needs versioning

---

### Option B: Flat atc_codes with one row per (CIS, ATC) pair

```
atc_codes(atc_code, cis, drug_name, detail_url,
           parent_5, parent_3, parent_1,
           PRIMARY KEY (atc_code, cis))
```

**Reality check:**
- In this file, (CIS, ATC) is unique — PK would be unique with just 1 row per pair
- BUT: In reality, drugs CAN have multiple ATC codes. This file may be an incomplete snapshot.

**PROS:**
- Single table for simple queries
- Direct CIS↔ATC join without intermediate table
- `parent_*` columns are naturally aligned with each ATC instance

**CONS:**
- Drug name (from MITM file) duplicated per ATC assignment
- Drug name also exists in `drugs` table from CIS_bdpm.txt — two sources for same data
- If ATC code is the PK, can't store multiple hierarchies in single row
- Wider PK than necessary (composite vs simple)

---

### Option C: atc_codes as pure WHO lookup + mitm as junction table

```
atc_codes(atc_code PK, drug_name_who, parent_5, parent_3, parent_1)
mitm(cis, atc_code, detail_url, PRIMARY KEY (cis, atc_code))
```

**Key insight:** The drug_name in CIS_MITM.txt is the MARKETED drug name (brand + dosage + form), not the WHO ATC classification name. These are different things:
- **WHO ATC purpose:** group drugs by therapeutic class
- **Market name:** brand-specific information

**PROS:**
- Clean separation of responsibilities:
  - `atc_codes`: WHO taxonomy (de-duplicated, stable)
  - `mitm`: French market mapping (with detail_url)
- `drug_name` in `atc_codes` becomes `drug_name_who` (the official ATC classification name, which would differ from marketed names)
- But actually: the CIS_MITM drug_name IS the marketed name, not ATX classification

**CONS:**
- `drug_name` from CIS_MITM doesn't belong in `atc_codes` (WHO ATC naming convention)
- `mitm.detail_url` belongs with the mapping, not the taxonomy

---

## The Critical Finding: drug_name Does Not Belong in atc_codes

The drug_name stored in CIS_MITM.txt is the **marketed drug name**, not the WHO ATC classification name. Example: "TRAMADOL EG L.P. 200 mg, comprimé à libération prolongée" is a branded product, not an ATC classification.

**Who publishes the official ATC classification names?**
These are available in the WHO ATC Index (official publication in January each year), but the BDPM CIS_MITM file only provides the mapping from French marketed names.

**The practical reality:**
- We don't have the OFFICIAL WHO ATC classification names in this file
- The `drug_name` in CIS_MITM is the marketed drug name (brand-specific)
- This marketed name ALSO exists in `drugs` table from `CIS_bdpm.txt`

**Conclusion:** `drug_name` should NOT be in `atc_codes`. If we want drug names, we join through `drugs` table via CIS.

---

## Recommendation: Hybrid Option C (Revised)

```
atc_codes(atc_code PK,
          parent_5_char,
          parent_3_char,
          parent_1_char)
          -- Note: drug_name NOT stored here (WHO taxonomy doesn't include marketed names)
          -- Note: detail_url NOT stored here (it belongs to the CIS mapping, not taxonomy)

mitm(cis        TEXT NOT NULL,
     atc_code   TEXT NOT NULL REFERENCES atc_codes(atc_code),
     detail_url TEXT,
     PRIMARY KEY (cis, atc_code))

-- drugs table keeps atc_code AS FIRST ATC (most specific = 7-char)
-- For drugs with multiple ATC, we query mitm table
```

**Query patterns this supports:**

1. "Drugs classified under A01AB02" (ATC → drugs):
```sql
SELECT d.* FROM drugs d
  JOIN mitm m ON m.cis = d.cis
  WHERE m.atc_code = 'A01AB02';
```

2. "All ATC codes for drug CIS X" (CIS → ATCs):
```sql
SELECT a.* FROM atc_codes a
  JOIN mitm m ON m.atc_code = a.atc_code
  WHERE m.cis = '60003620';
```

3. "ATC hierarchy lookup" (taxonomy alone):
```sql
SELECT * FROM atc_codes WHERE atc_code LIKE 'A01AB%';
```

4. "Drugs in therapeutic group" (5-char group):
```sql
SELECT d.* FROM drugs d
  JOIN mitm m ON m.cis = d.cis
  JOIN atc_codes a ON a.atc_code = m.atc_code
  WHERE a.parent_5_char = 'A01AB';
```

---

## Why Not Other Options

**Option A (current BRIEF.md):** Has the right structure (separate tables) but stores `drug_name` and `detail_url` in `atc_codes` where they don't belong. The `drugs` table already has `atc_code` column — this assumes one ATC per drug, but the design allows for many.

**Option A revised:** Remove `drug_name` and `detail_url` from `atc_codes`, keep them either:
- In `drugs` (detail_url per drug, but CIS_MITM has one detail_url per pair)
- In `mitm` (detail_url belongs to the mapping, not taxonomy)

**Option B:** Simplicity is appealing but creates data duplication and doesn't match WHO ATC design philosophy. ATC codes are classifications, not mappings.

---

## Implementation Notes

1. **atc_codes is a lookup table for WHO ATC hierarchy:**
   - Derived from ATC code analysis
   - 1,255 unique codes from this file (5-char + 7-char)
   - Parent hierarchy computed at ingest
   - Primary key: `atc_code TEXT PRIMARY KEY`

2. **mitm is the CIS ↔ ATC junction table:**
   - Source: CIS_MITM.txt
   - Currently 1:1 (CIS → ATC) for this file
   - Designed to handle 1:N (drugs can have multiple ATC codes)
   - `detail_url` stored here (mapping-specific, not taxonomy)
   - `drug_name` NOT stored — use `drugs` table via CIS join

3. **Constraint consideration:**
   - `mitm.atc_code REFERENCES atc_codes(atc_code)` enforces referential integrity
   - If a drug maps to an ATC not in our lookup table, reject at ingest (WHO publishes before BDPM updates)

4. **drugs table:**
   - Keep `atc_code` column as convenience for "most specific ATC" (7-char)
   - Full list via `mitm` join
   - OR remove `atc_code` from `drugs` entirely and force `mitm` join for all ATC queries

---

## Summary

**Choose: Hybrid Option C (Revised)**

- `atc_codes`: pure WHO taxonomy lookup (code + parent hierarchy)
- `mitm`: CIS ↔ ATC junction with detail_url
- `drugs`: master drug record, remove `atc_code` column (always query `mitm` for complete list)

This reflects the actual data reality:
- 7,711 CIS-ATC mappings (currently 1:1, designed for N:1)
- 1,255 ATC codes (hierarchical WHO classification)
- drug_name from CIS_MITM is marketed name, not ATC classification name → belongs in `drugs`, not `atc_codes`
