# Form and Route Canonicalization Analysis

**Date:** 2026-05-29
**Database:** /home/devadmin/Desktop/BDMP_DB/data/bdpm.db

---

## 1. Database Inventory

| Metric | Value |
|---|---|
| Total drugs with `form` | 14,363 |
| Total drugs with `route` | 14,363 |
| Unique form values | 347 |
| Unique route values | 153 |

---

## 2. Top Forms (by drug count)

| Rank | Form | Count |
|---|---|---|
| 1 | comprimé pelliculé | 2,888 |
| 2 | solution injectable | 1,281 |
| 3 | comprimé | 1,270 |
| 4 | gélule | 1,122 |
| 5 | comprimé pelliculé sécable | 679 |
| 6 | comprimé sécable | 674 |
| 7 | solution pour perfusion | 271 |
| 8 | solution à diluer pour perfusion | 270 |
| 9 | solution buvable | 269 |
| 10 | poudre et solvant pour solution injectable | 231 |

**Top 50 forms cover:** ~14,000 drugs (virtually all drugs have a form)

---

## 3. Top Routes

| Rank | Route | Count |
|---|---|---|
| 1 | orale | 9,261 |
| 2 | intraveineuse | 1,461 |
| 3 | sous-cutanée | 602 |
| 4 | cutanée | 528 |
| 5 | ophtalmique | 301 |
| 6 | inhalée | 295 |
| 7 | intramusculaire | 215 |
| 8 | intramusculaire;intraveineuse | 195 |
| 9 | intraveineuse;sous-cutanée | 167 |
| 10 | voie buccale autre | 143 |

**Multi-value routes exist:** e.g., `intramusculaire;intraveineuse` (195 drugs), `infiltration;péridurale;périneurale` (24 drugs)

---

## 4. FORM_CANONICAL Coverage Analysis

### What FORM_CANONICAL maps

The `phf_map` in `src/normalize/fields.rs` (~50 entries) maps lowercase ASCII French to short codes:

| Canonical Code | Keys covered | Approximate drugs |
|---|---|---|
| CPR | comprimé, comprimé enrobé | rare (keys use ASCII "e") |
| GEL | gelule, capsule molle | ~1,262 |
| INJ | solution injectable, solution pour injection | ~1,512 |
| SOL | solution buvable, sirop, suspension buvable | ~436 |
| COL | collyre, solution pour instillation | ~232 |
| SUP | suppositoire, suppo | ~54 |
| POM | pommade, creme, gel, pommade dermatologique | ~376 |
| PATCH | patch, dispositif transdermique | ~259 |
| GRAN | granules, globules | ~38 |
| POW | poudre, poudre pour solution buvable | ~522 |
| INJ_POW | poudre pour solution injectable, poudre injectable | ~417 |

### Accent mismatch: FORM_CANONICAL is broken by design

`canonicalize_form()` does `form.trim().to_lowercase()` which produces `"comprimé pelliculé"` (accent retained). The `FORM_CANONICAL` keys are ASCII (`"comprime pellicule"`, `"comprime enrobe"`, etc.). There is **zero overlap** — every key in FORM_CANONICAL will fall through.

Result: `canonicalize_form()` returns the lowercased raw string for every form in the database (e.g., `"comprimé pelliculé"` → `"comprimé pelliculé"`), making the map functionally useless.

### Coverage measurement (corrected for accent issue)

Using the corrected query that strips accents before matching:

- **Covered by FORM_CANONICAL (approximate):** ~5,983 drugs (41.7%)
- **Fall through:** ~8,380 drugs (58.3%)

The top-3 forms (`comprimé pelliculé` 2888, `comprimé` 1270, `gélule` 1122) are NOT in the map because:
1. The map uses ASCII `e` for `é`
2. Many variants (e.g., `comprimé pelliculé sécable`, `comprimé`) are missing entirely

### Gaps in FORM_CANONICAL (top forms not covered)

| Form | Count | Reason |
|---|---|---|
| comprimé pelliculé | 2,888 | ASCII key vs accented DB value |
| comprimé | 1,270 | Missing from map |
| gélule | 1,122 | ASCII key vs accented DB value |
| comprimé pelliculé sécable | 679 | Missing from map |
| comprimé sécable | 674 | Missing from map |
| solution pour perfusion | 271 | Missing from map |
| solution à diluer pour perfusion | 270 | Missing from map |
| poudre et solvant pour solution injectable | 231 | Missing from map |
| comprimé à libération prolongée | 211 | Missing from map |
| gélule à libération prolongée | 185 | Missing from map |
| gélule gastro-résistant(e) | 171 | Missing from map |

---

## 5. ROUTE_CANONICAL Coverage Analysis

### ROUTE_CANONICAL keys

The map in `src/normalize/fields.rs` (~20 entries) covers:

