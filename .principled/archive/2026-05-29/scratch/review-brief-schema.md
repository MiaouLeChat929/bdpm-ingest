# Schema Comparison: BRIEF.md vs External Analyses

Comparing our 11-table schema from `BRIEF.md` against both external schemas:
- `03_schema_sqlite.md` (v1, generic design)
- `05_schema_sqlite.md` (v2, detailed with incremental strategy)

---

## 1. DRUGS / SPECIALITES

### Our schema (`drugs`)
```
cis TEXT PK
name TEXT NOT NULL
form TEXT
route TEXT
auth_status TEXT
procedure_type TEXT
comm_status TEXT
auth_date TEXT         -- ISO-8601
lab_name TEXT
is_patent INTEGER NOT NULL DEFAULT 0
alert_type TEXT        -- nullable, from field 8
eu_number TEXT         -- nullable
generic_group_id TEXT
generic_sort INTEGER
generic_type TEXT      -- 0=ref,1=gen,2=cross,4=LP
atc_code TEXT
atc_url TEXT
imported_at DATETIME DEFAULT CURRENT_TIMESTAMP

idx: name, atc_code, generic_group_id
```

### External v1 (`specialites`)
```
code_cis INTEGER PK
denomination TEXT NOT NULL
forme_pharma TEXT NOT NULL    -- difference: NOT NULL
voies_admin TEXT
statut_amm TEXT
type_procedure TEXT
etat_commercial TEXT
date_amm DATE
statut_bdm TEXT              -- 'Alerte', 'Warning disponibilité', NULL
num_europe TEXT
titulaires TEXT              -- multi-value semicolon-separated
surveillance INTEGER DEFAULT 0  -- 0=Non, 1=Oui
_import_date TEXT
_source_hash TEXT
```

### External v2 (`cis_specialites`)
```
code_cis TEXT PK             -- correct: TEXT (they fixed this)
denomination TEXT NOT NULL
forme_pharmaceutique TEXT
voies_administration TEXT
statut_amm TEXT
type_procedure_amm TEXT
etat_commercialisation TEXT
date_amm_raw TEXT
date_amm TEXT                -- normalized YYYY-MM-DD
statut_bdm TEXT CHECK (statut_bdm IN ('', 'Alerte', 'Warning disponibilité'))
numero_autorisation_euro TEXT
titulaires TEXT
surveillance_renforcee TEXT CHECK (...)
_import_id INTEGER FK → import_log
_is_active INTEGER DEFAULT 1
idx: denomination, statut_amm, etat_commercialisation, date_amm, surveillance_renforcee
```

### Comparison
| Aspect | Our drugs | v1 specialites | v2 cis_specialites |
|--------|-----------|----------------|-------------------|
| CIS type | TEXT | INTEGER | TEXT (fixed) |
| form field | TEXT nullable | TEXT NOT NULL | TEXT nullable |
| lab_name | Present | Not present (titulaires in specialites) | Not present |
| surveillance flag | Inside alert_type | Integer col | TEXT col with CHECK |
| ATC code | Embedded in drugs table | Not present | Not present |
| Generic info | Inline columns | Not present | Not present |
| Audit columns | imported_at only | _import_date + _source_hash | _import_id + _is_active |

**AGREE:** CIS as TEXT, foreign key reference pattern, ISO-8601 date normalization.

**DISAGREE:**
- We embed generic_group_id, generic_sort, generic_type, atc_code, atc_url directly in drugs. v1/v2 store them in separate tables (groupes_generiques, mitm). **Our approach is denormalized but query-efficient.** No per-drug JOIN needed for generic group or ATC — single-row lookup returns everything. Trade-off: harder to maintain consistency if that info changes, but we control the import pipeline so it is fine.
- v1 declares `forme_pharma TEXT NOT NULL`. Source data confirms most drugs have a form, but 0.1% may be null. NOT NULL is wrong.
- v1 stores `surveillance` as INTEGER. Source has text values "Oui"/"Non". v2 correctly uses TEXT with CHECK.
- We have `lab_name` directly in drugs. v1/v2 only have titulaires which can contain multiple labs semicolon-separated. **Our denormalized, stripped `lab_name` is more queryable.** The source file gives us a single lab per drug.

