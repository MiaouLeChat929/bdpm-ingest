pub mod fts;

pub use fts::{create_fts_tables};

use rusqlite::Connection;

/// Open a database connection with PRAGMAs optimized for read-heavy API usage.
pub fn open_api_conn(path: &std::path::Path) -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL; \
         PRAGMA busy_timeout=5000; \
         PRAGMA foreign_keys=ON; \
         PRAGMA locking_mode=NORMAL;",
    )?;
    Ok(conn)
}

/// Initialize database: execute schema.sql and create FTS5 virtual table.
/// Dev mode: always runs --full import after this, so schema is fresh each time.
pub fn init_db(path: &std::path::Path) -> Connection {
    // All failures here are fatal init errors: unrecoverable without a working DB.
    #[expect(clippy::expect_used, reason = "Fatal init: database file must open successfully")]
    let conn = Connection::open(path).expect("Failed to open database");

    #[expect(clippy::expect_used, reason = "Fatal init: WAL+PRAGMA must succeed or the process cannot operate")]
    conn.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;"
    ).expect("Failed to set PRAGMA");

    // Execute consolidated schema (all 7 migrations merged into one SQL file)
    let schema = include_str!("schema.sql");
    #[expect(clippy::expect_used, reason = "Fatal init: schema DDL must succeed")]
    conn.execute_batch(schema).expect("Schema initialization failed");

    // Create FTS5 virtual table and sync triggers
    #[expect(clippy::expect_used, reason = "Fatal init: FTS5 setup must succeed")]
    fts::create_fts_tables(&conn).expect("FTS5 initialization failed");

    tracing::info!("Database initialized at {}", path.display());
    conn
}

/// Optimize for bulk insert (during --full import)
pub fn optimize_for_bulk_insert(conn: &Connection) {
    let _ = conn.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=OFF; PRAGMA cache_size=-128000; PRAGMA temp_store=MEMORY; PRAGMA locking_mode=EXCLUSIVE; PRAGMA wal_autocheckpoint=0;"
    );
}

/// Restore normal settings (after import completes)
pub fn restore_normal_settings(conn: &Connection) {
    let _ = conn.execute_batch("PRAGMA synchronous=NORMAL; PRAGMA cache_size=-2000; PRAGMA wal_checkpoint(TRUNCATE);");
}