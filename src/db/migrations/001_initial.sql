-- BDPM Database Schema
-- Migration 001: Initial schema
-- All 10 BDPM tables + import tracking
-- Target: SQLite 3.x (WAL mode)
-- Note: PRAGMAs set in init_db(), not here (SQLite disallows PRAGMA inside transaction)

-- =============================================================================
-- CORE TABLE: drugs
-- CIS = unique drug identifier (French product code)
-- =============================================================================
CREATE TABLE drugs (
    cis                 TEXT    PRIMARY KEY,

    -- Core identification
    name                TEXT    NOT NULL,
    form                TEXT,
    route               TEXT,
    auth_status         TEXT,
    procedure_type      TEXT,
    comm_status         TEXT,

    -- Dates (ISO-8601)
    auth_date           TEXT,

    -- Identifiers
    lab_name            TEXT,
    is_patent           INTEGER NOT NULL DEFAULT 0,      -- 0=Non, 1=Oui

    -- Warnings/metadata
    alert_type          TEXT,                            -- nullable (2254 rows carry data)
    eu_number           TEXT,                            -- nullable, malformed slashes stripped

    -- Generic group (informational)
    generic_group_id    TEXT,
    generic_sort        INTEGER,
    generic_type        TEXT,                            -- 0=ref,1=gen,2=cross,4=LP

    -- ATC (derived from CIS_MITM)
    atc_code            TEXT,
    atc_url             TEXT,

    -- Audit
    imported_at         DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_drugs_name           ON drugs(name);
CREATE INDEX idx_drugs_atc           ON drugs(atc_code);
CREATE INDEX idx_drugs_generic_group ON drugs(generic_group_id);
CREATE INDEX idx_drugs_lab           ON drugs(lab_name);

-- =============================================================================
-- TABLE: presentations
-- One drug (CIS) can have multiple CIP codes (presentations/packages)
-- =============================================================================
CREATE TABLE presentations (
    cis                TEXT    NOT NULL REFERENCES drugs(cis),
    cip                TEXT    PRIMARY KEY,               -- 7-digit canonical (34009 stripped)
    cip_raw            TEXT,                              -- raw value from CSV
    labels             TEXT,                              -- packaging description, may contain HTML
    pres_status        TEXT,                              -- commercial status
    comm_status        TEXT,
    comm_date          TEXT,                              -- ISO-8601

    -- EAN-13: 13-digit barcode, starts with 34009
    ean13              TEXT    UNIQUE,

    -- Reimbursement
    reimbursable       TEXT,                              -- "oui"/"non"
    reimb_rate         REAL,                              -- normalized: 0.65, 1.0 (not "65%")

    -- Prices (cents, NULL if non-commercialisé)
    prix_ht_cents      INTEGER,
    prix_ville_cents   INTEGER,
    prix_rate_cents    INTEGER,

    -- Conditions
    reimb_conditions   TEXT                               -- free-text, may contain HTML
);

CREATE INDEX idx_presentations_cis ON presentations(cis);
CREATE INDEX idx_presentations_ean13 ON presentations(ean13);

-- =============================================================================
-- TABLE: compositions
-- Substance breakdown per drug (after dedup: 27609 rows from 32389)
-- PK = (cis, substance_code, seq) -- seq handles dedup sequence number
-- =============================================================================
CREATE TABLE compositions (
    cis                TEXT    NOT NULL REFERENCES drugs(cis),
    form_label         TEXT,                              -- stripped
    substance_code     TEXT    NOT NULL,                  -- "42215" — TEXT preserves leading zeros
    substance_name     TEXT,
    dosage             TEXT,                              -- "1,00 mg"
    per_unit           TEXT,
    pharm_code         TEXT    CHECK (pharm_code IN ('SA', 'FT')),
    seq                INTEGER NOT NULL,                  -- dedup sequence number

    PRIMARY KEY (cis, substance_code, seq)
);

CREATE INDEX idx_compo_cis          ON compositions(cis);
CREATE INDEX idx_compo_substance   ON compositions(substance_code);

-- =============================================================================
-- TABLE: generic_groups
-- Generic substitution groups (informational, FK relaxed for withdrawn drugs)
-- =============================================================================
CREATE TABLE generic_groups (
    group_id           TEXT    NOT NULL,                  -- TEXT: "31", "968"
    group_name         TEXT,
    cis                TEXT    NOT NULL,
    type               TEXT    CHECK (type IN ('reference', 'generic', 'cross-group', 'sustained-release')),  -- normalized from 0/1/2/4
    sort_order         INTEGER,
    is_orphan          INTEGER NOT NULL DEFAULT 0,       -- 1 if CIS not in drugs (withdrawn drug)

    PRIMARY KEY (group_id, cis)
);

CREATE INDEX idx_gengroup_groupid ON generic_groups(group_id);
CREATE INDEX idx_gengroup_cis    ON generic_groups(cis);
CREATE INDEX idx_gengroup_orphan ON generic_groups(is_orphan);

-- =============================================================================
-- TABLE: prescription_rules
-- Per-drug prescription rules (multi-row per CIS)
-- =============================================================================
CREATE TABLE prescription_rules (
    cis                TEXT    NOT NULL REFERENCES drugs(cis),
    rule               TEXT    NOT NULL,

    PRIMARY KEY (cis, rule)
);

CREATE INDEX idx_rxrules_cis ON prescription_rules(cis);

-- =============================================================================
-- TABLE: smr (Service Medical Rendu)
-- HAS medical benefit ratings (orphans: 2806 CIS = 18.4%)
-- FK relaxed — references withdrawn drugs
-- =============================================================================
CREATE TABLE smr (
    cis                TEXT    NOT NULL,
    ct_id              TEXT    NOT NULL,                  -- unique HAS dossier ID
    decision_type      TEXT,
    decision_date      TEXT,                              -- ISO-8601 parsed from YYYYMMDD
    level              TEXT    CHECK (level IN (
                        'Important',
                        'Modéré',
                        'Modérée',
                        'Faible',
                        'Insuffisant',
                        'Insuffisant à HAS',
                        'Pas d''avis disponible',
                        'Légèrement important'
                    )),
    avis               TEXT,                              -- HTML-stripped, max ~2048 chars
    is_orphan          INTEGER NOT NULL DEFAULT 0,       -- 1 if CIS not in drugs (withdrawn drug)

    PRIMARY KEY (cis, ct_id)
);

CREATE INDEX idx_smr_cis       ON smr(cis);
CREATE INDEX idx_smr_level     ON smr(level);
CREATE INDEX idx_smr_orphan    ON smr(is_orphan);
CREATE INDEX idx_smr_date      ON smr(decision_date);

-- =============================================================================
-- TABLE: asmr (Amélioration du Service Médical Rendu)
-- HAS improvement ratings (orphans: 1567 CIS = 15.8%)
-- FK relaxed — references withdrawn drugs
-- =============================================================================
CREATE TABLE asmr (
    cis                TEXT    NOT NULL,
    ct_id              TEXT    NOT NULL,                  -- unique HAS dossier ID
    decision_type      TEXT,
    decision_date      TEXT,
    level              TEXT    CHECK (level IN (
                        'I', 'II', 'III', 'IV', 'V',
                        'III bis', 'IV bis', 'V bis'
                    )),
    avis               TEXT,
    is_orphan          INTEGER NOT NULL DEFAULT 0,       -- 1 if CIS not in drugs (withdrawn drug)

    PRIMARY KEY (cis, ct_id)
);

CREATE INDEX idx_asmr_cis      ON asmr(cis);
CREATE INDEX idx_asmr_level    ON asmr(level);
CREATE INDEX idx_asmr_orphan   ON asmr(is_orphan);
CREATE INDEX idx_asmr_date     ON asmr(decision_date);

-- =============================================================================
-- TABLE: availability
-- Stock status (live, weekly refresh)
-- Status codes: 1=Rupture, 2=Tension, 3=Arrêt, 4=Remise
-- =============================================================================
CREATE TABLE availability (
    cis                TEXT    NOT NULL,
    cip                TEXT,                              -- empty string valid here
    status_type        INTEGER NOT NULL CHECK (status_type IN (1, 2, 3, 4)),
    status             TEXT,
    date_start         TEXT,                              -- ISO-8601
    date_end           TEXT,                              -- ISO-8601 (nullable)
    date_remise        TEXT,                              -- ISO-8601 (nullable)
    source_url         TEXT,

    PRIMARY KEY (cis, status_type, date_start)
);

CREATE INDEX idx_avail_cis       ON availability(cis);
CREATE INDEX idx_avail_status    ON availability(status_type);
CREATE INDEX idx_avail_cip      ON availability(cip);

-- =============================================================================
-- TABLE: atc_codes
-- WHO Anatomical Therapeutic Chemical classification hierarchy
-- Pure taxonomy lookup — drug names come from drugs table via mitm join
-- PK: atc_code TEXT PRIMARY KEY
-- =============================================================================
CREATE TABLE atc_codes (
    atc_code           TEXT    PRIMARY KEY,               -- 7-char (specific) or 5-char (group)
    parent_5_char      TEXT,                              -- 5-char parent (e.g., N01AE from N01AB)
    parent_3_char      TEXT,                              -- 3-char parent (e.g., N01A from N01AE)
    parent_1_char      TEXT                               -- 1-char parent (e.g., N from N01A)
);

CREATE INDEX idx_atc_parent_5  ON atc_codes(parent_5_char);
CREATE INDEX idx_atc_parent_3  ON atc_codes(parent_3_char);
CREATE INDEX idx_atc_parent_1  ON atc_codes(parent_1_char);

-- =============================================================================
-- TABLE: mitm  (CIS ↔ ATC junction)
-- Source: CIS_MITM.txt — maps each drug (CIS) to its ATC classification
-- Each CIS has exactly one ATC in current file; designed for 1:N (drugs can have multiple ATC)
-- drug_name from CIS_MITM = marketed name → use drugs.name via join, not stored here
-- NOTE: FK to atc_codes(atc_code) removed in migration 003 (atc_codes empty; populate in Phase 2)
-- =============================================================================
CREATE TABLE mitm (
    cis                TEXT    NOT NULL,                   -- FK to drugs(cis)
    atc_code           TEXT    NOT NULL,                   -- no FK: validated against BDPM source data
    detail_url         TEXT,                              -- BDPM detail URL for this CIS-ATC pair
    PRIMARY KEY (cis, atc_code)
);

CREATE INDEX idx_mitm_cis      ON mitm(cis);
CREATE INDEX idx_mitm_atc      ON mitm(atc_code);

-- =============================================================================
-- TABLE: has_links
-- HAS CT page URLs (external links from SMR/ASMR tables)
-- =============================================================================
CREATE TABLE has_links (
    ct_id              TEXT    PRIMARY KEY,
    url                TEXT
);

-- =============================================================================
-- TABLE: import_log
-- Import audit trail for change detection and rollback verification
-- =============================================================================
CREATE TABLE import_log (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    file_name         TEXT    NOT NULL,
    file_hash         TEXT    NOT NULL,                   -- BLAKE3 hash
    file_size         INTEGER NOT NULL,
    row_count         INTEGER NOT NULL,
    status            TEXT    NOT NULL,                   -- success/partial/failed
    bad_rows          INTEGER DEFAULT 0,
    skipped_rows      INTEGER DEFAULT 0,
    imported_at       DATETIME DEFAULT CURRENT_TIMESTAMP,
    duration_ms       INTEGER
);

CREATE INDEX idx_import_log_file    ON import_log(file_name, imported_at DESC);
CREATE INDEX idx_import_log_date    ON import_log(imported_at DESC);
CREATE INDEX idx_import_log_status  ON import_log(status);