# External Review MD Catalog — BDMP Project

Created: 2026-05-26

## Files Overview

| # | File | Topic |
|---|------|-------|
| 00 | 00_index.md | Master index + key findings summary |
| 01 | 01_etat_des_lieux_sources.md | State of BDPM data sources |
| 01 | 01_inventaire_des_fichiers.md | Complete file inventory with schemas |
| 02 | 02_analyse_encodage.md | Deep encoding analysis (CP1252/UTF-8) |
| 02 | 02_rapport_qualite_donnees.md | Data quality issues and quirks |
| 03 | 03_methodologie_collecte.md | Data collection methodology |
| 03 | 03_schema_sqlite.md | SQLite schema design (version 1) |
| 04 | 04_strategie_mise_a_jour.md | Update detection strategy |
| 04 | 04_strategie_parsing_normalisation.md | Parsing pipeline with validation |
| 05 | 05_pipeline_transformation.md | Full transformation pipeline code |
| 05 | 05_schema_sqlite.md | SQLite schema design (version 2) |
| 06 | 06_architecture_pipeline_rust.md | Rust pipeline architecture (5 crates) |
| 06 | 06_architecture_rust.md | Rust project structure + CLI |
| 07 | 07_integrite_donnees.md | Referential integrity analysis |
| 07 | 07_roadmap_implementation.md | 6-phase implementation roadmap |
| 08 | 08_apis_communautaires.md | Community APIs and projects |
| 08 | 08_risques_validation.md | Technical risks and validation |

---

## File Summaries

### 00 — Index (00_index.md)

**Topic:** Master documentation index with key findings
**Key findings:**
- 9/11 files use Windows-1252 encoding (only CIS_CIP_bdpm.txt uses UTF-8)
- Zero malformed lines across all 163,451 total records
- 5,388 orphaned CIS codes in HAS/GENER files
- No RSS/API/ETag available — SHA-256 hash comparison is only reliable update detection method
- Two date formats coexist: DD/MM/YYYY and YYYYMMDD
**New info vs PDFs:** Contains consolidated key findings, no new data content.

---

### 01 — État des lieux (01_etat_des_lieux_sources.md)

**Topic:** Comprehensive state of BDPM data sources, organizations, and license
**Key findings:**
- Three source organisms: ANSM (AMM/stock), HAS (SMR/ASMR), Assurance Maladie (pricing)
- License Etalab 2.0 — free reproduction with attribution
- 11 files, ~27.6 MB total, 145,000+ records
- HTTP headers reveal NO ETag, NO Last-Modified, NO Content-Length in HEAD
- Update frequency: monthly declared, but CIS_CIP and Dispo_Spec update more frequently
- CIS_InfoImportantes.txt uses different URL path (/download/ not /download/file/) and is dynamically generated
**New info vs PDFs:** HTTP metadata behavior, cache control analysis, source organization details.

---

### 01 — Inventaire (01_inventaire_des_fichiers.md)

**Topic:** Complete file-by-file inventory with detailed schemas
**Key findings:**
- CIS_bdpm.txt: 15,848 lines, 12 fields, CP1252, CRLF, Code CIS is PK
- CIS_CIP_bdpm.txt: 20,903 lines, 13 fields, **UTF-8**, **LF** only — anomaly
- CIS_CIP_Dispo_Spec.txt: Uses ISO-8859-1 (not CP1252) — distinct from others
- 96% of CIS_CIP_bdpm lines have trailing tabs (creates phantom field 14)
- CIS_InfoImportantes.txt has 421 distinct HTML tags, links to ANSM
- No BOM in any file, no null bytes
**New info vs PDFs:** Precise byte counts, trailing tab analysis, HTML tag inventory.

---

### 02 — Analyse encodage (02_analyse_encodage.md)

