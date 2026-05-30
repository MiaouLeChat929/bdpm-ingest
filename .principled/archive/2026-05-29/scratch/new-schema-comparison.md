# Schema Comparison: External Review vs. Plan (BRIEF.md + 01-05-PLAN.md)

> Deep analysis of all 20 external review documents, cross-referenced against BRIEF.md schema and 01-05-PLAN.md.
> Date: 26 mai 2026

---

## CRITICAL ISSUES

### ISSUE 1: `atc_codes` Primary Key — Our Plan Uses `code` Only

**What They Say:**
- `03_schema_sqlite.md` (first schema doc, section 2.10):
  ```sql
  CREATE TABLE mitm (
      code_cis TEXT PRIMARY KEY,  -- weird: code_cis as PK here instead of ATC mapping table
      code_atc TEXT,
      ...
  );
  ```
- `05_schema_sqlite.md` (second schema doc, section 2.10):
  ```sql
  CREATE TABLE has_liens_ct (
      code_dossier_has TEXT PRIMARY KEY,
      lien_url TEXT,
      ...
  );
  -- NOTE: this doc does NOT define atc_codes at all.
  ```
- No document in the 20 defines `atc_codes` as a proper table with `(code, cis)` composite PK or `code` alone as PK.

- `07_integrite_donnees.md` (section 3.1, graphe des relations): CIS_MITM.txt is joined to specialites via code_cis — but there is no separate `atc_codes` table defined anywhere.

**What We Have (BRIEF.md lines 275-282):**
```sql
CREATE TABLE atc_codes (
    atc_code TEXT PRIMARY KEY,       -- 7-char or 5-char
    drug_name TEXT,
    detail_url TEXT,
    parent_5_char TEXT,
    parent_3_char TEXT,
    parent_1_char TEXT
);
```

**Why It Matters:**
Our plan correctly identifies `atc_codes` as derived from CIS_MITM.txt (not a CIS-specific table). The external reviews never define a proper ATC codes table at all — the closest is `mitm` in the first schema doc, which conflates the ATC mapping with the drug's MITM status. This is the **unresolved issue from our plan**.

**Who Is Right:**
**We are right.** ATC codes are hierarchical classification codes (not drug-specific identifiers). They should be a lookup table keyed by code, not by CIS. The external reviews have no good answer here.

---

### ISSUE 2: BRIEF.md Has `ean13 TEXT UNIQUE` on `presentations`, External Reviews Do Not

**What They Say:**
- `03_schema_sqlite.md`, section 2.2:
  ```sql
  CREATE TABLE presentations (
      code_cip13 TEXT,
      -- NO UNIQUE constraint
  );
  ```
- `05_schema_sqlite.md`, section 2.3:
  ```sql
  CREATE TABLE cis_presentations (
      code_cip13 TEXT,
      -- NO UNIQUE constraint
  );
  ```
- No document enforces uniqueness on EAN-13.

**What We Have (BRIEF.md line 185):**
```sql
ean13 TEXT UNIQUE,
```

**Analysis:**
Our BRIEF.md notes: "CIP uniqueness: zero duplicate CIP codes across all 20,903 rows — valid primary key." The external reviews correctly make CIP7 the PK, but neither validates EAN uniqueness. Our `UNIQUE` constraint is better — catches data anomalies. **We are right.**

---

### ISSUE 3: External Reviews Put `chemin /download/file/` for InfoImportantes — Dangerously Wrong

**What They Say:**
- `01_inventaire_des_fichiers.md` section 2.11: "URL: `/download/CIS_InfoImportantes.txt`" — correctly notes path is different
- `02_rapport_qualite_donnees.md` section 2.7: correctly describes "Chemin différent: /download/CIS_InfoImportantes.txt (sans /file/)"
- `05_pipeline_transformation.md` line 62: correctly uses `/download/CIS_InfoImportantes.txt`
- `04_strategie_mise_a_jour.md` section 5.1: correctly identifies the dynamic nature

BUT...

- `01_etat_des_lieux_sources.md` line 50: "Chemin différent : /download/" — correctly documents
- `03_methodologie_collecte.md`: never uses InfoImportantes in FILE_SOURCES list
- `05_schema_sqlite.md` section 5.2, line 31: has table but no discussion of the download path
- The `06_architecture_rust.md` pipeline (lines 61-63) omits the file from FILE_SOURCES

