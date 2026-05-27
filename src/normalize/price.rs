/// Parse a European-format price string to integer cents.
/// Handles:
/// - "24,34" → 2434 (single comma as decimal)
/// - "1,466,29" → 146629 (French format: commas as thousands, last comma as decimal)
/// - "1.234,56" → 123456 (periods as thousands, comma as decimal)
/// - "1 234,56" → 123456 (space as thousands separator)
/// - "0" → Some(0)
/// - "" → None
///
/// French BDMP format uses comma as decimal separator. For multi-comma prices:
/// - Last comma is always the decimal separator
/// - Remaining commas are thousands separators
///
/// Periods and spaces are also valid thousands separators when the string contains
/// both a comma (for decimal) and the alternative separator.
/// e.g., "6.659,625" → periods before comma are thousands separators
pub fn parse_price_cents(raw: &str) -> anyhow::Result<Option<i64>> {
    let trimmed = raw.trim();

    // Step 1: Handle whitespace (e.g., "1 234,56")
    let normalized = trimmed.replace(' ', "");

    if normalized.is_empty() {
        return Ok(None);
    }

    // Step 2: Detect format and parse
    let has_period = normalized.contains('.');
    let has_comma = normalized.contains(',');

    match (has_period, has_comma) {
        // Only comma: French format (single or multiple commas)
        (false, true) => parse_french_format(&normalized),
        // Both period and comma: European format with period as thousands
        // e.g., "6.659,625" or "1.234,56"
        (true, true) => parse_european_with_period_thousands(&normalized),
        // Only period: might be float with decimal (unlikely but handle)
        (true, false) => {
            let val: f64 = normalized.parse()
                .map_err(|_| anyhow::anyhow!("Invalid price: {}", trimmed))?;
            Ok(Some((val * 100.0).round() as i64))
        }
        // No separators: integer euros
        (false, false) => {
            let euros: f64 = normalized.parse()
                .map_err(|_| anyhow::anyhow!("Invalid price: {}", trimmed))?;
            Ok(Some((euros * 100.0).round() as i64))
        }
    }
}

/// Parse French format: comma(s) with last comma as decimal separator.
/// "1,466,29" → 146629
/// "1,000,000,00" → 100000000 (3+ commas handled)
fn parse_french_format(s: &str) -> anyhow::Result<Option<i64>> {
    let comma_count = s.matches(',').count();

    match comma_count {
        0 => {
            // Should not happen (handled in main function), but safe fallback
            let euros: f64 = s.parse()
                .map_err(|_| anyhow::anyhow!("Invalid price: {}", s))?;
            Ok(Some((euros * 100.0).round() as i64))
        }
        1 => {
            // "24,34" → 2434 cents
            let val: f64 = s.replace(',', ".").parse()
                .map_err(|_| anyhow::anyhow!("Invalid price: {}", s))?;
            Ok(Some((val * 100.0).round() as i64))
        }
        // 2+ commas: thousands separators with last comma as decimal
        // e.g., "1,466,29" → integer="1466", decimal="29" → 146629
        // e.g., "1,000,000,00" → integer="1000000", decimal="00" → 100000000
        _ => {
            let parts: Vec<&str> = s.split(',').collect();
            let last = parts.last().unwrap_or(&"");

            // If last part has exactly 2 digits, it's the decimal part
            if last.len() == 2 {
                let integer_part: String = parts[..parts.len()-1].join("");
                let full = format!("{}.{}", integer_part, last);
                let val: f64 = full.parse()
                    .map_err(|_| anyhow::anyhow!("Invalid price: {}", s))?;
                Ok(Some((val * 100.0).round() as i64))
            } else {
                // All commas are thousands separators, no decimal part
                let full: String = s.replace(',', "");
                let val: f64 = full.parse()
                    .map_err(|_| anyhow::anyhow!("Invalid price: {}", s))?;
                Ok(Some((val * 100.0).round() as i64))
            }
        }
    }
}

/// Parse European format: period as thousands separator, comma as decimal.
/// "6.659,625" → 6659625 (6,659.625 euros = 6659625 cents)
fn parse_european_with_period_thousands(s: &str) -> anyhow::Result<Option<i64>> {
    // Split on comma (decimal separator)
    let parts: Vec<&str> = s.split(',').collect();

    if parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid European format price: {}", s));
    }

    let integer_part = parts[0];  // e.g., "6.659" or "1.234"
    let decimal_part = parts[1];  // e.g., "625" or "56"

    // Remove periods (thousands separators) from integer part
    let clean_integer: String = integer_part.replace('.', "");

    // Combine: "6659" + "." + "625" = 6659.625
    let full = format!("{}.{}", clean_integer, decimal_part);
    let val: f64 = full.parse()
        .map_err(|_| anyhow::anyhow!("Invalid price: {}", s))?;

    Ok(Some((val * 100.0).round() as i64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_comma_decimal() {
        assert_eq!(parse_price_cents("24,34").unwrap(), Some(2434));
    }

    #[test]
    fn test_price_thousands_separator() {
        assert_eq!(parse_price_cents("1,466,29").unwrap(), Some(146_629));
    }

    #[test]
    fn test_price_null() {
        assert_eq!(parse_price_cents("").unwrap(), None);
    }

    #[test]
    fn test_price_zero() {
        assert_eq!(parse_price_cents("0").unwrap(), Some(0));
    }

    #[test]
    fn test_price_integer() {
        assert_eq!(parse_price_cents("24").unwrap(), Some(2400));
    }

    // New robustness tests

    #[test]
    fn test_price_period_thousands() {
        // "1.234,56" → 1,234.56 euros → 123456 cents
        assert_eq!(parse_price_cents("1.234,56").unwrap(), Some(123_456));
    }

    #[test]
    fn test_price_space_thousands() {
        // "1 234,56" → 1,234.56 euros → 123456 cents
        assert_eq!(parse_price_cents("1 234,56").unwrap(), Some(123_456));
    }

    #[test]
    fn test_price_three_commas() {
        // "1,000,000,00" → 1,000,000.00 euros → 100000000 cents
        assert_eq!(parse_price_cents("1,000,000,00").unwrap(), Some(100_000_000));
    }

    #[test]
    fn test_price_very_large() {
        // "6.659,625" → 6,659.625 euros → 665962.5 cents → rounds to 665963
        // Periods are thousands separators, comma is decimal
        assert_eq!(parse_price_cents("6.659,625").unwrap(), Some(665_963));
    }
}
