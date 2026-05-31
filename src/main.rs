#![allow(dead_code, non_camel_case_types, non_snake_case)]

use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

use anyhow::Result;
use clap::Parser;

mod api;
mod db;
mod download;
mod import;
mod normalize;
mod parse;

use crate::db::init_db;
use crate::download::{Fetcher, fetch_listing_dates, diff_listing_dates, BDPM_URL};
use crate::download::manifest::BDPMFile;
use crate::import::run_ingest;
use crate::api::openapi::ApiDoc;
use utoipa::OpenApi;

#[derive(Parser)]
#[command(name = "bdpm-ingest")]
#[command(about = "BDPM drug database ingest pipeline")]
enum Command {
    /// Fetch all files from BDPM
    Fetch {
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
    },
    /// Full rebuild: drop/create DB, import from raw/, build FTS5
    Ingest {
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
    },
    /// Print row counts and schema summary
    Stats {
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
    },
    /// Print orphan row counts per table
    OrphanStats {
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
    },
    /// Print import log history
    Logs {
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Poll BDPM HTML listing page for file date changes (stateless).
    /// Fetches ~5-10 Ko HTML page, parses embedded per-file update dates, reports changes.
    Poll {
        /// Previous listing dates file to compare against
        #[arg(long)]
        prev: Option<PathBuf>,
    },
    /// Start the HTTP API server for drug search.
    Serve {
        #[arg(long, default_value = "127.0.0.1:8080")]
        addr: String,
        #[arg(long, default_value = "data/bdpm.db")]
        db_path: PathBuf,
    },
    /// Dump OpenAPI spec as YAML to stdout
    DumpOpenApi,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("bdpm_ingest=info".parse()?))
        .init();

    let cmd = Command::parse();

    match cmd {
        Command::Serve { addr, db_path } => {
            if !db_path.exists() {
                anyhow::bail!("Database not found at {}. Run 'bdpm-ingest ingest' first.", db_path.display());
            }
            println!("Starting server on {}...", addr);
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(api::run_server(&addr, db_path));
            return Ok(());
        }

        Command::DumpOpenApi => {
            let yaml = ApiDoc::openapi().to_yaml().unwrap_or_default();
            println!("{}", yaml);
            return Ok(());
        }

        Command::Fetch { data_dir } => {
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(data_dir.join("raw"))?;
            let fetcher = Fetcher::new();

            for file in BDPMFile::all() {
                let url = format!("{}{}", BDPM_URL, file.download_path());
                let bytes = fetcher.fetch(&url, &data_dir.join("raw"))?;
                let hash = blake3::hash(&bytes).to_hex().to_string();
                println!("{}: {} bytes, hash={}", file.filename(), bytes.len(), &hash[..8]);
            }
        }

        Command::Ingest { data_dir } => {
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(data_dir.join("raw"))?;

            let db_path = data_dir.join("bdpm.db");
            let mut conn = init_db(&db_path);

            let report = run_ingest(&data_dir, &mut conn)?;
            report.print();
        }

        Command::Stats { data_dir } => {
            let db_path = data_dir.join("bdpm.db");
            if !db_path.exists() {
                anyhow::bail!("Database not found at {}. Run 'bdpm-ingest ingest' first.", db_path.display());
            }
            let conn = rusqlite::Connection::open(&db_path)?;

            let tables = [
                "drugs", "presentations", "compositions", "generic_groups",
                "prescription_rules", "smr", "asmr", "availability",
                "atc_codes", "mitm", "has_links", "safety_alerts",
            ];

            for table in tables {
                let count: i64 = conn
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                    .unwrap_or(0);
                println!("{}: {}", table, count);
            }
        }

        Command::OrphanStats { data_dir } => {
            let db_path = data_dir.join("bdpm.db");
            if !db_path.exists() {
                anyhow::bail!("Database not found at {}. Run 'bdpm-ingest ingest' first.", db_path.display());
            }
            let conn = rusqlite::Connection::open(&db_path)?;

            let orphan_tables = [
                ("presentations", "presentations"),
                ("compositions", "compositions"),
                ("generic_groups", "generic_groups"),
                ("smr", "smr"),
                ("asmr", "asmr"),
            ];

            println!("{:<20} {:>10} {:>10}", "table", "orphans", "total");
            println!("{}", "-".repeat(55));
            for (label, table) in &orphan_tables {
                let orphans: i64 = conn
                    .query_row(&format!("SELECT COUNT(*) FROM {table} WHERE is_orphan = 1"), [], |r| r.get(0))
                    .unwrap_or(0);
                let total: i64 = conn
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                    .unwrap_or(0);
                let pct = if total > 0 { orphans as f64 / total as f64 * 100.0 } else { 0.0 };
                println!("{:<20} {:>10} {:>10}  {:.1}%", label, orphans, total, pct);
            }
        }

        Command::Logs { data_dir, limit } => {
            let db_path = data_dir.join("bdpm.db");
            if !db_path.exists() {
                anyhow::bail!("Database not found at {}. Run 'bdpm-ingest ingest' first.", db_path.display());
            }
            let conn = rusqlite::Connection::open(&db_path)?;

            let mut stmt = conn.prepare(
                "SELECT file_name, row_count, status, bad_rows, duration_ms, imported_at
                 FROM import_log ORDER BY imported_at DESC LIMIT ?1"
            )?;

            let rows = stmt.query_map([limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?;

            println!("{:<25} {:>8} {:>10} {:>6} {:>8}  imported_at", "file", "rows", "status", "bad", "ms");
            println!("{}", "-".repeat(80));
            for row in rows.flatten() {
                let ms = row.4.map(|m| m.to_string()).unwrap_or_default();
                println!("{:<25} {:>8} {:>10} {:>6} {:>8}  {}", row.0, row.1, row.2, row.3, ms, row.5);
            }
        }

        Command::Poll { prev } => {
            let fetcher = Fetcher::new();

            // Fetch fresh listing page and parse dates
            let fresh = fetch_listing_dates(&fetcher)?;

            // Show all parsed dates
            println!("{:<30} {:<15}", "file", "listing date");
            println!("{}", "-".repeat(50));
            for file in BDPMFile::all() {
                let fname = file.filename();
                let fd = fresh.get(fname).map(|s| s.as_str()).unwrap_or("—");
                println!("{:<30} {:<15}", fname, fd);
            }

            // If previous dates file provided, diff against it
            if let Some(prev_path) = prev {
                if prev_path.exists() {
                    let content = std::fs::read_to_string(&prev_path)?;
                    let stored: std::collections::HashMap<String, String> =
                        serde_json::from_str(&content).unwrap_or_default();

                    let changed = diff_listing_dates(&fresh, &stored);
                    if changed.is_empty() {
                        println!("\nNo changes detected.");
                    } else {
                        println!("\nChanged files: {}",
                            changed.iter().map(|f| f.filename()).collect::<Vec<_>>().join(", "));
                    }
                    return Ok(());
                }
            }

            println!("\nRun with --prev <file> to detect changes from a previous poll.");
        }
    }

    Ok(())
}