**Topic:** Deep analysis of encoding issues with byte-level inventory
**Key findings:**
- CP1252 vs Latin-1 critical difference: byte 0x92 = apostrophe (CP1252) vs C1 control (Latin-1)
- Total 52,168 occurrences of 0x92 across all files (apostrophe)
- CIS_HAS_ASMR_bdpm.txt has highest CP1252 density (29,704 x92 occurrences)
- CIS_HAS files contain: bullets (0x95 = 11,322 total), em-dashes, smart quotes
- UTF-8 files (CIS_CIP) have € symbol and trademark characters as multi-byte sequences
- Recommendation: Hardcode encoding per file, don't use statistical detection
**New info vs PDFs:** Complete byte inventory by file, critical CP1252 vs Latin-1 distinction.

---

### 02 — Rapport qualité (02_rapport_qualite_donnees.md)

**Topic:** Comprehensive data quality issues taxonomy
**Key findings:**
- All 163,451 lines structurally correct (100% field count compliance)
- 31.1% orphaned CIS codes in HAS_SMR, 25.4% in ASMR (historical medications)
- Two date formats confirmed 100% compliant within their respective files
- HTML found in 4,849 lines (libellés SMR/ASMR and indications)
- ¿ character (U+00BF inverted question mark) appears in UTF-8 CIP file — source artifact
- Taux remboursement inconsistent: "65%" vs "65 %" (with/without space)
**New info vs PDFs:** Orphan percentages by table, ¿ character source artifact explanation.

---

### 03 — Méthodologie collecte (03_methodologie_collecte.md)

**Topic:** Respectful data collection methodology
**Key findings:**
- Minimum 5-second delay between HTTP requests (not 2 seconds as stated elsewhere)
- SHA-256 comparison is only reliable change detection method
- Weekly check for standard files, daily for CIS_CIP and Dispo_Spec
- CIS_InfoImportantes: check daily or on-demand (dynamically generated)
- Archive structure: `archive/YYYY-MM-DDTHHMMSS/` with manifest.json
- Manifest includes: sha256, size, HTTP status, download duration, content-disposition
**New info vs PDFs:** Request delay details (5s), archive directory structure format.

---

### 03 — Schema SQLite v1 (03_schema_sqlite.md)

**Topic:** SQLite schema design with 13 tables and metadata
**Key findings:**
- `specialites` table as master with code_cis as PK
- Columns `_import_date` and `_source_hash` for traceability
- `_is_active` column for soft delete (default 1)
- Indexes on frequently queried columns, partial indexes for nullable fields
- `cis_orphelins` VIEW aggregates all orphaned codes across tables
- `import_history` table tracks all imports with sha256, status, row counts
- Estimated total DB size: 60-70 MB with indexes
**New info vs PDFs:** Specific index recommendations, partial index syntax for NULL columns.

---

### 04 — Stratégie mise à jour (04_strategie_mise_a_jour.md)

**Topic:** Update detection and scheduling strategy
**Key findings:**
- No RSS, no API, no ETag — page scrape for date is lightweight trigger
- Three frequencies: Weekly (standard), Daily (CIP/Dispo), Monthly (MITM)
- scraping page date: look for `fr-badge fr-badge--success` element with "Dernière mise à jour"
- Fallback: always download + hash (SHA-256 only reliable method)
- API-Medicaments.fr updates 2x daily (6h, 18h) — can serve as auxiliary change signal
- CIS_InfoImportantes: check Content-Disposition for timestamp, handle empty files
**New info vs PDFs:** CSS selector for date extraction, API-Medicaments.fr as auxiliary signal.

---

### 04 — Stratégie parsing (04_strategie_parsing_normalisation.md)

**Topic:** 5-stage parsing pipeline with quality checks
**Key findings:**
- 5 stages: Fetch+Hash → Decode+Lines → Split+Validate → Normalize → Validate+Insert
- Validation categories: completeness, referential integrity, enum validity, temporal coherence, regression detection
- Quality report JSON format with pass/fail/warn status per check
- Regression detection: alert if record count drops >2% or orphan count rises >10%
- Validation warnings don't block import but are logged
**New info vs PDFs:** Quality check thresholds, JSON report schema format.

---

### 05 — Pipeline transformation (05_pipeline_transformation.md)

