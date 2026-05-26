//! Sync orchestrator — detects file changes and runs targeted imports.
//!
//! Public API:
//! - `detect_changes()` — fetch + hash all files, return change plans (for dry-run)
//! - `run_sync()` — full sync: detect changes, import only what changed (or all if full=true)
//! - `run_dispo_sync()` — sync only the weekly availability file

use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

use crate::download::{state::StateStore, BDPMFile, BDPM_URL, Fetcher};
use crate::import::{run_import, ImportReport};

/// Why a file is included in the sync plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeReason {
    /// First time seeing this file (no prior state entry).
    NewFile,
    /// Content hash changed — file content differs from last sync.
    HashChanged,
    /// File size differs from last sync (possible truncated download).
    SizeChanged,
}

/// Represents a planned sync for one file that has changed.
#[derive(Clone, Debug)]
pub struct SyncPlan {
    /// Which BDPM file changed.
    pub file: BDPMFile,
    /// Why it is in the plan.
    pub reason: ChangeReason,
    /// BLAKE3 hex digest of the downloaded content.
    pub hash: String,
    /// Size in bytes.
    pub size: u64,
}

/// Detect which files have changed since the last sync.
///
/// Fetches and hashes every file, then compares against stored state.
/// Returns a `SyncPlan` for each file that differs from the stored state.
///
/// Use this for a dry-run / preview before calling `run_sync`.
pub fn detect_changes(
    data_dir: &Path,
    state: &StateStore,
) -> Result<Vec<SyncPlan>> {
    let raw_dir = data_dir.join("raw");
    std::fs::create_dir_all(&raw_dir)?;
    let fetcher = Fetcher::new();

    let mut plans = Vec::new();

    for file in BDPMFile::all() {
        let url = format!("{}{}", BDPM_URL, file.download_path());
        let bytes = fetcher.fetch(&url, &raw_dir)?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let size = bytes.len() as u64;

        if state.needs_update(&file, &hash, size) {
            let reason = match state.files.get(file.filename()) {
                None => ChangeReason::NewFile,
                Some(prev) => {
                    if prev.size_bytes != size {
                        ChangeReason::SizeChanged
                    } else {
                        ChangeReason::HashChanged
                    }
                }
            };
            plans.push(SyncPlan {
                file,
                reason,
                hash,
                size,
            });
        }
    }

    Ok(plans)
}

/// Run full sync — import only changed files (unless `full` is true).
///
/// This is the main entry point used by the `Sync` CLI command.
/// Internally delegates to `run_import` in the `import` module.
///
/// - `full: true` — force re-import of all files, ignore hash checks.
/// - `file_filter: Some(name)` — import only the named file (e.g. `"CIS_CIP_Dispo_Spec.txt"`).
/// - `file_filter: None` — import all files that have changed.
pub fn run_sync(
    conn: &mut Connection,
    data_dir: &Path,
    state: &mut StateStore,
    full: bool,
    file_filter: Option<&str>,
) -> Result<ImportReport> {
    run_import(conn, data_dir, state, full, file_filter)
}

/// Sync only the weekly availability file (CIS_CIP_Dispo_Spec).
///
/// Convenience wrapper — equivalent to `run_sync` with `file_filter = "CIS_CIP_Dispo_Spec.txt"`.
pub fn run_dispo_sync(
    conn: &mut Connection,
    data_dir: &Path,
    state: &mut StateStore,
) -> Result<ImportReport> {
    run_import(conn, data_dir, state, false, Some("CIS_CIP_Dispo_Spec.txt"))
}