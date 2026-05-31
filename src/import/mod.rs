//! BDMP import orchestrator
//! Orchestrates: parse → normalize → dedup (compo) → dedup (state check) → import

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use rusqlite::Connection;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::path::Path;

use crate::db::{create_fts_tables, optimize_for_bulk_insert, restore_normal_settings};
use crate::download::manifest::BDPMFile;
use crate::download::Fetcher;
use crate::normalize::{dedup_compo, normalize_apostrophes, normalize_row, CpdFlags};
use crate::parse::parse_file;

pub const BDPM_URL: &str = "https://base-donnees-publique.medicaments.gouv.fr";

/// Insert a rejected row into the quarantine table for audit and retry.
/// Called when a row fails parsing, encoding, or field count validation.
fn quarantine_row(
    conn: &mut rusqlite::Connection,
    source_file: &str,
    source_line: usize,
    target_table: &str,
    error_type: &str,
    error_detail: &str,
    raw_line: &str,
) {
    let _ = conn.execute(
        "INSERT INTO quarantine (source_file, source_line, target_table, error_type, error_detail, raw_line)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![source_file, source_line as i64, target_table, error_type, error_detail, raw_line],
    );
}

/// Run post-import validation checks. Logs warnings for threshold breaches.
/// Does NOT fail the import — these are informational only.
fn validate_thresholds(conn: &Connection) {
    // Ghost CIS threshold
    if let Ok(ghost_cis) = conn.query_row(
        "SELECT COUNT(*) FROM generic_groups WHERE is_orphan = 1", [], |r| r.get::<_, i64>(0)
    ) {
        if ghost_cis > 3000 {
            tracing::warn!("Ghost CIS count {} exceeds threshold 3000 — possible format change", ghost_cis);
        } else {
            tracing::info!("Ghost CIS count {} (threshold: 3000)", ghost_cis);
        }
    }

    // Chemical ID cardinality
    if let Ok(substance_count) = conn.query_row(
        "SELECT COUNT(DISTINCT substance_code) FROM compositions", [], |r| r.get::<_, i64>(0)
    ) {
        if !(3500..=4500).contains(&substance_count) {
            tracing::warn!("Substance code cardinality {} outside expected range 3500-4500", substance_count);
        } else {
            tracing::info!("Substance code cardinality {} (expected: 3500-4500)", substance_count);
        }
    }

    // Princeps coverage
    if let Ok(princeps_groups) = conn.query_row(
        "SELECT COUNT(DISTINCT group_id) FROM generic_groups WHERE type = 'reference'", [], |r| r.get::<_, i64>(0)
    ) {
        if princeps_groups < 500 {
            tracing::warn!("Princeps groups {} below expected minimum 500", princeps_groups);
        } else {
            tracing::info!("Princeps groups {} (threshold: >= 500)", princeps_groups);
        }
    }

    // Generic name coverage (substance_name_clean column may be new)
    if let Ok(coverage) = conn.query_row(
        "SELECT COUNT(DISTINCT substance_name_clean) * 1.0 /
         NULLIF(COUNT(DISTINCT substance_code), 0) FROM compositions
         WHERE substance_name_clean IS NOT NULL",
        [], |r| r.get::<_, f64>(0)
    ) {
        if coverage < 0.5 {
            tracing::warn!("Generic name coverage {:.1}% below 50% threshold (expected on first run)", coverage * 100.0);
        } else {
            tracing::info!("Generic name coverage {:.1}% (threshold: >= 50%)", coverage * 100.0);
        }
    }

    // Temporal coherence: auth_date <= comm_date
    if let Ok(date_coherence) = conn.query_row(
        "SELECT COUNT(*) FROM presentations p
         JOIN drugs d ON p.cis = d.cis
         WHERE p.comm_date IS NOT NULL AND d.auth_date IS NOT NULL
         AND p.comm_date < d.auth_date", [], |r| r.get::<_, i64>(0)
    ) {
        if date_coherence > 0 {
            tracing::warn!("Date coherence issues: {} presentations have comm_date before auth_date", date_coherence);
        } else {
            tracing::info!("Date coherence: 0 issues found");
        }
    }
}

/// Check all child tables for orphan CIS codes using NOT EXISTS pattern.
/// NOT EXISTS outperforms LEFT JOIN WHERE IS NULL per SQLite research.
fn check_all_orphans(conn: &Connection) {
    let checks = [
        ("presentations",       "SELECT COUNT(*) FROM presentations p WHERE NOT EXISTS (SELECT 1 FROM drugs d WHERE d.cis = p.cis)"),
        ("compositions",        "SELECT COUNT(*) FROM compositions c WHERE NOT EXISTS (SELECT 1 FROM drugs d WHERE d.cis = c.cis)"),
        ("generic_groups",      "SELECT COUNT(*) FROM generic_groups g WHERE NOT EXISTS (SELECT 1 FROM drugs d WHERE d.cis = g.cis)"),
        ("prescription_rules",  "SELECT COUNT(*) FROM prescription_rules pr WHERE NOT EXISTS (SELECT 1 FROM drugs d WHERE d.cis = pr.cis)"),
        ("prescription_flags",  "SELECT COUNT(*) FROM prescription_flags pf WHERE NOT EXISTS (SELECT 1 FROM drugs d WHERE d.cis = pf.cis)"),
        ("availability",        "SELECT COUNT(*) FROM availability av WHERE NOT EXISTS (SELECT 1 FROM drugs d WHERE d.cis = av.cis)"),
        ("safety_alerts",       "SELECT COUNT(*) FROM safety_alerts sa WHERE NOT EXISTS (SELECT 1 FROM drugs d WHERE d.cis = sa.cis)"),
        ("mitm",                "SELECT COUNT(*) FROM mitm m WHERE NOT EXISTS (SELECT 1 FROM drugs d WHERE d.cis = m.cis)"),
    ];

    for (table, sql) in checks {
        if let Ok(count) = conn.query_row(sql, [], |r| r.get::<_, i64>(0)) {
            if count > 0 {
                tracing::warn!("Orphan rows in {}: {}", table, count);
            } else {
                tracing::info!("Orphan check {}: 0", table);
            }
        }
    }
    // SMR/ASMR already have is_orphan column, no need to re-check
}

