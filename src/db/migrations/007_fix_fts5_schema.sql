-- Fix FTS5: drop broken external-content table (name_clean column mismatch)
-- and triggers; recreate_fts_tables() in fts.rs will build the new standalone FTS5.
DROP TRIGGER IF EXISTS drugs_ai;
DROP TRIGGER IF EXISTS drugs_ad;
DROP TRIGGER IF EXISTS drugs_au;
DROP TABLE IF EXISTS drugs_fts;
