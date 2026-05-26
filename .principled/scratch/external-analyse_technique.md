# BDMP Technical Analysis Summary

**Source:** BDPM_Base de Donnees Publique des Medicaments  
**Analysis Date:** 26 May 2026  
**Data Volume:** 11 TSV files, ~26.3 MB, 144,829 records

This document provides a comprehensive technical analysis extracted from the original 933-line BDPM technical analysis, organized by domain for rapid reference during Rust project implementation.

---

## 1. Technical Architecture Recommendations

### Core Architecture
- **CLI-first modular design** with clear separation: fetch → parse → transform → store → monitor
- **Layered monitoring** with three detection layers (SHA-256 hash primary, data.gouv.fr API secondary, HTML scraping fallback)
- **Adaptive polling strategy** respecting file-specific update cadences

### Module Structure
```
bdpm-tools/
├── src/
│   ├── main.rs              // CLI entry point
│   ├── config.rs            // Configuration (URLs, intervals)
│   ├── error.rs             // Centralized error types
│   ├── fetch/               // Download + politeness layer
│   │   ├── mod.rs
│   │   ├── encoding.rs      // Encoding detection/normalization
│   │   └── hash.rs          // SHA-256 calculation
│   ├── parse/               // TSV paring per file type
│   │   ├── mod.rs
│   │   ├── cis.rs          // Specialites parser
│   │   ├── cip.rs          // Presentations parser
│   │   ├── compo.rs        // Compositions parser
│   │   ├── has.rs          // SMR/ASMR avis parser
│   │   ├── gener.rs        // Generiques parser
│   │   ├── dispo.rs        // Disruptions parser
│   │   └── normalise.rs    // Dates, apostrophes, percentages
│   ├── db/                  // Database layer
│   │   ├── mod.rs          // Schema + SQLite connection
│   │   ├── import.rs       // Base insertion
│   │   └── query.rs        // Predefined queries
│   └── monitor/            // Monitoring orchestrator
│       ├── mod.rs
│       ├── datagouv.rs     // data.gouv.fr API client
│       └── scheduler.rs    // Adaptive polling + jitter
├── migrations/
│   └── 001_init.sql        // Initial schema
└── tests/
    ├── test_parse_cis.rs
    ├── test_encoding.rs
    └── test_normalise.rs
```

### CLI Subcommands
| Command | Description |
|---------|-------------|
| `bdpm fetch` | Download all BDPM files (or modified only) |
| `bdpm import` | Parse and import into SQLite |
| `bdpm update` | Execute fetch + import (full workflow) |
| `bdpm monitor` | Launch adaptive monitoring daemon |
| `bdpm status` | Display current database state |
| `bdpm check` | Verify database integrity |
| `bdpm search` | Full-text search (FTS5) |
| `bdpm export` | Export to JSON/CSV/Parquet |

---

## 2. Schema Definitions and Database Design

### Primary Table: `specialites` (Master table)
```sql
CREATE TABLE specialites (
   code_cis TEXT PRIMARY KEY,                    -- 8 digits
   denomination TEXT NOT NULL,
   forme_pharmaceutique TEXT NOT NULL,
   voies_administration TEXT,                     -- separated by "; "
   statut_amm TEXT NOT NULL,                     -- enum: active/abrogee/archivee/retiree/suspendue
   type_procedure TEXT NOT NULL,
   etat_commercialisation TEXT NOT NULL,         -- Commercialisee / Non commercialisee
   date_amm TEXT NOT NULL,                       -- ISO 8601: YYYY-MM-DD
   statut_bdm TEXT,                              -- Alerte / Warning disponibilite / NULL
   numero_autorisation_europeenne TEXT,
   titulaire TEXT,                                -- separated by "; "
   surveillance_renforcee TEXT NOT NULL CHECK(surveillance_renforcee IN ('Oui','Non'))
);
CREATE INDEX idx_specialites_statut ON specialites(statut_amm);
CREATE INDEX idx_specialites_commercialisation ON specialites(etat_commercialisation);
CREATE INDEX idx_specialites_date_amm ON specialites(date_amm);
```

