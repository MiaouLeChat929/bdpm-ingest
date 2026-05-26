use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;

mod api;
mod cache;
mod db;
mod download;
mod import;
mod sync;
mod normalize;
mod parse;

use crate::db::init_db;
use crate::download::{state::StateStore, Fetcher, fetch_listing_dates, diff_listing_dates, ListingDates};
use crate::download::manifest::BDPMFile;
use crate::sync::{run_sync, run_dispo_sync, detect_changes};

fn state_path(data_dir: &PathBuf) -> PathBuf {
    data_dir.join("import_state.json")
}

#[derive(Parser)]
#[command(name = "bdpm-ingest")]
#[command(about = "BDPM drug database ingest pipeline")]
enum Command {
    /// Check which files have changed (no download)
    Check {
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
    },
    /// Fetch all files from BDPM
    Fetch {
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
    },
    /// Full pipeline: fetch + parse + validate + normalize + import
    Import {
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
        #[arg(long, short)]
        full: bool,
        #[arg(long)]
        file: Option<String>,
    },
    /// Print row counts and schema summary
    Stats {
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
    /// Poll BDPM HTML listing page for file date changes (no download).
    /// Reimplements medicaments-api.giygas.dev polling logic independently.
    /// Fetches ~5-10 Ko HTML page, parses embedded per-file update dates, reports changes.
    Poll {
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
    },
    /// Detect changed files and print a sync plan (dry run, no import).
    Sync {
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
    },
    /// Sync only the weekly availability file (CIS_CIP_Dispo_Spec).
    Dispo {
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
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
                anyhow::bail!("Database not found at {}. Run 'bdpm-ingest import' first.", db_path.display());
            }
            println!("Starting server on {}...", addr);
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(api::run_server(&addr, db_path));
            return Ok(());
        }

        Command::DumpOpenApi => {
            let yaml = serde_yaml::to_string(&api::openapi::ApiDoc::openapi())
                .unwrap_or_else(|e| e.to_string());
            println!("{}", yaml);
            return Ok(());
        }

        Command::Check { data_dir } => {
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(data_dir.join("raw"))?;
            let state = StateStore::load_or_create(&state_path(&data_dir))?;
            let fetcher = Fetcher::new();

            for file in BDPMFile::all() {
                let url = format!("{}{}", download::BDPM_URL, file.download_path());
                let bytes = fetcher.fetch(&url, &data_dir.join("raw"))?;
                let hash = blake3::hash(&bytes).to_hex().to_string();
                let size = bytes.len() as u64;

                if state.needs_update(&file, &hash, size) {
                    println!("{}: CHANGED", file.filename());
                } else {
                    println!("{}: unchanged", file.filename());
                }
            }
        }

        Command::Fetch { data_dir } => {
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(data_dir.join("raw"))?;
            let fetcher = Fetcher::new();

            for file in BDPMFile::all() {
                let url = format!("{}{}", download::BDPM_URL, file.download_path());
                let bytes = fetcher.fetch(&url, &data_dir.join("raw"))?;
                let hash = blake3::hash(&bytes).to_hex().to_string();
                println!("{}: {} bytes, hash={}", file.filename(), bytes.len(), &hash[..8]);
            }
        }

        Command::Import { data_dir, full, file } => {
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(data_dir.join("raw"))?;

            let mut state = StateStore::load_or_create(&state_path(&data_dir))?;
            let db_path = data_dir.join("bdpm.db");
            let mut conn = init_db(&db_path);

            let report = run_sync(&mut conn, &data_dir, &mut state, full, file.as_deref())?;
            report.print();

            state.save(&state_path(&data_dir))?;
        }

        Command::Stats { data_dir } => {
            let db_path = data_dir.join("bdpm.db");
            if !db_path.exists() {
                anyhow::bail!("Database not found at {}. Run 'bdpm-ingest import' first.", db_path.display());
            }
            let conn = rusqlite::Connection::open(&db_path)?;

            let tables = [
                "drugs", "presentations", "compositions", "generic_groups",
                "prescription_rules", "smr", "asmr", "availability",
                "atc_codes", "mitm", "has_links",
            ];

            for table in tables {
                let count: i64 = conn
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                    .unwrap_or(0);
                println!("{}: {}", table, count);
            }
        }

        Command::Logs { data_dir, limit } => {
            let db_path = data_dir.join("bdpm.db");
            if !db_path.exists() {
                anyhow::bail!("Database not found at {}. Run 'bdpm-ingest import' first.", db_path.display());
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

            println!("{:<25} {:>8} {:>10} {:>6} {:>8}  {}", "file", "rows", "status", "bad", "ms", "imported_at");
            println!("{}", "-".repeat(80));
            for row in rows.flatten() {
                let ms = row.4.map(|m| m.to_string()).unwrap_or_default();
                println!("{:<25} {:>8} {:>10} {:>6} {:>8}  {}", row.0, row.1, row.2, row.3, ms, row.5);
            }
        }

        Command::Poll { data_dir } => {
            std::fs::create_dir_all(&data_dir)?;
            let fetcher = Fetcher::new();

            // Load stored listing dates
            let stored = ListingDates::load(&data_dir)?;

            // Fetch fresh listing page and parse dates
            let fresh = fetch_listing_dates(&fetcher)?;

            // Show all parsed dates
            println!("{:<30} {:<15} {:<15}", "file", "listing date", "stored date");
            println!("{}", "-".repeat(60));
            for file in BDPMFile::all() {
                let fname = file.filename();
                let fd = fresh.get(fname).map(|s| s.as_str()).unwrap_or("—");
                let sd = stored.dates.get(fname).map(|s| s.as_str()).unwrap_or("—");
                let flag = if fresh.get(fname) != stored.dates.get(fname) {
                    " ← CHANGED"
                } else {
                    ""
                };
                println!("{:<30} {:<15} {:<15}{}", fname, fd, sd, flag);
            }

            // Diff and report
            let changed = diff_listing_dates(&fresh, &stored.dates);
            if changed.is_empty() {
                println!("\nNo changes detected.");
            } else {
                println!("\nChanged files: {}",
                    changed.iter().map(|f| f.filename()).collect::<Vec<_>>().join(", "));
            }

            // Save fresh dates to disk
            let fresh_state = ListingDates { dates: fresh };
            fresh_state.save(&data_dir)?;
            println!("Listing dates updated.");
        }

        Command::Sync { data_dir } => {
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(data_dir.join("raw"))?;
            let state = StateStore::load_or_create(&state_path(&data_dir))?;

            let plans = detect_changes(&data_dir, &state)?;
            if plans.is_empty() {
                println!("No changes detected.");
            } else {
                println!("Sync plan ({} files):", plans.len());
                for plan in &plans {
                    let reason = match plan.reason {
                        sync::ChangeReason::NewFile => "NEW",
                        sync::ChangeReason::HashChanged => "HASH_CHANGED",
                        sync::ChangeReason::SizeChanged => "SIZE_CHANGED",
                    };
                    println!("  {} {} {} bytes hash={}", reason, plan.file.filename(), plan.size, &plan.hash[..8]);
                }

                println!("\nRun 'bdpm-ingest import' to import these files.");
            }
        }

        Command::Dispo { data_dir } => {
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(data_dir.join("raw"))?;

            let mut state = StateStore::load_or_create(&state_path(&data_dir))?;
            let db_path = data_dir.join("bdpm.db");
            let mut conn = init_db(&db_path);

            let report = run_dispo_sync(&mut conn, &data_dir, &mut state)?;
            report.print();

            state.save(&state_path(&data_dir))?;
        }
    }

    Ok(())
}
