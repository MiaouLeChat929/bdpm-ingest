use std::collections::HashSet;
use crate::normalize::NormalizedRow;

/// Remove exact duplicate rows from CIS_COMPO.
/// Key: (cis, substance_code, dosage).
/// 4,780 duplicates in 32,389 total rows → 27,609 unique.
/// Malformed rows (len < 5) are kept for logging.
pub fn dedup_compo(rows: Vec<NormalizedRow>) -> Vec<NormalizedRow> {
    let mut seen: HashSet<(String, String, String)> = HashSet::new();
    rows.into_iter().filter(|r| {
        let vals = &r.values;
        if vals.len() < 5 {
            return true; // keep malformed for logging
        }
        let key = (
            vals[0].as_deref().unwrap_or("").to_string(),   // cis
            vals[2].as_deref().unwrap_or("").to_string(),   // substance_code
            vals[4].as_deref().unwrap_or("").to_string(),   // dosage
        );
        seen.insert(key)
    }).collect()
}
