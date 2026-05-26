use regex_lite::Regex;

/// Strip HTML tags from avis field text.
/// Replaces <br> variants with newline, strips <p>, <b>, and other tags.
/// Preserves text content.
pub fn strip_avis_html(raw: &str) -> String {
    let result = raw
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<BR>", "\n")
        .replace("<BR/>", "\n")
        .replace("<BR />", "\n")
        .replace("</p>", "\n")
        .replace("</P>", "\n");

    // Remove remaining HTML tags using regex-lite
    static TAG_RE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"<[^>]*>").unwrap());

    TAG_RE.replace_all(&result, "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_br_tags() {
        let input = "Le service médical rendu.<br><br>Indications:";
        let expected = "Le service médical rendu.\n\nIndications:";
        assert_eq!(strip_avis_html(input), expected);
    }

    #[test]
    fn test_strip_p_tags() {
        assert_eq!(strip_avis_html("<p>Texte</p>"), "Texte");
    }

    #[test]
    fn test_strip_b_tags() {
        assert_eq!(strip_avis_html("<b>Important</b>: avis."), "Important: avis.");
    }

    #[test]
    fn test_strip_mixed() {
        let input = "<p><b>Texte</b><br>suite</p>";
        assert_eq!(strip_avis_html(input), "Texte\nsuite");
    }
}
