-- Migration 003: Remove FK constraint from mitm
-- Problem: atc_codes is empty (Phase 2: populate from WHO ATC taxonomy)
-- mitm has FK to atc_codes(atc_code) which blocks all inserts
-- Solution: remove FK, validate atc_code against BDPM source data at data level
--
-- SQLite doesn't support ALTER TABLE DROP CONSTRAINT, so recreate the table.

-- Backup existing mitm data
CREATE TABLE mitm_backup AS SELECT * FROM mitm;

-- Drop and recreate without FK (atc_code still NOT NULL, just no REFERENCES clause)
DROP TABLE mitm;

CREATE TABLE mitm (
    cis                TEXT    NOT NULL,                   -- FK to drugs(cis)
    atc_code           TEXT    NOT NULL,                   -- no FK: validated against BDPM source data
    detail_url         TEXT,                              -- BDPM detail URL for this CIS-ATC pair
    PRIMARY KEY (cis, atc_code)
);

CREATE INDEX idx_mitm_cis ON mitm(cis);
CREATE INDEX idx_mitm_atc ON mitm(atc_code);

-- Restore data
INSERT INTO mitm SELECT * FROM mitm_backup;
DROP TABLE mitm_backup;