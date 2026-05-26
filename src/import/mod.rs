//! BDPM import orchestrator
//! Orchestrates: parse → normalize → dedup (compo) → dedup (state check) → import

use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

use crate::db::{init_db, optimize_for_bulk_insert, restore_normal_settings};
use crate::download::manifest::BDPMFile;
use crate::download::state::StateStore;
use crate::download::Fetcher;
use crate::normalize::{dedup_compo, normalize_apostrophes, normalize_row};
use crate::parse::parse_file;

pub const BDPM_URL: &str = "https://base-donnees-publique.medicaments.gouv.fr";

/// Full import orchestrator
pub fn run_import(
    conn: &mut Connection,
    data_dir: &Path,
    state: &mut StateStore,
    full: bool,
    file_filter: Option<&str>,
) -> Result<ImportReport> {
    let raw_dir = data_dir.join("raw");
    std::fs::create_dir_all(&raw_dir)?;
    let fetcher = Fetcher::new();

    // Order respects FK: drugs first (upsert), then all others
    let order = BDPMFile::all();

    let mut report = ImportReport::default();
    let start = std::time::Instant::now();

    for file in order {
        // Filter if --file flag set
        if let Some(f) = file_filter {
            if file.filename() != f {
                continue;
            }
        }

        let start_file = std::time::Instant::now();

        let url = format!("{}{}", BDPM_URL, file.download_path());
        let bytes = fetcher.fetch(&url, &raw_dir)?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let size = bytes.len() as u64;

        if !full && !state.needs_update(&file, &hash, size) {
            report.results.push(FileImportResult {
                file_name: file.filename().to_string(),
                status: ImportStatus::Unchanged,
                rows_imported: 0,
                skipped_rows: 0,
                bad_rows: 0,
                warnings: 0,
                duration_ms: start_file.elapsed().as_millis() as u64,
                error: None,
            });
            continue;
        }

        // Parse
        let path = raw_dir.join(file.filename());
        let raw_rows = match parse_file(&path, file) {
            Ok(rows) => rows,
            Err(e) => {
                report.results.push(FileImportResult {
                    file_name: file.filename().to_string(),
                    status: ImportStatus::Failed,
                    rows_imported: 0,
                    skipped_rows: 0,
                    bad_rows: 0,
                    warnings: 0,
                    duration_ms: start_file.elapsed().as_millis() as u64,
                    error: Some(e.to_string()),
                });
                continue;
            }
        };

        // Normalize
        let mut normalized: Vec<_> = raw_rows
            .iter()
            .map(|row| {
                let mut nr = normalize_row(file, row);
                normalize_apostrophes(&mut nr);
                nr
            })
            .collect();

        // Dedup CIS_COMPO
        if file == BDPMFile::CIS_COMPO_bdpm {
            let before = normalized.len();
            normalized = dedup_compo(normalized);
            tracing::info!("CIS_COMPO dedup: {} → {} rows", before, normalized.len());
        }

        // Import
        match import_file(conn, file, &normalized) {
            Ok(stats) => {
                state.mark_updated(&file, &hash, size);
                let duration = start_file.elapsed().as_millis() as u64;

                // Log to import_log
                let _ = conn.execute(
                    "INSERT INTO import_log (file_name, file_hash, file_size, row_count, status, bad_rows, skipped_rows, duration_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![
                        file.filename(),
                        hash,
                        size as i64,
                        stats.rows_imported as i64,
                        "success",
                        stats.bad_rows as i64,
                        stats.skipped_rows as i64,
                        duration as i64,
                    ],
                );

                report.results.push(FileImportResult {
                    file_name: file.filename().to_string(),
                    status: ImportStatus::Success,
                    rows_imported: stats.rows_imported,
                    skipped_rows: stats.skipped_rows,
                    bad_rows: stats.bad_rows,
                    warnings: stats.warnings,
                    duration_ms: duration,
                    error: None,
                });
            }
            Err(e) => {
                report.results.push(FileImportResult {
                    file_name: file.filename().to_string(),
                    status: ImportStatus::Failed,
                    rows_imported: 0,
                    skipped_rows: 0,
                    bad_rows: 0,
                    warnings: 0,
                    duration_ms: start_file.elapsed().as_millis() as u64,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    report.total_duration_ms = start.elapsed().as_millis() as u64;
    Ok(report)
}

/// Import a single normalized file into its target table.
/// Uses BEGIN transaction for write lock.
fn import_file(
    conn: &mut Connection,
    file: BDPMFile,
    rows: &[crate::normalize::NormalizedRow],
) -> Result<ImportStats> {
    let table = file.target_table();

    // Optimize for bulk insert before transaction
    optimize_for_bulk_insert(conn);

    let result = (|| -> Result<ImportStats> {
        // Wrap in transaction
        let mut tx = conn.transaction()?;

        // Clear existing data (except drugs — upsert preserves references)
        if file != BDPMFile::CIS_bdpm {
            tx.execute(&format!("DELETE FROM {table}"), [])?;
        }

        // Bulk insert — build prepared statement
        let mut stats = ImportStats::default();

        let sql = insert_sql(file);
        let mut stmt = tx.prepare_cached(&sql)?;

        for row in rows {
            let res = match file {
                BDPMFile::CIS_bdpm => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[1].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[2].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[3].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[4].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[5].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[6].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[7].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[8].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[9].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[10].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[11].as_ref().map(|s| s.as_str()).unwrap_or(""),
                    ])
                }
                BDPMFile::CIS_CIP_bdpm => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_ref().map(|s| s.as_str()).unwrap_or(""), v[1].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[2].as_ref().map(|s| s.as_str()).unwrap_or(""), v[3].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[4].as_ref().map(|s| s.as_str()).unwrap_or(""), v[5].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[6].as_ref().map(|s| s.as_str()).unwrap_or(""), v[7].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[8].as_ref().map(|s| s.as_str()).unwrap_or(""), v[9].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[10].as_ref().map(|s| s.as_str()).unwrap_or(""), v[11].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[12].as_ref().map(|s| s.as_str()).unwrap_or(""), v[13].as_ref().map(|s| s.as_str()).unwrap_or(""),
                    ])
                }
                BDPMFile::CIS_COMPO_bdpm => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_ref().map(|s| s.as_str()).unwrap_or(""), v[1].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[2].as_ref().map(|s| s.as_str()).unwrap_or(""), v[3].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[4].as_ref().map(|s| s.as_str()).unwrap_or(""), v[5].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[6].as_ref().map(|s| s.as_str()).unwrap_or(""), v[7].as_ref().map(|s| s.as_str()).unwrap_or(""),
                    ])
                }
                BDPMFile::CIS_HAS_SMR_bdpm | BDPMFile::CIS_HAS_ASMR_bdpm => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_ref().map(|s| s.as_str()).unwrap_or(""), v[1].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[2].as_ref().map(|s| s.as_str()).unwrap_or(""), v[3].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[4].as_ref().map(|s| s.as_str()).unwrap_or(""), v[5].as_ref().map(|s| s.as_str()).unwrap_or(""),
                    ])
                }
                BDPMFile::CIS_GENER_bdpm => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_ref().map(|s| s.as_str()).unwrap_or(""), v[1].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[2].as_ref().map(|s| s.as_str()).unwrap_or(""), v[3].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[4].as_ref().map(|s| s.as_str()).unwrap_or(""),
                    ])
                }
                BDPMFile::CIS_CPD_bdpm | BDPMFile::HAS_LiensPageCT_bdpm => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_ref().map(|s| s.as_str()).unwrap_or(""), v[1].as_ref().map(|s| s.as_str()).unwrap_or(""),
                    ])
                }
                BDPMFile::CIS_CIP_Dispo_Spec => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_ref().map(|s| s.as_str()).unwrap_or(""), v[1].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[2].as_ref().map(|s| s.as_str()).unwrap_or(""), v[3].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[4].as_ref().map(|s| s.as_str()).unwrap_or(""), v[5].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[6].as_ref().map(|s| s.as_str()).unwrap_or(""), v[7].as_ref().map(|s| s.as_str()).unwrap_or(""),
                    ])
                }
                BDPMFile::CIS_MITM => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_ref().map(|s| s.as_str()).unwrap_or(""), v[1].as_ref().map(|s| s.as_str()).unwrap_or(""),
                        v[2].as_ref().map(|s| s.as_str()).unwrap_or(""), v[3].as_ref().map(|s| s.as_str()).unwrap_or(""),
                    ])
                }
            };

            match res {
                Ok(_) => stats.rows_imported += 1,
                Err(e) => {
                    stats.bad_rows += 1;
                    if stats.bad_rows <= 3 {
                        let preview: Vec<&str> = row.values.iter().take(3)
                            .map(|v| v.as_ref().map(|s| s.as_str()).unwrap_or(""))
                            .collect();
                        tracing::warn!("Bad row in {}: {} — {}", table, e, preview.join(", "));
                    }
                }
            }
        }

        drop(stmt);
        tx.commit()?;
        Ok(stats)
    })();

    // Always restore normal settings after bulk insert
    restore_normal_settings(conn);

    result
}