### `presentations` (Product presentations)
```sql
CREATE TABLE presentations (
   code_cip7 TEXT PRIMARY KEY,                    -- 7 digits
   code_cis TEXT NOT NULL REFERENCES specialites(code_cis),
   libelle TEXT NOT NULL,
   statut_presentation TEXT NOT NULL,
   etat_commercialisation TEXT,
   date_declaration TEXT,                         -- ISO 8601
   code_cip13 TEXT NOT NULL UNIQUE,              -- 13 digits (prefix 34009)
   agrement_collectivites TEXT CHECK(agrement_collectivites IN ('oui','non')),
   taux_remboursement TEXT,                      -- normalized: "XX%" no space
   prix_euros REAL,
   prix_honoraires_inclus REAL,
   honoraires REAL,
   indications_remboursement TEXT                -- raw text (HTML cleaned)
);
CREATE INDEX idx_presentations_cis ON presentations(code_cis);
CREATE INDEX idx_presentations_cip13 ON presentations(code_cip13);
```

### `compositions` (Drug compositions)
```sql
CREATE TABLE compositions (
   id INTEGER PRIMARY KEY AUTOINCREMENT,
   code_cis TEXT NOT NULL REFERENCES specialites(code_cis),
   forme_pharmaceutique TEXT,
   code_substance TEXT NOT NULL,                 -- 5 digits with leading zero
   nom_substance TEXT NOT NULL,                  -- uppercase
   dosage TEXT,
   reference_dosage TEXT,
   nature_composant TEXT NOT NULL CHECK(nature_composant IN ('SA','FT')),
   numero_liaison INTEGER                        -- SA/FT link
);
CREATE INDEX idx_compositions_cis ON compositions(code_cis);
CREATE INDEX idx_compositions_substance ON compositions(code_substance);
```

### `avis_smr` (HAS SMR evaluations)
```sql
CREATE TABLE avis_smr (
   id INTEGER PRIMARY KEY AUTOINCREMENT,
   code_cis TEXT NOT NULL REFERENCES specialites(code_cis),
   code_dossier_has TEXT NOT NULL,             -- format CT-NNNNN
   motif_evaluation TEXT NOT NULL,
   date_avis TEXT NOT NULL,                    -- ISO 8601
   valeur_smr TEXT NOT NULL,
   libelle_smr TEXT NOT NULL
);
CREATE INDEX idx_avis_smr_cis ON avis_smr(code_cis);
```

### `avis_asmr` (HAS ASMR evaluations)
```sql
CREATE TABLE avis_asmr (
   id INTEGER PRIMARY KEY AUTOINCREMENT,
   code_cis TEXT NOT NULL REFERENCES specialites(code_cis),
   code_dossier_has TEXT NOT NULL,
   motif_evaluation TEXT NOT NULL,
   date_avis TEXT NOT NULL,                    -- ISO 8601
   valeur_asmr TEXT NOT NULL CHECK(valeur_asmr IN ('I','II','III','IV','V')),
   libelle_asmr TEXT NOT NULL
);
CREATE INDEX idx_avis_asmr_cis ON avis_asmr(code_cis);
```

### `liens_page_ct` (HAS transparency commission links)
```sql
CREATE TABLE liens_page_ct (
   code_dossier_has TEXT NOT NULL,
   lien_url TEXT NOT NULL,
   PRIMARY KEY (code_dossier_has, lien_url)
);
```

### `groupes_generiques` (Generic groups)
```sql
CREATE TABLE groupes_generiques (
   id INTEGER PRIMARY KEY AUTOINCREMENT,
   identifiant_groupe INTEGER NOT NULL,
   libelle_groupe TEXT NOT NULL,
   code_cis TEXT NOT NULL REFERENCES specialites(code_cis),
   type_generique INTEGER NOT NULL CHECK(type_generique IN (0,1,2,4)),
   numero_tri INTEGER
);
CREATE INDEX idx_generiques_cis ON groupes_generiques(code_cis);
CREATE INDEX idx_generiques_groupe ON groupes_generiques(identifiant_groupe);
```

### `conditions_prescription` (Prescription conditions)
```sql
CREATE TABLE conditions_prescription (
   id INTEGER PRIMARY KEY AUTOINCREMENT,
   code_cis TEXT NOT NULL REFERENCES specialites(code_cis),
   condition TEXT NOT NULL
);
CREATE INDEX idx_conditions_cis ON conditions_prescription(code_cis);
```