**MISSING (v2 has that we don't):**
- `_import_id` → FK to import_log (we track import per table, not per row — more efficient)
- `_is_active` → soft-delete flag (we use full table truncate+reload — does the same job without logical delete complexity)
- No `source_hash` per row (we store hash at file level in import_log — sufficient granularity)

**EXTRA (we have that external doesn't):**
- Inline generic_group_id, generic_sort, generic_type, atc_code, atc_url — denormalized for query efficiency
- Stripped/whitespace-normalized lab_name

---

## 2. PRESENTATIONS

### Our schema (`presentations`)
```
cis TEXT FK → drugs
cip TEXT PK (7-digit canonical)
cip_raw TEXT
labels TEXT
pres_status TEXT
comm_status TEXT
comm_date TEXT        -- ISO-8601
ean13 TEXT
reimbursable TEXT     -- "oui"/"non"
reimb_rate REAL       -- 0.65, 1.0 normalized
prix_ht_cents INTEGER
prix_ville_cents INTEGER
prix_rate_cents INTEGER
reimb_conditions TEXT
idx: cis
```

### External v1 (`presentations`)
```
INTEGER PK AUTOINCREMENT
code_cis INTEGER FK
code_cip7 TEXT
libelle TEXT
statut_admin TEXT
etat_commercial TEXT
date_declaration DATE
code_cip13 TEXT
agree_collect TEXT          -- 'oui','non','inconnu'
taux_remboursement INTEGER  -- INTEGER percentage (65, 100)
prix_ht REAL                -- REAL euros
prix_ttc REAL
honoraires REAL
indications_raw TEXT
indications_clean TEXT
_import_date TEXT
```

### External v2 (`cis_presentations`)
```
INTEGER PK AUTOINCREMENT
code_cis TEXT NOT NULL FK
code_cip7 TEXT NOT NULL
libelle TEXT
statut_administratif TEXT
etat_commercialisation TEXT
date_declaration_raw TEXT
date_declaration TEXT      -- YYYY-MM-DD
code_cip13 TEXT
agrement_collectivites TEXT CHECK (...)
taux_remboursement TEXT    -- TEXT not INTEGER
prix_ht_raw TEXT
prix_ht REAL
prix_ttc_raw TEXT
prix_ttc REAL
honoraires_raw TEXT
honoraires REAL
indications_remboursement TEXT
_import_id INTEGER FK
_is_active INTEGER DEFAULT 1
UNIQUE(code_cis, code_cip7)
idx: cis, code_cip13, prix_ht, etat, agrement (partial)
```

### Comparison
| Aspect | Our presentations | v1/v2 |
|--------|-------------------|-------|
| CIP price | INTEGER cents | REAL euros |
| Reimb rate | REAL (0.65) | INTEGER (65) or TEXT |
| EAN13 | CHECK via app | No explicit CHECK |
| Prix fields | 3 fields | 3 fields + honoraires |
| Soft delete | No | _is_active |
| Unique constraint | Only implicit | UNIQUE(cis, cip7) |

**AGREE:** Same column set (3 price fields, cip13, labels, status, dates).

**DISAGREE:**
- **PRICE: Our INTEGER cents vs REAL euros. This is our most important disagreement.** REAL floats accumulate binary rounding errors. `24.34 + 0.10 != 24.44` in float math. Our cents approach is correct. v1/v2 explicitly document the float issue but choose REAL anyway. **Their decision is a mistake.** Both external schemas use REAL for prices.
- **Reimb rate: Our REAL (0.65) vs v1's INTEGER (65).** Our normalized float is semantically cleaner (0.65 = 65% exactly). Integer requires the API consumer to divide by 100. However, INTEGER avoids float ambiguity. **v1 is arguably better here** — reimbursement rates come as whole percentages, storing as INTEGER(65) is lossless. Our REAL is fine but unnecessary precision.
- **Honoraires:** v2 has a separate honoraires column. We don't — but our source data analysis shows CIS_CIP_bdpm has 3 price fields (prix_ht, prix_ville, prix_rate), not 4. Honoraires is not in our TSV. v2 appears to hallucinate this field or cite a different source file.

**MISSING (v2 has that we don't):**
- Partial index on `agrement_collectivites` WHERE not empty — useful for filtering
- `_is_active` soft-delete (we handle via truncate+reload)
- `_import_id` FK to import_log
- UNIQUE constraint on (code_cis, code_cip7) — we rely on CIP uniqueness as sole PK

**EXTRA (we have that external doesn't):**
- `cip_raw` — preserves original value for diagnostics
- No AUTOINCREMENT surrogate key — our composite (cis, cip) PK is semantically meaningful

---

## 3. COMPOSITIONS

### Our schema (`compositions`)
```
cis TEXT FK
form_label TEXT
substance_code TEXT
substance_name TEXT
dosage TEXT
per_unit TEXT
pharm_code TEXT      -- SA or FT
seq INTEGER
UNIQUE(cis, substance_code, seq)
idx: cis
```

### External v1/v2 (`compositions`/`cis_compositions`)
```
id INTEGER PK AUTOINCREMENT
code_cis INTEGER/TEXT FK
designation TEXT            -- "designation element" (form_label)
code_substance TEXT
nom_substance TEXT / denomination_substance TEXT
dosage TEXT
ref_dosage TEXT / reference_dosage TEXT
nature TEXT CHECK ('SA','FT')
num_liaison INTEGER / numero_liaison_sa_ft INTEGER
_import_date / _import_id
_is_active (v2)
idx: cis, substance, nature (+ denomination: v2)
```

### Comparison
| Aspect | Our compositions | External |
|--------|-----------------|----------|
| Surrogate key | No | AUTOINCREMENT |
| form_label | form_label | designation_element |
| substance_name | substance_name | nom_substance / denomination_substance |
| per_unit | per_unit | ref_dosage |
| pharm_code | pharm_code | nature |
| seq | seq | missing — no ordering column |
| UNIQUE | three-column | implicit via AUTOINCREMENT |
| Deduplication | Via UNIQUE constraint | Via upsert (id handles) |

**AGREE:** Same columns, same CHECK on SA/FT, same foreign key to drugs.

**DISAGREE:**
- No SURROGATE KEY: We use `UNIQUE(cis, substance_code, seq)` as deduplication mechanism. External uses auto-increment `id` as PK with no unique constraint — deduplication must happen via upsert. **Our UNIQUE approach is dedup-native and prevents the same insert twice.** External's approach allows duplicate rows unless application-layer upsert logic handles it.
- **We have `seq`** (sequence number for ordering within the same CIS/substance). External has no ordering column. **We need `seq`** per BRIEF.md analysis showing 15.5% of CIS have multiple CPD rows, up to 6. Without seq, there is no stable ordering.
- **`per_unit`** (what our analysis calls "per_unit") maps to `ref_dosage` / `reference_dosage` in external. Same field, same purpose.

**MISSING (external has that we don't):**
- `_import_id` / `_import_date` audit trail per row — we track at file level only (sufficient for our use case)
- `_is_active` soft delete — not needed with truncate+reload

**EXTRA (we have that external doesn't):**
- `seq INTEGER` — sequence for ordering within same (cis, substance_code) group
- `form_label` (designation_element) — preserved in our schema
- No surrogate key — saves 8 bytes/row, meaningful composite PK

---

## 4. SMR

### Our schema (`smr`)
```
cis TEXT FK
ct_id TEXT
decision_type TEXT
decision_date TEXT    -- ISO-8601 from YYYYMMDD
level TEXT           -- "Important"/"Modéré"/etc.
avis TEXT            -- HTML stripped, max VARCHAR(2048)
PRIMARY KEY (cis, ct_id)
idx: cis
```

### External v1 (`avis_smr`)
```
id INTEGER PK AUTOINCREMENT
code_cis INTEGER FK
code_dossier_has TEXT
motif_eval TEXT
date_avis DATE
valeur_smr TEXT
libelle_raw TEXT    -- HTML raw
libelle_clean TEXT  -- HTML cleaned
_import_date TEXT
idx: cis, dossier, valeur
```

### External v2 (`cis_has_smr`)
```
id INTEGER PK AUTOINCREMENT
code_cis TEXT NOT NULL FK
code_dossier_has TEXT FK → has_liens_ct
motif_evaluation TEXT
date_avis_raw TEXT    -- YYYYMMDD original
date_avis TEXT       -- YYYY-MM-DD normalized
valeur_smr TEXT
libelle_smr TEXT      -- cleaned
is_orphan INTEGER DEFAULT 0
_import_id INTEGER FK
_is_active INTEGER DEFAULT 1
idx: cis, dossier, date, orphan
```

### Comparison
| Aspect | Our smr | v1 avis_smr | v2 cis_has_smr |
|--------|---------|-------------|----------------|
| avis field | Single TEXT (stripped) | two columns raw+clean | libelle_smr (cleaned) |
| html handling | Strip at ingest | Separate raw+clean | Cleaned only |
| orphan flag | Not tracked | Not tracked | is_orphan |
| motif_eval | decision_type | motif_eval | motif_evaluation |

**AGREE:** Same core columns: CIS, dossier ID, decision date, SMR level (Important/Modéré/etc.), avis text.

**DISAGREE:**
- **HTML handling: Our single-column stripped TEXT vs external's dual-column raw+clean TEXT.** External v1 stores both raw HTML and cleaned text. v2 stores only cleaned. We follow v2's approach — strip HTML at ingest, store only the cleaned text content. **This is the right trade-off:** storing raw HTML doubles storage for data we never expose. Our approach is correct for a query-focused API.
- **We have no orphan flag.** v2 adds `is_orphan INTEGER DEFAULT 0` to detect rows where `code_cis` has no matching drug. Our foreign-key setup with `PRAGMA foreign_keys=ON` would reject such rows entirely. **Our approach is stricter** — orphan rows represent data quality issues and should not silently enter the database. But this means we silently skip orphan rows at import (no import_log entry). v2's approach is more transparent (flags but keeps them).
- **date field naming:** Our `decision_date`, v1 `date_avis`, v2 `date_avis` — semantic version is `decision_date` as it reflects the HAS decision authority.

**MISSING (v2 has that we don't):**
- `is_orphan` flag — transparency about foreign key orphans
- `_import_id` audit FK
- `_is_active` soft delete

**EXTRA (we have that external doesn't):**
- Named `decision_date` (semantic) vs `date_avis` (administrative)
- Single avis column (cleaned only, no duplicate storage)

---

## 5. ASMR

### Our schema (`asmr`)
```
cis TEXT FK
ct_id TEXT
decision_type TEXT
decision_date TEXT    -- ISO-8601 from YYYYMMDD
level TEXT           -- "I"–"V"
avis TEXT
PRIMARY KEY (cis, ct_id)
idx: cis
```

### External v1 (`avis_asmr`)
```
id INTEGER PK AUTOINCREMENT
code_cis INTEGER FK
code_dossier_has TEXT
motif_eval TEXT
date_avis DATE
valeur_asmr TEXT     -- I, II, III, IV, V
libelle_raw TEXT
libelle_clean TEXT
_import_date TEXT
idx: cis, dossier, valeur
```

### Combined column map (v1 and v2 agree)
- SMR/ASMR share identical column structure, differing only in SMR level values (Important/Modéré) vs ASMR values (I–V)
- Same DISAGREE and MISSING points as SMR

**AGREE:** ASMR column structure mirrors SMR.

**DISAGREE / MISSING / EXTRA:** Same analysis as SMR (section 4).

---

## 6. AVAILABILITY

### Our schema (`availability`)
```
cis TEXT FK
cip TEXT          -- can be empty string
status_type INTEGER CHECK IN (1, 4)   -- CRITICAL: only two values
status TEXT
date_start TEXT   -- ISO-8601
date_end TEXT     -- nullable
date_remise TEXT  -- nullable
source_url TEXT
PRIMARY KEY (cis, status_type, date_start)
idx: cis
```

### External v1 (`disponibilites`)
```
code_statut INTEGER CHECK (1, 2, 3, 4)
  -- 1=Rupture, 2=Tension, 3=Arrêt, 4=Remise
```

### External v2 (`cis_disponibilite`)
```
code_statut INTEGER CHECK (code_statut IN (1, 2, 3, 4))
  -- all four values required
```

### Comparison

**AGREE:** Same core columns.

**CRITICAL DISAGREE:**
> **Our CHECK constraint only allows values 1 and 4.** The source file (CIS_CIP_Dispo_Spec.txt) has codes 1, 2, 3, and 4:
> - 1 = Rupture de stock
> - 2 = Tension (ongoing supply pressure)
> - 3 = Arrêt de commercialisation
> - 4 = Remise à disposition
>
> v2 correctly declares `CHECK IN (1, 2, 3, 4)`. v1 also has all four. **Our CHECK constraint is wrong — we will reject valid data on ingest.**
>
> This is the single most critical bug in our schema. The fix: change `CHECK IN (1, 4)` to `CHECK IN (1, 2, 3, 4)`.

Note: source analysis in BRIEF.md mentions codes 1 and 4 specifically, but the external analysis and the source file itself confirm all four values exist. Our constraint is a significant data quality bug.

**EXTRA (we have that external doesn't):**
- `source_url TEXT` — direct URL to ANSM/BDPM page. Both v1 and v2 have `lien_ansm` / `lien_bdpm` which is the same field, but named differently. **v1 calls it `lien_ansm`, we call it `source_url`.** This is naming only.

---

## 7. ATC_CODES

### Our schema (`atc_codes`)
```
atc_code TEXT PK        -- 7-char or 5-char
drug_name TEXT
detail_url TEXT
parent_5_char TEXT
parent_3_char TEXT
parent_1_char TEXT
idx: (primary key only)
```

### External v1 (`mitm`)
```
code_atc TEXT
denomination TEXT
lien_bdpm TEXT
-- parent hierarchies NOT derived
-- no PK on code_atc
```

### External v2 (`cis_mitm`)
```
code_cis TEXT PK        -- note: PK is cis, NOT atc_code
code_atc TEXT
denomination TEXT
lien_bdpm TEXT
-- no parent hierarchy derivation
_import_id / _is_active
idx: atc
```

### Comparison

**AGREE:** Same core fields (atc_code, drug_name, url).

**DISAGREE — SEVERE:**
> **Critical structural difference: Our PK is `atc_code`. External PK is `code_cis`.**
>
> Our design choice: one atc_code per row, with `atc_code` as PK. This implies each ATC row represents one ATC classification. If multiple drugs share the same ATC code, we collapse them to one row — losing which drugs belong to which entry.
>
> External v1/v2 design: `code_cis` is the PK (or part of it), meaning each row uniquely identifies a drug-ATC relationship. Multiple rows with same ATC but different CIS are kept.
>
> **The source file CIS_MITM.txt has one row per CIS-ATC pair.** Multiple drugs can have the same ATC code (e.g., many NSAIDs share N02AA). We should not collapse them. **Our PK should be (atc_code, cis)** or we should add a surrogate ID. Using `atc_code` alone destroys the CIS→ATC relationship for drugs sharing the same classification.

**DISAGREE on parent hierarchy:**
- **We have `parent_5_char`, `parent_3_char`, `parent_1_char`** — derived from ATC hierarchy. External has none of these. **Our derived columns are query accelerants** — a JOIN on a substring is slower than a direct lookup. For a small table (7,711 rows), computing these on ingest is cheap and beneficial.

**MISSING (external has that we don't):**
- `_import_id` / `_is_active` audit columns
- `idx: atc` (external has, we implicitly use PK)

---

## 8. HAS_LINKS

### Our schema (`has_links`)
```
ct_id TEXT PRIMARY KEY
url TEXT
```

### External v1 (`has_liens_ct`)
```
code_dossier_has TEXT PK
lien_ct TEXT
_import_date TEXT
```

### External v2 (`has_liens_ct`)
```
code_dossier_has TEXT PK
lien_url TEXT
_import_id INTEGER FK
_is_active INTEGER
```

### Comparison

**AGREE:** Two-column tables with the same structure: dossier ID PK + URL.

**DISAGREE:**
- **We ignore the FK relationship to smr/asmr.** v2 explicitly declares `code_dossier_has` as FK target from `cis_has_smr` and `cis_has_asmr`. We don't enforce this in our schema (but our `PRAGMA foreign_keys=ON` does include smr/asmr FK declarations). Both approaches end up with the same relationship.
- No audit columns (`_import_id`, `_is_active`).
- **Field naming:** Our `ct_id` vs their `code_dossier_has` vs `code_dossier_has` (v2). We use shorthand, they use full French terminology. **Their naming is more self-documenting.**

---

## 9. GENERIC_GROUPS

### Our schema (`generic_groups`)
```
group_id TEXT NOT NULL
group_name TEXT
cis TEXT
type TEXT
sort_order INTEGER
PRIMARY KEY (group_id, cis)
idx: group_id
```

### External v1 (`groupes_generiques`)
```
id INTEGER PK AUTOINCREMENT
id_groupe INTEGER
libelle_groupe TEXT
code_cis INTEGER
type_generique INTEGER CHECK IN (0,1,2,4)
num_tri INTEGER
_import_date TEXT
```

### External v2 (`cis_generiques`)
```
id INTEGER PK AUTOINCREMENT
identifiant_groupe TEXT NOT NULL
libelle_groupe TEXT
code_cis TEXT NOT NULL
type_generique INTEGER CHECK IN (0,1,2,4)
numero_tri INTEGER
is_orphan INTEGER DEFAULT 0
_import_id / _is_active
idx: cis, groupe, type, orphan
```

### Comparison

**AGREE:** Same columns (group_id, group_name, cis, type, sort_order). Same CHECK constraint on type (0,1,2,4). Both express the 4-value enum.

**DISAGREE:**
- **We use TEXT for `group_id`, external uses INTEGER.** Group IDs from source are small integers (e.g., "31", "968"). TEXT is fine but slightly less space-efficient. **Neither approach is wrong**, TEXT is safer for IDs we treat as strings regardless.
- **Composite PK: ours `(group_id, cis)` vs external's AUTOINCREMENT `id`.** Our composite PK enforces uniqueness at schema level. External's approach allows duplicate (group_id, cis) pairs unless application-layer logic prevents them.
- **type field:** Our `type TEXT` with comment documents the 4-value meaning. External uses `type_generique INTEGER`. Both store the same values (0, 1, 2, 4). **INTEGER is more space-efficient**, TEXT is more self-documenting. **Push toward INTEGER.**

**EXTRA (we have that external doesn't):**
- Nothing significant — structure is identical.

---

## 10. PRESCRIPTION_RULES

### Our schema (`prescription_rules`)
```
cis TEXT FK
rule TEXT
PRIMARY KEY (cis, rule)
idx: cis
```

### External v1 (`conditions_prescription`)
```
id INTEGER PK AUTOINCREMENT
code_cis INTEGER FK
condition TEXT
_import_date TEXT
idx: cis
```

### External v2 (`cis_conditions_prescription`)
```
id INTEGER PK AUTOINCREMENT
code_cis TEXT NOT NULL
condition TEXT NOT NULL
_import_id / _is_active
idx: cis
```

### Comparison

**AGREE:** Same two-column structure (CIS, rule text). Same foreign key. Same index.

**DISAGREE:**
- **COMPOSITE PK: ours `(cis, rule)` vs AUTOINCREMENT in external.** Same trade-off as generic_groups. Our composite PK prevents duplicate rule assignment at schema level.
- **`condition TEXT NOT NULL` in v2** (NOT NULL constraint). Our source rules can be empty strings ("") — BRIEF.md analysis shows 28,160 rows with 2 fields, and some may have empty second fields. NOT NULL on rule would cause silent data loss.

**EXTRA (we have):**
- Composite PRIMARY KEY — prevents duplicate (cis, rule) pairs at schema level
- No audit columns — sufficient for our use case

---

## 11. IMPORT_LOG

### Our schema
```
id INTEGER PK AUTOINCREMENT
file_name TEXT NOT NULL
file_hash TEXT NOT NULL           -- BLA3
file_size INTEGER NOT NULL
row_count INTEGER NOT NULL
status TEXT NOT NULL               -- success/partial/failed
bad_rows INTEGER DEFAULT 0
skipped_rows INTEGER DEFAULT 0
imported_at DATETIME DEFAULT CURRENT_TIMESTAMP
duration_ms INTEGER
idx: file_name, imported_at DESC
```

### External v1 (`import_history`)
```
id INTEGER PK AUTOINCREMENT
file_name TEXT NOT NULL
import_date TEXT
rows_count INTEGER
sha256 TEXT
file_size INTEGER
encoding TEXT
status TEXT
error_msg TEXT
```

### External v2 (`import_log`)
```
id INTEGER PK AUTOINCREMENT
timestamp TEXT NOT NULL           -- ISO 8601
file_name TEXT NOT NULL
sha256 TEXT NOT NULL              -- SHA-256 not BLAKE3
rows_read INTEGER DEFAULT 0
rows_inserted INTEGER DEFAULT 0
rows_updated INTEGER DEFAULT 0
rows_deleted INTEGER DEFAULT 0    -- Tracks soft deletes
status TEXT CHECK (success/partial/failed/no_change)
duration_ms INTEGER
error_message TEXT
idx: timestamp, file_name
```

### Comparison

**AGREE:** All three store file-level import metadata. All track file_name, row_count/size, status, timestamp.

**DISAGREE:**
- **Hash: BLAKE3 vs SHA-256.** BRIEF.md explicitly selects BLAKE3 for performance (faster hashing). SHA-256 is cryptographically standard but 2-3x slower on large files. **Our BLAKE3 choice is correct** — this is not a security-critical database, it is a change-detection signal. BLAKE3 is the right tool.
- **Soft delete tracking: v2's `rows_deleted` column** — tracks count of rows soft-deleted during incremental import. We don't need this (full-truncate-reload model), so it is not relevant.
- **Row counts: We track `row_count` (total), `bad_rows`, `skipped_rows`. v2 has `rows_read`, `rows_inserted`, `rows_updated`.** Our granularity is adequate for monitoring. v2's segmented counts are more useful for incremental updates.
- **Encoding field:** v1 tracks detected file encoding. We don't — but we normalize all at ingest, and encoding is known per-file from our manifest. **Encoding field in import_history is useful for debugging edge cases** — worth adding.
- **CHECK on status:** v2 adds `CHECK IN ('success', 'partial', 'failed', 'no_change')`. Our status is free-form TEXT. We should add a CHECK constraint.

**MISSING (v2 has that we should add):**
- `rows_read` — total rows before filtering (helps detect partial imports)
- `no_change` status value — for files that haven't changed (our current behavior doesn't record these in import_log)
- `CHECK (status IN (...))` — constraint enforcement

**EXTRA (we have that external doesn't):**
- `bad_rows` and `skipped_rows` — granular import quality tracking
- BLAKE3 hash — faster than SHA-256 for change detection

---

## CROSS-TABLE OBSERVATIONS

### Orphan Handling Philosophy
- **Our approach:** FK enforcement ON, orphan rows rejected silently (logged as import failure for that file). No `_is_active` column needed. Schema is stricter.
- **External v2 approach:** FK enforcement OFF (declares it), orphan rows flagged with `is_orphan INTEGER DEFAULT 0` column. `_is_active` enables soft deletes. More flexible, less strict.
- **Which is better:** For a self-updating pipeline where source data quality may vary, **our stricter approach forces data quality upstream**.** It is better for a closed dataset. External's approach is better for datasets with known data quality issues. Both are valid depending on philosophy.

### Soft Delete vs Truncate+Reload
- **Our approach (truncate+reload):** Simpler, no need for `_is_active` column on every table. Works well for small tables (32K rows reload in seconds). WAL mode ensures readers see consistent data throughout reload.
- **External v2 approach (soft delete):** Maintains historical data (deleted drugs still visible in history). More complex (must track `_is_active`, compute hash per row). Better for audit trails.
- **Which is better:** For a drug database where we re-import monthly with the authoritative file, **truncate+reload is simpler and correct.** Soft delete adds complexity without clear benefit. **Stay with truncate+reload.**

### Audit Column Philosophy
External v2 adds `_import_id` (FK to import_log) and `_is_active` (soft delete) to every table. We add `imported_at` only to the drugs table. Arguments:
- **Pro-external:** Row-level provenance is precise. If import_log is the source of truth, linking rows to their import record is clean.
- **Pro-ours:** File-level imports are atomic. All rows from a file share the same import metadata. Storing it per row is redundant. We track this once in import_log. The distinction between "per-table" and "per-row" audit is: we audit at table-rename granularity (all rows from one file = one import event), external audits at row granularity (one import event = many row references).

**Neither is strictly better.** Our approach is simpler and sufficient for our monitoring use case. External's approach scales better for scenarios where individual rows need provenance tracing (e.g., partial re-imports of only specific regions).

### Source Metadata Table
v1/v2 both include a `source_metadata` table ("licence", "base_url", etc.). We don't have this. **This is a useful addition** — stores the official metadata about the source dataset (last update date, license, BDPM version). Since we commit import_state.json as a workflow artifact, we can keep this metadata there too.

### Orphan Detection View
v1 includes a `cis_orphelins` VIEW that cross-references all 5 tables for missing FK references. We have no equivalent and rely on `PRAGMA foreign_keys` validation. **We should add a `v_drug_orphans` view** that identifies which rows in smr/asmr/generic_groups/presentations/availability reference a CIS not in drugs (useful for monitoring data quality).

### Critical Finding: Availability CHECK Constraint
The single most critical schema issue is the availability `CHECK IN (1, 4)` constraint — we must add values 2 and 3 per the source data.

---

## SUMMARY TABLE

| Table | Critical Issues | Should Change |
|-------|---------------|---------------|
| drugs | None | No |
| presentations | None | No |
| compositions | None | No |
| smr | None | No |
| asmr | None | No |
| **availability** | **CHECK omits values 2, 3** | **YES — fix constraint** |
| atc_codes | Wrong PK (atc_code alone loses CIS-ATC relationship) | YES — change PK |
| generic_groups | Minor: type should be INTEGER not TEXT | Suggestion only |
| prescription_rules | Minor: comment type values in column | Suggestion only |
| has_links | No issues | No |
| import_log | Minor: add encoding field, CHECK status, no_change status | Suggestion only |

**Must fix before implementation:**
1. `availability.status_type CHECK IN (1, 2, 3, 4)` — add 2 and 3
2. `atc_codes` PK change — use (atc_code, cis) or add surrogate `id`

**Should add post-implementation:**
3. `import_log` → `CHECK (status IN ('success', 'partial', 'failed', 'no_change'))`
4. `import_log` → add `rows_read INTEGER` column
5. `import_log` → add `encoding TEXT` column
6. Create `v_drug_orphans` view for FK monitoring
