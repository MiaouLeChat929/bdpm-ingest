# BDPM Feasibility Study: External Review Summary

**Source:** BDPM_Etude_Faisabilite.txt (768 lines, Mai 2026, Z.ai)
**Scope:** Feasibility analysis for a Rust + SQLite pipeline to ingest France's public drug database

---

## 1. Feasibility Analysis and Conclusions

**Verdict: Entirely feasible.** The study confirms that building a reliable, complete SQLite database from BDPM files is achievable. Data is structurally sound (zero malformed rows, uniform tab delimiters, compliant enumerations), but requires explicit handling of encoding nuances, date format inconsistencies, decimal separator conventions, embedded HTML, and referential integrity gaps.

**Key feasibility findings:**
- 11 files totaling ~27.6 MB of raw data available at `https://base-donnees-publique.medicaments.gouv.fr/telechargement`
- No existing Rust crate covers BDPM parsing (confirmed absent on crates.io/GitHub at study time)
- No public pre-built SQLite database exists for BDPM
- No ETag, Last-Modified, or Content-Length headers on server responses — full file download required for change detection
- Cache policy is uniformly `Cache-Control: private, must-revalidate; Pragma: no-cache; Expires: 0`

**Critical feasibility constraint:** CIS_bdpm.txt retains only drugs marketed or discontinued within the last 2 years. Historical data for older withdrawn drugs is lost unless already captured. The pipeline must use incremental import with soft-delete to avoid destroying historical references from HAS/ASMR evaluations.

---

## 2. Technical Architecture Recommendations

**5-crate Rust pipeline architecture:**

| Crate | Responsibility | Key Dependencies | Input | Output |
|---|---|---|---|---|
| `bdpm-core` | Shared data types, enums, traits, Config struct | serde, strum, thiserror | — | Shared types + Config |
| `bdpm-fetch` | HTTP download, SHA-256 hashing, archive management | reqwest, tokio, sha2, chrono | Config (URLs, encodings) | Raw files + JSON manifest |
| `bdpm-parse` | Decoding, TSV split, structural validation, normalization | encoding_rs, csv (TSV), serde | Raw files + encoding config | Vec<T> per file |
| `bdpm-validate` | Semantic checks, referential validation, enumeration checks | bdpm-core | Vec<T> + CIS referential | Vec<T> + validation report |
| `bdpm-db` | SQLite insertion, transactions, schema migrations, indexes | rusqlite, refinery | Vec<T> | SQLite DB + import_log |

**Incremental import model (core architectural decision):**
- **Insert phase:** Add records with new primary keys
- **Update phase:** Compare hashed record content; update changed records
- **Delete phase:** Soft-delete (set `is_active = 0`) records present in DB but absent from new file

This preserves historical references to withdrawn drugs that appear in HAS/ASMR evaluations.

---

## 3. Schema/Database Design

**Design principles:**
1. One SQLite table per source file — no additional normalization beyond source structure
2. Raw columns preserved with `_raw` prefix (e.g., `date_amm_raw TEXT`, `date_amm TEXT ISO8601`)
3. `FOREIGN KEY` constraints declared but disabled via `PRAGMA foreign_keys = OFF` by default
4. Each table includes `_import_id` referencing `import_log` for traceability
5. `PRAGMA journal_mode = WAL` for concurrent reads during import
6. CHECK constraints for enumerations (no strong typing in SQLite)

**Complete schema:**

