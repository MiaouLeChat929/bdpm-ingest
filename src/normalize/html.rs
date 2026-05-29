use regex_lite::Regex;

/// Decode HTML entities that survive tag stripping.
///
/// Handles named entities (`&nbsp;`, `&`, `<`, `>`, `"`, `&#39;`)
/// and numeric entities including high-bit values used in BDPM files.
pub fn decode_html_entities(raw: &str) -> String {
    let mut result = raw.to_string();

    // Named entities — most common ones from BDPM data
    result = result.replace("&nbsp;", " ");
    result = result.replace("&amp;", "&");
    result = result.replace("&lt;", "<");
    result = result.replace("&gt;", ">");
    result = result.replace("\u{201C}\u{201D}", "\"");   // curly quotes -> "
    result = result.replace("&#39;", "'");               // numeric apostrophe
    result = result.replace("\u{2018}\u{2019}", "'");   // curly apostrophes -> '
    result = result.replace("&ndash;", "\u{2013}");     // en dash
    result = result.replace("&mdash;", "\u{2014}");     // em dash
    result = result.replace("&hellip;", "\u{2026}");    // ellipsis
    result = result.replace("&copy;", "(c)");
    result = result.replace("&reg;", "(R)");
    result = result.replace("&trade;", "(TM)");

    // French accented named entities (present in BDPM HTML data)
    result = result.replace("&eacute;", "é");
    result = result.replace("&Eacute;", "É");
    result = result.replace("&egrave;", "è");
    result = result.replace("&Egrave;", "È");
    result = result.replace("&ecirc;", "ê");
    result = result.replace("&Ecirc;", "Ê");
    result = result.replace("&euml;", "ë");
    result = result.replace("&Euml;", "Ë");
    result = result.replace("&agrave;", "à");
    result = result.replace("&Agrave;", "À");
    result = result.replace("&acirc;", "â");
    result = result.replace("&Acirc;", "Â");
    result = result.replace("&ccedil;", "ç");
    result = result.replace("&Ccedil;", "Ç");
    result = result.replace("&ocirc;", "ô");
    result = result.replace("&Ocirc;", "Ô");
    result = result.replace("&ouml;", "ö");
    result = result.replace("&Ouml;", "Ö");
    result = result.replace("&ucirc;", "û");
    result = result.replace("&Ucirc;", "Û");
    result = result.replace("&ugrave;", "ù");
    result = result.replace("&Ugrave;", "Ù");
    result = result.replace("&uuml;", "ü");
    result = result.replace("&Uuml;", "Ü");
    result = result.replace("&icirc;", "î");
    result = result.replace("&Icirc;", "Î");
    result = result.replace("&iuml;", "ï");
    result = result.replace("&Iuml;", "Ï");

    // Numeric decimal entities (&#nnn;) — common accented chars for French content
    // French accented letters: é, è, ê, ë, ô, ö, û, ü, ù, ç
    decode_numeric_entities_in_place(&mut result);

    result
}

/// Decode numeric decimal entities in place.
fn decode_numeric_entities_in_place(s: &mut String) {
    // Pattern: &# followed by digits then semicolon
    static NUMERIC_RE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"&#(\d+);").unwrap());

    // Collect matches in reverse order to preserve indices
    let matches: Vec<_> = NUMERIC_RE.find_iter(s).collect();
    if matches.is_empty() {
        return;
    }

    // Process in reverse to reconstruct from end
    let mut chars: Vec<char> = s.chars().collect();
    for m in matches.iter().rev() {
        if let Ok(code) = m.as_str()[2..m.as_str().len() - 1].parse::<u32>() {
            if let Some(c) = char::from_u32(code) {
                let start = m.start();
                let end = m.end();
                for _ in 0..(end - start) {
                    chars.remove(start);
                }
                chars.insert(start, c);
            }
        }
    }

    *s = chars.into_iter().collect();
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
}
