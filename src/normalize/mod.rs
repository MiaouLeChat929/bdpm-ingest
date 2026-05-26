pub mod price;
pub mod date;
pub mod fields;
pub mod html;
pub mod dedup;

pub use price::parse_price_cents;
pub use date::{parse_date_ddmmYYYY, parse_date_YYYYMMDD};
pub use fields::{strip_field, normalize_spaces, normalize_generic_type};
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
            Some(strip_field(&f[1])),  // name (strip leading space)
            Some(strip_field(&f[2])),  // form
            Some(strip_field(&f[3])),  // route
            Some(strip_field(&f[4])),  // auth_status
            Some(strip_field(&f[5])),  // procedure_type
            Some(strip_field(&f[6])),  // comm_status
            parse_date_ddmmYYYY(&f[7]).ok(),  // auth_date ISO
            Some(strip_field(&f[9])),  // lab_name (strip leading space)
            Some(if f[10].trim() == "Oui" { "1" } else { "0" }.to_string()),  // is_patent
            if f[11].is_empty() { None } else { Some(strip_field(&f[11])) },  // alert_type
            Some(strip_eu_slash(&f[11])),  // eu_number (field 11)
        ],
    }
}

fn strip_eu_slash(s: &str) -> String {
    s.trim_end_matches('/').to_string()
}

fn normalize_cis_cip(f: &[String]) -> NormalizedRow {
    // 13 fields: cis, cip7, cip13, labels, pres_status, comm_status, comm_date, prix_ht, prix_ville, prix_rate, reimb_rate, reimb_conditions, ?(phantom)
    NormalizedRow {
        table: "presentations",
        values: vec![
            Some(f[0].clone()),  // cis
            Some(strip_cip7(&f[1])),  // cip (canonical 7-digit)
            Some(f[1].clone()),  // cip_raw
            Some(strip_field(&f[2])),  // labels
            Some(strip_field(&f[3])),  // pres_status
            Some(strip_field(&f[4])),  // comm_status
            parse_date_ddmmYYYY(&f[5]).ok(),  // comm_date ISO
            parse_price_cents(&f[6]).ok().flatten().map(|c| c.to_string()),  // prix_ht_cents
            parse_price_cents(&f[7]).ok().flatten().map(|c| c.to_string()),  // prix_ville_cents
            parse_price_cents(&f[8]).ok().flatten().map(|c| c.to_string()),  // prix_rate_cents
            normalize_reimb_rate(&f[9]).map(|r| r.to_string()),  // reimb_rate
            Some(strip_field(&f[10])),  // reimb_conditions
            if f[11].is_empty() { None } else { Some(f[11].clone()) },  // ean13
            Some(strip_field(&f[12])),  // reimbursable
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
            Some(strip_field(&f[1])),
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

/// Apply apostrophe normalization to all string fields in a row
pub fn normalize_apostrophes(row: &mut NormalizedRow) {
    for val in &mut row.values {
        if let Some(s) = val {
            *s = s.replace('\u{2019}', "'")
                   .replace('\u{2018}', "'");
        }
    }
}