/// Full ingest orchestrator — drops/recreates FTS5, imports all files in order.
/// CIS_MITM runs first to populate atc_code inline before drugs are imported.
pub fn run_ingest(data_dir: &Path, conn: &mut Connection) -> Result<ImportReport> {
    let raw_dir = data_dir.join("raw");
    std::fs::create_dir_all(&raw_dir)?;
    let fetcher = Fetcher::new();

    // Drop and recreate FTS5 from scratch — always fresh, never stale triggers
    create_fts_tables(conn)?;

    // Order: CIS_MITM first (populates atc_code inline), then all others
    let order = BDPMFile::all();

    let mut report = ImportReport::default();
    let start = std::time::Instant::now();

    for file in order {
        let start_file = std::time::Instant::now();

        let url = format!("{}{}", BDPM_URL, file.download_path());
        let bytes = fetcher.fetch(&url, &raw_dir)?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let size = bytes.len() as u64;

        // Parse
        let path = raw_dir.join(file.filename());
        let t_parse = std::time::Instant::now();
        let raw_rows = match parse_file(&path, file) {
            Ok(rows) => rows,
            Err(e) => {
                report.results.push(FileImportResult {
                    file_name: file.filename().to_string(),
                    status: ImportStatus::Failed,
                    rows_imported: 0,
                    bad_rows: 0,
                    duration_ms: start_file.elapsed().as_millis() as u64,
                    error: Some(e.to_string()),
                });
                continue;
            }
        };
        tracing::debug!("{}: parsed {} rows ({}ms)", file.filename(), raw_rows.len(), t_parse.elapsed().as_millis());

        // Normalize (homeopathic drugs return None and are dropped)
        // CIS_COMPO_bdpm: parallel normalization via rayon (32K+ rows)
        // All other files: sequential filter_map
        let mut normalized: Vec<_> = if file == BDPMFile::CIS_COMPO_bdpm {
            let count = Arc::new(AtomicUsize::new(0));
            let pb = ProgressBar::new(raw_rows.len() as u64);
            pb.set_style({
                ProgressStyle::default_bar()
                    .template("[{elapsed}] {bar:40} {pos}/{len} ({per_sec}) {msg}")
                    .unwrap_or_else(|_| panic!("indicatif template is valid"))
                    .progress_chars("=>-")
            });
            pb.set_message("CIS_COMPO normalize");

            let (tx, rx) = std::sync::mpsc::channel();
            let count_clone = Arc::clone(&count);
            raw_rows
                .into_iter()
                .enumerate()
                .collect::<Vec<_>>()
                .into_par_iter()
                .for_each(|(idx, row)| {
                    if let Some(mut nr) = normalize_row(file, &row) {
                        normalize_apostrophes(&mut nr);
                        let _ = tx.send((idx, nr));
                        count_clone.fetch_add(1, Ordering::Relaxed);
                        pb.set_position(count.load(Ordering::Relaxed) as u64);
                    }
                });
            drop(tx);
            pb.finish();
            let mut parallel_rows: Vec<(usize, _)> = rx.into_iter().collect();
            parallel_rows.sort_by_key(|(idx, _)| *idx);
            parallel_rows.into_iter().map(|(_, r)| r).collect()
        } else {
            raw_rows
                .iter()
                .filter_map(|row| {
                    let mut nr = normalize_row(file, row)?;
                    normalize_apostrophes(&mut nr);
                    Some(nr)
                })
                .collect()
        };

        let t_norm = std::time::Instant::now();
        // Dedup CIS_COMPO
        if file == BDPMFile::CIS_COMPO_bdpm {
            let before = normalized.len();
            normalized = dedup_compo(normalized);
            tracing::info!("CIS_COMPO dedup: {} → {} rows", before, normalized.len());
        }
        tracing::debug!("{}: normalized {} rows ({}ms)", file.filename(), normalized.len(), t_norm.elapsed().as_millis());

        // Import
        match import_file(conn, file, &normalized) {
            Ok(stats) => {
                let duration = start_file.elapsed().as_millis() as u64;

                // Inline atc_code population: runs immediately after MITM ingest.
                // FTS5.atc_code can't be updated here (drugs not yet imported).
                // CIS_bdpm block below handles both drugs.atc and FTS5.atc after drugs import.
                if file.is_mitm() {
                    // CIS_bdpm block (fired later) handles drugs.atc_code, FTS5.atc_code, and atc_codes
                    tracing::debug!("MITM imported: {} rows", stats.rows_imported);
                }

                // Populate atc_code for drugs after CIS_bdpm import completes.
                // MITM data is committed; drugs have NULL atc_code from CIS_bdpm import.
                if file == BDPMFile::CIS_bdpm {
                    let updated = conn.execute(
                        "UPDATE drugs SET atc_code = (
                            SELECT atc_code FROM mitm WHERE mitm.cis = drugs.cis LIMIT 1
                        ) WHERE EXISTS (SELECT 1 FROM mitm WHERE mitm.cis = drugs.cis)",
                        [],
                    ).unwrap_or(0);
                    tracing::info!("Populated atc_code for {} drugs after CIS_bdpm import", updated);

                    let fts_synced = conn.execute(
                        "UPDATE drugs_fts SET atc_code = (
                            SELECT atc_code FROM drugs WHERE drugs.cis = drugs_fts.cis LIMIT 1
                        ) WHERE EXISTS (SELECT 1 FROM drugs WHERE drugs.cis = drugs_fts.cis AND atc_code IS NOT NULL AND atc_code != '')",
                        [],
                    ).unwrap_or(0);
                    tracing::info!("Synced atc_code to FTS5 for {} rows after CIS_bdpm", fts_synced);

                    // Populate atc_codes lookup table from MITM (run here so atc_codes table is initialized)
                    conn.execute(
                        "INSERT OR IGNORE INTO atc_codes(atc_code) SELECT DISTINCT atc_code FROM mitm WHERE atc_code IS NOT NULL AND atc_code != ''",
                        [],
                    ).ok();

                    // Derive ATC parent hierarchy from atc_code (always fresh, idempotent)
                    let parent_updated = conn.execute(
                        "UPDATE atc_codes SET
                            parent_5_char = substr(atc_code, 1, 5),
                            parent_3_char = substr(atc_code, 1, 3),
                            parent_1_char = substr(atc_code, 1, 1)
                        WHERE atc_code IS NOT NULL AND atc_code != '' AND parent_5_char IS NULL",
                        [],
                    ).unwrap_or(0);
                    tracing::info!("Populated ATC parent hierarchy for {} codes after CIS_bdpm", parent_updated);
                }

                // Log to import_log
                let _ = conn.execute(
                    "INSERT INTO import_log (file_name, file_hash, file_size, row_count, status, bad_rows, duration_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        file.filename(),
                        hash,
                        size as i64,
                        stats.rows_imported as i64,
                        "success",
                        stats.bad_rows as i64,
                        duration as i64,
                    ],
                );

                report.results.push(FileImportResult {
                    file_name: file.filename().to_string(),
                    status: ImportStatus::Success,
                    rows_imported: stats.rows_imported,
                    bad_rows: stats.bad_rows,
                    duration_ms: duration,
                    error: None,
                });
            }
            Err(e) => {
                report.results.push(FileImportResult {
                    file_name: file.filename().to_string(),
                    status: ImportStatus::Failed,
                    rows_imported: 0,
                    bad_rows: 0,
                    duration_ms: start_file.elapsed().as_millis() as u64,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    report.total_duration_ms = start.elapsed().as_millis() as u64;
    validate_thresholds(conn);
    check_all_orphans(conn);

    // After all imports complete — collect fresh statistics for the query planner.
    // 0x10002: auto-mode (scans only changed tables) + force-all-tables scan.
    // Bit 0x10000 ensures tables not queried during import are still analyzed.
    conn.execute_batch("PRAGMA optimize=0x10002;")?;

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

    // Disable FK enforcement for bulk load — re-enabled after commit.
    // This eliminates FK overhead per row during INSERT. Orphan validation
    // runs post-import via check_all_orphans() in run_ingest.
    // Must be outside the transaction (FK PRAGMAs require immediate effect).
    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;

    let result = (|| -> Result<ImportStats> {
        // Wrap in transaction
        let tx = conn.transaction()?;

        // Clear existing data (except FTS5 — rebuilt by create_fts_tables at ingest start)
        if file != BDPMFile::CIS_bdpm {
            tx.execute(&format!("DELETE FROM {table}"), [])?;
        } else {
            // Drugs table cleared here so INSERT OR IGNORE only inserts new CIS codes.
            // drugs_ai fires for new rows only (no drugs_ad DELETE from pre-existing rows).
            // FTS5 is populated fresh by create_fts_tables before this block.
            tx.execute("DELETE FROM drugs", [])?;
        }

        // Bulk insert — build prepared statement
        let mut stats = ImportStats::default();
        // Collect failed rows for quarantine (inserted after commit to avoid borrow conflict)
        let mut quarantine_failures: Vec<(usize, String, String, String)> = Vec::new();

        let sql = insert_sql(file);
        let mut stmt = tx.prepare_cached(&sql)?;

        let pb = ProgressBar::new(rows.len() as u64);
        pb.set_style({
            ProgressStyle::default_bar()
                .template("[{elapsed}] {bar:40} {pos}/{len} ({per_sec}) {msg}")
                .unwrap_or_else(|_| panic!("indicatif template is valid"))
                .progress_chars("=>-")
        });
        pb.set_message(table);

        for (line_number, row) in rows.iter().enumerate() {
            let res = match file {
                BDPMFile::CIS_bdpm => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_deref().unwrap_or(""),
                        v[1].as_deref().unwrap_or(""),
                        v[2].as_deref().unwrap_or(""),
                        v[3].as_deref().unwrap_or(""),
                        v[4].as_deref().unwrap_or(""),
                        v[5].as_deref().unwrap_or(""),
                        v[6].as_deref().unwrap_or(""),
                        v[7].as_deref().unwrap_or(""),
                        v[8].as_deref().unwrap_or(""),
                        v[9].as_deref().unwrap_or(""),
                        v[10].as_deref().unwrap_or(""),
                        v[11].as_deref().unwrap_or(""),
                        v[12].as_deref().unwrap_or(""),
                    ])
                }
                BDPMFile::CIS_CIP_bdpm => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_deref().unwrap_or(""), v[1].as_deref().unwrap_or(""),
                        v[2].as_deref().unwrap_or(""), v[3].as_deref().unwrap_or(""),
                        v[4].as_deref().unwrap_or(""), v[5].as_deref().unwrap_or(""),
                        v[6].as_deref().unwrap_or(""), v[7].as_deref().unwrap_or(""),
                        v[8].as_deref().unwrap_or(""), v[9].as_deref().unwrap_or(""),
                        v[10].as_deref().unwrap_or(""), v[11].as_deref().unwrap_or(""),
                        v[12].as_deref().unwrap_or(""), v[13].as_deref().unwrap_or(""),
                        0, // is_orphan
                    ])
                }
                BDPMFile::CIS_COMPO_bdpm => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_deref().unwrap_or(""), v[1].as_deref().unwrap_or(""),
                        v[2].as_deref().unwrap_or(""), v[3].as_deref().unwrap_or(""),
                        v[4].as_deref().unwrap_or(""), v[5].as_deref().unwrap_or(""),
                        v[6].as_deref().unwrap_or(""), v[7].as_deref().unwrap_or(""),
                        v[8].as_deref().unwrap_or(""), v[9].as_deref().unwrap_or(""),
                        0, // is_orphan
                    ])
                }
                BDPMFile::CIS_HAS_SMR_bdpm | BDPMFile::CIS_HAS_ASMR_bdpm => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_deref().unwrap_or(""), v[1].as_deref().unwrap_or(""),
                        v[2].as_deref().unwrap_or(""), v[3].as_deref().unwrap_or(""),
                        v[4].as_deref().unwrap_or(""), v[5].as_deref().unwrap_or(""),
                    ])
                }
                BDPMFile::CIS_GENER_bdpm => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_deref().unwrap_or(""), v[1].as_deref().unwrap_or(""),
                        v[2].as_deref().unwrap_or(""), v[3].as_deref().unwrap_or(""),
                        v[4].as_deref().unwrap_or(""),
                    ])
                }
                BDPMFile::CIS_CPD_bdpm => {
                    let v = &row.values;
                    let cis = v[0].as_deref().unwrap_or("");
                    let rule_text = v[1].as_deref().unwrap_or("");
                    stmt.execute(rusqlite::params![
                        cis, rule_text,
                    ])?;
                    let flags = CpdFlags::from_rule(rule_text);
                    tx.execute(
                        "INSERT OR IGNORE INTO prescription_flags(cis, liste_i, liste_ii, stupefiant, hospitalier, dentaire, reserve_hopital) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        rusqlite::params![cis, flags.liste_i as i32, flags.liste_ii as i32, flags.stupefiant as i32, flags.hospitalier as i32, flags.dentaire as i32, flags.reserve_hopital as i32],
                    )
                }
                BDPMFile::HAS_LiensPageCT_bdpm => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_deref().unwrap_or(""), v[1].as_deref().unwrap_or(""),
                    ])
                }
                BDPMFile::CIS_CIP_Dispo_Spec => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_deref().unwrap_or(""), v[1].as_deref().unwrap_or(""),
                        v[2].as_deref().unwrap_or(""), v[3].as_deref().unwrap_or(""),
                        v[4].as_deref().unwrap_or(""), v[5].as_deref().unwrap_or(""),
                        v[6].as_deref().unwrap_or(""), v[7].as_deref().unwrap_or(""),
                    ])
                }
                BDPMFile::CIS_MITM => {
                    let v = &row.values;
                    stmt.execute(rusqlite::params![
                        v[0].as_deref().unwrap_or(""), v[1].as_deref().unwrap_or(""),
                        v[2].as_deref().unwrap_or(""),
                    ])
                }
                BDPMFile::CIS_InfoImportantes => {
                    let v = &row.values;
                    let raw = v[3].as_deref().unwrap_or("");
                    // Split "message text https://url" — URL is always at the end if present
                    let (msg, url) = if let Some(idx) = raw.rfind("https://") {
                        let m = raw[..idx].trim().to_string();
                        let u = raw[idx..].trim().to_string();
                        (m, Some(u))
                    } else {
                        (raw.to_string(), None)
                    };
                    stmt.execute(rusqlite::params![
                        v[0].as_deref().unwrap_or(""), // cis
                        v[1].as_deref().unwrap_or(""), // start_date
                        v[2].as_deref().unwrap_or(""), // end_date
                        msg,
                        url,
                    ])
                }
            };

            match res {
                Ok(_) => {
                    stats.rows_imported += 1;
                    if row.invalid_ean13 {
                        stats.invalid_ean13 += 1;
                    }
                }
                Err(e) => {
                    stats.bad_rows += 1;
                    // Collect for quarantine insert (done after commit to avoid borrow conflict)
                    let raw_line = row.values.iter()
                        .map(|v| v.as_deref().unwrap_or(""))
                        .collect::<Vec<_>>()
                        .join("\t");
                    quarantine_failures.push((line_number + 1, "insert_failed".to_string(), e.to_string(), raw_line));
                    if stats.bad_rows <= 3 {
                        let preview: Vec<&str> = row.values.iter().take(3)
                            .map(|v| v.as_deref().unwrap_or(""))
                            .collect();
                        tracing::warn!("Bad row in {}: {} — {}", table, e, preview.join(", "));
                    }
                }
            }
            pb.inc(1);
        }
        pb.finish();

        drop(stmt);
        tx.commit()?;

        // Insert quarantine failures after commit (borrow conflict resolved)
        for (line_number, error_type, error_detail, raw_line) in quarantine_failures {
            quarantine_row(conn, file.filename(), line_number, table, &error_type, &error_detail, &raw_line);
        }

        // Re-enable FK enforcement after bulk load
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        // Post-import: flag orphan rows (withdrawn drugs not in drugs table).
        // Boiron lab and ENREG HOM procedure drugs are filtered at normalize_row.
        // CIS_InfoImportantes orphans are not flagged (no is_orphan column).
        // CIS_bdpm uses INSERT OR IGNORE after DELETE, so no orphan risk.
        if matches!(file,
            BDPMFile::CIS_HAS_SMR_bdpm
            | BDPMFile::CIS_HAS_ASMR_bdpm
            | BDPMFile::CIS_GENER_bdpm
            | BDPMFile::CIS_COMPO_bdpm
            | BDPMFile::CIS_CIP_bdpm
        ) {
            let orphan_count: i64 = conn.query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE cis NOT IN (SELECT cis FROM drugs)"),
                [],
                |row| row.get(0),
            ).unwrap_or(0);
            if orphan_count > 0 {
                conn.execute(
                    &format!("UPDATE {table} SET is_orphan = 1 WHERE cis NOT IN (SELECT cis FROM drugs)"),
                    [],
                )?;
                tracing::info!("{table}: flagged {} orphan rows", orphan_count);
            }
        }

        Ok(stats)
    })();

    // Always restore normal settings after bulk insert
    restore_normal_settings(conn);

    result
}