```sql
-- Tracked imports
CREATE TABLE import_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT,           -- ISO 8601
    file_name TEXT,
    sha256 TEXT,
    rows_read INTEGER,
    rows_inserted INTEGER,
    rows_updated INTEGER,
    status TEXT               -- 'success', 'partial', 'failed'
);
CREATE INDEX idx_import_log_timestamp ON import_log(timestamp);

-- Root table: 15,848 unique specialites
CREATE TABLE cis_specialites (
    code_cis TEXT PRIMARY KEY,
    denomination TEXT,
    forme_pharma TEXT,
    voies_admin TEXT,
    statut_amm TEXT,
    type_proc TEXT,
    etat_commercialisation TEXT,
    date_amm TEXT,            -- ISO 8601 (normalized from DD/MM/YYYY)
    date_amm_raw TEXT,
    statut_bdm TEXT,
    num_auto_euro TEXT,
    titulaires TEXT,
    surveillance_renforcee TEXT,  -- CHECK IN ('Oui', 'Non')
    _import_id INTEGER REFERENCES import_log(id)
);
CREATE INDEX idx_specialites_denomination ON cis_specialites(denomination);
CREATE INDEX idx_specialites_statut_amm ON cis_specialites(statut_amm);
CREATE INDEX idx_specialites_etat_comm ON cis_specialites(etat_commercialisation);
CREATE INDEX idx_specialites_date_amm ON cis_specialites(date_amm);

-- Presentations: 20,903 rows, UTF-8
CREATE TABLE cis_presentations (
    code_cis TEXT,
    code_cip7 TEXT,
    libelle TEXT,
    statut_admin TEXT,
    etat_commercialisation TEXT,
    date_decla TEXT,          -- DD/MM/YYYY → ISO 8601
    date_decla_raw TEXT,
    code_cip13 TEXT,
    agrement TEXT,
    taux_remboursement TEXT,
    prix_ht REAL,             -- converted from "24,34" → 24.34
    prix_ttc REAL,
    honoraires REAL,
    indications_remboursement TEXT,
    _import_id INTEGER REFERENCES import_log(id),
    PRIMARY KEY (code_cis, code_cip7)
);
CREATE INDEX idx_presentations_cis ON cis_presentations(code_cis);
CREATE INDEX idx_presentations_cip13 ON cis_presentations(code_cip13);
CREATE INDEX idx_presentations_prix_ht ON cis_presentations(prix_ht);
CREATE INDEX idx_presentations_etat_comm ON cis_presentations(etat_commercialisation);

-- Compositions: 32,389 rows, composite key (CIS + substance)
CREATE TABLE cis_compositions (
    code_cis TEXT,
    designation TEXT,
    code_substance TEXT,
    denom_substance TEXT,
    dosage TEXT,
    ref_dosage TEXT,
    nature TEXT,              -- CHECK IN ('SA', 'FT')
    num_liaison_sa_ft TEXT,
    _import_id INTEGER REFERENCES import_log(id)
);
CREATE INDEX idx_compositions_cis ON cis_compositions(code_cis);
CREATE INDEX idx_compositions_substance ON cis_compositions(code_substance);
CREATE INDEX idx_compositions_nature ON cis_compositions(nature);

-- HAS SMR opinions: 15,257 rows, dates in YYYYMMDD format
CREATE TABLE cis_has_smr (
    code_cis TEXT,
    code_dossier_has TEXT,
    motif_eval TEXT,
    date_avis TEXT,           -- ISO 8601 (normalized from YYYYMMDD)
    date_avis_raw TEXT,
    valeur_smr TEXT,
    libelle_smr TEXT,
    _import_id INTEGER REFERENCES import_log(id)
);
CREATE INDEX idx_smr_cis ON cis_has_smr(code_cis);
CREATE INDEX idx_smr_dossier_has ON cis_has_smr(code_dossier_has);
CREATE INDEX idx_smr_date_avis ON cis_has_smr(date_avis);

-- HAS ASMR opinions: 9,906 rows, dates in YYYYMMDD format
CREATE TABLE cis_has_asmr (
    code_cis TEXT,
    code_dossier_has TEXT,
    motif_eval TEXT,
    date_avis TEXT,           -- ISO 8601 (normalized from YYYYMMDD)
    date_avis_raw TEXT,
    valeur_asmr TEXT,
    libelle_smr TEXT,
    _import_id INTEGER REFERENCES import_log(id)
);
CREATE INDEX idx_asmr_cis ON cis_has_asmr(code_cis);
CREATE INDEX idx_asmr_dossier_has ON cis_has_asmr(code_dossier_has);
CREATE INDEX idx_asmr_date_avis ON cis_has_asmr(date_avis);

-- HAS CT links: 10,342 rows, no CIS column (joins via dossier HAS)
CREATE TABLE has_liens_ct (
    code_dossier_has TEXT PRIMARY KEY,
    lien_url TEXT,
    _import_id INTEGER REFERENCES import_log(id)
);
-- PK suffices as index

-- Generic groups: 10,704 rows
CREATE TABLE cis_generiques (
    id_groupe TEXT,
    libelle_groupe TEXT,
    code_cis TEXT,
    type_generique INTEGER,   -- CHECK IN (0, 1, 2, 4)
    num_tri INTEGER,
    _import_id INTEGER REFERENCES import_log(id)
);
CREATE INDEX idx_generiques_cis ON cis_generiques(code_cis);
CREATE INDEX idx_generiques_groupe ON cis_generiques(id_groupe);
CREATE INDEX idx_generiques_type ON cis_generiques(type_generique);

-- Prescription conditions: 28,151 rows (contains \r\r\n anomalies → 9 empty lines)
CREATE TABLE cis_conditions_prescription (
    code_cis TEXT,
    condition TEXT,
    _import_id INTEGER REFERENCES import_log(id)
);
CREATE INDEX idx_cp_cis ON cis_conditions_prescription(code_cis);

-- Stock availability: 766 rows, latin-1 encoded
CREATE TABLE cis_disponibilite (
    code_cis TEXT,
    code_cip13 TEXT,
    code_statut INTEGER,      -- CHECK IN (1, 2, 3, 4)
    libelle_statut TEXT,
    date_debut TEXT,
    date_maj TEXT,
    date_remise TEXT,
    lien_ansm TEXT,
    _import_id INTEGER REFERENCES import_log(id)
);
CREATE INDEX idx_dispo_cis ON cis_disponibilite(code_cis);
CREATE INDEX idx_dispo_cip13 ON cis_disponibilite(code_cip13);
CREATE INDEX idx_dispo_statut ON cis_disponibilite(code_statut);
CREATE INDEX idx_dispo_date_debut ON cis_disponibilite(date_debut);

-- Major therapeutic interest: 7,711 rows
CREATE TABLE cis_mitm (
    code_cis TEXT PRIMARY KEY,
    code_atc TEXT,
    denomination TEXT,
    lien_bdpm TEXT,
    _import_id INTEGER REFERENCES import_log(id)
);
CREATE INDEX idx_mitm_cis ON cis_mitm(code_cis);
CREATE INDEX idx_mitm_atc ON cis_mitm(code_atc);

-- Security info: 10,189 rows, dynamically generated (timestamped filename)
CREATE TABLE cis_info_importantes (
    code_cis TEXT,
    date_debut TEXT,           -- ISO 8601
    date_fin TEXT,             -- ISO 8601
    texte TEXT,                -- HTML stripped to plain text
    texte_raw TEXT,            -- Original HTML preserved
    url TEXT,                  -- Extracted from <a href="...">
    _import_id INTEGER REFERENCES import_log(id)
);
CREATE INDEX idx_info_cis ON cis_info_importantes(code_cis);
CREATE INDEX idx_info_date_debut ON cis_info_importantes(date_debut);
CREATE INDEX idx_info_date_fin ON cis_info_importantes(date_fin);
```

