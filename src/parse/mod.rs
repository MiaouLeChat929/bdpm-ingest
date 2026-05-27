pub mod tab;
pub use tab::TabParser;

pub use crate::download::manifest::BDPMFile;

pub struct ValidatedRow {
    pub fields: Vec<String>,
    pub line_number: usize,
}

pub struct ValidationResult {
    pub total_rows: usize,
    pub skipped_rows: usize,
    pub warnings: usize,
    pub errors: usize,
}

/// Parse a file with the given BDPMFile schema
pub fn parse_file(path: &std::path::Path, file: BDPMFile) -> anyhow::Result<Vec<ValidatedRow>> {
    let parser = TabParser::from_path(path, file)?;
    let mut rows = Vec::new();
    let mut skipped = 0;
    let schema = file.schema();

    for item in parser {
        match item {
            Ok(fields) => {
                // For files with known trailing-tab issue, strip exactly one trailing empty
                let fields = if schema.has_trailing_tab_fix {
                    strip_one_trailing_empty(fields)
                } else {
                    fields
                };
                if fields.len() == schema.field_count {
                    rows.push(ValidatedRow {
                        fields,
                        line_number: rows.len() + skipped + 1,
                    });
                } else if fields.len() < schema.field_count && fields.len() >= schema.field_count / 2 {
                    // Pad short rows with empty strings (e.g., CIS_CIP 8-field non-commercialisé rows).
                    // Threshold is half of expected fields — safe for all files.
                    let mut padded = fields;
                    while padded.len() < schema.field_count {
                        padded.push(String::new());
                    }
                    rows.push(ValidatedRow {
                        fields: padded,
                        line_number: rows.len() + skipped + 1,
                    });
                } else if fields.len() < schema.field_count {
                    skipped += 1;
                } else {
                    // More fields than expected — keep but warn
                    rows.push(ValidatedRow {
                        fields,
                        line_number: rows.len() + skipped + 1,
                    });
                }
            }
            Err(e) => {
                tracing::warn!("Parse error: {}", e);
            }
        }
    }

    tracing::info!(
        "Parsed {}: {} rows, {} skipped",
        file.filename(),
        rows.len(),
        skipped
    );

    Ok(rows)
}

/// Strip exactly one trailing empty field (handles the phantom trailing-tab in CIS_CIP_bdpm).
/// Preserves legitimate empty fields in the middle.
fn strip_one_trailing_empty(mut fields: Vec<String>) -> Vec<String> {
    if fields.last().map(|s| s.is_empty()).unwrap_or(false) {
        fields.pop();
    }
    fields
}

pub fn count_rows(path: &std::path::Path) -> anyhow::Result<usize> {
    let bytes = std::fs::read(path)?;
    Ok(bytes.iter().filter(|&&b| b == b'\n').count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_trailing_empty_single() {
        let fields = vec!["a".to_string(), "b".to_string(), "".to_string()];
        let result = strip_one_trailing_empty(fields);
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_strip_trailing_empty_no_trailing() {
        let fields = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = strip_one_trailing_empty(fields.clone());
        assert_eq!(result, fields);
    }

    #[test]
    fn test_strip_trailing_empty_preserves_middle_empty() {
        let fields = vec!["a".to_string(), "".to_string(), "c".to_string(), "".to_string()];
        let result = strip_one_trailing_empty(fields);
        assert_eq!(result, vec!["a".to_string(), "".to_string(), "c".to_string()]);
    }

    #[test]
    fn test_strip_trailing_empty_empty_vec() {
        let fields: Vec<String> = vec![];
        let result = strip_one_trailing_empty(fields.clone());
        assert_eq!(result, fields);
    }

    #[test]
    fn test_strip_trailing_empty_only_empty() {
        let fields = vec!["".to_string()];
        let result = strip_one_trailing_empty(fields);
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_parse_file_short_row_padding() {
        // Row with fewer fields than expected gets padded with empty strings
        // Threshold is >= half of expected field_count (half of 12 = 6)
        let tmp = tempfile::NamedTempFile::new().unwrap();
        // 6 fields (meets half threshold), will be padded to 12
        std::fs::write(tmp.path(), "field1\tfield2\tfield3\tfield4\tfield5\tfield6\n").unwrap();
        let rows = parse_file(tmp.path(), BDPMFile::CIS_bdpm).unwrap();
        assert_eq!(rows.len(), 1);
        // CIS_bdpm has 12 fields, we gave 6 -> padded to 12
        assert_eq!(rows[0].fields.len(), 12);
    }

    #[test]
    fn test_parse_file_exact_half_fields() {
        // Row with exactly half the fields gets padded
        let tmp = tempfile::NamedTempFile::new().unwrap();
        // CIS_bdpm has 12 fields, half is 6
        std::fs::write(tmp.path(), "field1\tfield2\tfield3\tfield4\tfield5\tfield6\n").unwrap();
        let rows = parse_file(tmp.path(), BDPMFile::CIS_bdpm).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].fields.len(), 12);
    }

    #[test]
    fn test_parse_file_more_fields_than_expected() {
        // Row with more fields than expected is kept as-is
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "field1\tfield2\tfield3\tfield4\tfield5\tfield6\tfield7\tfield8\tfield9\tfield10\tfield11\tfield12\tfield13\tfield14\tfield15\n").unwrap();
        let rows = parse_file(tmp.path(), BDPMFile::CIS_bdpm).unwrap();
        assert_eq!(rows.len(), 1);
        // More fields kept as-is (15 fields)
        assert_eq!(rows[0].fields.len(), 15);
    }
}