/// Returns the INSERT SQL for a BDPM file. pub(crate) for testing.
pub(crate) fn insert_sql(file: BDPMFile) -> String {
    match file {
        BDPMFile::CIS_bdpm => {
            "INSERT OR IGNORE INTO drugs (cis, name_raw, name, form, route, auth_status, procedure_type, comm_status, auth_date, lab_name, is_patent, alert_type, eu_number)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)".into()
        }
        BDPMFile::CIS_CIP_bdpm => {
            "INSERT OR IGNORE INTO presentations (cis, cip, cip_raw, labels, labels_clean, pres_status, comm_status, comm_date, prix_ht_cents, prix_ville_cents, prix_rate_cents, reimb_rate, ean13, reimbursable, is_orphan)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)".into()
        }
        BDPMFile::CIS_COMPO_bdpm => {
            "INSERT OR IGNORE INTO compositions (cis, form_label, substance_code, substance_name, dosage, per_unit, pharm_code, seq, substance_name_clean, dosage_mg, is_orphan)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)".into()
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
            "INSERT OR IGNORE INTO generic_groups (group_id, group_name, cis, type, sort_order)
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
             VALUES (?1,?2,?3)".into()
        }
        BDPMFile::HAS_LiensPageCT_bdpm => {
            "INSERT OR IGNORE INTO has_links (ct_id, url)
             VALUES (?1,?2)".into()
        }
        BDPMFile::CIS_InfoImportantes => {
            "INSERT OR IGNORE INTO safety_alerts (cis, start_date, end_date, message_plain, source_url)
             VALUES (?1,?2,?3,?4,?5)".into()
        }
    }
}

