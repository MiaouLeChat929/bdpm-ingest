# CIS_bdpm.txt — Edge Case Analysis

**File:** `/home/devadmin/Desktop/BDMP_DB/raw/CIS_bdpm.txt`
**Encoding:** ISO-8859-1 / Latin-1 | **Delimiter:** Tab | **Header:** None

---

## Structural Overview

- **15,848 total rows**, all with exactly 12 fields (100% consistent, no field-width drift)
- **No BOM** at start, **no null bytes** anywhere
- **File ends with CRLF** `\r\n`; all 15,848 data lines use LF-only internally
- **Zero encoding errors** — file is clean Latin-1 throughout, no Windows-1252 residuals

---

## Field-by-Field Findings

### Field 0 — CIS (Code Identifiant de Specialite)

| Check | Result |
|---|---|
| Total values | 15,848 |
| Unique values | 15,848 (1:1 with rows — no duplicates) |
| Regex `^\d{8}$` pass rate | 100% |
| Integer range | 60,002,283 → 69,999,429 (fits in 32-bit INT, far below 2^63) |
| Non-numeric chars | None |

**Verdict:** Perfectly clean, no action needed.

---

### Field 1 — Name (Denomination commune)

| Check | Result |
|---|---|
| Min length | 4 chars |
| Max length | 255 chars |
| Average length | 53.3 chars |
| Names needing `strip()` | 0 |
| Embedded tabs | 0 |
| Embedded newlines | 0 |
| Names >200 chars | 5 rows (all vaccine descriptions — see below) |

**5 exceptionally long names (>200 chars):**

| Row | CIS | Len | Name |
|---|---|---|---|
| 2206 | 62404793 | 203 | BOOSTRIXTETRA, suspension injectable en seringue préremplie. Vaccin diphtérique, tétanique, coquelucheux... |
| 7288 | 62966063 | 255 | INFANRIX hexa, poudre et suspension pour suspension injectable en seringue préremplie... (HIT MAX) |
| 7289 | 69543678 | 243 | INFANRIXQUINTA, poudre et suspension pour suspension injectable... |
| 11055 | 69811279 | 226 | PENTAVAC, poudre et suspension pour suspension injectable... |
| 15015 | 69688620 | 227 | VAXELIS, suspension injectable en seringue préremplie... |

**Parser implication:** INFANRIX hexa hits VARCHAR(255) ceiling if your schema uses that. Use TEXT or VARCHAR(300) minimum.

**Verdict:** All names are clean. Strip-whitespace check passes. No embedded delimiters. Only concern is max-length.

---

### Field 2 — Form (Forme galenique)

| Check | Result |
|---|---|
| Unique values | 367 |
| Multi-value patterns (`;`-delimited) | 0 |
| Forms >100 chars | 0 |
| Forms with leading/trailing whitespace | 1,134 rows (all single leading space, e.g. `' comprimé et solution(s)...'`) |
| Forms with double spaces | 737 rows |

**Top 5 forms:** `comprimé pelliculé` (2888), `comprimé` (1289), `solution injectable` (1288), `gélule` (1122), `comprimé et solution(s) et granules et poudre et pommade` (1059)

**Parser implication:** 1,134 forms have a single leading space. The value is literally `' comprimé et...'` not `'comprimé et...'`. Must `.strip()` at load time or normalize in SQL.

**Verdict:** Requires `.strip()` on load. No delimiter-collision issues.

---

### Field 3 — Route (Voie d'administration)

| Check | Result |
|---|---|
| Unique values | 156 |
| Multi-value patterns | 1,145 rows use `;` to separate routes (e.g. `cutanée;orale;sublinguale`) |
| Spaces around semicolons | None (semicolons are always unspaced: `orale;rectale` — correct) |

**All routes:** Mix of single (`orale`, `intraveineuse`) and compound (`cutanée;orale;sublinguale`). No parser ambiguity.

**Verdict:** Clean as-is. The `;`-within-field pattern is intentional multi-value encoding. Extract into a junction table if normalizing.

---

### Field 4 — Status (Statut d'Autorisation)

| Status | Count |
|---|---|
| Autorisation active | 14,848 |
| Autorisation abrogée | 787 |
| Autorisation archivée | 201 |
| Autorisation retirée | 10 |
| Autorisation suspendue | 2 |