### `ruptures_stock` (Stock disruptions)
```sql
CREATE TABLE ruptures_stock (
   id INTEGER PRIMARY KEY AUTOINCREMENT,
   code_cis TEXT NOT NULL REFERENCES specialites(code_cis),
   code_cip13 TEXT,                            -- NULL if disruption at specialty level
   identifiant_disponibilite INTEGER NOT NULL CHECK(identifiant_disponibilite BETWEEN 1 AND 4),
   etat_disponibilite TEXT NOT NULL,           -- normalized: initial case
   date_debut TEXT NOT NULL,                   -- ISO 8601
   date_maj TEXT,                             -- ISO 8601
   date_fin TEXT,                             -- ISO 8601, NULL if ongoing
   lien_ansm TEXT
);
CREATE INDEX idx_ruptures_cis ON ruptures_stock(code_cis);
```

### `medicaments_mitm` (MITM drugs)
```sql
CREATE TABLE medicaments_mitm (
   code_cis TEXT PRIMARY KEY REFERENCES specialites(code_cis),
   code_atc TEXT NOT NULL,                     -- 7 characters (e.g., R03BA01)
   denomination TEXT NOT NULL,
   lien_bdpm TEXT NOT NULL                     -- HTTPS URL
);
CREATE INDEX idx_mitm_atc ON medicaments_mitm(code_atc);
```

### `infos_securite` (Safety information)
```sql
CREATE TABLE infos_securite (
   id INTEGER PRIMARY KEY AUTOINCREMENT,
   code_cis TEXT NOT NULL REFERENCES specialites(code_cis),
   date_debut TEXT NOT NULL,                  -- ISO 8601
   date_fin TEXT,                            -- ISO 8601, NULL if still active
   texte_html TEXT,                           -- raw HTML content
   url_info TEXT                              -- extracted URL from HTML
);
CREATE INDEX idx_infos_securite_cis ON infos_securite(code_cis);
```

### Metadata Tables
```sql
-- Import tracking
CREATE TABLE import_metadata (
   fichier TEXT PRIMARY KEY,
   date_import TEXT NOT NULL,                 -- ISO 8601 datetime
   hash_sha256 TEXT NOT NULL,
   nombre_lignes INTEGER NOT NULL,
   taille_octets INTEGER NOT NULL,
   encodage_detecte TEXT NOT NULL,           -- e.g., "windows-1252"
   anomalies TEXT                             -- JSON of detected anomalies
);

-- Update audit log
CREATE TABLE update_history (
   id INTEGER PRIMARY KEY AUTOINCREMENT,
   date_detection TEXT NOT NULL,
   fichiers_modifies TEXT NOT NULL,          -- JSON array
   source_detection TEXT NOT NULL,            -- "hash" / "datagouv" / "page_scrape"
   action TEXT NOT NULL,                      -- "downloaded" / "skipped" / "error"
   detail TEXT
);
```

### Schema Design Decisions
1. **CIS codes as TEXT** (not INTEGER) to preserve leading zeros
2. **All dates normalized to ISO 8601** for native SQLite date comparisons
3. **import_metadata** enables precise import state tracking
4. **update_history** provides complete audit trail

---

## 3. Data Processing Pipeline Details

### Stage 1: Fetch (Download)
- HTTP client with descriptive User-Agent header
- 2-second interval between consecutive requests
- SHA-256 hash calculated during download (stream hashing)
- Saves files temporarily with hash computed inline
- Polite behavior: single connection at a time

### Stage 2: Encoding Detection
- Inspect file bytes to determine encoding
- Decode using `encoding_rs` crate:
  - Windows-1252 for Windows-1252/Latin-1 files
  - UTF-8 for UTF-8 files
  - ASCII for ASCII files
- Normalize to UTF-8 for SQLite storage
- **Critical:** Replace `\u{2019}` (right single quotation mark) with standard apostrophe `'` after decoding Windows-1252 files (HAS files contain 22,253 and 29,704 instances respectively)