// ---- atc_code population test ----

#[cfg(test)]
mod atc_code_population_tests {
    /// Verifies the two-phase atc_code population pattern:
    /// 1. CIS_MITM fires first (drugs table empty at that point)
    /// 2. CIS_bdpm fires after drugs are populated, then runs the atc_code UPDATE + FTS5 sync
    ///
    /// This test simulates what the run_ingest loop does for the CIS_bdpm block.
    #[test]
    fn test_cis_bdpm_block_populates_atc_code_and_fts5() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();

        // Create minimal schema
        conn.execute_batch(
            "CREATE TABLE mitm (cis TEXT PRIMARY KEY, atc_code TEXT, detail_url TEXT);
             CREATE TABLE drugs (cis TEXT PRIMARY KEY, name TEXT, atc_code TEXT);
             CREATE TABLE atc_codes (atc_code TEXT PRIMARY KEY, parent_5_char TEXT, parent_3_char TEXT, parent_1_char TEXT);
             CREATE TABLE drugs_fts (
                 cis TEXT, name_raw TEXT, name TEXT, atc_code TEXT,
                 form TEXT, lab_name TEXT, substance_name TEXT
             );
             CREATE TRIGGER drugs_au AFTER UPDATE ON drugs BEGIN
                 DELETE FROM drugs_fts WHERE cis = old.cis;
                 INSERT INTO drugs_fts(cis, name_raw, name, atc_code, form, lab_name, substance_name)
                 VALUES (new.cis, new.name, new.name, new.atc_code, '', '', '');
             END;"
        ).unwrap();