**Orphan handling strategy:** 18.4% of CIS_HAS_SMR, 15.8% of CIS_HAS_ASMR, and 23.5% of CIS_GENER references point to withdrawn drugs absent from CIS_bdpm. Strategy: insert with `is_orphan = 1` flag, do not reject.

**Vacuity observations (index design implications):**
- `statut_bdm` in cis_specialites: 85.8% empty
- `agrement` in cis_presentations: 96.1% empty
- `CIP13` in cis_disponibilite: 95.4% empty

Do not create indexes on high-vacuity columns unless required for specific query patterns.

---

## 4. Data Processing Approaches

### 4.1 Encoding Handling (Critical)

Three distinct encodings coexist with no in-file indicators:

| Encoding | Files | Key Pitfall |
|---|---|---|
| `cp1252` (Windows-1252) | CIS_bdpm, CIS_COMPO, CIS_CPD, CIS_GENER, CIS_HAS_ASMR, CIS_HAS_SMR, CIS_MITM | 0x92 = right single quote (U+2019), not valid in latin-1. 29,704 occurrences in CIS_HAS_ASMR alone. |
| `latin-1` (ISO-8859-1) | CIS_CIP_Dispo_Spec | 0xE0 = à grave. Decoding as UTF-8 fails. |
| `utf-8` | CIS_CIP_bdpm, CIS_InfoImportantes, HAS_LiensPageCT | Contains native UTF-8 smart quotes (\xE2\x80\x99). Already in NFC. |

