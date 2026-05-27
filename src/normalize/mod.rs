pub mod price;
pub mod date;
pub mod fields;
pub mod html;
pub mod dedup;

pub use price::parse_price_cents;
pub use date::{parse_date_ddmmYYYY, parse_date_YYYYMMDD};
pub use fields::{strip_field, normalize_generic_type, normalize_spaces};
pub use html::strip_avis_html;
pub use dedup::dedup_compo;

/// Normalize a row from a BDPMFile based on field type.
/// Returns a normalized Vec of String fields ready for INSERT.
pub fn normalize_row(file: crate::download::manifest::BDPMFile, row: &crate::parse::ValidatedRow) -> NormalizedRow {
    let f = &row.fields;
    match file {
        crate::download::manifest::BDPMFile::CIS_bdpm => normalize_cis_bdpm(f),
        crate::download::manifest::BDPMFile::CIS_CIP_bdpm => normalize_cis_cip(f),
        crate::download::manifest::BDPMFile::CIS_COMPO_bdpm => normalize_compo(f),
        crate::download::manifest::BDPMFile::CIS_HAS_SMR_bdpm => normalize_smr(f),
        crate::download::manifest::BDPMFile::CIS_HAS_ASMR_bdpm => normalize_asmr(f),
        crate::download::manifest::BDPMFile::CIS_GENER_bdpm => normalize_gener(f),
        crate::download::manifest::BDPMFile::CIS_CPD_bdpm => normalize_cpd(f),
        crate::download::manifest::BDPMFile::CIS_CIP_Dispo_Spec => normalize_dispo(f),
        crate::download::manifest::BDPMFile::CIS_MITM => normalize_mitm(f),
        crate::download::manifest::BDPMFile::HAS_LiensPageCT_bdpm => normalize_liens(f),
        crate::download::manifest::BDPMFile::CIS_InfoImportantes => normalize_info_importantes(f),
    }
}

// NormalizedRow holds (table_name, Vec<Option<String>>) for INSERT
pub struct NormalizedRow {
    pub table: &'static str,
    pub values: Vec<Option<String>>,
}

// ---- Normalizers per file ----

fn normalize_cis_bdpm(f: &[String]) -> NormalizedRow {
    // 12 fields: cis, name, form, route, auth_status, procedure_type, comm_status, auth_date, lab_name, is_patent, alert_type, eu_number
    NormalizedRow {
        table: "drugs",
        values: vec![
            Some(f[0].clone()),  // cis
            Some(f[1].clone()),  // name_raw (original, before normalization)
            Some(normalize_spaces(&strip_field(&f[1]))),  // name (strip + normalize double-spaces)
            Some(strip_field(&f[2])),  // form
            Some(strip_field(&f[3])),  // route
            Some(strip_field(&f[4])),  // auth_status
            Some(strip_field(&f[5])),  // procedure_type
            Some(strip_field(&f[6])),  // comm_status
            parse_date_ddmmYYYY(&f[7]).ok(),  // auth_date ISO
            Some(normalize_spaces(&strip_field(&f[9]))),  // lab_name (strip + normalize double-spaces)
            Some(if f[10].trim() == "Oui" { "1" } else { "0" }.to_string()),  // is_patent
            if f[11].is_empty() { None } else { Some(strip_field(&f[11])) },  // alert_type
            Some(strip_eu_slash(&f[11])),  // eu_number (field 11)
        ],
    }
}

fn strip_eu_slash(s: &str) -> String {
    s.trim_end_matches('/').to_string()
}

/// Parse French dosage text to numeric mg equivalent.
/// Examples:
/// - "1000 mg" → Some(1000.0)
/// - "1,00 g" → Some(1000.0)
/// - "250 microg" → Some(0.25)
/// - "10 UI" → None (non-weight units)
/// - "" → None
fn parse_dosage_mg(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Normalize: lowercase and handle common variations
    let normalized = trimmed.to_lowercase();

    // Check for non-weight units (UI, etc.) — return None
    if normalized.contains("ui") || normalized.contains("unite") || normalized.contains("million") {
        return None;
    }

    // Parse micrograms: "250 microg", "250 µg", "250 mcg"
    if normalized.contains("micro") || normalized.contains("µg") || normalized.contains("mcg") {
        let num_part: String = normalized.chars().filter(|c| c.is_ascii_digit() || *c == ',').collect();
        if let Ok(val) = num_part.replace(',', ".").parse::<f64>() {
            return Some((val / 1000.0).to_string());
        }
        return None;
    }

    // Parse grams: "1,00 g", "1 g"
    if normalized.contains('g') && !normalized.contains("microg") {
        let num_part: String = normalized.chars().filter(|c| c.is_ascii_digit() || *c == ',').collect();
        if let Ok(val) = num_part.replace(',', ".").parse::<f64>() {
            return Some((val * 1000.0).to_string());
        }
        return None;
    }

    // Parse milligrams: "1000 mg", "500mg"
    if normalized.contains("mg") {
        let num_part: String = normalized.chars().filter(|c| c.is_ascii_digit() || *c == ',').collect();
        if let Ok(val) = num_part.replace(',', ".").parse::<f64>() {
            return Some(val.to_string());
        }
        return None;
    }

    // Fallback: try plain number
    let num_part: String = trimmed.chars().filter(|c| c.is_ascii_digit() || *c == ',').collect();
    if let Ok(val) = num_part.replace(',', ".").parse::<f64>() {
        return Some(val.to_string());
    }

    None
}

