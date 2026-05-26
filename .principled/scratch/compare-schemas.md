# BDPM Schema Comparison: External Analyses vs. Our Plan

**Sources**: External analysis 1 (BDPM_Etude_Faisabilite.txt), External analysis 2 (BDPM_Analyse_Technique_Final.txt), External analysis 3 (bdpm_feasibility_body.txt), Our plan (BRIEF.md)

**Date**: 2026-05-26

---

## 1. Table-by-Table Comparison Matrix

| Domain | Our Plan | External 1 (Feasibility) | External 2 (Technical) | External 3 (Feasibility body) |
|--------|----------|------------------------|------------------------|-------------------------------|
| Drug records | `drugs` | `cis_specialites` | `specialites` | `cis_specialites` |
| Presentations | `presentations` | `cis_presentations` | `presentations` | `cis_presentations` |
| Compositions | `compositions` | `cis_compositions` | `compositions` | `cis_compositions` |
| Generic groups | `generic_groups` | `cis_generiques` | `groupes_generiques` | `cis_generiques` |
| Prescription rules | `prescription_rules` | `cis_conditions_prescription` | `conditions_prescription` | `cis_conditions_prescription` |
| SMR ratings | `smr` | `cis_has_smr` | `avis_smr` | `cis_has_smr` |
| ASMR ratings | `asmr` | `cis_has_asmr` | `avis_asmr` | `cis_has_asmr` |
| Availability | `availability` | `cis_disponibilite` | `ruptures_stock` | `cis_disponibilite` |
| ATC codes | `atc_codes` | `cis_mitm` | `medicaments_mitm` | `cis_mitm` |
| HAS links | `has_links` | `has_liens_ct` | `liens_page_ct` | `has_liens_ct` |
| Import log | `import_log` | `import_log` | `import_metadata` | `import_log` |

### Column Naming Conventions

| Column | Our Plan | External 2 | External 3 |
|--------|----------|-----------|-----------|
| Drug identifier | `cis` (PK) | `code_cis` (PK) | `code_cis` (FK+PK) |
| CIP code | `cip` (PK) | `code_cip7` (PK) | `code_cip7` (PK) |
| CIP13 (EAN) | `ean13` | `code_cip13` | `code_cip13` |
| Drug name | `name` | `denomination` | `denomination` |
| Form | `form` | `forme_pharmaceutique` | `forme_pharma` |
| Route | `route` | `voies_administration` | `voies_admin` |
| Auth status | `auth_status` | `statut_amm` | `statut_amm` |
| Procedure type | `procedure_type` | `type_procedure` | `type_proc` |
| Commercial status | `comm_status` | `etat_commercialisation` | `etat_commercialisation` |
| Auth date | `auth_date` | `date_amm` | `date_amm` |
| Alert type | `alert_type` | `statut_bdm` | `statut_bdm` |
| EU number | `eu_number` | `numero_autorisation_europeenne` | `num_auto_euro` |
| Lab name | `lab_name` | `titulaire` | `titulaires` |
| Patent flag | `is_patent` | (absent) | `surveillance_renforcee` |
| Generic group | `generic_group_id` | `id_groupe` | — |
| Generic sort | `generic_sort` | `num_tri` | — |
| Generic type | `generic_type` | `type_generique` (0/1/2/4) | `type_generique` (0/1/2/4) |
| ATC code | `atc_code` | `code_atc` | `code_atc` |
| ATC URL | `atc_url` | `lien_bdpm` | `lien_bdpm` |
| Drug name (ATC) | (in atc_codes) | `denomination` | `denomination` |
| SMR level | `level` | `valeur_smr` | `valeur_smr` (I-V) |
| ASMR level | `level` | `valeur_asmr` | `valeur_asmr` (I-V) |
| avis field | `avis` | `libelle_smr` / `libelle_asmr` | `libelle_smr` / `libelle_asmr` |
| Decision type | `decision_type` | `motif_evaluation` | `motif_eval` |
| CT ID | `ct_id` | `code_dossier_has` | `code_dossier_has` |
| Substance code | `substance_code` | `code_substance` | `designation` |
| Substance name | `substance_name` | `nom_substance` | `denom_substance` |
| Dosage | `dosage` | `dosage` | `ref_dosage` |
| Per unit | `per_unit` | `ref_dosage` | (absent) |
| Pharm code | `pharm_code` | `nature_composant` | `nature_composant` (SA/FT) |
| Form label | `form_label` | `forme_pharmaceutique` | `designation` |
| Seq | `seq` | `numero_liaison` | `num_liaison_sa_ft` |
| Group name | (in generic_groups) | `libelle_groupe` | `libelle_groupe` |
| Group ID | `group_id` | `identifiant_groupe` | `id_groupe` |
| Rule | `rule` | `condition` | `condition` |
| Status type | `status_type` (1-4) | `code_statut` (1-4) | `identifiant_disponibilite` (1-4) |
| avail Status | `status` | `libelle_statut` | `etat_disponibilite` |
| avail Date start | `date_start` | `date_debut` | `date_debut` |
| avail Date end | `date_end` | `date_fin` | `date_maj` |
| avail Date remise | `date_remise` | `date_remise` | `date_maj` |
| avail source_url | `source_url` | `lien_ansm` | `lien_ansm` |
| Import timestamp | `imported_at` | `timestamp` | `date_import` |
| File name | (in import_log) | `file_name` | `fichier` |
| File hash | (in import_log) | `sha256` | `hash_sha256` |
| Row count | (in import_log) | `rows_read` | `nombre_lignes` |
| Import status | (in import_log) | `status` | `status` |