### Stage 3: TSV Parsing
- Split on tab characters
- Remove `\r` characters to handle mixed line endings
- Filter empty lines (especially around rows 27672-27677 in CIS_CPD)
- Clean each field: strip leading/trailing spaces, normalize percentages, convert dates to ISO 8601
- Validate column count per row; log anomalies

### Stage 4: SQLite Import
- Transaction-based insertion with `PRAGMA foreign_keys = ON`
- `INSERT OR IGNORE` for duplicate handling
- Cross-reference validation before commit

### Stage 5: Validation
- Verify record counts per table match expected ranges
- Check for unexpected NULL values
- Validate foreign key integrity
- Generate anomaly report

---

## 4. API Design Suggestions

### Local API Layer (Postgre-SQLite maturity)
- **Rust crates:** `axum` or `actix-web`
- Expose REST or GraphQL interface for:
  - Drug search by name, substance, CIS code
  - SMR/ASMR evaluation retrieval
  - Price and reimbursement information
  - Stock disruption status
  - Safety alerts

### Export Formats
- JSON (structured, API-friendly)
- CSV (spreadsheet-compatible)
- Parquet (analytics-optimized)

### Full-Text Search
- Enable SQLite FTS5 on fields:
  - `denomination` (specialties)
  - `libelle` (presentations)
  - `motif_avis` (HAS evaluations)
  - `nom_substance` (substances)
- Expose FTS capabilities via API

---

## 5. Edge Cases, Gotchas, and Anomalies

### Critical Issues (4)

| Issue | Detail | Resolution |
|-------|--------|------------|
| **Mixed encodings** | 9 files Latin-1/Windows-1252, 2 files UTF-8 | Always attempt Windows-1252 first; superset includes Windows-specific characters |
| **Curly apostrophes 0x92** | 22,253 (SMR) and 29,704 (ASMR) occurrences | Replace `\u{2019}` with `'` after Windows-1252 decode |
| **Inconsistent date formats** | DD/MM/YYYY, YYYYMMDD, YYYY-MM-DD | Normalize all to ISO 8601 (YYYY-MM-DD) during import |
| **Inconsistent line endings** | 9 files CRLF, 2 files LF, 1 file mixed | Strip all `\r` characters; filter empty lines |

### Moderate Issues (6)

| Issue | Detail | Resolution |
|-------|--------|------------|
| **Leading spaces in fields** | CIS_bdpm col 10: 15,847/15,848 lines; col 2: 1,134 lines | Systematic strip of leading/trailing spaces |
| **Inconsistent percentage format** | "100 %" vs "100%" without space | Normalize to "XX%" without space |
| **Case inconsistency** | "Remise a disposition" (R) vs "remise a disposition" (r) | Normalize to initial caps or all lowercase |
| **Ghost empty lines** | `\r\r\n` creates 3 phantom lines ~rows 27672-27677 in CIS_CPD | Filter empty lines; log anomalies |
| **Duplicate rows** | CIS_CIP_Dispo_Spec.txt: 17 exact duplicates | Deduplicate at import (keep first occurrence) |
| **HTML in data** | `<a>` tags with `&ecirc;` in InfoImportantes; `<br>` in CIP | Parse with lightweight HTML parser; extract text + URL separately |

### Minor Issues (5)

| Issue | Detail | Resolution |
|-------|--------|------------|
| **Orphan CIS codes** | 2,806 SMR orphans, 1,567 ASMR orphans, 2,503 GENER orphans | Create archive reference table; use LEFT JOIN with NULL |
| **No ETag/Last-Modified** | Conditional HTTP requests impossible | Hash-based change detection |
| **Dynamic InfoImportantes name** | Content-Disposition includes timestamp | Strip timestamp when saving locally |
| **HTTP URLs in MITM** | Uses `http://` instead of `https://` | Replace with HTTPS equivalent |
| **No column header row** | Files lack header rows | Use BDPM specification for column names |

### Referential Integrity Cross-Reference

