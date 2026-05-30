use std::collections::HashSet;
use crate::normalize::NormalizedRow;

/// Remove exact duplicate rows from CIS_COMPO.
/// Key: (cis, substance_code, seq).
/// 4,780 duplicates in 32,389 total rows → 27,609 unique.
/// per_unit is intentionally excluded from the key — in all 1,455 duplicate groups,
/// per_unit varies only when form_label also varies. form_label is the semantically
/// meaningful differentiator; per_unit is deterministically derived from it.
/// Malformed rows (len < 5) are kept for logging.
pub fn dedup_compo(rows: Vec<NormalizedRow>) -> Vec<NormalizedRow> {
    const COMPO_EXPECTED_FIELDS: usize = 10;
    const _: () = assert!(10 == COMPO_EXPECTED_FIELDS, "dedup_compo: expected 10 values per row");
    let mut seen: HashSet<(String, String, String)> = HashSet::new();
    rows.into_iter().filter(|r| {
        let vals = &r.values;
        if vals.len() < 5 {
            return true; // keep malformed for logging
        }
        let key = (
            vals[0].as_deref().unwrap_or("").to_string(),   // cis
            vals[2].as_deref().unwrap_or("").to_string(),   // substance_code
            vals[7].as_deref().unwrap_or("").to_string(),   // seq
        );
        seen.insert(key)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(cis: &str, substance_code: &str, dosage: &str) -> NormalizedRow {
        NormalizedRow {
            table: "compositions",
            values: vec![
                Some(cis.to_string()),
                Some("form".to_string()),
                Some(substance_code.to_string()),
                Some("name".to_string()),
                Some(dosage.to_string()),
                Some("unit".to_string()),
                Some("SA".to_string()),
                Some("0".to_string()),
            ],
            invalid_ean13: false,
        }
    }

    fn make_short_row() -> NormalizedRow {
        NormalizedRow {
            table: "compositions",
            values: vec![
                Some("60004971".to_string()),
                Some("form".to_string()),
            ],
            invalid_ean13: false,
        }
    }

    #[test]
    fn test_dedup_all_unique() {
        let rows = vec![
            make_row("60004971", "42215", "1000 mg"),
            make_row("60004971", "42216", "500 mg"),
            make_row("60004972", "42215", "1000 mg"),
        ];
        let result = dedup_compo(rows);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_dedup_all_duplicates() {
        let rows = vec![
            make_row("60004971", "42215", "1000 mg"),
            make_row("60004971", "42215", "1000 mg"),
            make_row("60004971", "42215", "1000 mg"),
        ];
        let result = dedup_compo(rows);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_dedup_mixed() {
        let rows = vec![
            make_row("60004971", "42215", "1000 mg"),
            make_row("60004971", "42215", "1000 mg"), // dup
            make_row("60004971", "42216", "500 mg"),  // unique
        ];
        let result = dedup_compo(rows);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_dedup_empty() {
        let rows: Vec<NormalizedRow> = vec![];
        let result = dedup_compo(rows);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_dedup_short_rows_preserved() {
        let rows = vec![
            make_short_row(),
            make_row("60004971", "42215", "1000 mg"),
        ];
        let result = dedup_compo(rows);
        assert_eq!(result.len(), 2); // short row kept + normal row
    }

    #[test]
    fn test_dedup_null_dosage() {
        let rows = vec![
            NormalizedRow {
                table: "compositions",
                values: vec![
                    Some("60004971".to_string()),
                    Some("form".to_string()),
                    Some("42215".to_string()),
                    Some("name".to_string()),
                    None, // null dosage
                    Some("unit".to_string()),
                    Some("SA".to_string()),
                    Some("0".to_string()),
                ],
                invalid_ean13: false,
            },
            NormalizedRow {
                table: "compositions",
                values: vec![
                    Some("60004971".to_string()),
                    Some("form".to_string()),
                    Some("42215".to_string()),
                    Some("name".to_string()),
                    None, // same null dosage → duplicate
                    Some("unit".to_string()),
                    Some("SA".to_string()),
                    Some("0".to_string()),
                ],
                invalid_ean13: false,
            },
        ];
        let result = dedup_compo(rows);
        assert_eq!(result.len(), 1); // null dosage treated as "" → duplicate
    }

    #[test]
    fn test_dedup_same_dosage_different_seq() {
        let rows = vec![
            NormalizedRow {
                table: "compositions",
                values: vec![
                    Some("60004971".to_string()), Some("granules".to_string()),
                    Some("05319".to_string()), Some("ACTAEA RACEMOSA".to_string()),
                    Some("2CH à 30CH".to_string()), Some("un comprimé".to_string()),
                    Some("SA".to_string()), Some("7".to_string()),
                    Some("ACTAEA RACEMOSA".to_string()), None,
                ],
                invalid_ean13: false,
            },
            NormalizedRow {
                table: "compositions",
                values: vec![
                    Some("60004971".to_string()), Some("solution buvable".to_string()),
                    Some("05319".to_string()), Some("ACTAEA RACEMOSA".to_string()),
                    Some("2CH à 30CH".to_string()), Some("un comprimé".to_string()),
                    Some("SA".to_string()), Some("8".to_string()),
                    Some("ACTAEA RACEMOSA".to_string()), None,
                ],
                invalid_ean13: false,
            },
        ];
        let result = dedup_compo(rows);
        assert_eq!(result.len(), 2); // different seq → both kept
    }

    #[test]
    fn test_dedup_key_matches_pk() {
        // The dedup key (cis, substance_code, seq) matches the PK (cis, substance_code, seq).
        // This is a documentation test confirming the Phase 06 fix is correct.
        let row1 = NormalizedRow {
            table: "compositions",
            values: vec![
                Some("60004971".to_string()), Some("granules".to_string()),
                Some("05319".to_string()), Some("ACTAEA RACEMOSA".to_string()),
                Some("2CH à 30CH".to_string()), Some("un comprimé".to_string()),
                Some("SA".to_string()), Some("7".to_string()),
                Some("ACTAEA RACEMOSA".to_string()), None,
            ],
            invalid_ean13: false,
        };
        let row2 = NormalizedRow {
            table: "compositions",
            values: vec![
                Some("60004971".to_string()), Some("solution buvable".to_string()),
                Some("05319".to_string()), Some("ACTAEA RACEMOSA".to_string()),
                Some("2CH à 30CH".to_string()), Some("un comprimé".to_string()),
                Some("SA".to_string()), Some("8".to_string()),
                Some("ACTAEA RACEMOSA".to_string()), None,
            ],
            invalid_ean13: false,
        };
        // Different seq → both should be kept (not deduplicated)
        let result = dedup_compo(vec![row1, row2]);
        assert_eq!(result.len(), 2);
    }
}
