-- BDPM Database Schema (consolidated from migrations 001-007)
-- Single initialization file — no migration runner in dev mode.
-- ATC codes are populated during import (no INSERT data here).
-- FTS5 is created by fts.rs.

-- =============================================================================
-- CORE TABLE: drugs
-- =============================================================================
CREATE TABLE IF NOT EXISTS drugs (
    cis                 TEXT    PRIMARY KEY,
    name                TEXT    NOT NULL,
    form                TEXT,
    route               TEXT,
    auth_status         TEXT,
    procedure_type      TEXT,
    comm_status         TEXT,
    auth_date           TEXT,
    lab_name            TEXT,
    is_patent           INTEGER NOT NULL DEFAULT 0,
    alert_type          TEXT,
    eu_number           TEXT,
    atc_code            TEXT,
    imported_at         DATETIME DEFAULT CURRENT_TIMESTAMP,
    name_raw            TEXT
);

CREATE INDEX IF NOT EXISTS idx_drugs_name ON drugs(name);
CREATE INDEX IF NOT EXISTS idx_drugs_atc ON drugs(atc_code);
CREATE INDEX IF NOT EXISTS idx_drugs_lab ON drugs(lab_name);
CREATE INDEX IF NOT EXISTS idx_drugs_name_sort ON drugs(name);
CREATE INDEX IF NOT EXISTS idx_drugs_name_raw_sort ON drugs(name_raw);

