use regex_lite::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

static CPD_PATTERNS: LazyLock<HashMap<&'static str, Regex>> = LazyLock::new(|| {
    HashMap::from([
        ("liste_i", Regex::new(r"(?i)\bliste\s*i\b").unwrap()),
        ("liste_ii", Regex::new(r"(?i)\bliste\s*ii\b").unwrap()),
        ("stupefiant", Regex::new(r"(?i)stup[eéÉ]fiant").unwrap()),
        ("hospitalier", Regex::new(r"(?i)hospitali[eéèEÉÈ]re|(?i)hospitalier|h[oôÔ]pital").unwrap()),
        ("dentaire", Regex::new(r"(?i)dentaire").unwrap()),
        ("reserve_hopital", Regex::new(r"(?i)r[eéÉ]serve\s*d'?h[oôÔ]pital|reste\s*[aàÀ]\s*l'?h[oôÔ]pital").unwrap()),
    ])
});

#[derive(Default, Debug, Clone)]
pub struct CpdFlags {
    pub liste_i: bool,
    pub liste_ii: bool,
    pub stupefiant: bool,
    pub hospitalier: bool,
    pub dentaire: bool,
    pub reserve_hopital: bool,
}

impl CpdFlags {
    pub fn from_rule(rule: &str) -> Self {
        let upper = rule.to_uppercase();
        let mut flags = CpdFlags::default();
        for (flag_name, re) in CPD_PATTERNS.iter() {
            match *flag_name {
                "liste_i" => flags.liste_i = re.is_match(&upper),
                "liste_ii" => flags.liste_ii = re.is_match(&upper),
                "stupefiant" => flags.stupefiant = re.is_match(&upper),
                "hospitalier" => flags.hospitalier = re.is_match(&upper),
                "dentaire" => flags.dentaire = re.is_match(&upper),
                "reserve_hopital" => flags.reserve_hopital = re.is_match(&upper),
                _ => {}
            }
        }
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpd_flags_liste_i() {
        let flags = CpdFlags::from_rule("Médicament soumis à prescription : Liste I");
        assert!(flags.liste_i);
        assert!(!flags.liste_ii);
    }

    #[test]
    fn test_cpd_flags_stupefiant() {
        let flags = CpdFlags::from_rule("Médicament stupéfiant — prescription restreinte");
        assert!(flags.stupefiant);
    }

    #[test]
    fn test_cpd_flags_multiple() {
        let flags = CpdFlags::from_rule("Liste II — Usage hospitalier");
        assert!(flags.liste_ii);
        assert!(flags.hospitalier);
    }

    #[test]
    fn test_cpd_flags_none() {
        let flags = CpdFlags::from_rule("Prescription libre");
        assert!(!flags.liste_i);
        assert!(!flags.liste_ii);
        assert!(!flags.stupefiant);
    }
}
