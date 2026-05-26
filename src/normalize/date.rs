/// Parse DD/MM/YYYY → "YYYY-MM-DD" (ISO-8601).
/// Rejects dates outside 1900–2100 range.
pub fn parse_date_ddmmYYYY(raw: &str) -> anyhow::Result<String> {
    let parts: Vec<&str> = raw.trim().split('/').collect();
    if parts.len() != 3 {
        anyhow::bail!("Invalid DD/MM/YYYY date: {}", raw);
    }

    let day: u32 = parts[0].parse()
        .map_err(|_| anyhow::anyhow!("Invalid day: {}", parts[0]))?;
    let month: u8 = parts[1].parse()
        .map_err(|_| anyhow::anyhow!("Invalid month: {}", parts[1]))?;
    let year: i32 = parts[2].parse()
        .map_err(|_| anyhow::anyhow!("Invalid year: {}", parts[2]))?;

    if !(1900..=2100).contains(&year) {
        anyhow::bail!("Date {} out of plausible range (1900–2100)", raw);
    }
    if !(1..=12).contains(&month) {
        anyhow::bail!("Invalid month: {}", month);
    }
    if !(1..=31).contains(&day) {
        anyhow::bail!("Invalid day: {}", day);
    }

    // Build ISO date manually: YYYY-MM-DD
    Ok(format!("{:04}-{:02}-{:02}", year, month, day))
}

/// Parse YYYYMMDD (integer or string) → "YYYY-MM-DD".
/// Rejects dates outside 1900–2100 range.
pub fn parse_date_YYYYMMDD(raw: &str) -> anyhow::Result<String> {
    let s = raw.trim();
    if s.len() != 8 || !s.chars().all(|c| c.is_ascii_digit()) {
        anyhow::bail!("Invalid YYYYMMDD date: {}", raw);
    }

    let year: i32 = s[0..4].parse()
        .map_err(|_| anyhow::anyhow!("Invalid year in: {}", raw))?;
    let month: u8 = s[4..6].parse()
        .map_err(|_| anyhow::anyhow!("Invalid month in: {}", raw))?;
    let day: u8 = s[6..8].parse()
        .map_err(|_| anyhow::anyhow!("Invalid day in: {}", raw))?;

    if !(1900..=2100).contains(&year) {
        anyhow::bail!("Date {} out of plausible range (1900–2100)", raw);
    }
    if !(1..=12).contains(&month) {
        anyhow::bail!("Invalid month: {}", month);
    }
    if !(1..=31).contains(&day) {
        anyhow::bail!("Invalid day: {}", day);
    }

    Ok(format!("{:04}-{:02}-{:02}", year, month, day))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ddmmyyyy() {
        assert_eq!(parse_date_ddmmYYYY("28/04/2026").unwrap(), "2026-04-28");
    }

    #[test]
    fn test_ddmmyyyy_far_future() {
        assert!(parse_date_ddmmYYYY("29/11/2924").is_err());
    }

    #[test]
    fn test_yyyymmdd() {
        assert_eq!(parse_date_YYYYMMDD("20260422").unwrap(), "2026-04-22");
    }

    #[test]
    fn test_yyyymmdd_far_future() {
        assert!(parse_date_YYYYMMDD("29241129").is_err());
    }
}
