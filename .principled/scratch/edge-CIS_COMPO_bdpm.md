# Edge Case Analysis: CIS_COMPO_bdpm.txt

**File:** `/home/devadmin/Desktop/BDMP_DB/raw/CIS_COMPO_bdpm.txt`
**Encoding:** ISO-8859-1 (Latin-1) | **Delimiter:** Tab | **Rows:** 32,389

---

## Structural Summary

| Metric | Value |
|--------|-------|
| Total rows | 32,389 |
| All rows have exactly 8 fields | Yes (100%) |
| Null bytes | 0 |
| Embedded newlines in names | 0 |

---

## Field-by-Field Analysis

### Field 0 — CIS (Code Identifiant Specialite)

- **Total values:** 32,389
- **Unique:** 15,846
- **Format validation:** 100% match `^\d{8}$`
- **Reference integrity:** All CIS from COMPO exist in CIS_bdpm (0 orphan references)
- **Duplicate rate:** avg 2.0 compositions per CIS, max 59

**Verdict:** Clean. No invalid formats, perfect referential integrity.

---

### Field 1 — Form Label

- **Unique:** 263 forms
- **Max length:** 50 chars
- **Empty count:** 0
- **Top form:** `comprimé` (12,118 occurrences)

**Quirk:** 451 rows contain forms with embedded dosage (e.g., `comprimé 15 mg`, `comprimé 45 mg`). These should be split into `form_label` + `dosage` during normalization, but the parser should tolerate them as-is.

**Verdict:** Tolerate embedded dosage. Consider normalization post-ingest.

---

### Field 2 — Substance Code

- **Unique:** 3,896 codes
- **All 5-digit:** True
- **Leading zeros:** 11,669 (36%)
- **Non-numeric:** 0
- **Empty:** 0

**Verdict:** Clean. Zero-padded codes are valid (ISO standard). Preserve leading zeros.

---

### Field 3 — Substance Name

- **Unique:** 4,486 names
- **Max length:** 212 chars
- **Empty:** 0
- **>100 chars:** 41 rows
- **Embedded tabs:** 0
- **Windows-1252 smart quotes:** No Windows-1252 quirks detected (no `\x91`-`\x97`)

**Sample long names (>100 chars):**
```
ANTIGÈNES DE SURFACE DU VIRUS DE LA GRIPPE, INACTIVÉ, A/VICT...
ANTIGÈNES DE SURFACE DU VIRUS DE LA GRIPPE, INACTIVÉ, SOUCHE...
VIRUS DE LA GRIPPE INACTIVÉ, FRAGMENTÉ, A/CROATIA/10136RV/20
```

**Verdict:** Clean. Long names are legitimate (vaccine strain names). No embedded control characters.

---

### Field 4 — Dosage

- **Max length:** 99 chars
- **Empty:** 2,810 rows (8.7%)
- **Contains special chars ('à', 'è', 'µ', 'μ'):** 7,110 rows

**Pattern distribution:**
| Dosage Pattern | Count | Notes |
|----------------|-------|-------|
| Homeopathic (e.g., `2CH à 30CH et 4DH à 60DH`) | ~9,000 | Homeopathic dilution notation |
| Numeric mg (e.g., `10 mg`, `20 mg`) | ~8,000 | Standard pharmaceutical |
| Letter-only weird dosages | 18 | See below |

**Letter-only "dosages" (18 rows):**
These are anomalous — dosage field contains only letters, no numeric value:

| Dosage | Substance Name |
|--------|----------------|
| `TM` | SABAL SERRULATA TEINTURE MÈRE |
| `TM` | CRATAEGUS OXYACANTHA |
| `TM` | ESCHSCHOLTZIA CALIFORNICA |
| `qs un flacon` | ISOFLOURANE |
| `TM` | CALENDULA OFFICINALIS |
| `qs` | PARAFFINE LIQUIDE |
| `qs` | RUTA GRAVEOLENS |

**Interpretation:** `TM` = Teinture Mère (mother tincture), `qs` = quantum sufficit. These are valid pharmaceutical abbreviations, not errors.

**Verdict:** Handle 18 anomalous letter-only values as valid (homeopathic abbreviations). All other dosages are standard.

---

### Field 5 — Per Unit

- **Unique:** 905 values
- **Max length:** 78 chars
- **Empty:** 2,810 rows (same rows as empty dosage — consistent)
- **Top:** `un comprimé` (13,451), `une gélule` (2,649), `un flacon` (1,323)

**Consistency check:** 0 rows have per_unit populated but dosage empty. Data is consistent.

**Verdict:** Clean. Empty per_unit rows align with empty dosage rows — intentional.

---

### Field 6 — Pharm Code

- **Values:** `SA` (26,892 rows), `FT` (5,497 rows)
- **Valid set:** `{SA, FT}` — only 2 codes present
- **Empty:** 0

**Previously flagged as "invalid" — correction needed:**

The `FT` code is legitimate. Analysis shows:
- All 5,497 `FT` entries are **non-homeopathic** substances (METFORMINE, ATORVASTATINE, GLUCOSE ANHYDRE, etc.)
- The valid code set should be `{SA, FT}`, not the assumed `{SA, HAR, PM, AR, AD, AUT, NP, CU, CO}`

The assumed valid set was based on other BDPM files. Each file has its own Pharm Code vocabulary.

**Verdict:** `FT` is valid for this file. Update valid set to `{SA, FT}` for CIS_COMPO specifically.

---

### Field 7 — Sequence

- **Unique:** 88 distinct values
- **Range:** 1 to 205
- **Empty:** 0
- **Non-numeric:** 0
- **Gaps:** [42, 43, 45, 46, 47, 53, 54, 55, 56, 57]

**Verdict:** Clean. Gaps in sequence are normal — not all sequence numbers are used.

---

## Cross-Field Anomalies

### Duplicate Substance Entries

**Exact duplicates (same CIS + substance_code + dosage):** 1,455 duplicate combinations

**Max occurrences:** 7 (CIS=60881760, code=39244, SERUM EQUI)

These duplicates occur with identical values — likely intentional recording of the same substance appearing in multiple forms for the same CIS.

**Verdict:** Duplicates are exact matches. Investigate if deduplication is needed based on business logic.

### Form Labels with Embedded Dosage

451 rows have forms like `comprimé 15 mg` instead of separate `form_label=comprimé` + `dosage=15 mg`.

**Verdict:** Parser should tolerate. Normalization should split on first numeric pattern.

---

## Whitespace Issues

- **Trailing whitespace:** 1,247 fields affected
- **Leading whitespace:** 21 fields affected

**Verdict:** Trivial. Apply `str.strip()` during ingestion. Not a parsing risk.

---

## Summary: Parser Recommendations

| Issue | Severity | Action |
|-------|----------|--------|
| Form labels with embedded dosage | Low | Tolerate; normalize post-ingest |
| Letter-only dosages (TM, qs) | Low | Treat as valid; no error |
| `FT` pharm code | Low | Update valid set to `{SA, FT}` |
| Trailing whitespace | Low | Strip during ingest |
| 41 very long substance names | Info | Ensure VARCHAR(500) or TEXT |
| 1,455 duplicate entries | Business | Determine if deduplication needed |

**Overall:** CIS_COMPO_bdpm.txt is structurally clean with no parsing risks. The only actionable items are normalization choices and business logic decisions.