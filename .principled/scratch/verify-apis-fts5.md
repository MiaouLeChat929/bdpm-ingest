# Verification Report: APIs, FTS5, and Parsing Order

Date: 2026-05-26

---

## 1. API Status: api-medicaments.fr

**Status: DOWN (not reachable)**

```
curl exit code: 6 (connection failed)
HTTP GET https://api.medicaments.fr/: exit code 6
```

The API is not responding. This contradicts the documentation in `08_apis_communautaires.md` which claimed it was "Actif" with "2 fois par jour" updates.

**Impact**: Cannot use this as a change-detection signal per the roadmap's Phase 3 monitoring plan.

---

## 2. API Status: api-bdpm-graphql.axel-op.fr

**Status: ERROR (503/500)**

```
HTTP HEAD: 503 Service Unavailable
GraphQL POST: 500 Server Error
```

This matches the documentation in `08_apis_communautaires.md` which already noted "Statut: 503 Service Unavailable (mai 2026)".

**Impact**: Cannot study GraphQL patterns via live API. The GitHub source at https://github.com/axel-op/api-bdpm-graphql remains available for reference.

---

## 3. FTS5 Corruption Patterns

### The Bug

FTS5 with `content=` and `content_rowid=` causes SQLite corruption:

```sql
-- ANTI-PATTERN (causes corruption):
CREATE VIRTUAL TABLE drugs_fts USING fts5(
    drug_name,
    content='drugs',
    content_rowid='id'
);
```

This pattern caused `database disk image is malformed` errors during synchronization in the Python implementation.

### The Fix

Standalone FTS5 without `content=` — manual synchronization:

```sql
-- CORRECT PATTERN:
CREATE VIRTUAL TABLE drugs_fts USING fts5(drug_name);
-- After each import:
-- INSERT INTO drugs_fts SELECT drug_name FROM drugs;
```

### Key Finding

The Python implementation already hit this bug and documented it. Our Rust Phase 2 implementation must:
1. Use standalone FTS5 virtual tables
2. Do NOT use `content=` attribute
3. Sync explicitly after each import via `INSERT INTO fts SELECT ...`
4. Document the `content=` anti-pattern explicitly

Source: `.principled/scratch/critique-architecture-pipeline.md` and `.principled/scratch/consolidated-audit.md`

---

## 4. Parsing Order Discrepancy

### External Recommendation (07_roadmap_implementation.md lines 58-69)

```
1. HAS_LiensPageCT_bdpm.txt (2 cols, ASCII pure)
2. CIS_MITM.txt (4 cols, cp1252 simple)
3. CIS_CPD_bdpm.txt (2 cols, cp1252)
4. CIS_InfoImportantes.txt (4 cols, UTF-8 + HTML)
5. CIS_GENER_bdpm.txt (5 cols, cp1252)
6. CIS_HAS_SMR_bdpm.txt (6 cols, cp1252)
7. CIS_HAS_ASMR_bdpm.txt (6 cols, cp1252)
8. CIS_COMPO_bdpm.txt (8 cols, cp1252)
9. CIS_CIP_Dispo_Spec.txt (8 cols, latin-1)
10. CIS_bdpm.txt (12 cols, cp1252)
11. CIS_CIP_bdpm.txt (13 cols, UTF-8 + decimal commas)  ← CIP LAST
```

### Our Plan (from 05_pipeline_transformation.md lines 487-501)

```
1. specialites (CIS_bdpm.txt)
2. presentations (CIS_CIP_bdpm.txt)  ← CIP SECOND
3. compositions (CIS_COMPO_bdpm.txt)
4. has_liens_ct (HAS_LiensPageCT_bdpm.txt)
5. avis_smr (CIS_HAS_SMR_bdpm.txt)
6. avis_asmr (CIS_HAS_ASMR_bdpm.txt)
7. groupes_generiques (CIS_GENER_bdpm.txt)
8. conditions_prescription (CIS_CPD_bdpm.txt)
9. disponibilites (CIS_CIP_Dispo_Spec.txt)
10. mitm (CIS_MITM.txt)
11. infos_importantes (CIS_InfoImportantes.txt)
```

### Why CIP Last in External Plan

The external plan is **parsing-first**: start with the simplest files to validate the pipeline before tackling complex ones. CIP is last because:
- 13 columns (most complex)
- UTF-8 with decimal commas requiring special handling
- Can validate against already-parsed tables

### Why CIP Second in Our Plan

Our plan is **import-first**: CIS_bdpm (specialites) establishes the central reference table first, then CIP (presentations) as the first dependent table. This makes validation queries meaningful earlier.

### Analysis

Both approaches have merit:
- **External approach**: Better for debugging parsing issues (pipeline stabilizes on easy files first)
- **Our approach**: Better for data validation (can query "orphans" after each import step)

The discrepancy is not a bug — it's a different optimization goal. The external plan optimizes for **parsing reliability**; our plan optimizes for **data integrity validation**.

**Recommendation**: Keep our order. The import validation checks in Phase 4 are more valuable with our ordering. The parsing complexity of CIP is manageable since it comes after CIS_bdpm establishes the data patterns.

---

## Summary

| Item | Status | Action |
|------|--------|--------|
| api-medicaments.fr | DOWN | Remove from monitoring signals; check GitHub for alternative |
| api-bdpm-graphql | 503/500 | Rely on GitHub source only |
| FTS5 content= | Corruption risk | Must use standalone FTS5 in Phase 2 |
| Parsing order | Different goals | Keep current order (data integrity > parsing debugging) |