---

## 2. Primary Key Strategy Difference

**Critical conflict.**

| Aspect | Our Plan | External Analyses |
|--------|----------|------------------|
| CIS as PK | TEXT (the code itself) | TEXT (the code itself) — agree here |
| AUTOINCREMENT id | Not used (TEXT CIS primary) | Used for non-CIS tables where appropriate |
| Composite PK in presentations | (cis, cip) | COMPOSITE (cis+cip7) in ext 1; CIP7 alone in ext 2 |
| CIP as PK | CIP alone — zero duplicates confirmed | CIP7 alone — same conclusion |

**External analysis 1** uses a COMPOSITE primary key `(code_cis, code_cip7)` for presentations. **External analysis 2** uses `code_cip7` alone as PRIMARY KEY. Our plan uses `(cis, cip)` as a compound primary key.

**Our plan's `(cis, cip)` composite PK is safer.** The zero-duplicate CIP confirmation applies to CIP alone being a valid PK. However, a compound `(cis, cip)` PK adds defensive correctness — it enforces the business rule that CIP is subordinated to CIS at the schema level. The query ergonomics via `PRIMARY KEY (cis, cip)` mean you need both columns to update/delete, but reads by CIP alone still work via existing index.

**Verdict**: Keep our compound `(cis, cip)` PK. The extra safety outweighs the marginal query complexity.

---

## 3. Price Storage: INTEGER Cents vs REAL Euros

**Critical conflict. Highest priority resolution.**

| Aspect | Our Plan | External Analyses |
|--------|----------|------------------|
| SQLite type | INTEGER (cents) | REAL (euros) |
| Internal representation | Integer cents (2434 = €24.34) | Float euros (24.34) |
| Precision | Exact | Floating-point |
| External recommendation | — | rust_decimal wrapper for precision |
| Display | `format!("{}/{:02}", cents/100, cents%100)` | Float formatting |

External analysis 1 explicitly states: *"Les prix sont representes en f64 avec un wrapper Decimal (crate rust_decimal) pour eviter les problemes de precision en virgule flottante."*

**Our plan is better for this use case.** The BDPM prices are limited to 2 decimal places (French convention comma → decimal). With integer cents:
1. No floating-point surprises (0.1 + 0.2 = 0.30000000000000004 in fp)
2. Easy aggregation (sum, avg) with integer ops
3. No dependency on `rust_decimal` crate complexity
4. Simpler normalization: strip commas → parse as integer → divide by 100 at display

For a full-table-truncate pipeline, integer cents is strictly superior. The external analyses are solving a problem (floating-point imprecision) that matters for arbitrary-precision financial apps, not for a fixed-2-decimal government database.

**Verdict**: **Keep our INTEGER cents design.** Add `DECIMAL` as internal representation note in the schema comments for clarity.

---

## 4. Primary Key Auto-Increment Conflict