| File | CIS unique | In master | Orphans | Missing from file |
|------|-----------|-----------|---------|-------------------|
| CIS_CIP | 14,573 | 14,569 | 4 | 1,279 |
| CIS_COMPO | 15,846 | 15,846 | 0 | 2 |
| CIS_HAS_SMR | 9,014 | 6,208 | 2,806 | 9,640 |
| CIS_HAS_ASMR | 6,172 | 4,605 | 1,567 | 11,243 |
| CIS_GENER | 10,628 | 8,125 | 2,503 | 7,723 |
| CIS_CPD | 12,493 | 12,492 | 1 | 3,356 |
| CIS_CIP_Dispo | 727 | 715 | 12 | 15,133 |
| CIS_MITM | 7,711 | 7,711 | 0 | 8,137 |
| CIS_InfoImportantes | 4,208 | 3,243 | 965 | 12,605 |

**Note:** Orphans represent archived/revoked specialties with preserved historical evaluations.

### HTTP Header Anomalies
- No ETag or Last-Modified headers
- Transfer-Encoding: chunked (no Content-Length available)
- Cache-Control: private, must-revalidate + Pragma: no-cache + Expires: 0
- X-Frame-Options duplicated (DENY + SAMEORIGIN)
- X-Content-Type-Options duplicated
- CSP includes dynamic nonce (changes every request)

---

## 6. Technology Stack Recommendations

### Core Crates

| Crate | Version | Purpose |
|-------|---------|---------|
| `reqwest` | 0.12.x | Async HTTP client with TLS |
| `tokio` | 1.x | Async runtime |
| `sha2` | 0.10.x | SHA-256 hashing for change detection |
| `rusqlite` | 0.31.x | SQLite bindings with FTS5 support |
| `encoding_rs` | 0.8.x | Windows-1252/Latin-1 to UTF-8 decoding |
| `chrono` | 0.4.x | Date parsing (DD/MM/YYYY, YYYYMMDD, YYYY-MM-DD) |
| `scraper` | 0.20.x | HTML parsing for page scraping |
| `serde` + `serde_json` | 1.x | JSON parsing for data.gouv.fr API |
| `clap` | 4.x | CLI with subcommands |
| `tracing` | 0.1.x | Structured logging |
| `thiserror` | 1.x | Idiomatic error handling |
| `rand` | 0.8.x | Jitter for polling intervals |

### Recommended Crate Features
```toml
rusqlite = { version = "0.31", features = ["bundled"] }  # Include SQLite in binary
```

### URL Pattern Discovery
- **Old pattern** (broken): `/telechargement?fich=XXX`
- **New pattern**: `/download/file/XXX` (10 files), `/download/XXX` (CIS_InfoImportantes.txt only)

---

## 7. Testing and Validation Strategies

### Unit Tests Required
- **Encoding tests**: Verify all encoding transitions (Windows-1252 → UTF-8, Latin-1 → UTF-8, UTF-8 → UTF-8)
- **Date parsing tests**: Cover all three date formats with edge cases
- **Normalize tests**: Apostrophe replacement, percentage normalization, space stripping
- **Parse tests per file type**: Verify column extraction, row counting, anomaly detection

### Integration Tests
- End-to-end import of sample files
- Verify record counts match source data
- Check foreign key constraint enforcement
- Validate anomaly report generation

### Validation Queries (for `bdpm check`)
```sql
-- Orphan detection
SELECT 'orphaned_smr' AS issue, COUNT(*) AS count
FROM avis_smr WHERE code_cis NOT IN (SELECT code_cis FROM specialites);

-- NULL check
SELECT 'null_denomination' AS issue, COUNT(*) AS count
FROM specialites WHERE denomination IS NULL;

-- Duplicate detection
SELECT code_cip7, COUNT(*) AS count
FROM presentations GROUP BY code_cip7 HAVING count > 1;

-- Foreign key integrity
SELECT 'fk_violation' AS issue, COUNT(*) AS count
FROM presentations p LEFT JOIN specialites s ON p.code_cis = s.code_cis
WHERE s.code_cis IS NULL;
```

### Monitoring Validation
- Verify hash comparison correctness
- Test adaptive polling interval calculation
- Simulate server responses (200, 304, error codes)
- Validate jitter application (±20% of interval)

---

## 8. Rust+SQLite+rouille Stack Assessment

### Alignment with Recommendations
The analysis **strongly supports** the Rust+SQLite choice with specific clarifications:

### Confirmed Positive Aspects
1. **rusqlite** with `bundled` feature: embedding SQLite in the binary is the correct approach for a portable CLI tool
2. **encoding_rs** compatibility: Rust ecosystem has excellent encoding support (encoding_rs is itself a Rust crate)
3. **chrono** crate: Ideal for handling the three Date formats
4. **tokio + reqwest**: Async runtime works well for HTTP polling with polite intervals
5. **Small binary consideration**: rusqlite's bundled feature keeps binary lean compared to PostgreSQL dependency

### Corrections/Refinements Requested

| Original Assumption | Correction |
|--------------------|------------|
| Consider PostgreSQL long-term | NOT recommended for BDPM use case; SQLite with FTS5 is sufficient and more portable |
| Generic encoding handling | Must specifically prioritize Windows-1252 over UTF-8 (not fallback order) |
| Simple date parsing | Must handle three distinct formats; chrono is mandatory, not optional |

### Storage Estimate
- Current data: 26.3 MB TSV compressed → ~50-80 MB SQLite (estimate)
- Index overhead: +30-40% beyond data size
- Total estimated: ~100-120 MB for complete database
- **Conclusion:** SQLite handles this data volume trivially; PostgreSQL overhead not justified

### Performance Characteristics
- Single-user local database: SQLite optimal
- FTS5 full-text search: SQLite adequate (not Elasticsearch-scale but sufficient for drug search)
- No concurrent write requirements: SQLite ACID sufficient
- Embedded deployment: SQLite wins over PostgreSQL

---

## File Inventory Summary

| File | Content | Rows | Cols | Size (KB) | Encoding | Line Endings | Update Date |
|------|---------|------|------|-----------|----------|--------------|-------------|
| CIS_bdpm.txt | Specialites | 15,848 | 12 | 3,091 | Latin-1 | CRLF | 28/04/2026 |
| CIS_CIP_bdpm.txt | Presentations | 20,903 | 13 | 4,054 | UTF-8 | LF | 25/05/2026 |
| CIS_COMPO_bdpm.txt | Compositions | 32,389 | 8 | 2,670 | Latin-1 | CRLF | 28/04/2026 |
| CIS_HAS_SMR_bdpm.txt | SMR Avis | 15,257 | 6 | 4,388 | Win-1252 | CRLF | 28/04/2026 |
| CIS_HAS_ASMR_bdpm.txt | ASMR Avis | 9,906 | 6 | 4,375 | Win-1252 | CRLF | 28/04/2026 |
| HAS_LiensPageCT_bdpm.txt | CT Liens | 10,342 | 2 | 499 | ASCII | CRLF | 28/04/2026 |
| CIS_GENER_bdpm.txt | Generiques | 10,704 | 5 | 1,188 | Latin-1 | CRLF | 28/04/2026 |
| CIS_CPD_bdpm.txt | Conditions | 28,154 | 2 | 1,283 | Latin-1 | Mixed | 28/04/2026 |
| CIS_CIP_Dispo_Spec.txt | Ruptures | 766 | 8 | 165 | Latin-1 | CRLF | 19/05/2026 |
| CIS_MITM.txt | MITM | 7,711 | 4 | 1,110 | Latin-1 | CRLF | 09/03/2026 |
| CIS_InfoImportantes.txt | Safety Info | 10,189 | 4 | 4,121 | UTF-8 | LF | Dynamic |

---

## Adaptive Polling Schedule

| Period | Interval | Justification |
|--------|----------|---------------|
| First week of month | Every 6 hours | Typical monthly update window |
| Second week of month | Every 12 hours | Update still possible |
| Rest of month | Every 24 hours | Unlikely to change |
| Post-update detection | Every 2 hours for 48h | Capture subsequent corrections |
| InfoImportantes (dynamic) | Every 6 hours | Real-time generated file |

---

## Key Implementation Priorities

1. **Priority 1:** Robust import pipeline (encoding → apostrophes → dates → validation)
2. **Priority 2:** Normalized SQLite schema with foreign keys enabled
3. **Priority 3:** Adaptive monitoring with three-layer detection
4. **Priority 4:** Local API + full-text search + export capabilities

---

*Generated from BDPM_Analyse_Technique_Final.txt (933 lines) analysis dated 26 May 2026*