**Encoding approach:** Hardcode encoding per file in parser config. Do not attempt dynamic detection — chardet/encoding_rs detectors cannot reliably distinguish cp1252 from latin-1. Use `encoding_rs` crate with zero-copy decoding producing `&str` directly.

### 4.2 Line Ending Normalization

Nine files use CRLF (`\r\n`), two use LF (`\n`). CIS_CPD_bdpm contains 9 empty lines from `\r\r\n` sequences (6 positions with double CR). **Pipeline must:**
- Normalize all line endings by stripping `\r` residually
- Filter empty lines before tab split

### 4.3 Date Normalization

Two formats, strictly separated by file (100% consistent):

| Format | Files | Example |
|---|---|---|
| `DD/MM/YYYY` | CIS_bdpm, CIS_CIP_bdpm, CIS_InfoImportantes, CIS_CIP_Dispo_Spec | 28/04/2026 |
| `YYYYMMDD` | CIS_HAS_SMR, CIS_HAS_ASMR | 20260428 |

**Convert all to ISO 8601 `YYYY-MM-DD` in database.** Use `chrono::NaiveDate` with dedicated parser handling both input formats.

### 4.4 Numeric Normalization

CIS_CIP_bdpm uses comma as decimal separator (French convention): `"24,34"` → `24.34` (f64). No prices use period decimal. Convert during import into REAL columns.

### 4.5 HTML Content (CIS_InfoImportantes)

421 distinct HTML tags found in field 4. Strategy:
- Extract plain text from `<a>` tags (strip markup)
- Extract `href` targets as separate `url` column
- Decode HTML entities (`&ecirc;` → `ê`, etc.)
- Preserve raw HTML in `_raw` column

### 4.6 Smart Quote Normalization

U+2019 (right single quotation mark, 0x92 in cp1252) appears massivement in HAS files. Normalize to U+0027 (straight apostrophe) for consistency in search and display.

### 4.7 Five-Stage Parsing Pipeline

1. **Download & integrity check:** Download binary → SHA-256 hash → compare to previous → archive raw to `raw/YYYY-MM-DD/` → generate JSON manifest
2. **Decode & normalize lines:** Decode with correct encoding → normalize line endings → filter empty lines → NFC normalize Unicode
3. **Tab split & structural validation:** Split on `\t` → verify column count → log malformed lines without interrupting → detect embedded tabs (none found but possible)
4. **Per-field normalization:** Dates → numbers (comma→period) → smart quotes → HTML → trim whitespace
5. **Semantic validation & insertion:** Check enumerations → check referential integrity → insert with `is_orphan` flag for orphans → transaction per file with batch size 1000

### 4.8 Quality Checks (Automated Post-Import)

| Category | Check | Threshold |
|---|---|---|
| Completeness | rows_imported vs rows_in_file | 0% tolerance |
| Referential coherence | orphan count per table | alert if +5% vs previous import |
| Enumeration validity | values outside expected domain | zero tolerance |
| Temporal coherence | AMM dates: 1950 < date < today | alert on violations |
| Regression detection | record count per table vs previous import | alert if decrease >2% |

---

## 5. API Design Recommendations

The study defers full API design to a future phase (Phase 6, estimated 3-4 weeks after foundation), but outlines:

- **Framework candidates:** `actix-web` or `axum` (both well-established in Rust ecosystem)
- **Query patterns needed:** drug search by name/CIS/CIP, filter by ATC class, filter by SMR/ASMR value, stock availability, generic group lookup, full-text search
- **Full-text search:** Consider FTS5 extension in SQLite for denomination/indication searches
- **Output format:** OpenAPI spec (Swagger) for documentation
- **Pagination:** cursor-based for large result sets
- **Rate limiting:** Implement at API layer (not at fetch layer, which uses respectful polling intervals)

**Critical prerequisite:** Phase 3 (SQLite base) must be complete and stable before API work begins.

---

## 6. Edge Cases and Risks Identified

### Identified Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Schema change without notification (new column, changed enumeration, encoding shift) | Medium | High | Parser logs every schema deviation, fails gracefully, does not silently corrupt |
| File renamed or deleted on server | Low | High | Fetcher checks HTTP 200, alerts on 404/other status codes |
| Encoding migration (ANSM shifts cp1252→UTF-8) | Low | Medium | Fallback detector using chardet/encoding_rs activates if decoding produces U+FFFD substitution characters |
| Rate-limiting or IP block | Low | Medium | 2-second minimum interval, explicit User-Agent (`BDPM-Importer/1.0`), exponential backoff on 429/503 |
| Historical data loss on full reload | High (if not mitigated) | High | Incremental import with soft-delete from day one; archive all raw files |
| Orphans accumulating silently | Medium | Low | `is_orphan` tracking, alert if orphan % increases >5% |

### Edge Cases

1. **CIS_CPD `\r\r\n` sequences:** 9 empty lines in current file. Monitor across months to determine if ANSM generator bug or recurrent pattern. Filter empty lines unconditionally.
2. **CIS_InfoImportantes dynamic filename:** HTTP Content-Disposition includes timestamp (e.g., `CIS_InfoImportantes_20260526111334_bdpm.txt`). Extract and preserve this timestamp in import_log.
3. **No conditional requests possible:** No ETag, Last-Modified, or Content-Length in HEAD responses. Must download full file + SHA-256 to detect changes.
4. **Orphan CIS references in HAS:** ~18% SMR, ~16% ASMR, ~23% GENER references point to drugs withdrawn >2 years ago. Handle via nullable FKs or orphan flag — do not reject.
5. **CIS_CIP_Dispo_Spec latin-1:** Only file using latin-1. Easy to miss if defaulting to cp1252.
6. **Smart quotes vs apostrophes:** 0x92 (cp1252) maps to U+2019, not U+0027. Normalization required for consistent search.
7. **Multiple update frequencies:** CIS_CIP (price/remimbursement) updated mid-cycle; CIS_CIP_Dispo_Spec (stock) updated weekly; CIS_MITM updated quarterly; CIS_InfoImportantes generated on-demand.

### Unresolved Blind Spots

1. **Schema stability over time:** PDF spec is v4 (581 Ko), suggesting historical changes. No changelog exists. Archives from multiple months needed to detect structural changes.
2. **MySQL dump channel:** Project betagouv/infomedicament references a MySQL dump + image directory. May contain ATC codes and RCP texts missing from TXT files. Unconfirmed availability.
3. **ANSM staging site:** `rec-bdm.ansm.integra.fr` may preview schema changes before production deployment. Observe but do not automate against without authorization.
4. **Site refactoring risk:** Current site uses legacy technology. URL/structure migration possible at unknown future date.
5. **Referential completeness:** No independent verification method for dataset completeness.

---

## 7. Technology Choices and Rationale

### Rust Crates

