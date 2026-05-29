//! Change detection — fetch + hash all files, compare against stored state.

use std::path::Path;

use anyhow::Result;

use crate::download::{state::StateStore, BDPMFile, BDPM_URL, Fetcher};

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
/// Use this for a dry-run / preview before running `import`.
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