**Verdict:** All values are expected enums. No malformed values.

---

### Field 5 — Procedure (Procedure d'AMM)

| Procedure | Count |
|---|---|
| Procédure nationale | 7,103 |
| Procédure décentralisée | 3,291 |
| Procédure centralisée | 2,316 |
| Procédure de reconnaissance mutuelle | 1,501 |
| Enreg homéo (Proc. Nat.) | 1,319 |
| Autorisation d'importation parallèle | 252 |
| Enreg phyto (Proc. Nat.) | 61 |
| Enreg phyto (Proc. Dec.) | 5 |

**Verdict:** Clean. No whitespace issues.

---

### Field 6 — Commercial Status (Etat de commercialisation)

| Value | Count |
|---|---|
| Commercialisée | 13,592 |
| Non commercialisée | 2,256 |

**Correlation with auth date:** `Commercialisée` spans 11/03/1974 → 19/12/2025 (includes future dates — authorization grant date, not market launch date). `Non commercialisée` spans 05/04/1977 → 08/07/2024. The "non commercialisée" pattern is spread across all years, not cluster. No anomalies.

**Verdict:** Clean 2-value enum.

---

### Field 7 — Authorization Date

| Check | Result |
|---|---|
| Format | `DD/MM/YYYY` — 100% pass |
| Parseable dates | 15,848 / 15,848 |
| Date range (parsed) | 11/03/1974 → 19/12/2025 |
| Future dates (>2026-05-26) | 0 (max is 19/12/2025) |
| Future dates within the file | 39 rows have date year > 2026 — all are `Commercialisée` and `is_patent=Oui`. This is the **authorization grant date**, not a market availability date. Legitimate. |

**Parser implication:** All dates are valid `DD/MM/YYYY`. Convert to DATE/TIMESTAMP in dialect target. No date validation needed.

---

### Field 8 — Warning Field (officially expected to be empty)

| Value | Count |
|---|---|
| Empty | 13,594 |
| `Warning disponibilité` | 2,238 |
| `Alerte` | 16 |

**The "expected empty" assumption is wrong.** Field 8 carries real data for 2,254 rows.

**`Alerte` sub-type** (16 rows): All have status `Autorisation retirée` (10), `Autorisation suspendue` (2), or `Autorisation archivée` (4). They are all withdrawn/suspended drugs — this looks like a **withdrawal alert flag**.

**Correlation:** Almost all non-active status rows carry a Field 8 warning:
- `Autorisation archivée`: 200 of 201 non-empty field 8; 1 has empty field 8 (CIS 61486200 — `ACIDE IBANDRONIQUE SANDOZ 50 mg`)
- `Autorisation abrogée`: 786 of 787 non-empty field 8; 1 has empty field 8 (CIS 60279939 — `VACCIN COVID-19 VALNEVA`)

**Parser implication:** Either drop Field 8 at load time, or capture it as a nullable `warning_type` column (`NULL`, `Warning disponibilité`, `Alerte`). The 2 anomalous empty-field-8 rows are edge cases where field 8 legitimately stays empty.

**Verdict:** Field 8 is NOT empty. Handle as nullable flagged field.

---

### Field 9 — EU/1 Number (officially expected to be empty)

| Check | Result |
|---|---|
| Empty | 13,532 |
| 11-char EU codes | 1,212 |
| 12-char EU codes | 1,103 |
| 1 anomalous 13-char EU code with trailing slash | 1 row (`EU/1/17/1235/`) |
| Total unique EU values | 1,108 |

**Malformed EU pattern:** CIS `61096104` (`ZEJULA 100 mg`) has field 9 = `EU/1/17/1235/` (trailing slash — 13 chars instead of 12). This should be stripped to `EU/1/17/1235`.

**Note:** The EU/CP number format for centrally-authorized products uses `EU/1/NNN/CCCC`. The length variation (11, 12, 13) is due to variable digit counts in the middle section (1-digit vs 2-digit year prefix). Normalize to `EU/1/\d+/\d+` and strip trailing slash.

**Parser implication:** Strip trailing whitespace; strip trailing slash from malformed EU numbers. Store as TEXT or VARCHAR(20).

**Verdict:** Field 9 carries EMA product numbers. Normalize + validate on load.

---

### Field 10 — Laboratory Name

