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
    // 12 fields: cis, name, form, route, auth_status, procedure_type, comm_status, auth_date, alert_type, lab_name, is_patent, eu_number
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
            Some(if f[10].trim().eq_ignore_ascii_case("oui") { "1" } else { "0" }.to_string()),  // is_patent
            if f[8].is_empty() { None } else { Some(strip_field(&f[8])) },  // alert_type
            Some(strip_eu_slash(&f[11])),  // eu_number (field 11)
        ],
    }
}

fn strip_eu_slash(s: &str) -> String {
    s.trim_end()
        .strip_suffix('/')
        .map(|rest| rest.to_string())
        .unwrap_or_else(|| s.to_string())
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
    // Note: "unite" must match accented "unités" too
    if normalized.contains("ui")
        || normalized.contains("unite")
        || normalized.contains("million")
    {
        return None;
    }

    // Reject homeopathic dilution patterns (CH, DH, K, X, LM)
    // e.g., "4CH à 30CH", "5 mg (4 DH)", "30K"
    if normalized
        .split(|c: char| !c.is_alphabetic())
        .any(|w| matches!(w, "ch" | "dh" | "k" | "x" | "lm"))
    {
        return None;
    }

    // Reject range patterns ("45 - 70 mg", "10 à 20 mg")
    // e.g., "45 - 70 mg" would produce 4570.0 which is wrong
    if (normalized.contains('-') || normalized.contains("à"))
        && (normalized.contains("mg")
            || normalized.contains('g')
            || normalized.contains("µg")
            || normalized.contains("mcg"))
    {
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

    // Parse milligrams: "1000 mg", "500mg" (check BEFORE g to catch "mg" first)
    if normalized.contains("mg") && !normalized.contains("microg") {
        let num_part: String = normalized.chars().filter(|c| c.is_ascii_digit() || *c == ',').collect();
        if let Ok(val) = num_part.replace(',', ".").parse::<f64>() {
            return Some(val.to_string());
        }
        return None;
    }

    // Parse grams: "1,00 g", "1 g"
    if normalized.contains('g') {
        let num_part: String = normalized.chars().filter(|c| c.is_ascii_digit() || *c == ',').collect();
        if let Ok(val) = num_part.replace(',', ".").parse::<f64>() {
            return Some((val * 1000.0).to_string());
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
    let binding = s.trim().replace(' ', "").replace(',', ".");
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
    // Raw: 4 fields (cis, atc_code, drug_name, url)
    // DB:  3 columns (cis, atc_code, detail_url) — drug_name not stored
    NormalizedRow {
        table: "mitm",
        values: vec![
            Some(f[0].clone()),
            Some(f[1].clone()),
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

    // --- normalize_cis_bdpm tests (via normalize_row) ---

    /// Helper to create a ValidatedRow for CIS_bdpm with 12 fields
    fn make_cis_bdpm_row(fields: [&str; 12]) -> crate::parse::ValidatedRow {
        crate::parse::ValidatedRow {
            fields: fields.map(String::from).to_vec(),
            line_number: 1,
        }
    }

    #[test]
    fn test_normalize_cis_bdpm_basic() {
        let row = make_cis_bdpm_row([
            "60004971",           // f[0]: cis
            "Doliprane",          // f[1]: name
            "comprimé",           // f[2]: form
            "orale",              // f[3]: route
            "Autorisation active", // f[4]: auth_status
            "Procédure nationale", // f[5]: procedure_type
            "Commercialisée",     // f[6]: comm_status
            "12/03/1998",         // f[7]: auth_date
            "",                   // f[8]: alert_type (empty)
            "SANOFI",             // f[9]: lab_name
            "Oui",                // f[10]: is_patent
            "EU/1/17/1235/",      // f[11]: eu_number (with trailing slash)
        ]);

        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_bdpm, &row);

        assert_eq!(result.table, "drugs");
        assert_eq!(result.values.len(), 13);

        // values[0]: cis
        assert_eq!(result.values[0], Some("60004971".to_string()));
        // values[1]: name_raw (original)
        assert_eq!(result.values[1], Some("Doliprane".to_string()));
        // values[2]: name (normalized)
        assert_eq!(result.values[2], Some("Doliprane".to_string()));
        // values[3]: form
        assert_eq!(result.values[3], Some("comprimé".to_string()));
        // values[4]: route
        assert_eq!(result.values[4], Some("orale".to_string()));
        // values[5]: auth_status
        assert_eq!(result.values[5], Some("Autorisation active".to_string()));
        // values[6]: procedure_type
        assert_eq!(result.values[6], Some("Procédure nationale".to_string()));
        // values[7]: comm_status
        assert_eq!(result.values[7], Some("Commercialisée".to_string()));
        // values[8]: auth_date (ISO-8601)
        assert_eq!(result.values[8], Some("1998-03-12".to_string()));
        // values[9]: lab_name
        assert_eq!(result.values[9], Some("SANOFI".to_string()));
        // values[10]: is_patent ("Oui" → "1")
        assert_eq!(result.values[10], Some("1".to_string()));
        // values[11]: alert_type (empty → None)
        assert_eq!(result.values[11], None);
        // values[12]: eu_number (trailing slash stripped)
        assert_eq!(result.values[12], Some("EU/1/17/1235".to_string()));
    }

    #[test]
    fn test_normalize_cis_bdpm_name_whitespace() {
        let row = make_cis_bdpm_row([
            "60004971",
            " Doliprane ",        // f[1]: name with leading/trailing space
            "comprimé",
            "orale",
            "Autorisation active",
            "Procédure nationale",
            "Commercialisée",
            "12/03/1998",
            "",
            "SANOFI",
            "Oui",
            "EU/1/17/1235/",
        ]);

        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_bdpm, &row);

        // name_raw should preserve original whitespace
        assert_eq!(result.values[1], Some(" Doliprane ".to_string()));
        // name should have whitespace stripped
        assert_eq!(result.values[2], Some("Doliprane".to_string()));
    }

    #[test]
    fn test_normalize_cis_bdpm_lab_name_whitespace() {
        let row = make_cis_bdpm_row([
            "60004971",
            "Doliprane",
            "comprimé",
            "orale",
            "Autorisation active",
            "Procédure nationale",
            "Commercialisée",
            "12/03/1998",
            "",
            " SANOFI ",            // f[9]: lab_name with leading/trailing space
            "Oui",
            "EU/1/17/1235/",
        ]);

        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_bdpm, &row);

        // lab_name should have whitespace stripped
        assert_eq!(result.values[9], Some("SANOFI".to_string()));
    }

    #[test]
    fn test_normalize_cis_bdpm_is_patent_oui_non() {
        // Test "Oui" → "1"
        let row_oui = make_cis_bdpm_row([
            "60004971",
            "Doliprane",
            "comprimé",
            "orale",
            "Autorisation active",
            "Procédure nationale",
            "Commercialisée",
            "12/03/1998",
            "",
            "SANOFI",
            "Oui",                 // f[10]: is_patent
            "EU/1/17/1235/",
        ]);
        let result_oui = normalize_row(crate::download::manifest::BDPMFile::CIS_bdpm, &row_oui);
        assert_eq!(result_oui.values[10], Some("1".to_string()));

        // Test "Non" → "0"
        let row_non = make_cis_bdpm_row([
            "60004971",
            "Doliprane",
            "comprimé",
            "orale",
            "Autorisation active",
            "Procédure nationale",
            "Commercialisée",
            "12/03/1998",
            "",
            "SANOFI",
            "Non",                // f[10]: is_patent
            "EU/1/17/1235/",
        ]);
        let result_non = normalize_row(crate::download::manifest::BDPMFile::CIS_bdpm, &row_non);
        assert_eq!(result_non.values[10], Some("0".to_string()));
    }

    #[test]
    fn test_normalize_cis_bdpm_eu_number_slash() {
        // Test with trailing slash
        let row_slash = make_cis_bdpm_row([
            "60004971",
            "Doliprane",
            "comprimé",
            "orale",
            "Autorisation active",
            "Procédure nationale",
            "Commercialisée",
            "12/03/1998",
            "",
            "SANOFI",
            "Oui",
            "EU/1/17/1235/",      // f[11]: eu_number with trailing slash
        ]);
        let result_slash = normalize_row(crate::download::manifest::BDPMFile::CIS_bdpm, &row_slash);
        assert_eq!(result_slash.values[12], Some("EU/1/17/1235".to_string()));

        // Test without trailing slash
        let row_no_slash = make_cis_bdpm_row([
            "60004971",
            "Doliprane",
            "comprimé",
            "orale",
            "Autorisation active",
            "Procédure nationale",
            "Commercialisée",
            "12/03/1998",
            "",
            "SANOFI",
            "Oui",
            "EU/1/17/1235",       // f[11]: eu_number without trailing slash
        ]);
        let result_no_slash = normalize_row(crate::download::manifest::BDPMFile::CIS_bdpm, &row_no_slash);
        assert_eq!(result_no_slash.values[12], Some("EU/1/17/1235".to_string()));
    }

    #[test]
    fn test_normalize_cis_bdpm_alert_type_null() {
        // Test empty alert_type → None
        let row_empty = make_cis_bdpm_row([
            "60004971",
            "Doliprane",
            "comprimé",
            "orale",
            "Autorisation active",
            "Procédure nationale",
            "Commercialisée",
            "12/03/1998",
            "",                   // f[8]: alert_type (empty)
            "SANOFI",
            "Oui",
            "EU/1/17/1235/",
        ]);
        let result_empty = normalize_row(crate::download::manifest::BDPMFile::CIS_bdpm, &row_empty);
        assert_eq!(result_empty.values[11], None);

        // Test non-empty alert_type → Some
        let row_with_alert = make_cis_bdpm_row([
            "60004971",
            "Doliprane",
            "comprimé",
            "orale",
            "Autorisation active",
            "Procédure nationale",
            "Commercialisée",
            "12/03/1998",
            "Rupture de stock",   // f[8]: alert_type (non-empty)
            "SANOFI",
            "Oui",
            "EU/1/17/1235/",
        ]);
        let result_with_alert = normalize_row(crate::download::manifest::BDPMFile::CIS_bdpm, &row_with_alert);
        assert_eq!(result_with_alert.values[11], Some("Rupture de stock".to_string()));
    }

    // --- normalize_dispo tests (via normalize_row) ---

    /// Helper to create a ValidatedRow for CIS_CIP_Dispo_Spec with 8 fields
    fn make_dispo_row(fields: [&str; 8]) -> crate::parse::ValidatedRow {
        crate::parse::ValidatedRow {
            fields: fields.map(String::from).to_vec(),
            line_number: 1,
        }
    }

    #[test]
    fn test_normalize_dispo_basic() {
        let row = make_dispo_row([
            "60004971",                   // f[0]: cis
            "3400930000017",              // f[1]: cip13
            "1",                          // f[2]: status_type (1=Rupture)
            "Rupture de stock",           // f[3]: status_label
            "01/05/2026",                 // f[4]: date_start
            "31/05/2026",                 // f[5]: date_end
            "",                           // f[6]: date_remise (empty)
            "https://ansm.gouv.fr/disp",  // f[7]: source_url
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_CIP_Dispo_Spec, &row);
        assert_eq!(result.table, "availability");
        assert_eq!(result.values[0], Some("60004971".to_string()));
        assert_eq!(result.values[1], Some("3400930000017".to_string()));
        assert_eq!(result.values[2], Some("1".to_string()));
        assert_eq!(result.values[3], Some("Rupture de stock".to_string()));
        assert_eq!(result.values[4], Some("2026-05-01".to_string()));
        assert_eq!(result.values[5], Some("2026-05-31".to_string()));
        assert_eq!(result.values[6], None); // empty date_remise → None
        assert_eq!(result.values[7], Some("https://ansm.gouv.fr/disp".to_string()));
    }

    #[test]
    fn test_normalize_dispo_empty_cip() {
        let row = make_dispo_row([
            "60004971", "", "1", "Rupture", "01/05/2026", "", "", "",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_CIP_Dispo_Spec, &row);
        assert_eq!(result.values[1], Some("".to_string())); // empty cip13 preserved as empty string
        assert_eq!(result.values[5], None); // empty date_end → None
        assert_eq!(result.values[6], None); // empty date_remise → None
        assert_eq!(result.values[7], None); // empty source_url → None
    }

    #[test]
    fn test_normalize_dispo_invalid_status_type() {
        let row = make_dispo_row([
            "60004971", "", "abc", "Unknown", "01/05/2026", "", "", "",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_CIP_Dispo_Spec, &row);
        assert_eq!(result.values[2], None); // non-numeric status_type → None
    }

    // --- normalize_info_importantes tests (via normalize_row) ---

    /// Helper to create a ValidatedRow for CIS_InfoImportantes with 4 fields
    fn make_info_row(fields: [&str; 4]) -> crate::parse::ValidatedRow {
        crate::parse::ValidatedRow {
            fields: fields.map(String::from).to_vec(),
            line_number: 1,
        }
    }

    #[test]
    fn test_normalize_info_importantes_basic() {
        let row = make_info_row([
            "60004971",
            "01/01/2026",
            "31/12/2026",
            "Alerte de sécurité importante https://ansm.gouv.fr/alarm/123",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_InfoImportantes, &row);
        assert_eq!(result.table, "safety_alerts");
        assert_eq!(result.values[0], Some("60004971".to_string()));
        assert_eq!(result.values[1], Some("2026-01-01".to_string()));
        assert_eq!(result.values[2], Some("2026-12-31".to_string()));
        // normalize_info_importantes does NOT extract the URL — that happens in import/mod.rs
        // So message_plain retains the full text including URL
        assert_eq!(result.values[3], Some("Alerte de sécurité importante https://ansm.gouv.fr/alarm/123".to_string()));
        assert_eq!(result.values[4], None); // source_url extracted at import time, not here
    }

    #[test]
    fn test_normalize_info_importantes_html_in_message() {
        let row = make_info_row([
            "60004971",
            "01/01/2026",
            "31/12/2026",
            "<p>Alerte <b>importante</b><br>Nouvelle info</p>",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_InfoImportantes, &row);
        // HTML should be stripped, no URL extracted
        assert_eq!(result.values[3], Some("Alerte importante\nNouvelle info".to_string()));
        assert_eq!(result.values[4], None); // no URL
    }

    #[test]
    fn test_normalize_info_importantes_no_url() {
        let row = make_info_row([
            "60004971",
            "01/01/2026",
            "",
            "Message sans URL",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_InfoImportantes, &row);
        assert_eq!(result.values[2], None); // empty end_date → None
        assert_eq!(result.values[3], Some("Message sans URL".to_string()));
        assert_eq!(result.values[4], None);
    }

    #[test]
    fn test_normalize_info_importantes_date_out_of_range() {
        let row = make_info_row([
            "60004971",
            "01/01/2924", // far future
            "31/12/2026",
            "Test",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_InfoImportantes, &row);
        assert_eq!(result.values[1], None); // out-of-range date → None
    }

    // --- parse_dosage_mg tests ---

    // Empty string
    #[test]
    fn test_parse_dosage_mg_empty() {
        assert_eq!(parse_dosage_mg(""), None);
        assert_eq!(parse_dosage_mg("   "), None);
    }

    // Milligrams
    #[test]
    fn test_parse_dosage_mg_milligrams() {
        assert_eq!(parse_dosage_mg("1000 mg"), Some("1000".to_string()));
        assert_eq!(parse_dosage_mg("500mg"), Some("500".to_string()));
        assert_eq!(parse_dosage_mg("250 MG"), Some("250".to_string()));
        assert_eq!(parse_dosage_mg("100 mg"), Some("100".to_string()));
    }

    // Grams
    #[test]
    fn test_parse_dosage_mg_grams() {
        assert_eq!(parse_dosage_mg("1,00 g"), Some("1000".to_string()));
        assert_eq!(parse_dosage_mg("1 g"), Some("1000".to_string()));
        assert_eq!(parse_dosage_mg("1,5 g"), Some("1500".to_string()));
        assert_eq!(parse_dosage_mg("0,5 g"), Some("500".to_string()));
    }

    // Micrograms
    #[test]
    fn test_parse_dosage_mg_micrograms() {
        assert_eq!(parse_dosage_mg("250 microg"), Some("0.25".to_string()));
        assert_eq!(parse_dosage_mg("250 µg"), Some("0.25".to_string()));
        assert_eq!(parse_dosage_mg("250 mcg"), Some("0.25".to_string()));
        assert_eq!(parse_dosage_mg("1000 microg"), Some("1".to_string()));
    }

    // Non-weight units → None
    #[test]
    fn test_parse_dosage_mg_non_weight_units() {
        assert_eq!(parse_dosage_mg("10 UI"), None);
        assert_eq!(parse_dosage_mg("500 unite"), None);
        assert_eq!(parse_dosage_mg("1 million"), None);
        assert_eq!(parse_dosage_mg("100 UI/0.5ml"), None); // mixed with mg should still be None due to UI
    }

    // Fallback plain number
    #[test]
    fn test_parse_dosage_mg_plain_number() {
        assert_eq!(parse_dosage_mg("500"), Some("500".to_string()));
        assert_eq!(parse_dosage_mg("1,5"), Some("1.5".to_string()));
    }

    // Invalid/unparseable
    #[test]
    fn test_parse_dosage_mg_invalid() {
        assert_eq!(parse_dosage_mg("abc"), None);
        assert_eq!(parse_dosage_mg("no numbers here"), None);
    }

    // --- strip_eu_slash edge cases ---

    #[test]
    fn test_strip_eu_slash_multiple() {
        // "EU/1/17/1235//" → strips only ONE trailing slash → "EU/1/17/1235/"
        // (BDPM sometimes sends double-slash, first is real, second is artifact)
        assert_eq!(strip_eu_slash("EU/1/17/1235//"), "EU/1/17/1235/");
    }

    #[test]
    fn test_strip_eu_slash_none() {
        // No trailing slash: unchanged
        assert_eq!(strip_eu_slash("EU/1/17/1235"), "EU/1/17/1235");
    }

    #[test]
    fn test_strip_eu_slash_empty() {
        // Empty string: no-op
        assert_eq!(strip_eu_slash(""), "");
    }

    // --- normalize_cis_bdpm is_patent case sensitivity ---

    #[test]
    fn test_normalize_cis_bdpm_is_patent_uppercase() {
        // "OUI" (uppercase) should match case-insensitively — "oui" → "1"
        let row_oui_upper = make_cis_bdpm_row([
            "60004971", "Doliprane", "comprimé", "orale",
            "Autorisation active", "Procédure nationale", "Commercialisée",
            "12/03/1998", "", "SANOFI",
            "OUI",    // f[10]: uppercase — matches case-insensitively
            "EU/1/17/1235/",
        ]);
        let result_oui_upper = normalize_row(crate::download::manifest::BDPMFile::CIS_bdpm, &row_oui_upper);
        assert_eq!(result_oui_upper.values[10], Some("1".to_string())); // matches "oui" case-insensitively

        // "Non" (proper-case): is_patent is case-sensitive, "Non" != "Oui" → "0"
        let row_non = make_cis_bdpm_row([
            "60004971", "Doliprane", "comprimé", "orale",
            "Autorisation active", "Procédure nationale", "Commercialisée",
            "12/03/1998", "", "SANOFI",
            "Non",    // f[10]: proper-case "Non" — only exact "Oui" maps to "1"
            "EU/1/17/1235/",
        ]);
        let result_non = normalize_row(crate::download::manifest::BDPMFile::CIS_bdpm, &row_non);
        assert_eq!(result_non.values[10], Some("0".to_string()));
    }

    // --- normalize_reimb_rate edge cases ---

    #[test]
    fn test_normalize_reimb_rate_variations() {
        // "65,0%" — comma as decimal separator in French locale
        // normalize_reimb_rate trims %, replaces comma with dot → "65.0" → parses as 65.0 → 0.65
        assert_eq!(normalize_reimb_rate("65,0%"), Some(0.65));

        // "065%" — leading zero: parse "65" as f32 → 65.0 → 0.65
        assert_eq!(normalize_reimb_rate("065%"), Some(0.65));

        // " 65%" — leading space trimmed by normalize_reimb_rate
        assert_eq!(normalize_reimb_rate(" 65%"), Some(0.65));
    }

    // --- normalize_cis_cip empty prices ---

    /// Helper: 12-field row for CIS_CIP_bdpm
    fn make_cis_cip_row(fields: [&str; 12]) -> crate::parse::ValidatedRow {
        crate::parse::ValidatedRow {
            fields: fields.map(String::from).to_vec(),
            line_number: 1,
        }
    }

    #[test]
    fn test_normalize_cis_cip_empty_prices() {
        // All price fields (f[9], f[10], f[11]) are empty → None
        let row = make_cis_cip_row([
            "60004971",       // f[0]: cis
            "3400930000017",  // f[1]: cip13 (34009-prefixed)
            "Label",          // f[2]: labels
            "Commercialisée", // f[3]: pres_status
            "Commercialisé",   // f[4]: comm_status
            "01/01/2020",     // f[5]: comm_date
            "",               // f[6]: ean13 (empty)
            "Oui",            // f[7]: reimbursable
            "65%",            // f[8]: reimb_rate
            "",               // f[9]: prix_ht (empty)
            "",               // f[10]: prix_ville (empty)
            "",               // f[11]: prix_rate (empty)
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_CIP_bdpm, &row);
        // prix_ht_cents (index 8), prix_ville_cents (index 9), prix_rate_cents (index 10)
        assert_eq!(result.values[8], None);  // empty prix_ht → None
        assert_eq!(result.values[9], None);  // empty prix_ville → None
        assert_eq!(result.values[10], None); // empty prix_rate → None
    }

    // --- normalize_dispo date out of range ---

    #[test]
    fn test_normalize_dispo_date_out_of_range() {
        // f[4]="29/11/2924" — year 2924 is outside the 1900–2100 range
        let row = make_dispo_row([
            "60004971", "3400930000017", "1", "Rupture",
            "29/11/2924",  // f[4]: date_start out of range
            "", "", "",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_CIP_Dispo_Spec, &row);
        assert_eq!(result.values[4], None); // out-of-range date → None
    }

    // --- strip_cip_ean non-34009 prefix ---

    #[test]
    fn test_strip_cip7_non_34009_prefix() {
        // "1234567890" — 10 digits, does NOT start with "34009"
        // strip_cip_ean should return unchanged (needs 13-digit 34009-prefix to strip)
        assert_eq!(fields::strip_cip_ean("1234567890"), "1234567890");
    }

    // --- normalize_cis_cip basic and strip_cip7 ---

    #[test]
    fn test_normalize_cis_cip_basic() {
        // 12 fields: cis, cip7, labels, pres_status, comm_status, comm_date, ean13, reimbursable, reimb_rate, prix_ht, prix_ville, prix_rate
        let row = make_cis_cip_row([
            "60004971",              // f[0]: cis
            "3400930000017",         // f[1]: cip7 (13-digit EAN)
            "Boite de 16",           // f[2]: labels
            "Prescrit",              // f[3]: pres_status
            "Commercialisé",         // f[4]: comm_status
            "12/03/1998",            // f[5]: comm_date
            "3400930000017",         // f[6]: ean13
            "Oui",                   // f[7]: reimbursable
            "65%",                   // f[8]: reimb_rate
            "5,99",                  // f[9]: prix_ht
            "6,57",                  // f[10]: prix_ville
            "5,99",                  // f[11]: prix_rate
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_CIP_bdpm, &row);

        assert_eq!(result.table, "presentations");
        assert_eq!(result.values.len(), 15);
        assert_eq!(result.values[0], Some("60004971".to_string())); // cis
        assert_eq!(result.values[1], Some("0000017".to_string())); // cip (strip_cip7)
        assert_eq!(result.values[2], Some("3400930000017".to_string())); // cip_raw
        assert_eq!(result.values[3], Some("Boite de 16".to_string())); // labels
        assert_eq!(result.values[4], Some("Boite de 16".to_string())); // labels_clean (HTML stripped)
        assert_eq!(result.values[5], Some("Prescrit".to_string())); // pres_status
        assert_eq!(result.values[6], Some("Commercialisé".to_string())); // comm_status
        assert_eq!(result.values[7], Some("1998-03-12".to_string())); // comm_date
        assert_eq!(result.values[8], Some("599".to_string())); // prix_ht_cents
        assert_eq!(result.values[9], Some("657".to_string())); // prix_ville_cents
        assert_eq!(result.values[10], Some("599".to_string())); // prix_rate_cents
        assert_eq!(result.values[11], Some("0.65".to_string())); // reimb_rate
        assert_eq!(result.values[12], None); // reimb_conditions (always None for CIS_CIP_bdpm)
        assert_eq!(result.values[13], Some("3400930000017".to_string())); // ean13
        assert_eq!(result.values[14], Some("Oui".to_string())); // reimbursable
    }

    #[test]
    fn test_normalize_cis_cip_price_with_comma() {
        // Test French comma decimal: "24,34" → 2434 cents
        let row = make_cis_cip_row([
            "60004971",
            "3400930000017",
            "Boite de 16",
            "Prescrit",
            "Commercialisé",
            "12/03/1998",
            "3400930000017",
            "Oui",
            "65%",
            "24,34",                 // f[9]: prix_ht with comma
            "25,99",
            "24,34",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_CIP_bdpm, &row);
        assert_eq!(result.values[8], Some("2434".to_string())); // "24,34" → 2434 cents
        assert_eq!(result.values[9], Some("2599".to_string())); // "25,99" → 2599 cents
    }

    #[test]
    fn test_normalize_cis_cip_reimb_rate_variations() {
        // Test reimb_rate: "65%" → "0.65"
        let row_pct = make_cis_cip_row([
            "60004971", "3400930000017", "Boite", "Prescrit", "Commercialisé",
            "12/03/1998", "3400930000017", "Oui", "65%", "", "", "",
        ]);
        let result_pct = normalize_row(crate::download::manifest::BDPMFile::CIS_CIP_bdpm, &row_pct);
        assert_eq!(result_pct.values[11], Some("0.65".to_string()));

        // Test reimb_rate: "65 %" (with space) → "0.65"
        let row_space = make_cis_cip_row([
            "60004971", "3400930000017", "Boite", "Prescrit", "Commercialisé",
            "12/03/1998", "3400930000017", "Oui", "65 %", "", "", "",
        ]);
        let result_space = normalize_row(crate::download::manifest::BDPMFile::CIS_CIP_bdpm, &row_space);
        assert_eq!(result_space.values[11], Some("0.65".to_string()));

        // Test reimb_rate: empty → None
        let row_empty = make_cis_cip_row([
            "60004971", "3400930000017", "Boite", "Prescrit", "Commercialisé",
            "12/03/1998", "3400930000017", "Oui", "", "", "", "",
        ]);
        let result_empty = normalize_row(crate::download::manifest::BDPMFile::CIS_CIP_bdpm, &row_empty);
        assert_eq!(result_empty.values[11], None);
    }

    #[test]
    fn test_normalize_cis_cip_empty_ean13() {
        let row = make_cis_cip_row([
            "60004971",
            "3400930000017",
            "Boite de 16",
            "Prescrit",
            "Commercialisé",
            "12/03/1998",
            "",                      // f[6]: ean13 (empty)
            "Oui",
            "65%",
            "",
            "",
            "",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_CIP_bdpm, &row);
        assert_eq!(result.values[13], None); // ean13 → None
        assert_eq!(result.values[14], Some("Oui".to_string())); // reimbursable (not empty)
    }

    #[test]
    fn test_normalize_cis_cip_strip_cip7() {
        // Test strip_cip7: "3400930000017" → cip = "0000017", cip_raw = "3400930000017"
        let row = make_cis_cip_row([
            "60004971",
            "3400930000017",         // 13-digit with 34009 prefix
            "Boite de 16",
            "Prescrit",
            "Commercialisé",
            "12/03/1998",
            "3400930000017",
            "Oui",
            "65%",
            "",
            "",
            "",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_CIP_bdpm, &row);
        assert_eq!(result.values[1], Some("0000017".to_string())); // cip (7-digit from pos 6)
        assert_eq!(result.values[2], Some("3400930000017".to_string())); // cip_raw (full)
    }

    // --- normalize_compo tests (via normalize_row) ---

    /// Helper to create a ValidatedRow for CIS_COMPO_bdpm with 8 fields
    fn make_compo_row(fields: [&str; 8]) -> crate::parse::ValidatedRow {
        crate::parse::ValidatedRow {
            fields: fields.map(String::from).to_vec(),
            line_number: 1,
        }
    }

    #[test]
    fn test_normalize_compo_basic() {
        // 8 fields: cis, form_label, substance_code, substance_name, dosage, per_unit, nature, seq
        let row = make_compo_row([
            "60004971",
            "Comprimé",
            "1124",
            "Paracétamol",
            "500 mg",
            "1 comprimé",
            "SA",
            "0",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_COMPO_bdpm, &row);

        assert_eq!(result.table, "compositions");
        assert_eq!(result.values.len(), 10);
        assert_eq!(result.values[0], Some("60004971".to_string())); // cis
        assert_eq!(result.values[1], Some("Comprimé".to_string())); // form_label
        assert_eq!(result.values[2], Some("1124".to_string())); // substance_code
        assert_eq!(result.values[3], Some("Paracétamol".to_string())); // substance_name
        assert_eq!(result.values[4], Some("500 mg".to_string())); // dosage
        assert_eq!(result.values[5], Some("1 comprimé".to_string())); // per_unit
        assert_eq!(result.values[6], Some("SA".to_string())); // nature
        assert_eq!(result.values[7], Some("0".to_string())); // seq
        assert_eq!(result.values[8], Some("Paracétamol".to_string())); // substance_name_clean
        assert_eq!(result.values[9], Some("500".to_string())); // dosage_mg
    }

    #[test]
    fn test_normalize_compo_substance_name_clean() {
        // Test substance_name_clean normalization with double-spaces
        let row = make_compo_row([
            "60004971",
            "Comprimé",
            "1124",
            "Paracétamol  500 mg",   // f[3]: double-space in name
            "500 mg",
            "1 comprimé",
            "SA",
            "0",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_COMPO_bdpm, &row);
        assert_eq!(result.values[8], Some("Paracétamol 500 mg".to_string())); // double-spaces normalized
    }

    #[test]
    fn test_normalize_compo_dosage_mg_variations() {
        // Test dosage_mg: "1000 mg" → Some("1000")
        let row_mg = make_compo_row([
            "60004971", "Comprimé", "1124", "Paracétamol", "1000 mg", "1 comprimé", "SA", "0",
        ]);
        let result_mg = normalize_row(crate::download::manifest::BDPMFile::CIS_COMPO_bdpm, &row_mg);
        assert_eq!(result_mg.values[9], Some("1000".to_string()));

        // Test dosage_mg: "1,00 g" → Some("1000")
        let row_g = make_compo_row([
            "60004971", "Comprimé", "1124", "Paracétamol", "1,00 g", "1 comprimé", "SA", "0",
        ]);
        let result_g = normalize_row(crate::download::manifest::BDPMFile::CIS_COMPO_bdpm, &row_g);
        assert_eq!(result_g.values[9], Some("1000".to_string()));

        // Test dosage_mg: "250 microg" → Some("0.25")
        let row_microg = make_compo_row([
            "60004971", "Comprimé", "1124", "Principe actif", "250 microg", "1 comprimé", "SA", "0",
        ]);
        let result_microg = normalize_row(crate::download::manifest::BDPMFile::CIS_COMPO_bdpm, &row_microg);
        assert_eq!(result_microg.values[9], Some("0.25".to_string()));

        // Test dosage_mg: "" → None
        let row_empty = make_compo_row([
            "60004971", "Comprimé", "1124", "Paracétamol", "", "1 comprimé", "SA", "0",
        ]);
        let result_empty = normalize_row(crate::download::manifest::BDPMFile::CIS_COMPO_bdpm, &row_empty);
        assert_eq!(result_empty.values[9], None);
    }

    // --- normalize_smr tests (via normalize_row) ---

    /// Helper to create a ValidatedRow for CIS_HAS_SMR_bdpm with 6 fields
    fn make_smr_row(fields: [&str; 6]) -> crate::parse::ValidatedRow {
        crate::parse::ValidatedRow {
            fields: fields.map(String::from).to_vec(),
            line_number: 1,
        }
    }

    #[test]
    fn test_normalize_smr_basic() {
        // 6 fields: cis, ct_id, decision_type, decision_date, level, avis
        let row = make_smr_row([
            "60004971",
            "ct12345",
            "Avis",
            "20250101",
            "I",
            "Service médical rendu important",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_HAS_SMR_bdpm, &row);

        assert_eq!(result.table, "smr");
        assert_eq!(result.values.len(), 6);
        assert_eq!(result.values[0], Some("60004971".to_string())); // cis
        assert_eq!(result.values[1], Some("ct12345".to_string())); // ct_id
        assert_eq!(result.values[2], Some("Avis".to_string())); // decision_type
        assert_eq!(result.values[3], Some("2025-01-01".to_string())); // decision_date
        assert_eq!(result.values[4], Some("I".to_string())); // level
        assert_eq!(result.values[5], Some("Service médical rendu important".to_string())); // avis
    }

    #[test]
    fn test_normalize_smr_date_parsing() {
        // Test date parsing: f[3]="20250101" → Some("2025-01-01")
        let row = make_smr_row([
            "60004971", "ct12345", "Avis", "20250101", "I", "Service médical rendu",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_HAS_SMR_bdpm, &row);
        assert_eq!(result.values[3], Some("2025-01-01".to_string()));
    }

    #[test]
    fn test_normalize_smr_html_stripping() {
        // Test HTML stripping in avis: f[5]="text<br>more" → "text\nmore"
        let row = make_smr_row([
            "60004971",
            "ct12345",
            "Avis",
            "20250101",
            "I",
            "texte avec <br> saut de ligne <b>et gras</b>",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_HAS_SMR_bdpm, &row);
        assert_eq!(result.values[5], Some("texte avec \n saut de ligne et gras".to_string()));
    }

    #[test]
    fn test_normalize_smr_out_of_range_date() {
        // Test out-of-range date: f[3]="29241129" → None for decision_date
        let row = make_smr_row([
            "60004971", "ct12345", "Avis", "29241129", "I", "Service médical rendu",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_HAS_SMR_bdpm, &row);
        assert_eq!(result.values[3], None); // out-of-range date → None
    }

    // --- normalize_asmr tests (via normalize_row) ---

    /// Helper to create a ValidatedRow for CIS_HAS_ASMR_bdpm with 6 fields
    fn make_asmr_row(fields: [&str; 6]) -> crate::parse::ValidatedRow {
        crate::parse::ValidatedRow {
            fields: fields.map(String::from).to_vec(),
            line_number: 1,
        }
    }

    #[test]
    fn test_normalize_asmr_basic() {
        // 6 fields: cis, ct_id, decision_type, decision_date, level, avis (same as SMR)
        let row = make_asmr_row([
            "60004971",
            "ct67890",
            "Avis",
            "20250315",
            "II",
            "Amélioration du service rendu modérée",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_HAS_ASMR_bdpm, &row);

        assert_eq!(result.table, "asmr");
        assert_eq!(result.values.len(), 6);
        assert_eq!(result.values[0], Some("60004971".to_string())); // cis
        assert_eq!(result.values[1], Some("ct67890".to_string())); // ct_id
        assert_eq!(result.values[2], Some("Avis".to_string())); // decision_type
        assert_eq!(result.values[3], Some("2025-03-15".to_string())); // decision_date
        assert_eq!(result.values[4], Some("II".to_string())); // level
        assert_eq!(result.values[5], Some("Amélioration du service rendu modérée".to_string())); // avis
    }

    #[test]
    fn test_normalize_asmr_level_field() {
        // Test level field preserved: f[4]="I" → Some("I")
        let row = make_asmr_row([
            "60004971", "ct67890", "Avis", "20250315", "I", "Amélioration majeure",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_HAS_ASMR_bdpm, &row);
        assert_eq!(result.values[4], Some("I".to_string()));

        // Test level II
        let row_ii = make_asmr_row([
            "60004971", "ct67890", "Avis", "20250315", "II", "Amélioration modérée",
        ]);
        let result_ii = normalize_row(crate::download::manifest::BDPMFile::CIS_HAS_ASMR_bdpm, &row_ii);
        assert_eq!(result_ii.values[4], Some("II".to_string()));
    }

    #[test]
    fn test_normalize_asmr_html_stripping() {
        // Test HTML stripping in avis
        let row = make_asmr_row([
            "60004971",
            "ct67890",
            "Avis",
            "20250315",
            "III",
            "texte avec <br> et <b>gras</b>",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_HAS_ASMR_bdpm, &row);
        assert_eq!(result.values[5], Some("texte avec \n et gras".to_string()));
    }

    // --- normalize_gener tests (via normalize_row) ---

    /// Helper to create a ValidatedRow for CIS_GENER_bdpm with 5 fields
    fn make_gener_row(fields: [&str; 5]) -> crate::parse::ValidatedRow {
        crate::parse::ValidatedRow {
            fields: fields.map(String::from).to_vec(),
            line_number: 1,
        }
    }

    #[test]
    fn test_normalize_gener_basic() {
        // 5 fields: group_id, group_name, cis, type_raw, sort_order
        let row = make_gener_row([
            "GRP001",
            "Doliprane et génériques",
            "60004971",
            "0",
            "1",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_GENER_bdpm, &row);

        assert_eq!(result.table, "generic_groups");
        assert_eq!(result.values.len(), 5);
        assert_eq!(result.values[0], Some("GRP001".to_string())); // group_id
        assert_eq!(result.values[1], Some("Doliprane et génériques".to_string())); // group_name
        assert_eq!(result.values[2], Some("60004971".to_string())); // cis
        assert_eq!(result.values[3], Some("reference".to_string())); // type (0 → reference)
        assert_eq!(result.values[4], Some("1".to_string())); // sort_order
    }

    #[test]
    fn test_normalize_gener_group_name_whitespace() {
        // Test group_name normalization with double-spaces
        let row = make_gener_row([
            "GRP001",
            "Doliprane  et  génériques",   // f[1]: double-spaces
            "60004971",
            "0",
            "1",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_GENER_bdpm, &row);
        assert_eq!(result.values[1], Some("Doliprane et génériques".to_string())); // double-spaces normalized
    }

    #[test]
    fn test_normalize_gener_type_normalization() {
        // Test type normalization: f[3]="0" → "reference", f[3]="1" → "generic", f[3]="4" → "sustained-release"
        let row_ref = make_gener_row(["GRP001", "Groupe ref", "60004971", "0", "1"]);
        let result_ref = normalize_row(crate::download::manifest::BDPMFile::CIS_GENER_bdpm, &row_ref);
        assert_eq!(result_ref.values[3], Some("reference".to_string()));

        let row_gen = make_gener_row(["GRP001", "Groupe gen", "60004972", "1", "2"]);
        let result_gen = normalize_row(crate::download::manifest::BDPMFile::CIS_GENER_bdpm, &row_gen);
        assert_eq!(result_gen.values[3], Some("generic".to_string()));

        let row_sr = make_gener_row(["GRP001", "Groupe SR", "60004973", "4", "3"]);
        let result_sr = normalize_row(crate::download::manifest::BDPMFile::CIS_GENER_bdpm, &row_sr);
        assert_eq!(result_sr.values[3], Some("sustained-release".to_string()));
    }

    #[test]
    fn test_normalize_gener_sort_order() {
        // Test sort_order: f[4]="1" → Some("1")
        let row = make_gener_row(["GRP001", "Groupe", "60004971", "0", "1"]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_GENER_bdpm, &row);
        assert_eq!(result.values[4], Some("1".to_string()));

        // Test invalid sort_order (non-numeric) → None
        let row_invalid = make_gener_row(["GRP001", "Groupe", "60004971", "0", "abc"]);
        let result_invalid = normalize_row(crate::download::manifest::BDPMFile::CIS_GENER_bdpm, &row_invalid);
        assert_eq!(result_invalid.values[4], None);
    }

    // --- normalize_cpd tests (via normalize_row) ---

    /// Helper to create a ValidatedRow for CIS_CPD_bdpm with 2 fields
    fn make_cpd_row(fields: [&str; 2]) -> crate::parse::ValidatedRow {
        crate::parse::ValidatedRow {
            fields: fields.map(String::from).to_vec(),
            line_number: 1,
        }
    }

    #[test]
    fn test_normalize_cpd_basic() {
        // 2 fields: cis, rule
        let row = make_cpd_row([
            "60004971",
            "Médicament sujet à prescription obligatoire",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_CPD_bdpm, &row);

        assert_eq!(result.table, "prescription_rules");
        assert_eq!(result.values.len(), 2);
        assert_eq!(result.values[0], Some("60004971".to_string())); // cis
        assert_eq!(result.values[1], Some("Médicament sujet à prescription obligatoire".to_string())); // rule
    }

    #[test]
    fn test_normalize_cpd_empty_rule() {
        let row = make_cpd_row(["60004971", ""]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_CPD_bdpm, &row);
        assert_eq!(result.values[1], Some("".to_string())); // empty rule preserved
    }

    // --- normalize_mitm tests (via normalize_row) ---

    /// Helper to create a ValidatedRow for CIS_MITM with 4 fields
    fn make_mitm_row(fields: [&str; 4]) -> crate::parse::ValidatedRow {
        crate::parse::ValidatedRow {
            fields: fields.map(String::from).to_vec(),
            line_number: 1,
        }
    }

    #[test]
    fn test_normalize_mitm_basic() {
        let row = make_mitm_row([
            "60004971",
            "N02BE01",
            "Paracétamol",
            "https://base-donnees-publique.medicaments.gouv.fr/displayDoc.php",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_MITM, &row);

        assert_eq!(result.table, "mitm");
        assert_eq!(result.values.len(), 3);
        assert_eq!(result.values[0], Some("60004971".to_string())); // cis
        assert_eq!(result.values[1], Some("N02BE01".to_string())); // atc_code
        assert_eq!(result.values[2], Some("https://base-donnees-publique.medicaments.gouv.fr/displayDoc.php".to_string())); // detail_url
    }

    #[test]
    fn test_normalize_mitm_empty_drug_name() {
        // drug_name (f[2]) is not stored — only cis, atc_code, detail_url
        let row = make_mitm_row([
            "60004971",
            "N02BE01",
            "",
            "https://example.com",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_MITM, &row);
        assert_eq!(result.values.len(), 3);
        assert_eq!(result.values[2], Some("https://example.com".to_string()));
    }

    #[test]
    fn test_normalize_mitm_empty_url() {
        // Test empty url: f[3]="" → None
        let row = make_mitm_row([
            "60004971",
            "N02BE01",
            "Paracétamol",
            "",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::CIS_MITM, &row);
        assert_eq!(result.values[2], None); // empty url → None
    }

    // --- normalize_liens tests (via normalize_row) ---

    /// Helper to create a ValidatedRow for HAS_LiensPageCT_bdpm with 2 fields
    fn make_liens_row(fields: [&str; 2]) -> crate::parse::ValidatedRow {
        crate::parse::ValidatedRow {
            fields: fields.map(String::from).to_vec(),
            line_number: 1,
        }
    }

    #[test]
    fn test_normalize_liens_basic() {
        // 2 fields: ct_id, url
        let row = make_liens_row([
            "ct12345",
            "https://base-donnees-publique.medicaments.gouv.fr/avis",
        ]);
        let result = normalize_row(crate::download::manifest::BDPMFile::HAS_LiensPageCT_bdpm, &row);

        assert_eq!(result.table, "has_links");
        assert_eq!(result.values.len(), 2);
        assert_eq!(result.values[0], Some("ct12345".to_string())); // ct_id
        assert_eq!(result.values[1], Some("https://base-donnees-publique.medicaments.gouv.fr/avis".to_string())); // url
    }

    #[test]
    fn test_normalize_liens_empty_url() {
        // URL can be empty (preserve as empty string)
        let row = make_liens_row(["ct12345", ""]);
        let result = normalize_row(crate::download::manifest::BDPMFile::HAS_LiensPageCT_bdpm, &row);
        assert_eq!(result.values[1], Some("".to_string())); // empty url preserved
    }
}