**Topic:** Complete transformation pipeline with Rust code
**Key findings:**
- Pipeline order: specialites → presentations → compositions → has_liens_ct → avis_smr → avis_asmr → generiques → conditions → disponibilites → mitm → infos_importantes
- Transaction per file, batch inserts (1000 rows per transaction)
- PRAGMA settings: journal_mode=WAL, synchronous=NORMAL, cache_size=64MB, temp_store=MEMORY
- HTML cleaning: replace <br> with \n, strip other tags, decode HTML entities, handle ¿ character
- Decimal normalization: comma → period via `replace(',', '.')`
**New info vs PDFs:** Transaction batching strategy, PRAGMA tuning, HTML cleaning code.

---

### 05 — Schema SQLite v2 (05_schema_sqlite.md)

**Topic:** Alternative SQLite schema with naming conventions
**Key findings:**
- Uses snake_case naming: `cis_specialites`, `cis_presentations`, `cis_compositions`
- `_raw` columns store original values (dates, decimals) before normalization
- `is_orphan` columns in HAS/GENER tables (integer 0/1)
- `import_log` table with sha256, rows_read/inserted/updated/deleted, duration_ms
- Import strategy: DELETE → INSERT per file within transaction
- FTS5 recommendation for future full-text search
**New info vs PDFs:** Naming convention differences, `_raw` column pattern, FTS5 mention.

---

### 06 — Architecture pipeline (06_architecture_pipeline_rust.md)

**Topic:** 5-crate architecture design
**Key findings:**
- bdpm-core: shared types, enums (using strum crate), FileConfig, traits
- bdpm-fetch: HTTP client, SHA-256, archive management
- bdpm-parse: decoding, TSV split, normalization, HTML extraction
- bdpm-validate: semantic validation, referential checks, regression detection
- bdpm-db: SQLite with refinery migrations, batch upsert, soft delete
- Trait `BdpmRecord` defines: `from_fields()`, `validate()`, `code_cis()`
**New info vs PDFs:** 5-crate decomposition, trait definition, crate responsibilities.

---

### 06 — Architecture Rust (06_architecture_rust.md)

**Topic:** Full Rust project structure and CLI design
**Key findings:**
- Project layout: src/{config,fetch,decode,parse,transform,db,models,pipeline}/
- CLI commands: import (--full/--incremental/--file), check-updates, validate, export, stats, serve
- Dependencies: reqwest, rusqlite (bundled), encoding_rs, chrono, sha2, scraper, clap, tracing
- 6-phase development plan: Foundation → Parsing → SQLite → Validation → Incremental → API
- M1-M6 milestones, M3 (SQLite exploitable) is key deliverable
- Total estimated: 10-15 weeks
**New info vs PDFs:** Directory structure, CLI command inventory, dependency list.

---

### 07 — Intégrité données (07_integrite_donnees.md)

**Topic:** Referential integrity and data quality deep analysis
**Key findings:**
- CIS_bdpm.txt is master with 15,848 CIS codes
- CIS_HAS_SMR: 31.1% orphan rate (2,806 codes) — historical medications
- CIS_HAS_ASMR: 25.4% orphan rate (1,567 codes)
- CIS_GENER: 23.5% orphan rate (2,503 codes)
- CIS_CIP: only 0.03% orphan (4 codes) — timing desync between files
- Explanation: CIS_bdpm.txt only keeps medications marketed or retired <5 years
- 1,017 HAS codes have no corresponding CT page link
- Zero duplicate CIS codes in any file
**New info vs PDFs:** Orphan explanation (5-year retention rule), link coverage percentages.

---

### 07 — Roadmap (07_roadmap_implementation.md)

**Topic:** 6-phase implementation roadmap with dependencies
**Key findings:**
- Phase 1 (1-2w): Foundation — fetcher with SHA-256, archive, manifest
- Phase 2 (2-3w): Parsing — all 11 files, encoding, normalization
- Phase 3 (1-2w): SQLite — complete database, migrations, indexes
- Phase 4 (1-2w): Validation — quality checks, regression detection
- Phase 5 (2w): Incremental — upsert, soft delete, scheduling
- Phase 6 (3-4w): API — REST endpoints, FTS5, OpenAPI docs
- Recommended parsing order: LiensPageCT (2 cols) → MITM → CPD → InfoImportantes → GENER → SMR → ASMR → COMPO → Dispo → CIS → CIP
**New info vs PDFs:** Parsing order by complexity, phase dependencies diagram.

