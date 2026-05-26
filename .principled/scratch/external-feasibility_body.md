# BDPM Feasibility Study — Comprehensive Summary

The Base de Donnees Publique des Medicaments (BDPM) is France's official open data reference for marketed medications. It is fed by ANSM ( Agence nationale de securite du medicament), HAS (Haute Autorite de Sante), and Assurance Maladie, created under article L. 161-40-1 of the Code de la Securite Sociale (law 2011-2012 of 29/12/2011). Data is distributed under Etalab 2.0 license, which permits reproduction and distribution provided the data is not altered or distorted and the source with update date is cited.

The feasibility study confirms that building a reliable, complete SQLite database from BDPM files is entirely feasible. The data is structurally sound (zero malformed lines, uniform tab separators, conforming enumerations), but presents encoding subtleties (three norms coexist, cp1252 dominant), format quirks (two date formats, decimal comma, embedded HTML), and referential coherence issues (significant HAS and GENER orphans) that must be handled explicitly in code.

---

## 1. Feasibility Analysis and Conclusions

### 1.1 Source File Inventory

The official download page (https://base-donnees-publique.medicaments.gouv.fr/telechargement) publishes 11 tab-separated TXT data files plus a PDF format specification (v4, 581 Ko). Raw data totals approximately 27.6 Mo. Inspection confirms the separator is uniformly the tab character, no file contains a header row, and no field delimiter (quotes) is used.

| File | Content | Lines | Columns | Encoding | Line Endings |
|------|---------|-------|---------|----------|--------------|
| CIS_bdpm.txt | Specialites | 15,848 | 12 | cp1252 | CRLF |
| CIS_CIP_bdpm.txt | Presentations | 20,903 | 13 | utf-8 | LF |
| CIS_COMPO_bdpm.txt | Compositions | 32,389 | 8 | cp1252 | CRLF |
| CIS_HAS_SMR_bdpm.txt | Avis SMR | 15,257 | 6 | cp1252 | CRLF |
| CIS_HAS_ASMR_bdpm.txt | Avis ASMR | 9,906 | 6 | cp1252 | CRLF |
| HAS_LiensPageCT_bdpm.txt | Liens CT | 10,342 | 2 | utf-8/ASCII | CRLF |
| CIS_GENER_bdpm.txt | Groupes generiques | 10,704 | 5 | cp1252 | CRLF |
| CIS_CPD_bdpm.txt | Conditions prescription | 28,151 | 2 | cp1252 | CRLF* |
| CIS_CIP_Dispo_Spec.txt | Ruptures de stock | 766 | 8 | latin-1 | CRLF |
| CIS_MITM.txt | Med. interet ther. majeur | 7,711 | 4 | cp1252 | CRLF |
| CIS_InfoImportantes.txt | Infos securite (dynamique) | 10,189 | 4 | utf-8 | LF |

* CIS_CPD contains 9 empty lines due to CR+CR+LF sequences.

### 1.2 Update Cadence

BDPM is officially updated monthly, but file-level inspection reveals variable cadence. Most files carry the date 28/04/2026, but CIS_CIP_bdpm.txt indicates 25/05/2026 (inter-cycle refresh for pricing/reimbursement data), CIS_CIP_Dispo_Spec.txt shows 19/05/2026 (stock shortages updated more frequently), and CIS_MITM.txt shows 09/03/2026 (less frequent update).

CIS_InfoImportantes.txt is a special case: it is dynamically generated at each download, evidenced by the HTTP Content-Disposition header including a precise timestamp (e.g., CIS_InfoImportantes_20260526111334_bdpm.txt). This file can change at any time between monthly cycles since safety information may be published urgently.

There is no RSS feed, no notification API, no official changelog, and no semantic versioning mechanism for the data. The only way to detect changes is to download files and compare with the previous version.

### 1.3 Server Response Characteristics

The server provides no ETag, no Last-Modified, and no Content-Length in HEAD responses. Cache policy is uniformly: `Cache-Control: private, must-revalidate; Pragma: no-cache; Expires: 0`. This means every request triggers a full file download, with no possibility of conditional requests (If-None-Match or If-Modified-Since).

### 1.4 File Relationships

The Code CIS (8 digits, prefix 6) is the universal join key, present in 10 of 11 files. CIS_bdpm.txt is the master table with 15,848 unique specialties and no duplicates. The sole structural exception is HAS_LiensPageCT_bdpm.txt, which contains no Code CIS but rather a HAS dossier code, serving as the join key to SMR and ASMR files. The relational schema deploys in a star around Code CIS, with a secondary HAS branch via the dossier code.

| File | Primary Key | Foreign Key | References |
|------|------------|-------------|------------|
| CIS_bdpm.txt | Code CIS | - | (root) |
| CIS_CIP_bdpm.txt | CIP7 / CIP13 | Code CIS | CIS_bdpm |
| CIS_COMPO_bdpm.txt | (CIS + substance code) | Code CIS | CIS_bdpm |
| CIS_HAS_SMR_bdpm.txt | (CIS + HAS dossier + date) | Code CIS + HAS dossier | CIS_bdpm + LiensPageCT |
| CIS_HAS_ASMR_bdpm.txt | (CIS + HAS dossier + date) | Code CIS + HAS dossier | CIS_bdpm + LiensPageCT |
| HAS_LiensPageCT_bdpm.txt | HAS dossier code | - | (reference for SMR/ASMR) |
| CIS_GENER_bdpm.txt | (Group ID + CIS) | Code CIS | CIS_bdpm |
| CIS_CPD_bdpm.txt | (Code CIS) | Code CIS | CIS_bdpm |
| CIS_InfoImportantes.txt | (CIS + dates) | Code CIS | CIS_bdpm |
| CIS_CIP_Dispo_Spec.txt | (CIS + CIP13) | Code CIS + CIP13 | CIS_bdpm + CIS_CIP |
| CIS_MITM.txt | (Code CIS) | Code CIS | CIS_bdpm |

### 1.5 Orphan References

Cross-file CIS code analysis reveals significant orphan references (CIS codes present in secondary files but absent from the master table CIS_bdpm.txt):

| File | Unique CIS | Orphans | % | Interpretation |
|------|-----------|---------|---|----------------|
| CIS_CIP_bdpm | 14,573 | 4 | 0.03% | Negligible, probably transitional |
| CIS_COMPO_bdpm | 15,846 | 0 | 0% | Perfectly coherent |
| CIS_HAS_SMR | 9,014 | 2,806 | 18.4% | Evaluations of withdrawn medications |
| CIS_HAS_ASMR | 6,172 | 1,567 | 15.8% | Same as SMR |
| CIS_GENER | 10,628 | 2,503 | 23.5% | Generic groups including withdrawn substances |
| CIS_InfoImportantes | 4,208 | 965 | 9.5% | Safety alerts on withdrawn medications |
| CIS_CPD | 12,492 | 0 | 0% | Perfectly coherent |
| CIS_MITM | 7,711 | 0 | 0% | Perfectly coherent |

Orphans correspond to medications withdrawn from the market more than 2 years ago (CIS_bdpm only retains marketed or stopped specialties for less than 2 years), but whose HAS evaluations remain in history.

### 1.6 Overall Verdict

**The project is entirely feasible.** Data quality is good with no malformed rows, but encoding heterogeneity is the single most critical discovery. No existing Rust crate covers BDPM parsing and no pre-built SQLite database is publicly available, making this an opportunity to create a reference tool for the community.

---

## 2. Technical Architecture Recommendations

### 2.1 Pipeline Overview

The proposed architecture decomposes into 5 Rust crates organized in a sequential pipeline, each with a single responsibility and clear interface. This decomposition enables incremental development, isolated unit tests, and parallel compilation.

| Crate | Responsibility | Rust Dependencies | Input | Output |
|-------|---------------|-------------------|-------|--------|
| bdpm-fetch | HTTP download, archiving, SHA-256 hashing | reqwest, tokio, sha2, chrono | Config (URLs, encodings) | Raw files + JSON manifest |
| bdpm-parse | Decoding, TSV split, structural validation, normalization | encoding_rs, csv (TSV), serde | Raw files + encoding config | Vec of typed structs per file |
| bdpm-validate | Semantic checks, referential checks, enumerations, coherence | bdpm-core (traits) | Vec + CIS referential | Vec + validation report |
| bdpm-db | SQLite insertion, transactions, schema migrations, indexes | rusqlite, refinery (migrations) | Vec | SQLite DB + import_log |
| bdpm-core | Data structures, traits, enum types, configuration | serde, strum (enums), thiserror | - | Shared types + Config struct |

### 2.2 Critical Design Decisions

**Encoding strategy:** Encoding is declared in static configuration per file (hardcoded, not dynamically detected). The encoding_rs crate provides zero-copy decoders that produce valid UTF-8 directly. Since cp1252->UTF-8 is deterministic and lossless, hardcoding is safe. Detection fallback (chardet or encoding_rs detective) should activate if default decoding produces U+FFFD substitution characters.

**SQLite integration:** The rusqlite crate is the de facto standard for SQLite in Rust, supporting transactions, prepared statements, WAL mode, and user-defined functions. The refinery crate handles schema migrations, versioning changes as numbered SQL files (V1__initial_schema.sql, V2__add_fulltext_search.sql, etc.).

**Batch insertion:** Use explicit transactions with a batch size of 1,000 rows to balance memory and performance. SQLite can insert 50,000+ rows/second in WAL mode with batched transactions.

**JSON manifest:** The fetcher produces a JSON manifest alongside raw files containing SHA-256 hashes, download timestamps, and file sizes for each archived file.

---

## 3. Schema/Database Design

### 3.1 Design Principles

Each BDPM file corresponds exactly to one SQLite table without additional normalization. This facilitates incremental updates and diagnostics. Columns use native SQLite types (INTEGER, TEXT, REAL) with CHECK constraints for enumerations. Foreign keys are declared but PRAGMA foreign_keys = OFF by default to avoid blocking orphan reference insertion. A `_raw` prefix is used for columns storing original values before normalization. Each table has an `_import_id` column referencing the import_log table for traceability.

### 3.2 Detailed Schema

#### import_log
| Column | Type | Constraints |
|--------|------|-------------|
| id | INTEGER AUTO | PRIMARY KEY |
| timestamp | TEXT | |
| file_name | TEXT | indexed |
| sha256 | TEXT | |
| rows_read | INTEGER | |
| rows_inserted | INTEGER | |
| rows_updated | INTEGER | |
| status | TEXT | (success/partial/failure) |
| duration_ms | INTEGER | |

#### cis_specialites
| Column | Type | Constraints |
|--------|------|-------------|
| code_cis | TEXT | PRIMARY KEY (code CIS) |
| denomination | TEXT | indexed |
| forme_pharma | TEXT | |
| voies_admin | TEXT | |
| statut_amm | TEXT | indexed |
| type_proc | TEXT | |
| etat_commercialisation | TEXT | indexed |
| date_amm | TEXT (ISO YYYY-MM-DD) | indexed |
| statut_bdm | TEXT | |
| num_auto_euro | TEXT | |
| titulaires | TEXT | |
| surveillance_renforcee | TEXT | CHECK (IN ('Oui', 'Non')) |
| _import_id | INTEGER | FK -> import_log |

#### cis_presentations
| Column | Type | Constraints |
|--------|------|-------------|
| code_cis | TEXT | FK, indexed |
| code_cip7 | TEXT | part of composite PK |
| libelle | TEXT | |
| statut_admin | TEXT | |
| etat_commercialisation | TEXT | indexed |
| date_decla | TEXT | |
| code_cip13 | TEXT | indexed |
| agrement | TEXT | |
| taux_remboursement | TEXT | |
| prix_ht | REAL | indexed |
| prix_ttc | REAL | |
| honoraires | REAL | |
| indications_remboursement | TEXT | |
| _import_id | INTEGER | FK -> import_log |

Primary key: COMPOSITE (code_cis + code_cip7)

#### cis_compositions
| Column | Type | Constraints |
|--------|------|-------------|
| code_cis | TEXT | FK, indexed |
| designation | TEXT | |
| code_substance | TEXT | indexed |
| denom_substance | TEXT | |
| dosage | TEXT | |
| ref_dosage | TEXT | |
| nature | TEXT | CHECK (IN ('SA', 'FT')), indexed |
| num_liaison_sa_ft | TEXT | |
| _import_id | INTEGER | FK -> import_log |

Primary key: ROWID AUTO

#### cis_has_smr
| Column | Type | Constraints |
|--------|------|-------------|
| code_cis | TEXT | FK, indexed |
| code_dossier_has | TEXT | FK to has_liens_ct, indexed |
| motif_eval | TEXT | |
| date_avis | TEXT (ISO YYYY-MM-DD) | indexed |
| valeur_smr | TEXT | |
| libelle_smr | TEXT | |
| _import_id | INTEGER | FK -> import_log |

Primary key: ROWID AUTO

#### cis_has_asmr
| Column | Type | Constraints |
|--------|------|-------------|
| code_cis | TEXT | FK, indexed |
| code_dossier_has | TEXT | FK to has_liens_ct, indexed |
| motif_eval | TEXT | |
| date_avis | TEXT (ISO YYYY-MM-DD) | indexed |
| valeur_asmr | TEXT | |
| libelle_asmr | TEXT | |
| _import_id | INTEGER | FK -> import_log |

Primary key: ROWID AUTO

#### has_liens_ct
| Column | Type | Constraints |
|--------|------|-------------|
| code_dossier_has | TEXT | PRIMARY KEY |
| lien_url | TEXT | |
| _import_id | INTEGER | FK -> import_log |

#### cis_generiques
| Column | Type | Constraints |
|--------|------|-------------|
| id_groupe | TEXT | indexed |
| libelle_groupe | TEXT | |
| code_cis | TEXT | FK, indexed |
| type_generique | INTEGER | CHECK (IN (0, 1, 2, 4)), indexed |
| num_tri | TEXT | |
| _import_id | INTEGER | FK -> import_log |

Primary key: ROWID AUTO

#### cis_conditions_prescription
| Column | Type | Constraints |
|--------|------|-------------|
| code_cis | TEXT | FK, indexed |
| condition | TEXT | |
| _import_id | INTEGER | FK -> import_log |

Primary key: ROWID AUTO

#### cis_disponibilite
| Column | Type | Constraints |
|--------|------|-------------|
| code_cis | TEXT | FK, indexed |
| code_cip13 | TEXT | indexed |
| code_statut | INTEGER | CHECK (IN (1, 2, 3, 4)), indexed |
| libelle_statut | TEXT | |
| date_debut | TEXT | indexed |
| date_maj | TEXT | |
| date_remise | TEXT | |
| lien_ansm | TEXT | |
| _import_id | INTEGER | FK -> import_log |

Primary key: ROWID AUTO

#### cis_mitm
| Column | Type | Constraints |
|--------|------|-------------|
| code_cis | TEXT | PRIMARY KEY (code CIS), indexed |
| code_atc | TEXT | indexed |
| denomination | TEXT | |
| lien_bdpm | TEXT | |
| _import_id | INTEGER | FK -> import_log |

#### cis_info_importantes
| Column | Type | Constraints |
|--------|------|-------------|
| code_cis | TEXT | FK, indexed |
| date_debut | TEXT (ISO YYYY-MM-DD) | indexed |
| date_fin | TEXT (ISO YYYY-MM-DD) | indexed |
| texte | TEXT | |
| url | TEXT | extracted from HTML |
| _import_id | INTEGER | FK -> import_log |

Primary key: ROWID AUTO

### 3.3 SQLite Type and Constraint Strategy

Since SQLite has no native strong typing, constraints are essential for quality assurance. Date columns use TEXT in ISO 8601 format (YYYY-MM-DD). Price columns use REAL (f64). Codes CIS/CIP use TEXT (not INTEGER) since they are identifiers with potential zero-padding. PRAGMA journal_mode = WAL is recommended for concurrent reads during import. CHECK constraints enforce enumeration validity.

---

## 4. Data Processing Approaches

### 4.1 Five-Stage Parsing Pipeline

**Stage 1: Download and Integrity Verification**
- Download binary file (no text decoding at this stage)
- Calculate SHA-256 and compare against previous import hash; if identical, skip
- Store raw file in timestamped archive directory (e.g., raw/2026-05-26/)

**Stage 2: Decoding and Line Normalization**
- Decode binary content with file-specific encoding (cp1252, latin-1, or utf-8). Encoding is hardcoded in parser configuration, not dynamically detected.
- Normalize line endings: strip all residual \r characters. Filter empty lines resulting from \r\r\n sequences (trap identified in CIS_CPD).
- Normalize Unicode: convert to NFC if necessary (UTF-8 files are already in NFC, but cp1252 decoded output may produce decomposed forms in some cases).

**Stage 3: Tab Split and Structural Validation**
- Split each line on the tab character (\t). Verify expected column count. Analysis confirms 0% malformed rows in current files, but the parser must implement a recovery mode: log abnormal lines without interrupting import.
- Detect embedded tabs in fields (none observed in current data, but theoretical risk if a text field contains a tab).

**Stage 4: Field-by-Field Normalization**
- Dates: Convert DD/MM/YYYY and YYYYMMDD to YYYY-MM-DD (ISO 8601). Empty dates remain NULL.
- Numbers: Replace decimal comma with period (24,34 becomes 24.34). Convert to f64.
- Apostrophes: Normalize smart quotes (U+2019) from cp1252 to right apostrophe (U+0027). Optional but recommended for database consistency.
- HTML: For CIS_InfoImportantes, extract URLs and text from `<a>` tags. Decode HTML entities (&ecirc; to e, etc.).
- Whitespace: Trim leading and trailing spaces. Do not modify internal spaces.

**Stage 5: Semantic Validation and Insertion**
- Verify enumerations: StatutBdm in {empty, Alerte, Warning disponibilite}, Nature composant in {SA, FT}, Type generique in {0, 1, 2, 4}, etc. Log unexpected values.
- Verify Code CIS existence in master table. Insert orphans with is_orphan=1 flag rather than rejecting.
- Insert into SQLite via per-file transaction (autocommit disabled for performance).

### 4.2 Quality Control Checks

Post-import automated checks should be implemented as SQL queries on the imported database, with results systematically logged. Five categories of checks are recommended:

1. **Completeness:** Imported row count vs source file row count, with 0% tolerance threshold
2. **Referential coherence:** Orphan CIS count per table, alert if count increases more than 5% vs previous import
3. **Enumeration validity:** Zero out-of-domain values
4. **Temporal coherence:** AMM dates are after 1950 and before current date
5. **Regression detection:** Record count comparison per table vs previous import, alert if decrease exceeds 2%

### 4.3 Incremental Import Model

Rather than rebuilding the entire database on each update, the pipeline compares new data with existing state and applies only differences. For each table, three phases:

1. **Insertion phase:** Adds records whose primary key is new
2. **Update phase:** Compares hashed content of each existing record and updates those whose hash changed
3. **Soft-delete phase:** Marks as inactive records present in the database but absent from the new file (soft delete via is_active column)

This model preserves history while keeping the database current. It is particularly important for HAS files (SMR/ASMR) containing references to medications withdrawn from CIS_bdpm.txt.

---

## 5. API Design Recommendations

### 5.1 Future API Layer

The final phase (Phase 6) proposes a REST/GraphQL layer above SQLite using actix-web or axum, with OpenAPI documentation. Key features:

- Full-text search capability
- Filtering by any column (code CIS, date range, generic type, etc.)
- Pagination for large result sets
- Rate limiting and caching headers
- OpenAPI 3.0 specification auto-generated from code

### 5.2 API Design Considerations

API design should follow agent-first principles for headless consumption:
- JSON output only, with predictable field names matching the database schema
- Error responses include error code, message, and affected field
- Cursor-based pagination for large datasets
- HTTP status codes follow REST conventions (200, 400, 404, 500)
- Support for JSON streaming for large result sets (newline-delimited JSON)

---

## 6. Edge Cases and Risks Identified

### 6.1 Critical Edge Cases

**Encoding heterogeneity:** Seven files use cp1252, one uses latin-1, three use utf-8. The distinction between cp1252 and latin-1 is critical: cp1252 defines characters in the 0x80-0x9F range that latin-1 leaves undefined. Character 0x92 (right single quotation mark, U+2019 in Unicode) appears massively in HAS files: 29,704 occurrences in CIS_HAS_ASMR and 22,253 in CIS_HAS_SMR. Decoding these files in latin-1 would produce invisible control characters instead of expected apostrophes. No BOM, XML declaration, or header exists in the files themselves.

**Double CR sequences in CIS_CPD:** Nine positions contain \r\r\n sequences (double CR before LF), generating 9 empty parasitic lines. A robust parser must normalize line endings and filter empty lines before tab splitting.

**Dynamic file generation:** CIS_InfoImportantes.txt includes a precise timestamp in its filename (e.g., CIS_InfoImportantes_20260526111334_bdpm.txt), meaning the file changes unpredictably between monthly cycles.

**Embedded HTML:** CIS_InfoImportantes contains 421 distinct HTML tags, primarily `<a>` tags with target='_blank' pointing to ANSM pages. HTML entities like &ecirc; are present.

**Decimal comma:** Numeric fields (prices, reimbursement rates, fees) in CIS_CIP_bdpm.txt use comma as decimal separator per French convention. Of 20,903 lines, 13,546 contain a comma-separated price and 7,357 are empty. No price uses period decimal.

**High null-rate columns:** StatutBdm is empty 85.8% of the time. CIP13 in CIS_CIP_Dispo_Spec is empty 95.4%. Agrement aux collectivites is empty 96.1%.

### 6.2 Identified Risks

| Risk | Description | Mitigation |
|------|-------------|-----------|
| Schema change without notification | No changelog or versioning; format change (column addition, enumeration modification, encoding change) only detected at parsing time | Parser logs any deviation from expected schema; fail gracefully |
| File disappearance/move | If a file is renamed or deleted, pipeline must detect (HTTP 404) and alert | Verify HTTP status code; alert for any status other than 200 |
| Encoding migration | If ANSM migrates a file from cp1252 to UTF-8 without announcement, parser will fail on cp1252-specific characters | Implement fallback encoding detector that activates if decoding produces U+FFFD |
| Rate limiting or IP blocking | Server may block excessive requests | Respect minimum interval between requests; use explicit User-Agent; implement exponential backoff on HTTP 429 or 503 |
| Historical data loss | Medications removed from CIS_bdpm.txt disappear from future imports; if pipeline overwrites database on each import instead of incremental, historical data is lost | Implement incremental import with soft delete from the start; archive raw files |

### 6.3 Unknowns (Angles Morts)

1. **Data completeness:** ANSM data is presumed complete for declared scope (marketed or stopped less than 2 years), but no independent verification method exists
2. **Update frequency precision:** Exact per-file update frequency is not officially documented; observed variations could be accidental rather than deliberate
3. **MySQL dump channel:** A MySQL dump of BDPM (SQL file + image directory) is referenced by betagouv/infomedicament project; existence and accessibility unverified; may contain additional data (ATC codes, images, RCP text)
4. **Potential site redesign:** Impact of BDPM site refactor (CMS migration, URL change) cannot be anticipated but exists as medium-term risk

---

## 7. Technology Choices and Rationale

### 7.1 Language: Rust

**Rationale:** Performance for data processing, compile-time safety, zero-cost abstractions, excellent ecosystem for system programming. The project is in greenfield territory (no existing BDPM parsing crate), making Rust's type safety valuable for catching encoding, date, and referential issues at compile time.

### 7.2 Crate Selection

| Purpose | Crate | Rationale |
|---------|-------|-----------|
| HTTP client | reqwest | De facto standard for Rust HTTP; async with tokio runtime |
| Async runtime | tokio | Most mature async runtime; integrates with reqwest |
| Encoding | encoding_rs | Reference implementation; zero-copy decoders producing valid UTF-8 directly; cp1252->UTF-8 is deterministic and lossless |
| Serialization | serde | Ubiquitous for Serialize/Deserialize derives |
| Enums | strum | Clean derive macros for enum conversion from/to strings |
| SQLite | rusqlite | De facto standard for SQLite in Rust; supports transactions, prepared statements, WAL mode, UDFs |
| Migrations | refinery | Versioned migration files (V1__, V2__ naming); integrates with rusqlite |
| Decimal | rust_decimal | Wrapper for f64 to avoid floating-point precision issues in financial calculations |
| Date/Time | chrono (NaiveDate) | NaiveDate handles date-only values; dedicated parser for DD/MM/YYYY and YYYYMMDD formats |
| Error handling | thiserror | Clean error enum derivation for structured error types |
| Hashing | sha2 | SHA-256 for file integrity verification |
| CLI | clap or structopt | Command-line argument parsing |

### 7.3 Change Detection Strategy

SHA-256 of file content is the only reliable change detection method given server limitations (no ETag, no Last-Modified, no Content-Length in HEAD). Comparison of size or scraping the HTML page are insufficient or unreliable.

| Approach | Feasibility | Reliability | Verdict |
|----------|-------------|-------------|---------|
| ETag / If-None-Match | Not available | N/A | Discarded |
| Last-Modified / If-Modified-Since | Not available | N/A | Discarded |
| HEAD + Content-Length | Not available | N/A | Discarded |
| SHA-256 of content | Full: download and hash | 100%: any change detected | **Recommended** |
| HTML page date | Possible via scraping | Partial: page may not update immediately | Complementary |
| File size comparison | Simple but imprecise | Low: change can keep same size | Insufficient |

---

## 8. Implementation Phasing Suggestions

### 8.1 Phase Ordering Rationale

Phase 1 (fetch) must be functional before Phase 2 (parse), which must be validated before Phase 3 (SQLite). Phases 4 and 5 can be partially parallelized, but Phase 5 depends on Phase 3. Phase 6 (API) is the end goal but must wait until the database is stable and complete. A key milestone is the end of Phase 3, which marks availability of a SQLite database exploitable by third-party tools.

### 8.2 Detailed Phase Plan

| Phase | Objective | Deliverable | Duration |
|-------|-----------|-------------|----------|
| **1. Foundation** | bdpm-core: types, enums, config, traits. bdpm-fetch: download + hash + archiving | CLI binary that downloads and archives all 11 files with JSON manifest | 1-2 weeks |
| **2. Parsing** | bdpm-parse: cp1252/latin-1/utf-8 decoding, TSV split, date/number/apostrophe normalization | CLI binary that parses all 11 files and exports valid JSON/CSV | 2-3 weeks |
| **3. SQLite Database** | bdpm-db: schema, migrations, batch insertion, import_log, indexes | Complete SQLite database queryable with 11 tables + import logs | 1-2 weeks |
| **4. Validation** | bdpm-validate: referential checks, enumerations, temporal coherence, regression | HTML/CLI validation report after each import | 1-2 weeks |
| **5. Incremental** | Incremental import: hash diff, upsert, soft delete, notifications | Complete pipeline that updates database without full reconstruction | 2 weeks |
| **6. API (future)** | REST/GraphQL layer: actix-web or axum, OpenAPI, full-text search | Documented HTTP API with search, filtering, pagination | 3-4 weeks |

### 8.3 Parsing Sequence Recommendation

Parse files in this order:
1. **CIS_bdpm.txt** first (simplest and most central)
2. **CIS_CIP_bdpm.txt** second (the only utf-8 among cp1252 files)
3. **HAS files** (CIS_HAS_SMR, CIS_HAS_ASMR) next (YYYYMMDD date format)
4. **Special cases last:** CIS_CPD (empty lines), CIS_InfoImportantes (embedded HTML), CIS_CIP_Dispo_Spec (latin-1)

### 8.4 Pre-Implementation Validations

Before launching implementation, the following must be validated:

1. **Schema stability:** Archive files across multiple consecutive months; compare structures; monitor download page for file additions/removals; implement schema_version in parser configuration
2. **Server behavior under load:** Respectful load tests (one file at a time, 5-second interval, no parallel requests); determine rate limiting, acceptable User-Agent, and Referer
3. **ANSM staging site:** rec-bdm.ansm.integra.fr may preview schema changes; observe without automatic download
4. **MySQL dump existence:** Verify if betagouv/infomedicament MySQL dump is publicly available; may contain additional data (ATC codes, images, RCP text)
5. **CIS_CPD empty lines:** Determine if 9 empty lines are one-time anomaly or recurring; adapt filtering strategy accordingly
6. **Rust crate absence:** Reconfirm no BDPM parsing crate exists on crates.io or GitHub; reserve crate name bdpm or bdpm-parser early

---

## Key Data Quality Findings

- **Encoding map:** cp1252 for CIS_bdpm, CIS_COMPO, CIS_CPD, CIS_GENER, CIS_HAS_ASMR, CIS_HAS_SMR, CIS_MITM; latin-1 for CIS_CIP_Dispo_Spec; utf-8 for CIS_CIP_bdpm, CIS_InfoImportantes, HAS_LiensPageCT
- **Date format map:** DD/MM/YYYY for CIS_bdpm, CIS_CIP_bdpm, CIS_InfoImportantes, CIS_CIP_Dispo_Spec; YYYYMMDD for CIS_HAS_SMR, CIS_HAS_ASMR
- **Decimal format:** Comma separator throughout CIS_CIP_bdpm; no period decimal found
- **Smart quotes:** U+2019 (apostrophe courbe) appears 51,957 times across HAS files; must normalize to U+0027
- **HTML entities:** CIS_InfoImportantes contains 421 distinct HTML tags requiring extraction
- **Orphan rates:** Up to 23.5% (CIS_GENER), requiring nullable or unconstrained foreign key design
- **Empty line bug:** CIS_CPD contains 9 \r\r\n sequences generating parasitic empty lines
- **Dynamic file:** CIS_InfoImportantes timestamped per download; requires separate handling

---

*Study compiled from BDPM feasibility analysis. Total source data: 27.6 Mo across 11 files, 142,266 total lines.*