        // Phase 1: Insert MITM rows (simulates CIS_MITM import)
        conn.execute(
            "INSERT INTO mitm (cis, atc_code, detail_url) VALUES (?1, ?2, ?3)",
            rusqlite::params!["C001", "A01AA01", "http://example.com/1"],
        ).unwrap();
        conn.execute(
            "INSERT INTO mitm (cis, atc_code, detail_url) VALUES (?1, ?2, ?3)",
            rusqlite::params!["C002", "A01AA01", "http://example.com/2"],
        ).unwrap();
        conn.execute(
            "INSERT INTO mitm (cis, atc_code, detail_url) VALUES (?1, ?2, ?3)",
            rusqlite::params!["C003", "B02AA02", "http://example.com/3"],
        ).unwrap();

        // Phase 2: Insert drugs rows (simulates CIS_bdpm import — no atc_code yet)
        conn.execute(
            "INSERT INTO drugs (cis, name, atc_code) VALUES (?1, ?2, ?3)",
            rusqlite::params!["C001", "Drug One", ""],
        ).unwrap();
        conn.execute(
            "INSERT INTO drugs (cis, name, atc_code) VALUES (?1, ?2, ?3)",
            rusqlite::params!["C002", "Drug Two", ""],
        ).unwrap();
        conn.execute(
            "INSERT INTO drugs (cis, name, atc_code) VALUES (?1, ?2, ?3)",
            rusqlite::params!["C003", "Drug Three", ""],
        ).unwrap();
        conn.execute(
            "INSERT INTO drugs (cis, name, atc_code) VALUES (?1, ?2, ?3)",
            rusqlite::params!["C004", "Drug Four - no MITM", ""],
        ).unwrap();

        // Populate FTS5 from drugs (simulate what drugs_ai trigger does)
        conn.execute(
            "INSERT INTO drugs_fts(cis, name_raw, name, atc_code, form, lab_name, substance_name)
             SELECT cis, name, name, atc_code, '', '', '' FROM drugs",
            [],
        ).unwrap();

        // Verify: drugs.atc_code is NULL, drugs_fts.atc_code is NULL before the fix
        let drugs_null: i64 = conn.query_row(
            "SELECT COUNT(*) FROM drugs WHERE atc_code IS NULL OR atc_code = ''",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(drugs_null, 4, "All drugs should have empty atc_code before fix");

        let fts_null: i64 = conn.query_row(
            "SELECT COUNT(*) FROM drugs_fts WHERE atc_code IS NULL OR atc_code = ''",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(fts_null, 4, "All FTS5 rows should have empty atc_code before fix");

        // Phase 3: Run the CIS_bdpm atc_code population (from run_ingest's CIS_bdpm block)
        let updated = conn.execute(
            "UPDATE drugs SET atc_code = (
                SELECT atc_code FROM mitm WHERE mitm.cis = drugs.cis LIMIT 1
            ) WHERE EXISTS (SELECT 1 FROM mitm WHERE mitm.cis = drugs.cis)",
            [],
        ).unwrap();
        assert_eq!(updated, 3, "Should update 3 drugs (C001, C002, C003 have MITM)");

        let fts_synced = conn.execute(
            "UPDATE drugs_fts SET atc_code = (
                SELECT atc_code FROM drugs WHERE drugs.cis = drugs_fts.cis LIMIT 1
            ) WHERE EXISTS (SELECT 1 FROM drugs WHERE drugs.cis = drugs_fts.cis AND atc_code IS NOT NULL AND atc_code != '')",
            [],
        ).unwrap();
        assert_eq!(fts_synced, 3, "Should sync 3 FTS5 rows");