---

### 08 — APIs communautaires (08_apis_communautaires.md)

**Topic:** Ecosystem survey of existing BDPM projects
**Key findings:**
- api-medicaments.fr: REST API, updates 2x/day, rate limit 1000 tokens/IP, 100 req/day free
- api-bdpm-graphql.axel-op.fr: GraphQL API — currently 503 unavailable
- betagouv/api-medicaments: official government project
- betagouv/infomedicament: web search app
- scossin/FrenchSPC: extracts RCP/SmPC documents (future enrichment)
- NCBO BioPortal: BDPM ontology with SPARQL endpoint
- data.gouv.fr: abandoned since 2014 — don't use
**New info vs PDFs:** API availability status, rate limits, BioPortal ontology reference.

---

### 08 — Risques validation (08_risques_validation.md)

**Topic:** Technical risks, blind spots, and architecture decisions
**Key findings:**
- R1 (critical): Schema changes without notification — mitigation: schema_version, alert on column count changes
- R2 (critical): File disappearance — mitigation: HTTP status checks, 11/11 table coverage verification
- R3 (important): Encoding change — mitigation: fallback detector with substitution character counting
- R4 (important): Rate limiting — mitigation: 5s interval, explicit User-Agent, exponential backoff
- R5 (important): Historical data loss — mitigation: soft delete, systematic archiving
- D1: Historical storage — soft delete recommended over shadow tables
- D2: Smart quotes — normalize U+2019 → U+0027 with raw column preservation
- D3: Orphan handling — flag is_orphan=1, no FK constraints
- D4: HTML in InfoImportantes — extract text+URL, keep raw optional
- D5: Update strategy — upsert incremental with soft delete
**New info vs PDFs:** Risk scoring matrix, specific mitigation code snippets, architecture decisions.

---

## Cross-Cutting New Information

The following information appears in the MD files but was NOT in the PDF-extracted TXT files:

1. **HTTP header analysis** — No ETag/Last-Modified/Content-Length in HEAD requests
2. **Byte-level encoding inventory** — Complete breakdown of 0x80-0x9F bytes by file
3. **CP1252 vs Latin-1 distinction** — Critical for apostrophe decoding (0x92)
4. **Orphan explanation** — CIS_bdpm.txt only retains medications <5 years after market withdrawal
5. **Trailing tabs** — 96% of CIS_CIP lines affected, creates phantom field
6. **\r\r\n anomaly** — CIS_CPD has double-CR sequences creating empty lines
7. **¿ character** — Source artifact in UTF-8 file (not encoding error)
8. **API-Medicaments.fr** — External API as auxiliary update signal
9. **Request timing** — 5-second minimum between HTTP requests
10. **HTML tag inventory** — 421 distinct tags in CIS_InfoImportantes
11. **Community project status** — GraphQL API currently 503, data.gouv.fr abandoned since 2014
12. **BioPortal ontology** — SPARQL endpoint for semantic queries
13. **Technical risks** — Comprehensive risk matrix with mitigations
14. **Architecture decisions** — Rationale for soft delete, orphan handling, HTML extraction approach
15. **Phase dependencies** — Which phases depend on which others

---

## Schema Version Differences

Two SQLite schema versions exist:

| Aspect | v1 (03_schema_sqlite.md) | v2 (05_schema_sqlite.md) |
|--------|--------------------------|---------------------------|
| Table naming | `specialites`, `presentations` | `cis_specialites`, `cis_presentations` |
| Raw columns | `_source_hash` | `_raw` suffix pattern |
| Import tracking |分散 | Centralized `import_log` |
| Orphan handling | VIEW only | `is_orphan` column per table |

Recommendation: Use v2 as baseline, incorporate `_source_hash` from v1.