| Aspect | Our Plan | External 1 | External 2 |
|--------|----------|------------|------------|
| ROWID AUTOINCREMENT | Not used | Used for 8/11 tables | Used for 8/11 tables |
| Tables with AUTOINCREMENT | 0 | 8 tables: compositions, smr, asmr, generiques, conditions, disponibilite, infos_securite, groups | 8 tables: avis_smr, avis_asmr, groupes_generiques, infos_securite, ruptures_stock |
| TEXT PK tables | `drugs (cis)`, `presentations (cis+cip)`, `atc_codes (atc_code)`, `has_links (ct_id)` | `cis_specialites (code_cis)`, `has_liens_ct (code_dossier_has)`, `cis_mitm (code_cis)` | `specialites (code_cis)`, `presentations (code_cip7)`, `medicaments_mitm (code_cis)` |

External analyses use ROWID AUTOINCREMENT for tables where there is no natural TEXT key (composition_id, smr_id, asmr_id, etc.), then add the CIS code as a separate FK column. In our plan, the natural key IS the identifier (`cis` in drugs, `cis+cip` in presentations, `atc_code` in atc_codes), and the rest use compositenatural keys.

**Our approach is simpler and more semantically correct.** Using TEXT PK for CIS/CIP/ATC/ct_id makes the data self-documenting and avoids the unnecessary integer surrogate key that serves only as a row identifier, not as a meaningful domain key. External analyses introduce 8 unnecessary AUTOINCREMENT columns.

**Verdict**: **Keep our approach.** The AUTOINCREMENT tables in our design use composite natural keys (e.g., `smr: cis+ct_id`, `generic_groups: group_id+cis`) which are more meaningful than synthetic integers.

---

## 5. `_raw` Columns

**Decisive external recommendation we are missing.**

| Implementation | Presence |
|----------------|----------|
| Our plan | Only `cip_raw` in presentations |
| External 1 (Faisabilite) | Explicit principle: "_raw prefix for columns storing value before normalization" |
| External 2 (Technical) | Mentions: "preserve raw values for iterative parser refinement" |
| External 3 (Feasibility body) | Same principle: "_raw for source fidelity" |

External analysis 1 specifically calls out `_import_id` in every table and `_raw` for date/normalization columns. With our current truncate-and-reload pipeline, we **do not retain source fidelity** — the raw values are lost after normalitization.

**However, the value of `_raw` columns is severely reduced by our full-table-truncate design.** If we re-import from the file each time, we don't need `_raw` columns for debugging historical parses. The normalize-on-import strategy means raw values are available in committed-at-import versions of the raw files (in `raw/` archive).

**Verdict**: **Partially adopt.** Keep `cip_raw` in presentations (uniquely valuable — CIP13 normalization loss is irreversible). Add `date_raw` column for `auth_date` in drugs (for far-future date 2924 case), and `prix_raw` in presentations if we ever need to debug pricing normalization. Do NOT add `_raw` to every column — this would double row size for minimal benefit given our truncate pipeline.

---

## 6. `_import_id` Foreign Key in Every Table

**Present in all external analyses. Absent from our plan.**

| Implementation | Our Plan | External 1 | External 2 | External 3 |
|----------------|----------|-----------|-----------|-----------|
| Per-table import FK | NO | YES (every table) | YES (every table) | YES (every table) |
| Implementation | import_log only | `_import_id INT REFERENCES import_log(id)` | `_import_id` (implicit in diagram) | `_import_id` in every table |

External analysis 1: *"chaque table possede une colonne _import_id referencant la table import_log pour la tracabilite."* And: *"PRAGMA foreign_keys = OFF par defaut pour ne pas bloquer l'insertion des references orphelines."*

**`_import_id` is not worth the complexity for our use case.** The key arguments:
1. Our full-table-truncate pipeline means each row's `_import_id` would always reference the same import session (the last one, since we delete before insert)
2. The foreign key FK is orphaned when `PRAGMA foreign_keys = OFF` — the very thing that enables orphan inserts defeats the purpose of the FK
3. The import state is already tracked in `import_log` with row counts, bad_rows, status per file
4. Adding `_import_id` to 11 tables adds 11 columns with marginal investigative value (we know which import loaded a row — it was the most recent one)

**Verdict**: **Do NOT adopt.** The import_log already provides traceability. Our simpler design with a single `import_log` table tracking per-file results is sufficient and cleaner.

---

## 7. `is_orphan` Flag

