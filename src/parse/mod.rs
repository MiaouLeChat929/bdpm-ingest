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