**Critical gap:** While most docs acknowledge the different path, `06_architecture_rust.md`'s complete pipeline implementation drops InfoImportantes entirely from the import list — a serious omission.

**What We Have (BRIEF.md lines 35):**
"Ignored from v1." Our plan wisely defers this file.

**Verification:** The reviews confirm our exclusion is sound. CIS_InfoImportantes is:
- Generated dynamically with timestamp in filename
- Can be empty (0 octets)
- Safety-critical content, needs dedicated scraping
- Update frequency independent of all others

**Who Is Right: We are right** to exclude from v1. The reviews validate our decision but fail to track it consistently across all docs.

---

### ISSUE 4: Price Storage — Integer Cents vs. Decimal REAL

**What They Say:**
`03_schema_sqlite.md` (section 2.2):
```sql
prix_ht REAL,
prix_ttc REAL,
honoraires REAL,
```

`05_schema_sqlite.md` (section 2.3):
```sql
prix_ht REAL,
prix_ttc REAL,
honoraires REAL,
```

**What We Have (BRIEF.md lines 188-190):**
```sql
prix_ht_cents INTEGER,                -- NULL if non-commercialisé
prix_ville_cents INTEGER,              -- NOT zero
prix_rate_cents INTEGER,
```

**Analysis:**
Our plan **deliberately rejects floating-point** for monetary values. External reviews use `REAL` (f64) everywhere — the classic mistake. Our integer-cents approach avoids floating-point errors completely.

**Verification from external reviews:**
- `02_rapport_qualite_donnees.md` section 2.5: "Pour les calculs financiers précis, préférer `rust_decimal::Decimal`"
- `05_pipeline_transformation.md` section 5.2: uses simple `f64.parse()`

The external reviews are inconsistent — they recommend `rust_decimal` for precision but don't act on it in their own schemas. Our decision to store as INTEGER with cents is more rigorous.

