/// Parse a European-format price string to integer cents.
/// Handles:
/// - "24,34" → 2434
/// - "1,466,29" → 146629 (thousands separator: remove all commas, treat as decimal)
/// - "0" → Some(0)
/// - "" → None
pub fn parse_price_cents(raw: &str) -> anyhow::Result<Option<i64>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let comma_count = trimmed.chars().filter(|&c| c == ',').count();
    match comma_count {
        0 => {
            // "24" → 2400 cents (integer euros)
            let euros: f64 = trimmed.parse()
                .map_err(|_| anyhow::anyhow!("Invalid price: {}", trimmed))?;
            Ok(Some((euros * 100.0).round() as i64))
        }
        1 => {
            // "24,34" → 2434 cents
            let normalized = trimmed.replace(',', ".");
            let val: f64 = normalized.parse()
                .map_err(|_| anyhow::anyhow!("Invalid price: {}", trimmed))?;
            Ok(Some((val * 100.0).round() as i64))
        }
        2.. => {
            // "1,466,29" → thousands separator: remove ALL commas → "146629" → parse → 146629
            // Logic: last comma is decimal separator, rest are thousands separators
            let parts: Vec<&str> = trimmed.split(',').collect();
            let last = parts.last().unwrap_or(&"");
            if last.len() == 2 {
                // Decimal part is 2 digits → treat everything before last comma as integer, append decimal
                let integer_part: String = parts[..parts.len()-1].join("");
                let full = format!("{}.{}", integer_part, last);
                let val: f64 = full.parse()
                    .map_err(|_| anyhow::anyhow!("Invalid price: {}", trimmed))?;
                Ok(Some((val * 100.0).round() as i64))
            } else {
                // All commas are thousands separators, no decimal part
                let full: String = trimmed.replace(',', "");
                let val: f64 = full.parse()
                    .map_err(|_| anyhow::anyhow!("Invalid price: {}", trimmed))?;
                Ok(Some((val * 100.0).round() as i64))
            }
        }
    }
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
}