fn normalize_cis_cip(f: &[String]) -> NormalizedRow {
    // 12 fields (after trailing empty strip): cis, cip7, labels, pres_status, comm_status, comm_date, ean13, reimbursable, reimb_rate, prix_ht, prix_ville, prix_rate
    NormalizedRow {
        table: "presentations",
        values: vec![
            Some(f[0].clone()),  // cis
            Some(strip_cip7(&f[1])),  // cip (canonical 7-digit from 34009-prefixed CIP13)
            Some(f[1].clone()),  // cip_raw
            Some(strip_field(&f[2])),  // labels
            Some(strip_avis_html(&f[2])),  // labels_clean (HTML stripped)
            Some(strip_field(&f[3])),  // pres_status
            Some(strip_field(&f[4])),  // comm_status
            parse_date_ddmmYYYY(&f[5]).ok(),  // comm_date ISO
            parse_price_cents(&f[9]).ok().flatten().map(|c| c.to_string()),  // prix_ht_cents
            parse_price_cents(&f[10]).ok().flatten().map(|c| c.to_string()),  // prix_ville_cents
            parse_price_cents(&f[11]).ok().flatten().map(|c| c.to_string()),  // prix_rate_cents
            normalize_reimb_rate(&f[8]).map(|r| r.to_string()),  // reimb_rate
            None,  // reimb_conditions (not in CIS_CIP_bdpm source)
            if f[6].is_empty() { None } else { Some(f[6].clone()) },  // ean13
            Some(strip_field(&f[7])),  // reimbursable (oui/non)
        ],
    }
}

fn strip_cip7(cip: &str) -> String {
    let t = cip.trim();
    if t.len() == 13 && t.starts_with("34009") {
        t[6..].to_string()
    } else {
        t.to_string()
    }
}

fn normalize_reimb_rate(s: &str) -> Option<f32> {
    let binding = s.trim().replace(' ', "");
    let cleaned = binding.trim_end_matches('%');
    cleaned.parse::<f32>().ok().map(|v| v / 100.0)
}

fn normalize_compo(f: &[String]) -> NormalizedRow {
    // 8 fields: cis, form_label, substance_code, substance_name, dosage, per_unit, nature, seq
    NormalizedRow {
        table: "compositions",
        values: vec![
            Some(f[0].clone()),
            Some(strip_field(&f[1])),
            Some(f[2].clone()),
            Some(strip_field(&f[3])),
            Some(f[4].clone()),
            Some(f[5].clone()),
            Some(f[6].clone()),
            Some(f[7].clone()),
            Some(normalize_spaces(&strip_field(&f[3]))),  // substance_name_clean
            parse_dosage_mg(&f[4]),                       // dosage_mg (numeric mg equivalent)
        ],
    }
}

fn normalize_smr(f: &[String]) -> NormalizedRow {
    // 6 fields: cis, ct_id, decision_type, decision_date, level, avis
    NormalizedRow {
        table: "smr",
        values: vec![
            Some(f[0].clone()),
            Some(f[1].clone()),
            Some(strip_field(&f[2])),
            parse_date_YYYYMMDD(&f[3]).ok(),
            Some(strip_field(&f[4])),
            Some(strip_avis_html(&f[5])),
        ],
    }
}

fn normalize_asmr(f: &[String]) -> NormalizedRow {
    // 6 fields: same structure as SMR
    NormalizedRow {
        table: "asmr",
        values: vec![
            Some(f[0].clone()),
            Some(f[1].clone()),
            Some(strip_field(&f[2])),
            parse_date_YYYYMMDD(&f[3]).ok(),
            Some(strip_field(&f[4])),
            Some(strip_avis_html(&f[5])),
        ],
    }
}

fn normalize_gener(f: &[String]) -> NormalizedRow {
    // 5 fields: group_id, group_name, cis, type_raw, sort_order
    NormalizedRow {
        table: "generic_groups",
        values: vec![
            Some(f[0].clone()),
            Some(normalize_spaces(&strip_field(&f[1]))), // group_name (strip + normalize double-spaces)
            Some(f[2].clone()),
            Some(normalize_generic_type(&f[3]).to_string()),
            f[4].parse::<i32>().ok().map(|n| n.to_string()),
        ],
    }
}

fn normalize_cpd(f: &[String]) -> NormalizedRow {
    // 2 fields: cis, rule
    NormalizedRow {
        table: "prescription_rules",
        values: vec![
            Some(f[0].clone()),
            Some(strip_field(&f[1])),
        ],
    }
}

