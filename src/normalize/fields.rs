/// Strip leading/trailing whitespace from a field.
/// Handles BDPM quirk: lab names start with ' '.
pub fn strip_field(raw: &str) -> String {
    raw.trim().to_string()
}

/// Normalize double-spaces in a string (CIS_GENER names).
pub fn normalize_spaces(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Strip 34009 prefix from EAN to get canonical CIP-7.
/// If already 7 digits, return as-is.
pub fn strip_cip_ean(raw: &str) -> String {
    let t = raw.trim();
    if t.len() == 13 && t.starts_with("34009") {
        t[6..].to_string()
    } else if t.len() == 7 && t.chars().all(|c| c.is_ascii_digit()) {
        t.to_string()
    } else {
        t.to_string()
    }
}

/// Normalize generic type: "0"→"reference", "1"→"generic", "2"→"cross-group", "4"→"sustained-release".
pub fn normalize_generic_type(raw: &str) -> &'static str {
    match raw.trim() {
        "0" => "reference",
        "1" => "generic",
        "2" => "cross-group",
        "4" => "sustained-release",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_lab_space() {
        assert_eq!(strip_field(" SANOFI"), "SANOFI");
        assert_eq!(strip_field("SANOFI "), "SANOFI");
    }

    #[test]
    fn test_generic_type_0() {
        assert_eq!(normalize_generic_type("0"), "reference");
    }

    #[test]
    fn test_generic_type_2() {
        assert_eq!(normalize_generic_type("2"), "cross-group");
    }

    #[test]
    fn test_generic_type_4() {
        assert_eq!(normalize_generic_type("4"), "sustained-release");
    }

    #[test]
    fn test_double_space() {
        assert_eq!(normalize_spaces("PARACETAMOL  1000  mg"), "PARACETAMOL 1000 mg");
    }
}
