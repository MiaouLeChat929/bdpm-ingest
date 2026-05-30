# BDMP Data Quality: CIS_HAS_SMR / CIS_HAS_ASMR Analysis

## CIS_HAS_SMR_bdpm.txt

**File**: `CIS_HAS_SMR_bdpm.txt` (15,269 total rows)

### Structural

| Check | Result |
|-------|--------|
| Rows with 6 fields | 15,257 (99.92%) |
| Rows with 1 field (malformed) | 12 |
| Null bytes in raw file | 0 |
| Null bytes in decoded text | 0 |

**Malformed rows** (12 instances, 1 field each): All are tab-split fragments of avis text that got severed from their parent row. They all contain the string "détection de lésions positives à l'antigène membranaire spécifique de la prostate (PSMA)" — all belong to the same drug (Pluvicto/Lutetium, CIS unknown). The tab characters inside the avis text triggered the CSV parser to split one row into two. See malformed-row details below.

### Field 0 — CIS (Code Identifiant Specialite)

- Total: 15,269 | Unique: 9,023
- All values pass 8-digit integer check — no non-numeric CIS values.

### Field 1 — CT Decision ID

- Unique: 7,437
- All pass `^CT-\d+$` format — zero malformed IDs.
- Zero `CT-0` instances.
- No IDs exceeding 1,000,000 (safe from integer overflow).
- 2,424 (CIS, CT_ID) pairs appear more than once (likely different decision types or dates for same product/review).

### Field 2 — Decision Type

| Type | Count |
|------|-------|
| Inscription (CT) | 6,576 |
| Renouvellement d'inscription (CT) | 4,879 |
| Réévaluation SMR | 922 |
| Extension d'indication | 1,589 |
| Réévaluation SMR et ASMR | 580 |
| Réévaluation suite saisine Ministères (CT) | 195 |
| Réévaluation suite à résultats étude post-inscript | 165 |
| Modification des conditions d'inscription (CT) | 101 |
| Réévaluation ASMR | 100 |
| Nouvel examen suite au dépôt de nouvelles données | 69 |
| Extension d'indication non sollicitée | 67 |
| Autre demande | 14 |

### Field 3 — Decision Date (YYYYMMDD integer)

- All 15,257 valid rows: exactly 8 digits, zero bad-format values.
- Range: `20021218` (2002-12-18) to `20260422` (2026-04-22).
- Year distribution: peaks in 2015-2017 (1,357 / 1,729 / 1,188), tail off to 2026 (140 rows dated into the future — likely pre-loaded upcoming reviews).

### Field 4 — SMR Level

| Level | Count |
|-------|-------|
| Important | 9,491 |
| Insuffisant | 2,765 |
| Modéré | 1,622 |
| Faible | 1,151 |
| Non précisé | 88 |
| Commentaires | 99 |
| Important conditionnel | 18 |
| Modéré conditionnel | 10 |
| Faible conditionnel | 13 |

No null/empty values.

### Field 5 — Avis (opinion text)

- Empty: 0
- Length stats: max=2,018 | avg=231 | median=153
- Values >2,048: 0 | >5,000: 0
- Contains HTML `<br>`/`<p>`/`<b>` tags: **1,971 rows** (13.0%) — text is not plain; contains structured HTML fragments.
- Contains embedded tab: 0
- Contains embedded `\n` or `\r`: 0
- Quirk char codes present: none detected

---

## CIS_HAS_ASMR_bdpm.txt

**File**: `CIS_HAS_ASMR_bdpm.txt` (9,912 total rows)

### Structural

| Check | Result |
|-------|--------|
| Rows with 6 fields | 9,906 (99.94%) |
| Rows with 1 field (malformed) | 6 |
| Null bytes in raw file | 0 |
| Null bytes in decoded text | 0 |

**Malformed rows** (6 instances): Same root cause as SMR — tab characters embedded in avis text caused split. Three distinct drugs involved (rispéridone OKEDI, frémanezumab AJOVY, midostaurine RYDAPT). All are ASMR V opinions.

### Field 0 — CIS

- Total: 9,912 | Unique: 6,176
- All values pass 8-digit integer check.

### Field 1 — CT Decision ID

- Unique: 5,895
- All pass format check. Zero CT-0. Zero overflow-risk IDs.
- 292 (CIS, CT_ID) pairs are duplicated (different decision types or dates).

### Field 2 — Decision Type

| Type | Count |
|------|-------|
| Inscription (CT) | 6,653 |
| Extension d'indication | 2,015 |
| Réévaluation SMR et ASMR | 458 |
| Réévaluation ASMR | 133 |
| Réévaluation suite à résultats étude post-inscript | 105 |
| Modification des conditions d'inscription (CT) | 99 |
| Réévaluation suite saisine Ministères (CT) | 40 |
| Nouvel examen suite au dépôt de nouvelles données | 53 |
| Réévaluation SMR | 135 |
| Renouvellement d'inscription (CT) | 208 |
| Autre demande | 7 |

### Field 3 — Decision Date

- All 9,906 valid rows: exactly 8 digits.
- Range: `20021218` to `20260422` — identical range as SMR.
- Year distribution: flatter than SMR; peaks 2007-2008 and 2016 (not a single dominant year).

### Field 4 — ASMR Level

| Level | Count |
|-------|-------|
| V | 7,595 |
| IV | 1,246 |
| III | 600 |
| II | 207 |
| I | 85 |
| V dans l'attente de données | 13 |
| Commentaires sans chiffrage de l'ASMR | 160 |