fn insert_sql(file: BDPMFile) -> String {
    match file {
        BDPMFile::CIS_bdpm => {
            "INSERT OR REPLACE INTO drugs (cis, name, form, route, auth_status, procedure_type, comm_status, auth_date, lab_name, is_patent, alert_type, eu_number)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)".into()
        }
        BDPMFile::CIS_CIP_bdpm => {
            "INSERT OR IGNORE INTO presentations (cis, cip, cip_raw, labels, pres_status, comm_status, comm_date, prix_ht_cents, prix_ville_cents, prix_rate_cents, reimb_rate, reimb_conditions, ean13, reimbursable)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)".into()
        }
        BDPMFile::CIS_COMPO_bdpm => {
            "INSERT OR IGNORE INTO compositions (cis, form_label, substance_code, substance_name, dosage, per_unit, pharm_code, seq)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)".into()
        }
        BDPMFile::CIS_HAS_SMR_bdpm => {
            "INSERT OR IGNORE INTO smr (cis, ct_id, decision_type, decision_date, level, avis)
             VALUES (?1,?2,?3,?4,?5,?6)".into()
        }
        BDPMFile::CIS_HAS_ASMR_bdpm => {
            "INSERT OR IGNORE INTO asmr (cis, ct_id, decision_type, decision_date, level, avis)
             VALUES (?1,?2,?3,?4,?5,?6)".into()
        }
        BDPMFile::CIS_GENER_bdpm => {
            "INSERT INTO generic_groups (group_id, group_name, cis, type, sort_order)
             VALUES (?1,?2,?3,?4,?5)".into()
        }
        BDPMFile::CIS_CPD_bdpm => {
            "INSERT OR IGNORE INTO prescription_rules (cis, rule)
             VALUES (?1,?2)".into()
        }
        BDPMFile::CIS_CIP_Dispo_Spec => {
            "INSERT OR IGNORE INTO availability (cis, cip, status_type, status, date_start, date_end, date_remise, source_url)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)".into()
        }
        BDPMFile::CIS_MITM => {
            "INSERT OR IGNORE INTO mitm (cis, atc_code, detail_url)
             VALUES (?1,?2,?4)".into()
        }
        BDPMFile::HAS_LiensPageCT_bdpm => {
            "INSERT OR IGNORE INTO has_links (ct_id, url)
             VALUES (?1,?2)".into()
        }
    }
}

