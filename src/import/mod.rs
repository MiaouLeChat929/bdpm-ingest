//! BDPM import orchestrator
//! Orchestrates: parse → normalize → dedup (compo) → dedup (state check) → import

use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

use crate::db::{optimize_for_bulk_insert, rebuild_fts, restore_normal_settings};
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
                bad_rows: 0,
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
                    bad_rows: 0,
                    duration_ms: start_file.elapsed().as_millis() as u64,
                    error: Some(e.to_string()),
                });
                continue;
            }
        };

        // Normalize (homeopathic drugs return None and are dropped)
        let mut normalized: Vec<_> = raw_rows
            .iter()
            .filter_map(|row| {
                let mut nr = normalize_row(file, row)?;
                normalize_apostrophes(&mut nr);
                Some(nr)
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
        let tx = conn.transaction()?;

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
                        v[14].as_deref().unwrap_or(""),
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
                BDPMFile::CIS_CPD_bdpm | BDPMFile::HAS_LiensPageCT_bdpm => {
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
                Ok(_) => stats.rows_imported += 1,
                Err(e) => {
                    stats.bad_rows += 1;
                    if stats.bad_rows <= 3 {
                        let preview: Vec<&str> = row.values.iter().take(3)
                            .map(|v| v.as_deref().unwrap_or(""))
                            .collect();
                        tracing::warn!("Bad row in {}: {} — {}", table, e, preview.join(", "));
                    }
                }
            }
        }

        drop(stmt);
        tx.commit()?;

        // Post-import: rebuild FTS5 index after drugs table changes
        // INSERT OR REPLACE doesn't fire the drugs_ad DELETE trigger for the
        // implicit delete, leaving orphaned FTS entries. Rebuild fixes this.
        if file == BDPMFile::CIS_bdpm {
            rebuild_fts(conn).ok();
        }

        // Post-import: flag orphan rows (withdrawn drugs not in drugs table)
        // BRIEF.md: 2,806 SMR / 1,567 ASMR / 2,503 GENER orphan rows expected
        if matches!(file, BDPMFile::CIS_HAS_SMR_bdpm | BDPMFile::CIS_HAS_ASMR_bdpm | BDPMFile::CIS_GENER_bdpm) {
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

        // Post-import: populate drugs.atc_code from MITM data
        if file == BDPMFile::CIS_MITM {
            let updated = conn.execute(
                "UPDATE drugs SET atc_code = (SELECT atc_code FROM mitm WHERE mitm.cis = drugs.cis)",
                [],
            ).unwrap_or(0);
            tracing::info!("Populated atc_code for {} drugs from MITM data", updated);

            let atc_codes_count = conn.execute(
                "INSERT OR IGNORE INTO atc_codes(atc_code) SELECT DISTINCT atc_code FROM mitm WHERE atc_code IS NOT NULL AND atc_code != ''",
                [],
            ).unwrap_or(0);
            tracing::info!("Inserted {} distinct ATC codes", atc_codes_count);
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
            "INSERT OR REPLACE INTO drugs (cis, name_raw, name, form, route, auth_status, procedure_type, comm_status, auth_date, lab_name, is_patent, alert_type, eu_number)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)".into()
        }
        BDPMFile::CIS_CIP_bdpm => {
            "INSERT OR IGNORE INTO presentations (cis, cip, cip_raw, labels, labels_clean, pres_status, comm_status, comm_date, prix_ht_cents, prix_ville_cents, prix_rate_cents, reimb_rate, reimb_conditions, ean13, reimbursable)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)".into()
        }
        BDPMFile::CIS_COMPO_bdpm => {
            "INSERT OR IGNORE INTO compositions (cis, form_label, substance_code, substance_name, dosage, per_unit, pharm_code, seq, substance_name_clean, dosage_mg)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)".into()
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

// ---- Report types ----

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

    // Count params: iterate through SQL string, counting '?' and any following digits
    fn count_params(sql: &str) -> usize {
        let mut count = 0;
        let mut chars = sql.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '?' {
                // Check if it's followed by digits (numbered param)
                let mut num_str = String::new();
                while let Some(&next_c) = chars.peek() {
                    if next_c.is_ascii_digit() {
                        num_str.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                count += 1;
            }
        }
        count
    }

    #[test]
    fn test_insert_sql_drugs() {
        let sql = insert_sql(BDPMFile::CIS_bdpm);
        assert!(
            sql.contains("INSERT OR REPLACE INTO drugs"),
            "Expected INSERT OR REPLACE INTO drugs, got: {sql}"
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
            "Expected 15 params for presentations, got: {sql}"
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
            10,
            "Expected 10 params for compositions, got: {sql}"
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
            (BDPMFile::CIS_CIP_bdpm, 12),
            (BDPMFile::CIS_COMPO_bdpm, 8),
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
            let normalized = normalize_row(file, &row).expect("non-homeopathic rows should return Some");
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

#[derive(Default)]
struct ImportStats {
    rows_imported: usize,
    bad_rows: usize,
}