| Canonical | Keys | Drugs covered |
|---|---|---|
| orale | orale, voie orale, usage oral | 9,261 |
| cutanee | cutanee, usage cutane, dermique | 528 |
| rectale | rectale, voie rectale, usage rectal | 102 |
| vaginale | vaginale, voie vaginale, usage vaginal | 71 |
| inhalation | inhalation, usage inhalation, pour inhalation | 296 |
| oculaire | oculaire, voie oculaire, usage ophtalmique | 0 (DB uses `ophtalmique`) |
| auriculaire | auriculaire, voie auriculaire | 15 |
| sublinguale | sublinguale, voie sublinguale | 136 |
| transdermique | transdermique, dispositif transdermique | 133 |

### Coverage measurement

- **Covered by ROUTE_CANONICAL:** ~9,659 drugs (67.2%)
- **Fall through:** ~4,704 drugs (32.8%)

### Gaps in ROUTE_CANONICAL

| Route | Count | Issue |
|---|---|---|
| intraveineuse | 1,461 | NOT in map |
| sous-cutanée | 602 | NOT in map (key uses `cutanee`) |
| ophtalmique | 301 | NOT in map (map has `oculaire`) |
| inhalée | 295 | NOT in map (accent vs map's `inhalation`) |
| intramusculaire | 215 | NOT in map |
| voie buccale autre | 143 | NOT in map |
| Multi-value routes | ~500+ | NOT handled (split or map) |

### Multi-value routes: `;`-delimited combinations

The database stores multiple routes joined by `;`:
- `intramusculaire;intraveineuse` (195)
- `intraveineuse;sous-cutanée` (167)
- `intramusculaire;intraveineuse;sous-cutanée` (31)
- `infiltration;péridurale;périneurale` (24)
- `orale;sublinguale` (30)

These will never match ROUTE_CANONICAL since no key contains a semicolon.

---

## 6. Wiring: Are Canonicalization Functions Called in Ingest?

**Answer: NO.**

### Evidence

1. **`src/import/mod.rs`**: The ingest pipeline calls `normalize_row()` and `normalize_apostrophes()` only. `canonicalize_form` and `canonicalize_route` are **never imported or called**.

2. **Drugs table schema** (`src/db/schema.sql`): No `form_canonical` or `route_canonical` columns exist. The table has only `form` and `route` columns storing the raw normalized values.

3. **Drugs FTS trigger**: `drugs_fts` is populated from the raw `form` and `route` columns, not from any canonical values.

4. **`src/normalize/fields.rs`** exports `canonicalize_form`, `canonicalize_route`, and `canonicalize_lab` as public functions, but they are only tested (unit tests in `mod tests`) and never used outside that file.

### The canonicalization code is dead code

The functions exist, have unit tests, but have never been wired into the ingest pipeline. Running `ingest` today produces the same output as before the Phase 05 canonicalization work was done.

---

## 7. Accented Forms

French accented forms are stored as-is in the database:

```
comprimé pelliculé, gélule, poudre et solvant pour solution injectable,
comprimé à libération prolongée, gélule gastro-résistant(e),
collyre en solution, solution à diluer pour perfusion
```

These are correct — the database stores raw normalized forms. The problem is FORM_CANONICAL keys use ASCII, creating a mismatch.

---

## 8. Summary Findings

| Finding | Severity |
|---|---|
| FORM_CANONICAL is broken due to accent mismatch (ASCII keys vs accented DB values) | HIGH |
| FORM_CANONICAL covers only ~41.7% of drugs | HIGH |
| ROUTE_CANONICAL covers ~67.2% of drugs | MEDIUM |
| `canonicalize_form`/`canonicalize_route` are dead code — never called in ingest | HIGH |
| Multi-value routes (`;`-delimited) not handled at all | MEDIUM |
| No `form_canonical`/`route_canonical` columns in schema | MEDIUM |
| `ophtalmique` in DB vs `oculaire` key in ROUTE_CANONICAL | LOW |
| `inhalée` in DB vs `inhalation` key in ROUTE_CANONICAL | LOW |
| `sous-cutanée` in DB vs `cutanee` in ROUTE_CANONICAL | LOW |
| `intraveineuse`, `intramusculaire` missing from ROUTE_CANONICAL | MEDIUM |

---

## 9. Recommendations

1. **Fix FORM_CANONICAL accent handling**: Either (a) use `strip_diacritics()` before lookup in `canonicalize_form()`, or (b) add accented keys directly to the map (simpler, no dependency on unicode-normalization at lookup time).

2. **Wire canonicalization into ingest**: Add `form_canonical` and `route_canonical` columns to `drugs` table, call `canonicalize_form()`/`canonicalize_route()` during CIS_bdpm import.

3. **Extend FORM_CANONICAL**: Add missing top forms — `comprimé pelliculé`, `comprimé`, `gélule`, `comprimé sécable`, `solution pour perfusion`, `comprimé à libération prolongée`, etc.

4. **Extend ROUTE_CANONICAL**: Add `intraveineuse`, `sous-cutanée`, `ophtalmique`, `inhalée`, `intramusculaire`, `voie buccale autre`.

5. **Handle multi-value routes**: Split on `;` and canonicalize each segment, or map the whole string if the combination has semantic meaning.

6. **Add FTS-friendly combined index**: Store both raw and canonical in FTS5 for flexible search (e.g., filter by `CPR` code OR search free text).
