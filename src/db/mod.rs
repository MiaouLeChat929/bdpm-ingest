pub mod fts;

pub use fts::rebuild_fts;

use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

/// Open a database connection with PRAGMAs optimized for read-heavy API usage.
///
/// Sets WAL mode, busy_timeout (5s), and foreign_keys ON.
/// Every API handler should use this instead of `Connection::open()` directly.
pub fn open_api_conn(path: &std::path::Path) -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL; \
         PRAGMA busy_timeout=5000; \
         PRAGMA foreign_keys=ON;",
    )?;
    Ok(conn)
}

pub fn init_db(path: &std::path::Path) -> Connection {
    let mut conn = Connection::open(path).expect("Failed to open database");

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;")
        .expect("Failed to set PRAGMA");

    let migrations = Migrations::new(vec![
        M::up(include_str!("migrations/001_initial.sql")),
        M::up(include_str!("migrations/002_fix_smr_asmr_constraints.sql")),
        M::up(include_str!("migrations/003_remove_mitm_fk.sql")),
        M::up(include_str!("migrations/004_populate_atc_codes.sql")),
        M::up(include_str!("migrations/005_safety_alerts.sql")),
        M::up(include_str!("migrations/006_clean_data.sql")),
        M::up(include_str!("migrations/007_fix_fts5_schema.sql")),
    ]);
    migrations.to_latest(&mut conn).expect("Migration failed");

    fix_smr_asmr_constraints(&mut conn);

    // Create FTS5 virtual table and sync triggers
    fts::create_fts_tables(&conn).ok();

    tracing::info!("Database initialized at {}", path.display());
    conn
}

/// Recreate SMR/ASMR tables with correct CHECK constraints covering all actual level values.
/// SQLite doesn't support ALTER CONSTRAINT, so we drop+recreate.
fn fix_smr_asmr_constraints(conn: &mut Connection) {
    let _ = conn.execute_batch("PRAGMA foreign_keys=OFF;");

    match conn.transaction() {
        Ok(tx) => {
            let _ = tx.execute("CREATE TABLE smr_backup AS SELECT * FROM smr", []);
            let _ = tx.execute("CREATE TABLE asmr_backup AS SELECT * FROM asmr", []);
            let _ = tx.execute("DROP TABLE smr", []);
            let _ = tx.execute("DROP TABLE asmr", []);

            let _ = tx.execute_batch(
                "CREATE TABLE smr (\
                    cis TEXT NOT NULL, ct_id TEXT NOT NULL, decision_type TEXT, decision_date TEXT, \
                    level TEXT CHECK (level IN (\
                        'Important','Important conditionnel','Faible','Faible conditionnel',\
                        'Insuffisant','Insuffisant à HAS','Inscription (CT)','Légèrement important',\
                        'Modéré','Modéré conditionnel','Modérée','Non précisé',\
                        'Pas d''avis disponible','Commentaires'\
                    )), avis TEXT, is_orphan INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (cis, ct_id)\
                )"
            );
            let _ = tx.execute("CREATE INDEX idx_smr_cis ON smr(cis)", []);
            let _ = tx.execute("CREATE INDEX idx_smr_level ON smr(level)", []);
            let _ = tx.execute("CREATE INDEX idx_smr_orphan ON smr(is_orphan)", []);
            let _ = tx.execute("CREATE INDEX idx_smr_date ON smr(decision_date)", []);

            let _ = tx.execute_batch(
                "CREATE TABLE asmr (\
                    cis TEXT NOT NULL, ct_id TEXT NOT NULL, decision_type TEXT, decision_date TEXT, \
                    level TEXT CHECK (level IN (\
                        'I','II','III','IV','V','III bis','IV bis','V bis',\
                        'V dans l''attente de données','Commentaires',\
                        'Commentaires sans chiffrage de l''ASMR'\
                    )), avis TEXT, is_orphan INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (cis, ct_id)\
                )"
            );
            let _ = tx.execute("CREATE INDEX idx_asmr_cis ON asmr(cis)", []);
            let _ = tx.execute("CREATE INDEX idx_asmr_level ON asmr(level)", []);
            let _ = tx.execute("CREATE INDEX idx_asmr_orphan ON asmr(is_orphan)", []);
            let _ = tx.execute("CREATE INDEX idx_asmr_date ON asmr(decision_date)", []);

            let _ = tx.execute("INSERT INTO smr SELECT * FROM smr_backup", []);
            let _ = tx.execute("INSERT INTO asmr SELECT * FROM asmr_backup", []);
            let _ = tx.execute("DROP TABLE smr_backup", []);
            let _ = tx.execute("DROP TABLE asmr_backup", []);
            let _ = tx.commit();
        }
        Err(_) => {
            tracing::warn!("SMR/ASMR constraint fix skipped — could not acquire transaction");
        }
    }

    let _ = conn.execute_batch("PRAGMA foreign_keys=ON;");
}

pub fn optimize_for_bulk_insert(conn: &Connection) {
    let _ = conn.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=OFF; PRAGMA cache_size=-64000; PRAGMA temp_store=MEMORY;"
    );
}

pub fn restore_normal_settings(conn: &Connection) {
    let _ = conn.execute_batch("PRAGMA synchronous=NORMAL; PRAGMA cache_size=-2000;");
}