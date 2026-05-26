use rusqlite::Connection;

/// Create FTS5 virtual table for drug full-text search.
///
/// Uses external content FTS5 (`content='drugs'`) so the FTS table
/// references the actual drugs table without shadowing it.
///
/// Triggers keep drugs_fts in sync with INSERT/UPDATE/DELETE on drugs.
pub fn create_fts_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS drugs_fts USING fts5(
            cis,
            name,
            form,
            lab_name,
            content='drugs',
            content_rowid='rowid'
        );

        -- Populate from drugs table
        INSERT INTO drugs_fts(rowid, cis, name, form, lab_name)
        SELECT rowid, cis, name, form, lab_name FROM drugs;

        -- Sync triggers
        CREATE TRIGGER IF NOT EXISTS drugs_ai AFTER INSERT ON drugs BEGIN
            INSERT INTO drugs_fts(rowid, cis, name, form, lab_name)
            VALUES (new.rowid, new.cis, new.name, new.form, new.lab_name);
        END;

        CREATE TRIGGER IF NOT EXISTS drugs_ad AFTER DELETE ON drugs BEGIN
            INSERT INTO drugs_fts(drugs_fts, rowid, cis, name, form, lab_name)
            VALUES ('delete', old.rowid, old.cis, old.name, old.form, old.lab_name);
        END;

        CREATE TRIGGER IF NOT EXISTS drugs_au AFTER UPDATE ON drugs BEGIN
            INSERT INTO drugs_fts(drugs_fts, rowid, cis, name, form, lab_name)
            VALUES ('delete', old.rowid, old.cis, old.name, old.form, old.lab_name);
            INSERT INTO drugs_fts(rowid, cis, name, form, lab_name)
            VALUES (new.rowid, new.cis, new.name, new.form, new.lab_name);
        END;
    "#)?;
    Ok(())
}