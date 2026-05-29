use rusqlite::Connection;

/// Create FTS5 virtual table for drug full-text search.
///
/// Standalone FTS5 (no content= table) — the triggers handle all sync.
/// Columns: name_raw (original), name (clean), atc_code, form, lab_name, substance_name.
pub fn create_fts_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(r#"
        DROP TABLE IF EXISTS drugs_fts;

        CREATE VIRTUAL TABLE IF NOT EXISTS drugs_fts USING fts5(
            cis,
            name_raw,
            name,
            atc_code,
            form,
            lab_name,
            substance_name
        );

        -- Populate from drugs table, aggregating substance names from compositions
        INSERT INTO drugs_fts(cis, name_raw, name, atc_code, form, lab_name, substance_name)
        SELECT d.cis, d.name_raw, d.name, d.atc_code, d.form, d.lab_name,
               COALESCE((SELECT GROUP_CONCAT(substance_name, ' ') FROM compositions WHERE cis = d.cis), '')
        FROM drugs d;

        -- Sync triggers
        CREATE TRIGGER IF NOT EXISTS drugs_ai AFTER INSERT ON drugs BEGIN
            INSERT INTO drugs_fts(cis, name_raw, name, atc_code, form, lab_name, substance_name)
            VALUES (new.cis, new.name_raw, new.name, new.atc_code, new.form, new.lab_name,
                    COALESCE((SELECT GROUP_CONCAT(substance_name, ' ') FROM compositions WHERE cis = new.cis), ''));
        END;

        CREATE TRIGGER IF NOT EXISTS drugs_ad AFTER DELETE ON drugs BEGIN
            DELETE FROM drugs_fts WHERE cis = old.cis;
        END;

        CREATE TRIGGER IF NOT EXISTS drugs_au AFTER UPDATE ON drugs BEGIN
            DELETE FROM drugs_fts WHERE cis = old.cis;
            INSERT INTO drugs_fts(cis, name_raw, name, atc_code, form, lab_name, substance_name)
            VALUES (new.cis, new.name_raw, new.name, new.atc_code, new.form, new.lab_name,
                    COALESCE((SELECT GROUP_CONCAT(substance_name, ' ') FROM compositions WHERE cis = new.cis), ''));
        END;

        -- Composition triggers: update FTS5 substance_name when compositions change
        CREATE TRIGGER IF NOT EXISTS compositions_ai AFTER INSERT ON compositions BEGIN
            DELETE FROM drugs_fts WHERE cis = new.cis;
            INSERT INTO drugs_fts(cis, name_raw, name, atc_code, form, lab_name, substance_name)
            SELECT d.cis, d.name_raw, d.name, d.atc_code, d.form, d.lab_name,
                   COALESCE((SELECT GROUP_CONCAT(substance_name, ' ') FROM compositions WHERE cis = new.cis), '')
            FROM drugs d WHERE d.cis = new.cis;
        END;

        CREATE TRIGGER IF NOT EXISTS compositions_ad AFTER DELETE ON compositions BEGIN
            DELETE FROM drugs_fts WHERE cis = old.cis;
            INSERT INTO drugs_fts(cis, name_raw, name, atc_code, form, lab_name, substance_name)
            SELECT d.cis, d.name_raw, d.name, d.atc_code, d.form, d.lab_name,
                   COALESCE((SELECT GROUP_CONCAT(substance_name, ' ') FROM compositions WHERE cis = old.cis), '')
            FROM drugs d WHERE d.cis = old.cis;
        END;

        CREATE TRIGGER IF NOT EXISTS compositions_au AFTER UPDATE ON compositions BEGIN
            DELETE FROM drugs_fts WHERE cis = new.cis;
            INSERT INTO drugs_fts(cis, name_raw, name, atc_code, form, lab_name, substance_name)
            SELECT d.cis, d.name_raw, d.name, d.atc_code, d.form, d.lab_name,
                   COALESCE((SELECT GROUP_CONCAT(substance_name, ' ') FROM compositions WHERE cis = new.cis), '')
            FROM drugs d WHERE d.cis = new.cis;
        END;
    "#)?;
    Ok(())
}

