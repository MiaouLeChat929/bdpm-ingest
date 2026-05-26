use anyhow::Result;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

pub fn init_db(path: &std::path::Path) -> Connection {
    let mut conn = Connection::open(path).expect("Failed to open database");

    // WAL mode for concurrent reads during sync
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;")
        .expect("Failed to set PRAGMA");

    // Run migrations
    let migrations = Migrations::new(vec![
        M::up(include_str!("migrations/001_initial.sql")),
    ]);
    migrations.to_latest(&mut conn).expect("Migration failed");

    tracing::info!("Database initialized at {}", path.display());
    conn
}

/// Optimize connection for bulk insert (max write throughput)
pub fn optimize_for_bulk_insert(conn: &Connection) {
    let _ = conn.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=OFF; PRAGMA cache_size=-64000; PRAGMA temp_store=MEMORY;"
    );
}

/// Restore normal settings after bulk insert
pub fn restore_normal_settings(conn: &Connection) {
    let _ = conn.execute_batch("PRAGMA synchronous=NORMAL; PRAGMA cache_size=-2000;");
}