// ---- Report types ----

pub struct ImportReport {
    pub results: Vec<FileImportResult>,
    pub total_duration_ms: u64,
}

impl Default for ImportReport {
    fn default() -> Self {
        Self { results: Vec::new(), total_duration_ms: 0 }
    }
}

pub struct FileImportResult {
    pub file_name: String,
    pub status: ImportStatus,
    pub rows_imported: usize,
    pub skipped_rows: usize,
    pub bad_rows: usize,
    pub warnings: usize,
    pub duration_ms: u64,
    pub error: Option<String>,
}

pub enum ImportStatus {
    Success,
    Partial,
    Failed,
    Unchanged,
}

impl ImportReport {
    pub fn print(&self) {
        for r in &self.results {
            let status = match r.status {
                ImportStatus::Success => "✓",
                ImportStatus::Partial => "⚠",
                ImportStatus::Failed => "✗",
                ImportStatus::Unchanged => "=",
            };
            if let Some(e) = &r.error {
                eprintln!("{} {}: {} — ERROR: {}", status, r.file_name, r.rows_imported, e);
            } else {
                println!("{} {}: {} rows, {} bad, {} skipped ({}ms)",
                    status, r.file_name, r.rows_imported, r.bad_rows, r.skipped_rows, r.duration_ms);
            }
        }
        println!("\nTotal: {}ms", self.total_duration_ms);
    }
}

#[derive(Default)]
struct ImportStats {
    rows_imported: usize,
    skipped_rows: usize,
    bad_rows: usize,
    warnings: usize,
}
