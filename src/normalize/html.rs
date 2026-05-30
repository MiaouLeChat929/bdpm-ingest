use regex_lite::Regex;

/// Decode HTML entities using WHATWG-compliant decoder.
///
/// Handles named entities (`&nbsp;`, `&eacute;`, `&ecirc;`, etc.)
/// and numeric entities (decimal `&#233;` and hex `&#xE9;`).
/// Follows the WHATWG HTML spec via the `htmlize` crate.
/// Non-breaking spaces (U+00A0 from `&nbsp;`) are normalized to regular spaces.
pub fn decode_html_entities(raw: &str) -> String {
    htmlize::unescape(raw)
        .replace('\u{A0}', " ")
}

/// Normalize multiple consecutive newlines to a maximum of 2.
///
/// Avoids runaway whitespace from multiple `<br>` tags.
fn normalize_newlines(raw: &str) -> String {
    static MULTIPLE_NEWLINE_RE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\n{3,}").unwrap());

    MULTIPLE_NEWLINE_RE.replace_all(raw, "\n\n").to_string()
}

/// Strip HTML tags from avis field text.
/// Replaces `<br>` variants with newline, strips `<p>`, `<b>`, and other tags.
/// Decodes HTML entities, normalizes consecutive newlines, and trims whitespace.
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

    let text = TAG_RE.replace_all(&result, "");

    // Decode HTML entities that survived tag stripping
    let text = decode_html_entities(&text);

    // Normalize multiple consecutive newlines to max 2
    normalize_newlines(&text).trim().to_string()
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

    #[test]
    fn test_strip_html_entities() {
        assert_eq!(decode_html_entities("text&nbsp;more"), "text more");
        assert_eq!(decode_html_entities("stop&go"), "stop&go");
    }

    #[test]
    fn test_strip_numeric_entities() {
        assert_eq!(decode_html_entities("&#39;accent&#39;"), "'accent'");
    }

    #[test]
    fn test_strip_mixed_html_and_entities() {
        assert_eq!(decode_html_entities("text&nbsp;more"), "text more");
    }

    #[test]
    fn test_strip_avis_html_full_pipeline() {
        // Tag stripping + entity decoding + newline normalization
        assert_eq!(strip_avis_html("<p>caf&#233;</p>"), "café");
        assert_eq!(strip_avis_html("<b>Arr&#234;t</b><br>suite"), "Arrêt\nsuite");
        assert_eq!(strip_avis_html("<br>&#233;<br>"), "é");
        assert_eq!(strip_avis_html("<p>text&nbsp;more</p>"), "text more");
    }

    #[test]
    fn test_strip_consecutive_br() {
        let input = "a<br><br><br>b";
        let expected = "a\n\nb";
        assert_eq!(strip_avis_html(input), expected);
    }

    #[test]
    fn test_decode_french_named_entities() {
        // Basic accented named entities
        assert_eq!(decode_html_entities("&ecirc;"), "ê");
        assert_eq!(decode_html_entities("&eacute;"), "é");
        assert_eq!(decode_html_entities("&ccedil;"), "ç");
        // Mixed case
        assert_eq!(
            decode_html_entities("fen&ecirc;tre fran&ccedil;aise"),
            "fenêtre française"
        );
        // Uppercase
        assert_eq!(decode_html_entities("&Eacute;cole"), "École");
    }

    #[test]
    fn test_decode_french_accents() {
        // Decimal codes: é=233, è=232, ê=234, ë=235, ô=244, û=251, ù=249, ç=231
        // These verify the numeric entity decoder handles French accented letters
        assert_eq!(decode_html_entities("caf&#233; noir"), "café noir");
        assert_eq!(decode_html_entities("d&#233;j&#224;"), "déjà");
        assert_eq!(decode_html_entities("fran&#231;ais"), "français");
        // Verify individual accented chars decode correctly
        assert_eq!(decode_html_entities("&#233;"), "é");
        assert_eq!(decode_html_entities("&#232;"), "è");
        assert_eq!(decode_html_entities("&#224;"), "à");
        assert_eq!(decode_html_entities("&#234;"), "ê");
        assert_eq!(decode_html_entities("&#235;"), "ë");
        assert_eq!(decode_html_entities("&#244;"), "ô");
        assert_eq!(decode_html_entities("&#251;"), "û");
        assert_eq!(decode_html_entities("&#249;"), "ù");
        assert_eq!(decode_html_entities("&#231;"), "ç");
    }

    #[test]
    fn test_normalize_newlines() {
        assert_eq!(normalize_newlines("a\n\n\n\nb"), "a\n\nb");
        assert_eq!(normalize_newlines("x\n\n\n\n\n\ny"), "x\n\ny");
        assert_eq!(normalize_newlines("normal\ntext"), "normal\ntext");
    }

    #[test]
    fn test_decode_hex_entities() {
        assert_eq!(decode_html_entities("caf&#xE9; noir"), "café noir");
        assert_eq!(decode_html_entities("&#x00E9;"), "é");
        assert_eq!(decode_html_entities("&#xC9;cole"), "École");
    }
}