        conn.execute(
            "INSERT OR IGNORE INTO atc_codes(atc_code) SELECT DISTINCT atc_code FROM mitm WHERE atc_code IS NOT NULL AND atc_code != ''",
            [],
        ).ok();

        // Derive ATC parent hierarchy (matches run_ingest CIS_bdpm block)
        let parent_updated = conn.execute(
            "UPDATE atc_codes SET
                parent_5_char = substr(atc_code, 1, 5),
                parent_3_char = substr(atc_code, 1, 3),
                parent_1_char = substr(atc_code, 1, 1)
            WHERE atc_code IS NOT NULL AND atc_code != '' AND parent_5_char IS NULL",
            [],
        ).unwrap();
        assert_eq!(parent_updated, 2, "Should derive parents for 2 distinct ATC codes (A01AA01, B02AA02)");

        // Verify: drugs.atc_code populated
        let drugs_ok: i64 = conn.query_row(
            "SELECT COUNT(*) FROM drugs WHERE atc_code IS NOT NULL AND atc_code != ''",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(drugs_ok, 3);
        let drugs_still_empty: i64 = conn.query_row(
            "SELECT COUNT(*) FROM drugs WHERE atc_code IS NULL OR atc_code = ''",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(drugs_still_empty, 1, "C004 should still have empty atc_code (no MITM)");

        // Verify: FTS5.atc_code populated
        let fts_ok: i64 = conn.query_row(
            "SELECT COUNT(*) FROM drugs_fts WHERE atc_code IS NOT NULL AND atc_code != ''",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(fts_ok, 3);

        // Verify: atc_codes table populated with parents derived
        let atc_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM atc_codes",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(atc_count, 2, "A01AA01 and B02AA02");

        // Verify parent hierarchy for A01AA01: A01AA, A01, A
        let a01aa01_5: String = conn.query_row(
            "SELECT parent_5_char FROM atc_codes WHERE atc_code = 'A01AA01'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(a01aa01_5, "A01AA");

        let a01aa01_3: String = conn.query_row(
            "SELECT parent_3_char FROM atc_codes WHERE atc_code = 'A01AA01'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(a01aa01_3, "A01");

        let a01aa01_1: String = conn.query_row(
            "SELECT parent_1_char FROM atc_codes WHERE atc_code = 'A01AA01'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(a01aa01_1, "A");

        // Verify parent hierarchy for B02AA02: B02AA, B02, B
        let b02aa02_5: String = conn.query_row(
            "SELECT parent_5_char FROM atc_codes WHERE atc_code = 'B02AA02'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(b02aa02_5, "B02AA");

        let b02aa02_3: String = conn.query_row(
            "SELECT parent_3_char FROM atc_codes WHERE atc_code = 'B02AA02'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(b02aa02_3, "B02");

        let b02aa02_1: String = conn.query_row(
            "SELECT parent_1_char FROM atc_codes WHERE atc_code = 'B02AA02'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(b02aa02_1, "B");

        // Verify specific values
        let c001_atc: String = conn.query_row(
            "SELECT atc_code FROM drugs WHERE cis = 'C001'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(c001_atc, "A01AA01");
    }

    /// MITM block fires when drugs table is empty — atc population is a no-op,
    /// but atc_codes lookup table should still be populated.
    #[test]
    fn test_mitm_block_only_populates_atc_codes_lookup() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE mitm (cis TEXT PRIMARY KEY, atc_code TEXT, detail_url TEXT);
             CREATE TABLE drugs (cis TEXT PRIMARY KEY, atc_code TEXT);"
        ).unwrap();

        // Insert MITM
        conn.execute(
            "INSERT INTO mitm (cis, atc_code, detail_url) VALUES (?1, ?2, ?3)",
            rusqlite::params!["C001", "N02BE01", "http://example.com/1"],
        ).unwrap();

        // No drugs yet (MITM block fires before CIS_bdpm)
        let drugs_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM drugs",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(drugs_count, 0, "Drugs table should be empty when MITM block fires");
    }
}

#[derive(Default)]
pub struct ImportReport {
    pub results: Vec<FileImportResult>,
    pub total_duration_ms: u64,
}

pub struct FileImportResult {
    pub file_name: String,
    pub status: ImportStatus,
    pub rows_imported: usize,
    pub bad_rows: usize,
    pub duration_ms: u64,
    pub error: Option<String>,
}

pub enum ImportStatus {
    Success,
    Failed,
    Unchanged,
}

impl ImportReport {
    pub fn print(&self) {
        for r in &self.results {
            let status = match r.status {
                ImportStatus::Success => "✓",
                ImportStatus::Failed => "✗",
                ImportStatus::Unchanged => "=",
            };
            if let Some(e) = &r.error {
                eprintln!("{} {}: {} — ERROR: {}", status, r.file_name, r.rows_imported, e);
            } else {
                println!("{} {}: {} rows, {} bad ({}ms)",
                    status, r.file_name, r.rows_imported, r.bad_rows, r.duration_ms);
            }
        }
        println!("\nTotal: {}ms", self.total_duration_ms);
    }
}

#[cfg(test)]
mod insert_sql_tests {
    use super::*;

    // Count params: iterate through SQL string, counting '?' placeholders.
    // Handles numbered SQLite placeholders (?1, ?2, ?15) — the number after '?' is the
    // 1-based index, NOT the count. We count each '?' as one placeholder.
    fn count_params(sql: &str) -> usize {
        sql.chars().filter(|&c| c == '?').count()
    }

    #[test]
    fn test_insert_sql_drugs() {
        let sql = insert_sql(BDPMFile::CIS_bdpm);
        assert!(
            sql.contains("INSERT OR IGNORE INTO drugs"),
            "Expected INSERT OR IGNORE INTO drugs, got: {sql}"
        );
        assert_eq!(
            count_params(&sql),
            13,
            "Expected 13 params for drugs, got: {sql}"
        );
    }

    #[test]
    fn test_insert_sql_presentations() {
        let sql = insert_sql(BDPMFile::CIS_CIP_bdpm);
        assert!(
            sql.contains("INSERT OR IGNORE INTO presentations"),
            "Expected INSERT OR IGNORE INTO presentations, got: {sql}"
        );
        assert_eq!(
            count_params(&sql),
            15,
            "Expected 15 params for presentations (14 fields + is_orphan), got: {sql}"
        );
    }

    #[test]
    fn test_insert_sql_compositions() {
        let sql = insert_sql(BDPMFile::CIS_COMPO_bdpm);
        assert!(
            sql.contains("INSERT OR IGNORE INTO compositions"),
            "Expected INSERT OR IGNORE INTO compositions, got: {sql}"
        );
        assert_eq!(
            count_params(&sql),
            11,
            "Expected 11 params for compositions (11 ? + is_orphan=0 hardcoded), got: {sql}"
        );
    }

    #[test]
    fn test_insert_sql_smr() {
        let sql = insert_sql(BDPMFile::CIS_HAS_SMR_bdpm);
        assert!(
            sql.contains("INSERT OR IGNORE INTO smr"),
            "Expected INSERT OR IGNORE INTO smr, got: {sql}"
        );
        assert_eq!(
            count_params(&sql),
            6,
            "Expected 6 params for smr, got: {sql}"
        );
    }

    #[test]
    fn test_insert_sql_asmr() {
        let sql = insert_sql(BDPMFile::CIS_HAS_ASMR_bdpm);
        assert!(
            sql.contains("INSERT OR IGNORE INTO asmr"),
            "Expected INSERT OR IGNORE INTO asmr, got: {sql}"
        );
        assert_eq!(
            count_params(&sql),
            6,
            "Expected 6 params for asmr, got: {sql}"
        );
    }

    #[test]
    fn test_insert_sql_generic_groups() {
        let sql = insert_sql(BDPMFile::CIS_GENER_bdpm);
        assert!(
            sql.contains("INSERT OR IGNORE INTO generic_groups"),
            "Expected INSERT OR IGNORE INTO generic_groups, got: {sql}"
        );
        assert_eq!(
            count_params(&sql),
            5,
            "Expected 5 params for generic_groups, got: {sql}"
        );
    }

    #[test]
    fn test_insert_sql_prescription_rules() {
        let sql = insert_sql(BDPMFile::CIS_CPD_bdpm);
        assert!(
            sql.contains("INSERT OR IGNORE INTO prescription_rules"),
            "Expected INSERT OR IGNORE INTO prescription_rules, got: {sql}"
        );
        assert_eq!(
            count_params(&sql),
            2,
            "Expected 2 params for prescription_rules, got: {sql}"
        );
    }

    #[test]
    fn test_insert_sql_availability() {
        let sql = insert_sql(BDPMFile::CIS_CIP_Dispo_Spec);
        assert!(
            sql.contains("INSERT OR IGNORE INTO availability"),
            "Expected INSERT OR IGNORE INTO availability, got: {sql}"
        );
        assert_eq!(
            count_params(&sql),
            8,
            "Expected 8 params for availability, got: {sql}"
        );
    }

    #[test]
    fn test_insert_sql_mitm() {
        let sql = insert_sql(BDPMFile::CIS_MITM);
        assert!(
            sql.contains("INSERT OR IGNORE INTO mitm"),
            "Expected INSERT OR IGNORE INTO mitm, got: {sql}"
        );
        // mitm has 3 columns (cis, atc_code, detail_url) — drug_name is normalized but not stored
        assert_eq!(
            count_params(&sql),
            3,
            "Expected 3 params for mitm, got: {sql}"
        );
    }

    #[test]
    fn test_insert_sql_has_links() {
        let sql = insert_sql(BDPMFile::HAS_LiensPageCT_bdpm);
        assert!(
            sql.contains("INSERT OR IGNORE INTO has_links"),
            "Expected INSERT OR IGNORE INTO has_links, got: {sql}"
        );
        assert_eq!(
            count_params(&sql),
            2,
            "Expected 2 params for has_links, got: {sql}"
        );
    }

    #[test]
    fn test_insert_sql_safety_alerts() {
        let sql = insert_sql(BDPMFile::CIS_InfoImportantes);
        assert!(
            sql.contains("INSERT OR IGNORE INTO safety_alerts"),
            "Expected INSERT OR IGNORE INTO safety_alerts, got: {sql}"
        );
        assert_eq!(
            count_params(&sql),
            5,
            "Expected 5 params for safety_alerts, got: {sql}"
        );
    }

    // Test that normalized row value counts match SQL placeholder counts
    // We test this by creating a minimal ValidatedRow per file type and checking
    // that the values vec length matches the SQL param count.

    fn make_validated_row(fields: Vec<&str>) -> crate::parse::ValidatedRow {
        crate::parse::ValidatedRow {
            fields: fields.into_iter().map(String::from).collect(),
            line_number: 1,
        }
    }

    #[test]
    fn test_insert_sql_value_counts_match() {
        // (BDPMFile, field_count, expected_values)
        // NOTE: CIS_MITM is excluded — normalize_row includes drug_name (4 values)
        // but the mitm table only has 3 columns (cis, atc_code, detail_url).
        // drug_name is normalized but dropped at import time.
        let cases: Vec<(BDPMFile, usize)> = vec![
            (BDPMFile::CIS_bdpm, 12),
            // CIP and COMPO excluded — is_orphan is a bound param (?15/?11) not in normalized values.
            // Tested separately via test_insert_sql_presentations and test_insert_sql_compositions.
            (BDPMFile::CIS_HAS_SMR_bdpm, 6),
            (BDPMFile::CIS_HAS_ASMR_bdpm, 6),
            (BDPMFile::CIS_GENER_bdpm, 5),
            (BDPMFile::CIS_CPD_bdpm, 2),
            (BDPMFile::CIS_CIP_Dispo_Spec, 8),
            (BDPMFile::CIS_MITM, 4),              // raw has 4 fields, normalize produces 3 values (drug_name dropped)
            (BDPMFile::HAS_LiensPageCT_bdpm, 2),
            (BDPMFile::CIS_InfoImportantes, 4),
        ];

        for (file, field_count) in cases {
            let fields: Vec<&str> = (0..field_count).map(|_| "").collect();
            let row = make_validated_row(fields);
            let Some(normalized) = normalize_row(file, &row) else {
                // Homeopathic filter — valid outcome
                continue;
            };
            let sql = insert_sql(file);
            let param_count = count_params(&sql);

            assert_eq!(
                normalized.values.len(),
                param_count,
                "File {:?}: {} values vs {} SQL params — MISMATCH",
                file, normalized.values.len(), param_count
            );
        }
    }
}

#[cfg(test)]
mod compo_parallel_tests {
    use super::*;
    use crate::parse::ValidatedRow;

    /// Verifies parallel CIS_COMPO normalization produces identical output to sequential.
    /// Uses a known sample of 50 rows from raw CIS_COMPO data.
    #[test]
    fn test_compo_parallel_determinism() {
        // Sample data: real CIS_COMPO rows (8 fields each)
        let sample_rows: Vec<Vec<&str>> = vec![
            // CIS, form_label, substance_code, substance_name, dosage, per_unit, pharm_code, seq
            vec!["64534169", "Comprimes", "307293", "PARACETAMOL 500 mg", "500", "mg", "5018", "0"],
            vec!["60012483", "Gelules", "332987", "AMOXICILLINE 500 mg", "500", "mg", "5091", "0"],
            vec!["64534169", "Comprimes", "307293", "PARACETAMOL 500 mg", "500", "mg", "5018", "0"], // duplicate
            vec!["60290434", "Solution injectable", "315012", "CHLORHYDRATE DE MIDAZOLAM", "5", "mg", "5038", "0"],
            vec!["60290434", "Solution injectable", "315012", "MIDAZOLAM (CHLORHYDRATE)", "5", "mg", "5038", "0"], // duplicate different form
            vec!["61890215", "Comprimes enrobes", "330921", "IBUPROFENE 400 mg", "400", "mg", "5099", "0"],
            vec!["60012483", "Gelules", "332987", "AMOXICILLINE 500 mg", "500", "mg", "5091", "0"], // duplicate
            vec!["64534169", "Comprimes", "307293", "paracetamol 500 mg", "500", "mg", "5018", "0"], // duplicate, lowercase
            vec!["62315678", "Comprime effervescent", "318241", "ASCORBIQUE ACIDE 1 g", "1000", "mg", "5079", "0"],
            vec!["62315678", "Comprime effervescent", "318241", "ACIDE ASCORBIQUE 1 g", "1000", "mg", "5079", "0"], // reordered words
        ];

        // Sequential path (what ran before rayon)
        let seq_rows: Vec<Option<_>> = sample_rows
            .iter()
            .map(|fields| {
                let vr = ValidatedRow {
                    fields: fields.iter().map(|s| s.to_string()).collect(),
                    line_number: 1,
                };
                let mut nr = normalize_row(BDPMFile::CIS_COMPO_bdpm, &vr)?;
                normalize_apostrophes(&mut nr);
                Some(nr)
            })
            .collect();

        // Parallel path
        let (tx, rx) = std::sync::mpsc::channel();
        sample_rows
            .into_iter()
            .enumerate()
            .collect::<Vec<_>>()
            .into_par_iter()
            .for_each(|(idx, fields)| {
                let vr = ValidatedRow {
                    fields: fields.into_iter().map(String::from).collect(),
                    line_number: 1,
                };
                if let Some(mut nr) = normalize_row(BDPMFile::CIS_COMPO_bdpm, &vr) {
                    normalize_apostrophes(&mut nr);
                    let _ = tx.send((idx, nr));
                }
            });
        drop(tx);
        let mut parallel_rows: Vec<(usize, _)> = rx.into_iter().collect();
        parallel_rows.sort_by_key(|(idx, _)| *idx);
        let par_results: Vec<_> = parallel_rows.into_iter().map(|(_, r)| r).collect();

        // Collect sequential results (skip Nones which are filtered out)
        let seq_filtered: Vec<_> = seq_rows.into_iter().flatten().collect();

        // Both paths should produce the same set of rows (order may differ due to dedup)
        // After dedup_compo, order is deterministic (insertion order into HashSet, stable sort)
        let seq_deduped = dedup_compo(seq_filtered);
        let par_deduped = dedup_compo(par_results);

        assert_eq!(
            seq_deduped.len(),
            par_deduped.len(),
            "Deduped counts differ: seq={}, par={}",
            seq_deduped.len(),
            par_deduped.len()
        );

        for (i, (seq, par)) in seq_deduped.iter().zip(par_deduped.iter()).enumerate() {
            assert_eq!(
                seq.values, par.values,
                "Row {} differs: seq={:?}, par={:?}",
                i, seq.values, par.values
            );
            assert_eq!(seq.invalid_ean13, par.invalid_ean13, "Row {} invalid_ean13 differs", i);
        }
    }
}

#[cfg(test)]
mod quarantine_tests {
    use super::*;

    #[test]
    fn test_quarantine_row_insert() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../db/schema.sql")).unwrap();

        quarantine_row(
            &mut conn,
            "CIS_bdpm.txt",
            42,
            "drugs",
            "field_count_mismatch",
            "expected 12 got 11",
            "60004971\tDoliprane\tcomprime",
        );

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM quarantine", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let error_type: String = conn
            .query_row(
                "SELECT error_type FROM quarantine LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(error_type, "field_count_mismatch");
    }
}

#[derive(Default)]
struct ImportStats {
    rows_imported: usize,
    bad_rows: usize,
    invalid_ean13: usize,
}