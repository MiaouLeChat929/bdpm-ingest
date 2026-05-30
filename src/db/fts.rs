use rusqlite::Connection;

/// Returns true if the SQLite version supports the trigram tokenizer (3.35+).
/// rusqlite 0.31 bundles SQLite 3.45, which includes trigram support.
fn trigram_available() -> bool {
    rusqlite::version_number() >= 3_035_000
}

/// Create FTS5 virtual table for drug full-text search.
///
/// Standalone FTS5 (no content= table) — the triggers handle all sync.
/// Uses `trigram` tokenizer (with `unicode61` fallback) and `remove_diacritics=1`
/// for accent-insensitive search. Trigram enables substring/partial matching.
///
/// Columns: name_raw (original), name (clean), atc_code, form, lab_name, substance_name.
pub fn create_fts_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    let tokenizer = if trigram_available() {
        "trigram remove_diacritics 1"
    } else {
        "unicode61 remove_diacritics 1"
    };
    let sql = format!(
        r#"
        DROP TABLE IF EXISTS drugs_fts;

        CREATE VIRTUAL TABLE IF NOT EXISTS drugs_fts USING fts5(
            cis,
            name_raw,
            name,
            atc_code,
            form,
            lab_name,
            substance_name,
            tokenize='{}'
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
        "#,
        tokenizer
    );
    conn.execute_batch(&sql)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigram_available() {
        // Should be true for rusqlite bundled SQLite3.35+ (rusqlite 0.31 bundles 3.45)
        assert!(trigram_available());
    }

    #[test]
    fn test_trigram_partial_match() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE VIRTUAL TABLE drugs_fts USING fts5(
                cis, name, tokenize='trigram remove_diacritics 1'
            )"
        ).unwrap();
        conn.execute(
            "INSERT INTO drugs_fts(cis, name) VALUES ('1', 'Doliprane')",
            [],
        ).unwrap();

        // Trigram enables substring matching
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM drugs_fts WHERE drugs_fts MATCH 'dol'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}