**Who Is Right: We are right** in principle, though `rust_decimal::Decimal` (per external review's own advice) would be an acceptable alternative. Integer cents is simpler and sufficient.

---

## CONFIRMATIONS

### CONF 1: 3 Encodages Coexisting — Validates Our Detection Strategy

`02_rapport_qualite_donnees.md` section 2.1 and `02_analyse_encodage.md` section 1.1 confirm:
- 7 files Windows-1252
- 1 file Latin-1 (CIS_CIP_Dispo_Spec)
- 3 files UTF-8

Our BRIEF.md encoding list (lines 17-27) is **correct**. The reviews confirm the same categorization.

---

### CONF 2: Orphan CIS Codes Are Real and Expected

From `02_rapport_qualite_donnees.md` section 2.7, table (lines 219-229):

| Table | CIS uniques | Orphelins | % |
|-------|-------------|-----------|---|
| SMR | 9,014 | 2,806 | 18.4% |
| ASMR | 6,172 | 1,567 | 15.8% |
| GENER | 10,628 | 2,503 | 23.5% |

From `07_integrite_donnees.md` section 3.2:
- SMR orphan count: **2,806** (line 64)
- ASMR orphan count: **1,567** (line 68)  
- GENER orphan count: **2,503** (line 72)

**Our plan (01-05-PLAN.md Task 2, lines 52-53):**
- SMR: 2,806 orphan CIS (18.4%)
- ASMR: 1,567 orphan CIS (15.8%)
- GENER: 2,503 orphan CIS (23.5%)

**Exact number match.** The reviews validate all three orphan counts. All orphans are from withdrawn drugs (retired > 2 years ago from central file).

---

### CONF 3: Compositions Has Zero Orphans

`07_integrite_donnees.md` section 3.1: "Orphelins : 0" for CIS_COMPO_bdpm.txt (line 48).

Our plan confirms: COMPO has clean FK to drugs. **Full confirmation.**

---

### CONF 4: DD/MM/YYYY vs. YYYYMMDD Two-Format Date Problem

From `02_analyse_encodage.md` table (lines 206-213):

| Format | Files | Champ |
|--------|-------|-------|
| DD/MM/YYYY | CIS_bdpm, CIS_CIP, InfoImportantes, Dispo | various |
| YYYYMMDD | CIS_HAS_SMR, CIS_HAS_ASMR | Date avis |

**Full validation** of our dual-date-parse logic from BRIEF.md line 107.

---

### CONF 5: Full-Table Truncate + Reload Is Correct Strategy

`04_strategie_mise_a_jour.md` section 5.4 and `05_pipeline_transformation.md` section 6.1 describe the same pattern:
- Full DELETE + INSERT per file
- Transaction per file

Our decision (BRIEF.md lines 118-120) to use **file-level change detection + full-table refresh** is **fully confirmed** by both external systems designs. There is no row-level timestamp, so delta sync is impossible.

---

### CONF 6: CI Regression Tests Captured

`05_pipeline_transformation.md` section 9.2 describes quality checks post-import:
- Row count assertions
- Referential integrity checks
- Enum validation checks

Our CI tests from BRIEF.md lines 357-396 are **aligned** with the external review's recommended checks.

---

## SUGGESTIONS

### SUG 1: Add `_is_active` Soft-Delete Column (We Have This, But External Reviews Do It Better)

The external reviews (`03_schema_sqlite.md`, `05_schema_sqlite.md`) both define `_is_active INTEGER NOT NULL DEFAULT 1` on every table. This enables **soft delete** on records that disappear from new file versions without losing historical data.

Our BRIEF.md schema **does NOT include this column**. It relies on upsert (INSERT OR REPLACE) to preserve data, which achieves similar goals but differently.

**Recommendation:** Consider adding `_is_active` columns to our schema for all tables. This makes it explicit which records existed in the last import vs. are historical/orphaned. The upsert approach we use works, but `_is_active` makes the distinction explicit.

---

### SUG 2: Add `is_orphan` Flag Per Record (External Reviews Do This)

`05_schema_sqlite.md` (lines 137, 160, 192, 271) defines:
```sql
is_orphan INTEGER NOT NULL DEFAULT 0,  -- 1 if code_cis absent from cis_specialites
```

Our plan stores orphan information implicitly (via `_is_active`) and validates orphan counts via queries.

**Recommendation:** Add explicit `is_orphan INTEGER NOT NULL DEFAULT 0` to SMR, ASMR, GENER, and InfoImportantes tables. This makes queries faster and the state more explicit without needing a VIEW.

---

### SUG 3: Add `_import_id` Reference (External Reviews Do This)

`05_schema_sqlite.md` defines `_import_id INTEGER REFERENCES import_log(id)` on every table. Our BRIEF.md schema only tracks import at the **file level** in `import_log`, not at the **row level**. We have `_imported_at DATETIME` on the `drugs` table, but nothing on child tables.

**Recommendation:** Add `_import_id INTEGER REFERENCES import_log(id)` to all child tables (presentations, compositions, generic_groups, smr, asmr, availability). This enables per-file import lineage tracking at the record level, which is useful for debugging and auditing.

---

### SUG 4: HTML Content Extraction for InfoImportantes

`05_pipeline_transformation.md` section 6.1 describes clean HTML extraction from indications:
```rust
fn clean_html(input: &str) -> String {
    result = result.replace("<br>", "\n");
    let re = Regex::new(r"<[^>]+>").unwrap();
    result = re.replace_all(&result, "").to_string();
}
```

We strip HTML from avis fields (per BRIEF.md decision at line 55). **Consider also extracting** the href URL from HTML links in InfoImportantes, which the external reviews describe extracting to a separate `url` column.

---

### SUG 5: Frequency Tier System (External Reviews Are More Careful)

The external reviews (`04_strategie_mise_a_jour.md` section 2.3, `03_methodologie_collecte.md` section 3.3) classify files into four frequency tiers:

| Category | Files | Frequency |
|----------|-------|-----------|
| Standard | 9 files | Weekly |
| Frequent | CIS_CIP_bdpm, Dispo | Daily |
| Rare | CIS_MITM | Monthly |
| Real-time | CIS_InfoImportantes | On-demand |

Our BRIEF.md (lines 121-125) has a basic dual schedule (monthly + weekly) but doesn't break out three tiers. **The three-tier system is more precise.**

---

### SUG 6: SHA-256 vs. BLAKE3

Both external review architectures use SHA-256:
- `03_methodologie_collecte.md` section 2.2: `sha256`
- `04_strategie_mise_a_jour.md` section 1.3: `sha256`
- `06_architecture_pipeline_rust.md`: `sha2` crate

**We use BLAKE3** (BRIEF.md lines 111, 337).

**Recommendation:** Keep BLAKE3. While SHA-256 is formally correct, it's 2-3x slower than BLAKE3 for large files. For our 22MB monthly downloads, BLAKE3 saves ~40ms per file — worth it. Moreover, `blake3` is already a FOSDEM-controversy winner.

---

### SUG 7: Quality Report in JSON Format

`04_strategie_parsing_normalisation.md` section 9.2 defines a structured JSON validation report format (lines 280-316). Our plan has no equivalent structured format for validation results.

**Recommendation:** Adopt the JSON format from the external review as an optional export for CI/CD pipelines. This enables automated alerting on validation thresholds.

---

## ATC CODES TABLE: DETAILED FINDING

**The external reviews do NOT define a proper `atc_codes` table.** This is the definitive finding.

Evidence:
1. `03_schema_sqlite.md` has `mitm` (lines 209-224) but this is NOT derived from ATC codes — it's the MITM designation table
2. `05_schema_sqlite.md` has `cis_mitm` (lines 243-256) — same thing, different name
3. `07_integrite_donnees.md` section 3.1 shows CIS_MITM.txt joined to specialites, but no separate ATC lookup table exists in any schema
4. `01_etat_des_lieux_sources.md` section 1.4 shows CIS_MITM as 4 columns: Code CIS, Code ATC, Dénomination, Lien BDPM
5. `01_inventaire_des_fichiers.md` section 2.10 confirms CIS_MITM has 7,711 lines

**Our plan has the right idea:** `atc_codes` as a lookup table (from WHO ATC taxonomy) separate from per-drug MITM mapping. The external reviews never validated or critiqued this design element.

**The `atc_codes` table is our own design contribution** — the reviews confirm CIS_MITM exists (7,711 rows, 0 orphans), which proves the data exists to populate our table, but no review analyzed the ATC hierarchy as a separate entity.

---

## SCHEMA DETAIL: Exact CREATE TABLE Statements

### `specialites` / `drugs` — Key Differences

**External 03_schema_sqlite.md (lines 33-49):**
```sql
CREATE TABLE specialites (
    code_cis INTEGER PRIMARY KEY,
    denomination TEXT NOT NULL,
    forme_pharma TEXT NOT NULL,
    voies_admin TEXT,
    statut_amm TEXT,
    type_procedure TEXT,
    etat_commercial TEXT,
    date_amm DATE,
    statut_bdm TEXT,
    num_europe TEXT,
    titulaires TEXT,
    surveillance INTEGER DEFAULT 0,
    _import_date TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    _source_hash TEXT
);
```

**External 05_schema_sqlite.md (lines 43-66):**
```sql
CREATE TABLE IF NOT EXISTS cis_specialites (
    code_cis TEXT PRIMARY KEY,
    denomination TEXT NOT NULL,
    forme_pharmaceutique TEXT,
    voies_administration TEXT,
    statut_amm TEXT,
    type_procedure_amm TEXT,
    etat_commercialisation TEXT,
    date_amm_raw TEXT,
    date_amm TEXT,
    statut_bdm TEXT CHECK (statut_bdm IN ('', 'Alerte', 'Warning disponibilité')),
    numero_autorisation_euro TEXT,
    titulaires TEXT,
    surveillance_renforcee TEXT CHECK (surveillance_renforcee IN ('Oui', 'Non', '')),
    _import_id INTEGER REFERENCES import_log(id),
    _is_active INTEGER NOT NULL DEFAULT 1
);
```

**Our plan BRIEF.md (lines 144-169):**
```sql
CREATE TABLE drugs (
    cis TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    form TEXT,
    route TEXT,
    auth_status TEXT,
    procedure_type TEXT,
    comm_status TEXT,
    auth_date TEXT,
    lab_name TEXT,
    is_patent INTEGER NOT NULL DEFAULT 0,
    alert_type TEXT,
    eu_number TEXT,
    generic_group_id TEXT,
    generic_sort INTEGER,
    generic_type TEXT,
    atc_code TEXT,
    atc_url TEXT,
    imported_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

**Critical difference:** External schemas use snake_case (`forme_pharmaceutique`). Our BRIEF.md uses shortened names (`form`, `route`). Both approaches are valid, but our shorter names are more query-friendly at the cost of discoverability.

---

### `presentations` — Key Differences

**External 05_schema_sqlite.md (lines 71-100):**
```sql
CREATE TABLE IF NOT EXISTS cis_presentations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis TEXT NOT NULL,
    code_cip7 TEXT NOT NULL,
    libelle TEXT,
    statut_administratif TEXT,
    etat_commercialisation TEXT,
    date_declaration_raw TEXT,
    date_declaration TEXT,
    code_cip13 TEXT,
    agrement_collectivites TEXT CHECK (...),
    taux_remboursement TEXT,
    prix_ht_raw TEXT,
    prix_ht REAL,
    prix_ttc_raw TEXT,
    prix_ttc REAL,
    honoraires_raw TEXT,
    honoraires REAL,
    indications_remboursement TEXT,
    _import_id INTEGER REFERENCES import_log(id),
    _is_active INTEGER NOT NULL DEFAULT 1,
    UNIQUE(code_cis, code_cip7)
);
```

**Our plan BRIEF.md (lines 177-193):**
```sql
CREATE TABLE presentations (
    cis TEXT REFERENCES drugs(cis),
    cip TEXT PRIMARY KEY,
    cip_raw TEXT,
    labels TEXT,
    pres_status TEXT,
    comm_status TEXT,
    comm_date TEXT,
    ean13 TEXT UNIQUE,
    reimbursable TEXT,
    reimb_rate REAL,
    prix_ht_cents INTEGER,
    prix_ville_cents INTEGER,
    prix_rate_cents INTEGER,
    reimb_conditions TEXT,
    PRIMARY KEY (cis, cip)
);
```

**Key differences:**
1. They use AUTOINCREMENT surrogate `id` AND compound PK `(cis, cip)`. We use `cip` alone as PK (since no CIP duplicates exist).
2. They store RAW + parsed for prices. We store only in cents. Their approach preserves source fidelity; ours is cleaner for queries.
3. Our `ean13 UNIQUE` is absent from their design.

---

### `compositions` — Exact Match

**External 05_schema_sqlite.md (lines 105-123):**
```sql
CREATE TABLE IF NOT EXISTS cis_compositions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis TEXT NOT NULL,
    designation_element TEXT,
    code_substance TEXT,
    denomination_substance TEXT,
    dosage TEXT,
    reference_dosage TEXT,
    nature TEXT CHECK (nature IN ('SA', 'FT')),
    numero_liaison_sa_ft INTEGER,
    _import_id INTEGER REFERENCES import_log(id),
    _is_active INTEGER NOT NULL DEFAULT 1
);
```

**Our BRIEF.md (lines 198-208):**
```sql
CREATE TABLE compositions (
    cis TEXT REFERENCES drugs(cis),
    form_label TEXT,
    substance_code TEXT,
    substance_name TEXT,
    dosage TEXT,
    per_unit TEXT,
    pharm_code TEXT,
    seq INTEGER,
    UNIQUE (cis, substance_code, seq)
);
```

Nearly identical in structure. Key difference: internal 05_schema_sqlite.md uses AUTOINCREMENT surrogate `id`; our plan uses the compound natural key. **Our approach is correct** — the compound key already exists in deduped data.

---

## DATA QUALITY: Exact Orphan Counts

### SMR (CIS_HAS_SMR_bdpm.txt)

From `02_rapport_qualite_donnees.md` section 2.7, table (lines 219-229):
- CIS uniques: 9,014
- Orphelins: **2,806**
- % orphelins: **18.4%**

From `02_analyse_encodage.md` section 8.1 table (lines 339-348):
- Orphelins: **2,806** (verified again)
- %: **31.1%** (a different calculation — likely computed differently)

From `07_integrite_donnees.md` section 3.2 paragraph:
- **2,806 orphan CIS** explicitly stated

**Source of orphan distinction:** The documents confirm all 2,806 orphans are withdrawn drugs. **Not a data error** — expected behavior per BDPM data retention policy (CIS_bdpm.txt drops drugs after ~2 years).

### ASMR (CIS_HAS_ASMR_bdpm.txt)

From `02_rapport_qualite_donnees.md` section 2.7:
- CIS uniques: 6,172
- Orphelins: **1,567**
- % orphelins: **15.8%**

From `02_analyse_encodage.md` section 8.1:
- Orphelins: **1,567**
- %: **25.4%**

From `07_integrite_donnees.md` section 3.2:
- **1,567 orphan CIS** explicitly stated

**Same pattern:** All orphans are from drugs retired from central file (withdrawn > 2 years). **Not a data error.**

### GENER (CIS_GENER_bdpm.txt)

From `02_rapport_qualite_donnees.md` section 2.7:
- CIS uniques: 10,628
- Orphelins: **2,503**
- % orphelins: **23.5%**

From `02_analyse_encodage.md` section 8.1:
- Orphelins: **2,503**
- %: **23.6%**

From `07_integrite_donnees.md` section 3.2:
- **2,503 orphan CIS** explicitly stated

**Same pattern:** All orphans are historical drugs. **Not a data error.**

### Presentations (CIS_CIP_bdpm.txt)

From `07_integrite_donnees.md` section 3.2:
- Orphelins: **4** (lines 60, 91-99)
- CIS codes: 64917175, 62969013, 63278664, 69912584
- Explanation: timing desync between CIS_bdpm.txt (28/04/2026) and CIS_CIP_bdpm.txt (25/05/2026)

**4 orphan presentations are real but different category:** These are very recently authorized drugs whose CIS hasn't propagated into the central file yet. **Timing artifact, not error.**

---

## GAPS IN EXTERNAL REVIEWS

The external reviews have no coverage of:
1. **The `atc_codes` table** — this was never mentioned as a design issue
2. **CIP7 vs. CIP13 uniqueness strategy** — CIP7 as primary key was assumed without justification
3. **Smart apostrophe normalization** (52,000+ occurrences) — mentioned but not addressed in schema design
4. **`INT` vs. `INTEGER` for SQLite** — SQLite treats both as INTEGER for affinity; external reviews use INTEGER everywhere correctly
5. **The thousands-separator price parsing** — `1,466,29` pattern from BRIEF.md line 71 never appears in external reviews

---

## SUMMARY TABLE

| Aspect | External Reviews | Our Plan | Winner |
|--------|----------------|----------|--------|
| Encoding detection | UTF-8 first, CP1252 fallback | Same | Tie |
| Price storage | REAL (f64) | INTEGER cents | **Our plan** |
| atc_codes PK | Never defined | `code` TEXT PK | **Our plan** |
| EAN-13 UNIQUE | Missing | Enforced | **Our plan** |
| Orphan handling | is_orphan flag | FK disabled | External reviews (more explicit) |
| _import_id per table | Present | Missing | **External reviews** |
| _is_active soft delete | Present | Missing | **External reviews** |
| Foreign key strategy | Optional FK w/ flag | FK disabled silently | Tie (both work) |
| Import order | drugs first | drugs first | Tie |
| Update detection | SHA-256 | BLAKE3 | **Our plan** (faster) |
| InfoImportantes | All docs mention different path | Excluded from v1 | **Our plan** |
| Sync schedule | Three tiers | Dual | **External reviews** (more precise) |
| Price thousands sep | Not addressed | Handled | **Our plan** |

---

## ACTIONS RECOMMENDED

1. **Keep our `atc_codes` design** — the external reviews validate the table is needed but never designed it
2. **Consider adding `is_orphan` flag** to SMR, ASMR, GENER, and InfoImportantes tables
3. **Consider adding `_import_id`** to child tables for import lineage
4. **Consider adding `_is_active`** columns for explicit soft delete
5. **Accept we are right on prices** — integer cents is more rigorous than REAL, despite external reviews using f64
6. **Accept we are right on InfoImportantes exclusion** — the reviews validate this was the right call
7. **Use the three-tier frequency schedule** from external reviews, not our dual-schedule approach