**Explicitly mentioned in external analysis 1. Absent from our plan.**

External analysis 1 section 6.2: *"Chaque structure inclut un champ _meta contenant l'import_id et un flag is_orphan pour les references orphelines."*

The orphan problem (18.4% of SMR CIS, 15.8% of ASMR CIS, 23.5% of GENER CIS exist without a parent in drugs) is real. External analysis recommends inserting these with `is_orphan=1` rather than rejecting them.

**Our plan handles this without an explicit flag** by using `PRAGMA foreign_keys = OFF` (or by avoiding FK enforcement entirely) and just logging orphan counts per import. The import_log already tracks `bad_rows` and `skipped_rows`, plus we have `referential_integrity` tests that COUNT orphans.

**Verdict**: **Do NOT add explicit `is_orphan` column.** Our approach is equivalent — orphan tracking is done at import-time via test assertions and log counts, not via a per-row boolean that would be always 0 after first import due to truncate+reload. If we ever move to incremental import, re-evaluate.

---

## 8. `is_active` Soft-Delete Column

**External analysis 1 recommends. Our plan uses full-table truncate. Convergent on our choice.**

External analysis 1 section 6.5 (modele d'import incremental): *"La phase de suppression logique marque comme inactifs les enregistrements presents dans la base mais absents du nouveau fichier (soft delete via colonne is_active)."*

External analysis 2 (Technical): Implements row deletion via hash comparison.

All external analyses are designed for **incremental upsert** pipelines. Our plan explicitly rejects incremental imports in favor of **full-table truncate+reload** because: (a) no row-level timestamps exist, (b) 32K row tables reload in seconds, (c) name it what it is.

**For our full-truncate pipeline, `is_active` is irrelevant.** We delete all rows for a table, then re-insert. Historical data is preserved in the archived raw files (`raw/YYYY-MM-DD/` folder), not in soft-delete columns.

**Verdict**: **Do NOT adopt `is_active`.** Our full-truncate+archive-raw approach is correct for this use case. The soft-delete model is strictly for incremental pipelines, which we're not building.

---

## 9. CHECK Constraints

**External analyses have them. Our plan is missing most.**

| Constraint | Our Plan | External 1 | External 2 | External 3 |
|------------|----------|------------|------------|------------|
| `surveillance_renforcee IN ('Oui','Non')` | NO | YES | YES | YES |
| `pharm_code IN ('SA','FT')` | NO (just TEXT) | YES | YES | YES |
| `generic_type IN (0,1,2,4)` | NO | YES | YES | YES |
| `status_type IN (1,2,3,4)` | YES | YES | YES | YES |
| `reimbursable IN ('oui','non')` | NO | NO | CHECK in ext 2 | CHECK in ext 2 |
| EAN13 CHECK (starts with 34009) | COMMENT only | NO | NO | NO |

External analysis 1: *"Chaque colonne de type enumeration recoit une CHECK constraint."*

**Verdict**: **Adopt CHECK constraints.** Add before implementing parsers:
```sql
CHECK (pharm_code IN ('SA', 'FT'))
CHECK (generic_type IN (0, 1, 2, 4))
CHECK (status_type IN (1, 2, 3, 4))
-- Note: reimbursable is TEXT ('oui'/'non') — no CHECK needed if nullable
-- Note: EAN13 should have CHECK (ean13 IS NULL OR ean13 LIKE '34009%')
```

---

## 10. avis Field

**Convergent on HTML stripping.**

All sources agree: avis field (SMR/ASMR assessments) contains HTML `<br>` tags (4,031 rows, 13% SMR, 21% ASMR). All agree to strip on store, preserve text. Our plan is explicit; external analyses are implicit. **Both agree. No conflict.**

 avis max size: 2,018 chars for SMR, 2,019 for ASMR (found by our profiling, confirmed by external analysis 1 implicit validation). Store as `TEXT` (no arbitrary limit needed; SQLite stores `TEXT` as dynamic-length).

---

## 11. Hash Algorithm

**Critical conflict.**

| Aspect | Our Plan | External Analyses |
|--------|----------|------------------|
| Hash function | BLAKE3 | SHA-256 |
| External justification | — | Required and fast in Rust (sha2 crate) |
| Our justification | Faster, modern Rust ecosystem | Well-established standard |

External analysis 1 section 7.1: SHA-256 is recommended. Our plan (SECTION 2.2) uses BLAKE3 explicitly.

**BLAKE3 vs SHA-256**: BLAKE3 is a modern, highly parallelizable hash function (6-8x faster than SHA-256 on multi-core, 3x on single-core for large files). For ~27MB total data, the raw speed difference is negligible (sub-millisecond). Both functions are cryptographically appropriate for change detection.

**Verdict**: **Keep BLAKE3.** It's a valid standard choice. The small ecosystem familiarity difference doesn't outweigh the performance benefit. Document the choice clearly.

---

## 12. Async Stack (reqwest + tokio)

**Critical conflict.**

| Aspect | Our Plan | External Analyses |
|--------|----------|------------------|
| Async runtime | Sync stack (ureq + rusqlite + rouille) | reqwest + tokio (async) |
| Reason | "150K records, zero benefit, +60s compile time, +4MB binary" | Standard for modern Rust HTTP clients |
| Compile time impact | Lower | ~60s higher (tokio) |

Our plan explicitly avoids tokio/async for Phase 1 (fetch + parse). External analyses recommend reqwest+tokio as standard Rust practice.

**Our decision is sound for Phase 1.** The monthly CLI download-and-import pipeline has zero concurrency requirement. The data volume (150K rows, ~27MB total) is small enough that async's concurrency benefits don't apply. The compile-time overhead and binary-size cost are real.

**However, Phase 2 (API server)** will need async if we serve concurrent HTTP requests from a sync SQLite connection pool. External analyses are correct for the eventual API layer — `axum` or `actix-web` are async by design. But we deferred the API to Phase 4.

**Verdict**: **Keep sync stack for Phase 1.** Pre-plan the async transition carefully when Phase 2 begins — the API layer will require rethinking the connection pattern.

---

## 13. Price Normalization

**All sources agree on the problem. Our plan has the most detailed solution.**

The thousands-separator problem (466 rows with values like `1,466,29` where naive ReplaceAll breaks): External analyses identify the problem but propose the naive `replace(',', '.')` fix. Our plan has the correct pattern: **detect 2 commas → remove both commas entirely**.

**Verdict**: Our approach is the only correct one. External analyses underestimate this edge case.

---

## 14. EAN13 Field

**Our plan has it. External analysis 2 has UNIQUE constraint. Our plan has CHECK comment.**

| Aspect | Our Plan | External 2 |
|-------|---------|-----------|
| EAN13 field | `ean13 TEXT` | `code_cip13 TEXT NOT NULL UNIQUE` |
| Validation | CHECK (ean13 IS NULL OR ean13 LIKE '34009%') | No explicit CHECK |
| Uniqueness | 100% start with 34009 (French mandate) | `UNIQUE` on cip13 alone |

External analysis 2 adds `UNIQUE` on `code_cip13` — valid since CIP13 codes are indeed unique per drug presentation.

**Verdict**: Add explicit `UNIQUE` on `ean13`. Our CHECK comment is appropriate but should be formalized as a schema comment or actual CHECK if SQLite supports it (note: SQLite CHECK on LIKE pattern might not be supported; use a trigger or application-level validation).

---

## 15. Missing Tables in Our Plan

| Table (External) | Present in Our Plan | Notes |
|-----------------|---------------------|-------`/`---|
| `update_history` (External 2) | NO | Tracks detection history, source, action. Useful for monitoring. |
| `infos_securite` | NO | CIS_InfoImportantes — deferred to Phase 3.5 per our plan |
| `import_metadata` (External 2) | No separate table | Merged into `import_log` — acceptable |

External analysis 2 splits metadata into two tables: `import_metadata` (per-import static metadata) + `update_history` (change detection log). Our plan merges these into `import_log`. Both are functionally equivalent — our approach has fewer tables.

**Verdict**: Keep our design.

---

## 16. Key Design Decision Differences Summary

| Decision | Our Plan | External Recommendation | Adopt? |
|----------|----------|------------------------|--------|
| Table naming | English / plural nouns | French / singular | **Keep our English naming** — clearer for code |
| Column naming | Concise English | Verbose French | **Keep our naming** — better API ergonomics |
| Primary key for CIS/CIP | TEXT PK (code itself) | TEXT PK (code itself) | **Agreement** — no conflict |
| AUTOINCREMENT ids | None (natural composite keys) | 8 tables use ROWID | **Keep our approach** — more semantically meaningful |
| Price storage | INTEGER cents | REAL euros + rust_decimal | **Keep our INTEGER cents** — exact arithmetic, no extra crate |
| Prices checked to DECIMAL | No | Yes (rust_decimal wrapper) | **Keep integer cents** — simpler |
| `_raw` columns | `cip_raw` only | Every normalized column | **Partial adoption** — keep cip_raw, add date_raw for auth_date |
| `_import_id` FK per table | No | Yes every table | **Do NOT adopt** — import_log is sufficient for our truncate model |
| `is_orphan` flag per row | No — log counts instead | Yes | **Do NOT adopt** — equivalent via test assertions + import_log |
| `is_active` soft delete | No — full truncate | Yes for incremental | **Do NOT adopt** — full-truncate makes it unnecessary |
| CHECK constraints | Minimal | Per enumeration column | **Adopt ALL** — add before parser implementation |
| avis field | TEXT (strip HTML) | TEXT | **Agreement** — all sources agree |
| avis max length | VARCHAR(2048) (no hard limit) | Implicit 2048 | **Keep as TEXT** — no hard limit needed |
| avis HTML stripping | Yes (strip on store) | Yes | **Agreement** |
| Hash function | BLAKE3 | SHA-256 | **Keep BLAKE3** — faster, modern |
| Async/tokio stack | Sync only for v1 | Recommended | **Keep sync for Phase 1** — correct for batch CLI |
| Full-table truncate | Yes (explicit) | No (incremental upsert) | **Keep truncate+archive** — correct decision for data without timestamps |
| EAN13 UNIQUE | No explicit UNIQUE | UNIQUE (code_cip13) | **Adopt UNIQUE constraint** on ean13 column |
| EAN13 CHECK pattern | Comment only | None | **Adopt application-level validation** (pattern check in ingestion code, not SQL) |

---

## 17. Final Verdict

### Adopt FROM External Analyses

1. **CHECK constraints** on all enumeration columns (`pharm_code IN ('SA','FT')`, `generic_type IN (0,1,2,4)`, `status_type IN (1,2,3,4)`)
2. **EAN13 UNIQUE constraint** on the ean13 column (adds DB-level enforcement of a known invariant)
3. **Partial `_raw` columns**: keep `cip_raw`, add `date_raw` for `auth_date` (for far-future date edge case)

### Reject FROM External Analyses

1. **INTEGER AUTOINCREMENT ids** — our natural composite keys are semantically superior
2. **`_import_id` in every table** — import_log is sufficient
3. **`is_orphan` flag per row** — solved at import time via assertions
4. **`is_active` soft delete** — unnecessary with full truncated design
5. **SHA-256** — keep BLAKE3
6. **tokio/reqwest async stack** — keep sync for Phase 1
7. **rust_decimal** — unnecessary with integer cents
8. **Incremental import model** — full truncate is correct for our use case
9. **Verbose French column names** — keep our concise English equivalents

### Unchanged in Our Plan

- BLAKE3 hash (keep)
- Sync stack for Phase 1 (keep)
- Full table truncate + raw file archive (keep)
- INTEGER cent prices (keep)
- English table/column names (keep)
- Composite natural keys (keep)
- avis HTML stripping (keep)
- Dual sync schedule (monthly + weekly for Dispo) (keep)

---

## 18. Recommended Schema Tweak (CHECK constraints to add)

Insert at the beginning of the schema section in BRIEF.md:

```sql
-- CHECK constraints (to be added to each table)
-- On presentations: CHECK (reimbursable IS NULL OR reimbursable IN ('oui', 'non'))
-- On compositions:  CHECK (pharm_code IN ('SA', 'FT'))
-- On generic_groups: CHECK (generic_type IN (0, 1, 2, 4))
-- On availability:  CHECK (status_type IN (1, 2, 3, 4))
-- On smr:           CHECK (level IN ('Important', 'Modéré', 'Faible', 'Insuffisant',
--                                   'Insuffisant à HAS', 'Pas d\'avis disponible',
--                                   'Légèrement important'))
-- On asmr:          CHECK (level IN ('I', 'II', 'III', 'IV', 'V',
--                                   'III bis', 'IV bis', 'V bis'))
```
