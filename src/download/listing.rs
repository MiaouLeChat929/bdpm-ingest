//! BDPM HTML listing page fetcher and date extractor.
//!
//! Replicates the polling logic of medicaments-api.giygas.dev independently:
//! - Fetch the HTML listing page (/telechargement/)
//! - Parse embedded per-file update dates from the HTML table
//! - Compare dates to detect changes without downloading files
//!
//! BDPM server provides no ETag, no Last-Modified, no Content-Length on TXT files.
//! The HTML listing page is the only lightweight change-detection signal available.

use std::collections::HashMap;

use regex_lite::Regex;
use std::sync::LazyLock;

use crate::download::manifest::BDPMFile;
use crate::download::Fetcher;

/// The BDPM HTML listing page URL (not the download URL).
pub const LISTING_URL: &str =
    "https://base-donnees-publique.medicaments.gouv.fr/telechargement/";

/// Static regex: find each file entry in the BDPM HTML listing page and extract its date.
///
/// Matches the French Design System table structure:
///   `download='CIS_CIP_Dispo_Spec.txt'` then `(Date de mise à jour : 19/05/2026, 165 Ko)`
/// The `[^)]+` skips the intermediate text without matching the `)` before `(`.
static FILE_DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| {
        // (?i) = case-insensitive, (?s) = dotall (dot matches newlines)
        Regex::new(r"(?is)download='([^']+)'[^)]+\(Date de mise à jour : (\d{2}/\d{2}/\d{4})").unwrap()
    });

/// Fetch the BDPM HTML listing page and extract per-file update dates.
/// Returns a map of filename → DD/MM/YYYY date string.
pub fn fetch_listing_dates(fetcher: &Fetcher) -> anyhow::Result<HashMap<String, String>> {
    let html = fetcher.fetch_text(LISTING_URL)?;
    let mut dates = HashMap::new();

    for cap in FILE_DATE_RE.captures_iter(&html) {
        let filename = cap.get(1).unwrap().as_str().to_string();
        let date = cap.get(2).unwrap().as_str().to_string();
        dates.insert(filename, date);
    }

    if dates.is_empty() {
        tracing::warn!(
            "No file dates extracted from listing page. HTML structure may have changed."
        );
    } else {
        tracing::debug!(
            "Extracted dates for {} files from listing page",
            dates.len()
        );
    }

    if dates.len() < BDPMFile::all().len() {
        tracing::warn!(
            "Listing page returned only {}/{} expected files. HTML structure may have changed.",
            dates.len(),
            BDPMFile::all().len()
        );
    }

    Ok(dates)
}

/// Check which files have changed by comparing fresh listing dates to stored ones.
/// Returns files whose date is newer or absent from stored state.
pub fn diff_listing_dates(
    fresh: &HashMap<String, String>,
    stored: &HashMap<String, String>,
) -> Vec<BDPMFile> {
    let mut changed = Vec::new();

    for file in BDPMFile::all() {
        let fname = file.filename();
        let fresh_date = match fresh.get(fname) {
            Some(d) => d,
            None => continue, // file not in listing
        };
        let stored_date = stored.get(fname);

        let is_new = stored_date.map(|s| s != fresh_date).unwrap_or(true);
        if is_new {
            tracing::info!(
                "{}: listing date changed from {:?} → {}",
                fname,
                stored_date,
                fresh_date
            );
            changed.push(file);
        }
    }

    changed
}