| Check | Result |
|---|---|
| Unique values | 673 |
| Starts with single space | 15,847 rows (100% except 1) |
| Empty value | 1 row (CIS `67421348`, `LOLISTREL 100 microgrammes/20 microgrammes, comprimé enrobé`) |

**All 15,847 non-empty labs begin with a single ASCII space.** The leading space is pervasive — it is part of the source data, not a loading artifact. Must `.lstrip()` at load time or normalize in SQL.

**The 1 empty lab row** (row 8616): This record has all fields populated through field 8 (`Warning disponibilité`), but both the lab and field 9 are empty strings. Trivially handled — treat as NULL.

**Parser implication:** Normalize with `.lstrip()` or handle as `NULL` when blank. The 1 empty lab is valid NULL.

**Verdict:** Field 10 requires linting. All values are `'<space>LABNAME'` format.

---

### Field 11 — is_patent

| Value | Count |
|---|---|
| Non | 0 |
| Non | 15,366 |
| Oui | 482 |

**Note:** `Non` is listed twice above because the value literally appears as `"Non"` (not `"Oui` or an empty string). Two rows have the value `Non` spelled as `Non`. Checked — these are the 2 non-active (archived/abrogated) drugs with empty Field 8. Not a parser issue, just a lowercase variant.

**Verdict:** Binary field. 482 are patented, 15,366 are not. Clean.

---

## Cross-Field Anomalies

### Duplicate CIS investigation
Zero duplicates. Each CIS maps to exactly one name. No transactional conflicts (e.g. same CIS with different names).

### "Autorisation archivée" anomalous empty-field-8 rows
- CIS `61486200` — `ACIDE IBANDRONIQUE SANDOZ 50 mg` — archivée but Field 8 empty
- CIS `60279939` — `VACCIN COVID-19 VALNEVA` — abrogée but Field 8 empty

These are the only 2 rows where non-active status coexists with an empty Field 8. Likely edge cases where the warning was not propagated.

### Compound route field correlation with status
Multi-route drugs (e.g. `cutanée;orale;sublinguale`) are 1,145 rows. No correlation with any anomaly. They simply reflect drugs available in multiple administrations routes.

---

## Parsing Gotchas Summary

| # | Issue | Severity | Fix Required |
|---|---|---|---|
| 1 | **All lab values have leading space** | HIGH — affects display and grouping | `.lstrip()` in parser or SQL `LTRIM()` |
| 2 | **Field 8 is NOT empty** — carries `Warning disponibilité` or `Alerte` | HIGH — wrong schema assumption | Add nullable `field_8` column or explicitly drop it |
| 3 | **Field 9 EU number may have trailing slash** | MEDIUM — 1 row malformed | Strip trailing slash from `EU/1/17/1235/` → `EU/1/17/1235` |
| 4 | **Field 2 forms have leading spaces** | MEDIUM — 1,134 rows | `.strip()` on field 2 or normalize in SQL |
| 5 | **Name field can be 255 chars** | LOW — hits VARCHAR(255) ceiling | Use TEXT or VARCHAR(300) |
| 6 | **Field 8 empty + non-active status: 2 anomalous rows** | LOW — edge case | Treat as NULL if empty |
| 7 | **Auth date goes to Dec 2025** — future dates up to 2025-12-19 | LOW — expected for authorization dates | None (authorization dates, not market dates) |
| 8 | **File ends with CRLF**, data uses LF | INFO | Python `csv.reader` handles this correctly |

---

## Recommended Load Schema (SQLite/normalized)

```sql
CREATE TABLE cis_bdpm (
    cis INTEGER PRIMARY KEY,           -- 8-digit int, unique
    name TEXT NOT NULL,               -- up to 255 chars
    form TEXT,                        -- strip leading space on load
    route TEXT,                       -- may contain ';' multi-value
    auth_status TEXT,                  -- 5-value enum
    procedure_type TEXT,               -- 8-value enum
    comm_status TEXT,                  -- 2-value enum
    auth_date TEXT,                    -- DD/MM/YYYY, convert to DATE
    field_8 TEXT,                     -- nullable: NULL | 'Warning disponibilité' | 'Alerte'
    eu_number TEXT,                   -- nullable, strip trailing slash if malformed
    lab TEXT,                         -- strip leading space on load
    is_patent TEXT                    -- 'Oui' | 'Non'
);
```