| Crate | Choice | Rationale |
|---|---|---|
| **HTTP client** | `reqwest` with `tokio` runtime | Standard for async HTTP in Rust; handles redirects, connection pooling |
| **Encoding** | `encoding_rs` | Zero-copy UTF-8 decoding; cp1252/latin-1/UTF-8 support; encoding resolved at compile time |
| **CSV/TSV** | `csv` crate with custom tab delimiter | Standard, battle-tested; handles edge cases |
| **Serialization** | `serde` + `serde_json` | Derive Serialize/Deserialize for all BDPM types |
| **Enums** | `strum` | Type-safe conversion between enum variants and source strings |
| **SQLite** | `rusqlite` | De facto standard for SQLite in Rust; supports WAL, prepared statements, UDFs |
| **Migrations** | `refinery` | Versioned SQL migrations (V1__*.sql, V2__*.sql); reproducible schema evolution |
| **Error handling** | `thiserror` | Derive `Error` traits cleanly; structured error types |
| **Dates** | `chrono::NaiveDate` | Handles both DD/MM/YYYY and YYYYMMDD parsing; ISO 8601 storage |
| **Decimal precision** | `rust_decimal` (optional wrapper over f64) | Avoids floating-point precision issues in price calculations |
| **Hashing** | `sha2` | SHA-256 for file comparison |
| **Time** | `chrono` | Timestamps in import_log |
| **Async runtime** | `tokio` | Required by reqwest; enables parallel file fetching |

### Crate Name

Recommended: `bdpm` or `bdpm-parser`. Reserve name on crates.io at project start.

---

## 8. Implementation Phasing Suggestions

| Phase | Duration | Objective | Deliverable | Dependencies |
|---|---|---|---|---|
| **1. Foundation** | 1-2 weeks | Build `bdpm-core` (types, enums, Config, traits) + `bdpm-fetch` (download, SHA-256, archive, JSON manifest) | CLI binary that downloads and archives all 11 files with checksums | None |
| **2. Parsing** | 2-3 weeks | Build `bdpm-parse` (encoding decoding, TSV split, normalization for dates/numbers/apostrophes) | CLI binary that parses all 11 files and exports valid JSON/CSV | Phase 1 |
| **3. SQLite Base** | 1-2 weeks | Build `bdpm-db` (schema migrations, batch insertion, import_log, indexes) | Fully queryable SQLite database with all 11 tables + import history | Phase 2 |
| **4. Validation** | 1-2 weeks | Build `bdpm-validate` (referential checks, enumeration validation, temporal coherence, regression detection) | HTML/CLI validation report after each import | Phase 3 |
| **5. Incremental** | 2 weeks | Implement hash-based diff, upsert logic, soft-delete, change notifications | Pipeline that updates DB without full reconstruction | Phase 3 |
| **6. API** (future) | 3-4 weeks | REST/GraphQL layer with actix-web or axum, OpenAPI docs, full-text search | Documented HTTP API with search, filtering, pagination | Phase 5 |

**Critical path:** Phase 1 → 2 → 3 → 5. Phases 4 and 5 can partially overlap. Phase 3 completion (functional SQLite base) is the key milestone — at this point the data is exploitable by third-party tools.

**Parsing order within Phase 2:**
1. CIS_bdpm.txt (simplest, central)
2. CIS_CIP_bdpm.txt (only UTF-8 among cp1252 files)
3. CIS_HAS_SMR + CIS_HAS_ASMR (YYYYMMDD dates, most orphans)
4. CIS_CPD (empty lines, simple structure)
5. CIS_InfoImportantes (HTML extraction)
6. CIS_CIP_Dispo_Spec (latin-1, smallest)
7. Remaining files (COMPO, GENER, MITM, LiensCT)

**Early data strategy:** Do not wait for perfect parser before building SQLite. Populate database with `_raw` columns from day one; validate queries while refining parsing iteratively.

---

## Summary

The BDPM feasibility study is comprehensive and technically sound. The dataset is well-structured and stable despite encoding and date-format complexities that require explicit handling. The proposed Rust 5-crate pipeline is appropriately modular for incremental development, and the incremental import model is essential to preserve historical data for withdrawn drugs that retain valid HAS evaluations. The primary technical risks are schema changes without notice and historical data loss from full overwrites — both mitigated by the proposed architecture. No existing open-source solution covers this need, making this a genuine reference implementation opportunity.