No null/empty values.

### Field 5 — Avis

- Empty: 0
- Length stats: max=2,019 | avg=400 | median=212
- Values >2,048: 0 | >5,000: 0
- Contains HTML tags: **2,060 rows** (20.8%) — higher than SMR.
- Contains embedded tab: 0
- Contains embedded `\n` or `\r`: 0

---

## Cross-File Consistency

### Overlap Analysis

| Metric | Value |
|--------|-------|
| SMR unique (CIS, CT_ID) pairs | 12,558 |
| ASMR unique (CIS, CT_ID) pairs | 9,609 |
| Overlapping pairs (in both files) | 7,252 |
| SMR-only pairs | 5,306 (42%) |
| ASMR-only pairs | 2,357 (25%) |
| Date mismatches in overlapping pairs | **0** |

**Finding**: Zero date mismatches. When a (CIS, CT_ID) pair appears in both files, the decision date is identical. This is structurally correct — the same CT review produces both an SMR and an ASMR opinion simultaneously.

### SMR Level vs ASMR Level Cross-Tabulation (7,252 common pairs)

| SMR \ ASMR | I | II | III | IV | V | Com. sans chiffrage | V attend données |
|---|---|---|---|---|---|---|---|
| Important | 38 | 100 | 326 | 572 | 4,138 | 53 | 7 |
| Important conditionnel | 0 | 0 | 0 | 2 | 5 | 0 | 0 |
| Modéré | 0 | 0 | 9 | 100 | 713 | 0 | 0 |
| Modéré conditionnel | 0 | 0 | 0 | 3 | 7 | 0 | 0 |
| Faible | 0 | 0 | 0 | 12 | 285 | 1 | 0 |
| Faible conditionnel | 0 | 0 | 0 | 0 | 5 | 0 | 0 |
| Insuffisant | 0 | 15 | 72 | 180 | 580 | 1 | 6 |
| Non précisé | 0 | 0 | 0 | 2 | 14 | 0 | 0 |
| Commentaires | 0 | 0 | 0 | 0 | 2 | 4 | 0 |

**Correlation logic check** (semantic: Important SMR = high ASMR, Insuffisant SMR = ASMR V):

The cross-tab shows the expected inverse relationship — as SMR decreases, ASMR V dominates:
- **SMR Important**: mostly ASMR V (4,138/5,244 = 79%), with notable ASMR IV (572) and III (326). The 38 ASMR I/II cases where SMR is Important are notable outliers — worth investigating if these represent exceptional cases.
- **SMR Modéré**: overwhelmingly ASMR V (713/822 = 87%), with 100 ASMR IV.
- **SMR Faible**: 96% ASMR V (285/298).
- **SMR Insuffisant**: 74% ASMR V (580/780), but 23% spread across ASMR II/III/IV — logically consistent since insufficient SMR can still carry minor improvements.

**Notable anomaly**: 38 cases where SMR is "Important" but ASMR is I. This is unusual — an Important SMR with ASMR I means the drug provides major improvement AND major added benefit. Cross-check these individually if the downstream pipeline assigns decision rules based on the SMR+ASMR combination.

### Duplicate CIS with Multiple CT_IDs

| | SMR | ASMR |
|---|---|---|
| CIS with multiple CT_IDs | 2,116 | 1,848 |
| Max CT_IDs per CIS | 32 | 29 |
| Distribution | skewed 2 (1,410), with long tail | skewed 2 (1,066), with long tail |

Both files show the same pattern: a single drug (one CIS) can have up to 29-32 separate CT reviews over its lifecycle (new indications, renewals, re-evaluations). Sample CIS 65083759 appears in both files with CT-20090 and CT-21773 — same pair, same dates, consistent across files.

### Malformed Row Pattern (Tab-Split Artifacts)

All 18 malformed rows (12 SMR + 6 ASMR) share the same root cause: **embedded tab characters within the avis text field**. When Python's `csv.reader` encounters a `\t` inside a quoted field, it should handle it correctly. However, these rows appear to have been produced by a prior process that did not properly quote the avis field before exporting.

Evidence: All SMR malformed rows contain identical long avis text about PSMA PET imaging (Pluvicto). The tab splits the avis text at different positions in the same drug's avis — row 354 is an earlier portion, row 1188 is a later continuation, row 1189 is the closing sentence `'] ».'`. These are NOT valid rows; they are fragments.

Recommended action: **Flag and exclude rows where field count != 6** during import.

---

## Summary of Issues

| Severity | Issue | Affected |
|----------|-------|---------|
| HIGH | 18 malformed rows (field count != 6) — tab-split avis fragments | 18 rows across both files |
| MEDIUM | HTML `<br>`/`<p>`/`<b>` tags embedded in avis text | 4,031 rows (13% SMR, 21% ASMR) |
| LOW | Future-dated rows (2026) — pre-loaded upcoming reviews | 251 rows total |
| LOW | "Conditionnel" SMR levels — non-standard values | 41 rows (18 Important, 10 Modéré, 13 Faible) |
| INFO | 38 ASMR I/II cases where SMR is "Important" | 38 overlapping pairs |
| INFO | 2,424 duplicate (CIS, CT_ID) in SMR; 292 in ASMR | Logical variants, not errors |
| CLEAN | Null bytes, embedded newlines in avis, CT-0, integer overflow | None found |