fn normalize_dispo(f: &[String]) -> NormalizedRow {
    // 8 fields: cis, cip13(empty), status_type, status_label, date_start, date_end, date_remise(empty), url
    NormalizedRow {
        table: "availability",
        values: vec![
            Some(f[0].clone()),
            Some(f[1].clone()),  // cip13 (empty string valid)
            f[2].parse::<i32>().ok().map(|n| n.to_string()),  // status_type
            Some(strip_field(&f[3])),
            parse_date_ddmmYYYY(&f[4]).ok(),
            parse_date_ddmmYYYY(&f[5]).ok(),
            parse_date_ddmmYYYY(&f[6]).ok(),
            if f[7].is_empty() { None } else { Some(f[7].clone()) },
        ],
    }
}

fn normalize_mitm(f: &[String]) -> NormalizedRow {
    // 4 fields: cis, atc_code, drug_name, url
    NormalizedRow {
        table: "mitm",
        values: vec![
            Some(f[0].clone()),
            Some(f[1].clone()),
            Some(strip_field(&f[2])),
            if f[3].is_empty() { None } else { Some(f[3].clone()) },
        ],
    }
}

fn normalize_liens(f: &[String]) -> NormalizedRow {
    // 2 fields: ct_id, url
    NormalizedRow {
        table: "has_links",
        values: vec![
            Some(f[0].clone()),
            Some(f[1].clone()),
        ],
    }
}

fn normalize_info_importantes(f: &[String]) -> NormalizedRow {
    // 4 fields: cis, start_date (JJ/MM/AAAA), end_date (JJ/MM/AAAA), message + url (HTML)
    // The message field contains embedded <a href="..."> links — strip HTML, keep text
    let raw_msg = strip_avis_html(&f[3]);
    NormalizedRow {
        table: "safety_alerts",
        values: vec![
            Some(f[0].clone()),                        // cis
            parse_date_ddmmYYYY(&f[1]).ok(),           // start_date → ISO
            parse_date_ddmmYYYY(&f[2]).ok(),           // end_date → ISO
            Some(raw_msg),                             // message_plain (HTML stripped)
            None,                                      // source_url (extracted at import time)
        ],
    }
}

#[allow(clippy::manual_flatten)]
pub fn normalize_apostrophes(row: &mut NormalizedRow) {
    for val in &mut row.values {
        if let Some(s) = val {
            *s = s.replace(['\u{2019}', '\u{2018}'], "'");
        }
    }
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
        }
    }

    // --- normalize_apostrophes ---

    #[test]
    fn test_apostrophes_replaced() {
        let mut row = NormalizedRow {
            table: "smr",
            values: vec![Some("L\u{2019}important est d\u{2018}avoir".to_string())],
        };
        normalize_apostrophes(&mut row);
        assert_eq!(row.values[0], Some("L'important est d'avoir".to_string()));
    }

    #[test]
    fn test_apostrophes_no_change() {
        let mut row = NormalizedRow {
            table: "smr",
            values: vec![Some("normal text".to_string())],
        };
        normalize_apostrophes(&mut row);
        assert_eq!(row.values[0], Some("normal text".to_string()));
    }

    #[test]
    fn test_apostrophes_null_field() {
        let mut row = NormalizedRow {
            table: "smr",
            values: vec![None, Some("L\u{2019}important".to_string())],
        };
        normalize_apostrophes(&mut row);
        assert_eq!(row.values[0], None);
        assert_eq!(row.values[1], Some("L'important".to_string()));
    }

    // --- strip_cip7 (used in normalize_cis_cip) ---

    fn strip_cip7(cip: &str) -> String {
        let t = cip.trim();
        if t.len() == 13 && t.starts_with("34009") {
            t[6..].to_string()
        } else {
            t.to_string()
        }
    }

    #[test]
    fn test_strip_cip7_13_digit() {
        // 13-digit EAN with 34009 prefix → 7-digit CIP from position 6
        assert_eq!(strip_cip7("3400930000017"), "0000017");
    }

    #[test]
    fn test_strip_cip7_7_digit() {
        assert_eq!(strip_cip7("3000001"), "3000001");
    }

    #[test]
    fn test_strip_cip7_other() {
        assert_eq!(strip_cip7("1234567890"), "1234567890");
        assert_eq!(strip_cip7(""), "");
    }

    // --- normalize_spaces ---

    #[test]
    fn test_normalize_spaces_double() {
        assert_eq!(normalize_spaces("PARACETAMOL  1000  mg"), "PARACETAMOL 1000 mg");
    }

    #[test]
    fn test_normalize_spaces_leading_trailing() {
        assert_eq!(normalize_spaces("  hello  world  "), "hello world");
    }

    #[test]
    fn test_normalize_spaces_empty() {
        assert_eq!(normalize_spaces(""), "");
    }
}