-- =============================================================================
-- TABLE: presentations
-- =============================================================================
CREATE TABLE IF NOT EXISTS presentations (
    cis                TEXT    NOT NULL REFERENCES drugs(cis),
    cip                TEXT    PRIMARY KEY CHECK (cip IS NULL OR (LENGTH(cip) = 7 AND cip GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9]')),
    cip_raw            TEXT,
    labels             TEXT,
    pres_status        TEXT,
    comm_status        TEXT,
    comm_date          TEXT,
    ean13              TEXT    UNIQUE,
    reimbursable       TEXT,
    reimb_rate         REAL,
    prix_ht_cents      INTEGER,
    prix_ville_cents   INTEGER,
    prix_rate_cents   INTEGER,
    labels_clean      TEXT,
    is_orphan          INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_presentations_cis ON presentations(cis);
CREATE INDEX IF NOT EXISTS idx_presentations_ean13 ON presentations(ean13);
CREATE INDEX IF NOT EXISTS idx_presentations_orphan ON presentations(is_orphan);

-- =============================================================================
-- TABLE: compositions
-- =============================================================================
CREATE TABLE IF NOT EXISTS compositions (
    cis                TEXT    NOT NULL REFERENCES drugs(cis),
    form_label         TEXT,
    substance_code     TEXT    NOT NULL,
    substance_name     TEXT,
    dosage             TEXT,
    per_unit           TEXT,
    pharm_code         TEXT    CHECK (pharm_code IN ('SA', 'FT')),
    seq                INTEGER NOT NULL,
    dosage_mg          REAL,
    substance_name_clean TEXT,
    is_orphan          INTEGER NOT NULL DEFAULT 0,

    PRIMARY KEY (cis, substance_code, seq)
);

CREATE INDEX IF NOT EXISTS idx_compo_cis ON compositions(cis);
CREATE INDEX IF NOT EXISTS idx_compo_substance ON compositions(substance_code);
CREATE INDEX IF NOT EXISTS idx_compo_dosage ON compositions(dosage_mg);
CREATE INDEX IF NOT EXISTS idx_compo_substance_clean ON compositions(substance_name_clean);
CREATE INDEX IF NOT EXISTS idx_compo_orphan ON compositions(is_orphan);

-- =============================================================================
-- TABLE: generic_groups
-- =============================================================================
CREATE TABLE IF NOT EXISTS generic_groups (
    group_id           TEXT    NOT NULL,
    group_name         TEXT,
    cis                TEXT    NOT NULL,
    type               TEXT    CHECK (type IN ('reference', 'generic', 'cross-group', 'sustained-release')),
    sort_order         INTEGER,
    is_orphan          INTEGER NOT NULL DEFAULT 0,

    PRIMARY KEY (group_id, cis)
);

CREATE INDEX IF NOT EXISTS idx_gengroup_groupid ON generic_groups(group_id);
CREATE INDEX IF NOT EXISTS idx_gengroup_cis ON generic_groups(cis);
CREATE INDEX IF NOT EXISTS idx_gengroup_orphan ON generic_groups(is_orphan);
CREATE INDEX IF NOT EXISTS idx_gengroup_order ON generic_groups(group_id, sort_order, cis);

-- =============================================================================
-- TABLE: prescription_rules
-- =============================================================================
CREATE TABLE IF NOT EXISTS prescription_rules (
    cis                TEXT    NOT NULL REFERENCES drugs(cis),
    rule               TEXT    NOT NULL,

    PRIMARY KEY (cis, rule)
);

CREATE INDEX IF NOT EXISTS idx_rxrules_cis ON prescription_rules(cis);

-- =============================================================================
-- TABLE: prescription_flags
-- =============================================================================
CREATE TABLE IF NOT EXISTS prescription_flags (
    cis TEXT PRIMARY KEY REFERENCES drugs(cis),
    liste_i INTEGER NOT NULL DEFAULT 0,
    liste_ii INTEGER NOT NULL DEFAULT 0,
    stupefiant INTEGER NOT NULL DEFAULT 0,
    hospitalier INTEGER NOT NULL DEFAULT 0,
    dentaire INTEGER NOT NULL DEFAULT 0,
    reserve_hopital INTEGER NOT NULL DEFAULT 0
);

-- =============================================================================
-- TABLE: smr (Service Medical Rendu)
-- =============================================================================
CREATE TABLE IF NOT EXISTS smr (
    cis                TEXT    NOT NULL,
    ct_id              TEXT    NOT NULL,
    decision_type      TEXT,
    decision_date      TEXT,
    level              TEXT    CHECK (level IN (
                        'Important', 'Important conditionnel', 'Modéré', 'Modérée',
                        'Faible', 'Faible conditionnel', 'Insuffisant', 'Insuffisant à HAS',
                        'Pas d''avis disponible', 'Légèrement important',
                        'Inscription (CT)', 'Non précisé', 'Commentaires'
                    )),
    avis               TEXT,
    is_orphan          INTEGER NOT NULL DEFAULT 0,

    PRIMARY KEY (cis, ct_id)
);

CREATE INDEX IF NOT EXISTS idx_smr_cis ON smr(cis);
CREATE INDEX IF NOT EXISTS idx_smr_level ON smr(level);
CREATE INDEX IF NOT EXISTS idx_smr_orphan ON smr(is_orphan);
CREATE INDEX IF NOT EXISTS idx_smr_date ON smr(decision_date);

-- =============================================================================
-- TABLE: asmr (Amélioration du Service Médical Rendu)
-- =============================================================================
CREATE TABLE IF NOT EXISTS asmr (
    cis                TEXT    NOT NULL,
    ct_id              TEXT    NOT NULL,
    decision_type      TEXT,
    decision_date      TEXT,
    level              TEXT    CHECK (level IN (
                        'I', 'II', 'III', 'IV', 'V',
                        'III bis', 'IV bis', 'V bis',
                        'V dans l''attente de données', 'Commentaires',
                        'Commentaires sans chiffrage de l''ASMR'
                    )),
    avis               TEXT,
    is_orphan          INTEGER NOT NULL DEFAULT 0,

    PRIMARY KEY (cis, ct_id)
);

CREATE INDEX IF NOT EXISTS idx_asmr_cis ON asmr(cis);
CREATE INDEX IF NOT EXISTS idx_asmr_level ON asmr(level);
CREATE INDEX IF NOT EXISTS idx_asmr_orphan ON asmr(is_orphan);
CREATE INDEX IF NOT EXISTS idx_asmr_date ON asmr(decision_date);

-- =============================================================================
-- TABLE: availability
-- =============================================================================
CREATE TABLE IF NOT EXISTS availability (
    cis                TEXT    NOT NULL,
    cip                TEXT,
    status_type        INTEGER NOT NULL CHECK (status_type IN (1, 2, 3, 4)),
    status             TEXT,
    date_start         TEXT,
    date_end           TEXT,
    date_remise        TEXT,
    source_url         TEXT,

    PRIMARY KEY (cis, status_type, date_start)
);

CREATE INDEX IF NOT EXISTS idx_avail_cis ON availability(cis);
CREATE INDEX IF NOT EXISTS idx_avail_status ON availability(status_type);
CREATE INDEX IF NOT EXISTS idx_avail_cip ON availability(cip);

-- =============================================================================
-- TABLE: atc_codes (populated during import from CIS_MITM)
-- =============================================================================
CREATE TABLE IF NOT EXISTS atc_codes (
    atc_code           TEXT    PRIMARY KEY,
    parent_5_char      TEXT,
    parent_3_char      TEXT,
    parent_1_char      TEXT
);

CREATE INDEX IF NOT EXISTS idx_atc_parent_5 ON atc_codes(parent_5_char);
CREATE INDEX IF NOT EXISTS idx_atc_parent_3 ON atc_codes(parent_3_char);
CREATE INDEX IF NOT EXISTS idx_atc_parent_1 ON atc_codes(parent_1_char);

-- =============================================================================
-- TABLE: mitm
-- =============================================================================
CREATE TABLE IF NOT EXISTS mitm (
    cis                TEXT    NOT NULL,
    atc_code           TEXT    NOT NULL,
    detail_url         TEXT,

    PRIMARY KEY (cis, atc_code)
);

CREATE INDEX IF NOT EXISTS idx_mitm_cis ON mitm(cis);
CREATE INDEX IF NOT EXISTS idx_mitm_atc ON mitm(atc_code);

-- =============================================================================
-- TABLE: has_links
-- =============================================================================
CREATE TABLE IF NOT EXISTS has_links (
    ct_id              TEXT    PRIMARY KEY,
    url                TEXT
);

-- =============================================================================
-- TABLE: safety_alerts
-- =============================================================================
CREATE TABLE IF NOT EXISTS safety_alerts (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    cis                 TEXT NOT NULL,
    start_date          TEXT,
    end_date            TEXT,
    message_plain       TEXT,
    source_url          TEXT,
    imported_at         DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_safety_alerts_cis ON safety_alerts(cis);
CREATE INDEX IF NOT EXISTS idx_safety_alerts_dates ON safety_alerts(start_date, end_date);

-- =============================================================================
-- TABLE: import_log
-- =============================================================================
CREATE TABLE IF NOT EXISTS import_log (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    file_name         TEXT    NOT NULL,
    file_hash         TEXT    NOT NULL,
    file_size         INTEGER NOT NULL,
    row_count         INTEGER NOT NULL,
    status            TEXT    NOT NULL,
    bad_rows          INTEGER DEFAULT 0,
    imported_at       DATETIME DEFAULT CURRENT_TIMESTAMP,
    duration_ms       INTEGER
);

CREATE INDEX IF NOT EXISTS idx_import_log_file ON import_log(file_name, imported_at DESC);
CREATE INDEX IF NOT EXISTS idx_import_log_date ON import_log(imported_at DESC);
CREATE INDEX IF NOT EXISTS idx_import_log_status ON import_log(status);

-- =============================================================================
-- TABLE: quarantine (captures rejected rows for audit and retry)
-- =============================================================================
CREATE TABLE IF NOT EXISTS quarantine (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_file     TEXT    NOT NULL,
    source_line     INTEGER NOT NULL,
    target_table    TEXT    NOT NULL,
    error_type      TEXT    NOT NULL,
    error_detail    TEXT,
    raw_line        TEXT    NOT NULL,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_quarantine_file ON quarantine(source_file);
CREATE INDEX IF NOT EXISTS idx_quarantine_type ON quarantine(error_type);
CREATE INDEX IF NOT EXISTS idx_quarantine_date ON quarantine(created_at);