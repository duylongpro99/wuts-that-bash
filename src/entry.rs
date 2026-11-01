/// One executed command, from either `.zsh_history` or wib's own log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Unix timestamp (seconds) when the command ran.
    pub ts: i64,
    pub cmd: String,
    /// Working directory; only known for entries recorded by wib itself.
    pub cwd: Option<String>,
    /// Exit status; only known for entries recorded by wib itself.
    pub exit: Option<i32>,
}

/// Parse a period such as `12h`, `7d`, `2w`, `30d` into seconds.
pub fn parse_period(s: &str) -> Result<i64, String> {
    let s = s.trim();
    let split = s.char_indices().last().map_or(0, |(i, _)| i);
    let (num, unit) = s.split_at(split);
    let n: i64 = num
        .parse()
        .map_err(|_| format!("invalid period '{s}': expected e.g. 12h, 7d, 2w, 30d"))?;
    let mult = match unit {
        "h" => 3_600,
        "d" => 86_400,
        "w" => 7 * 86_400,
        _ => return Err(format!("invalid period '{s}': unit must be h, d or w")),
    };
    if n <= 0 {
        return Err(format!("invalid period '{s}': must be positive"));
    }
    n.checked_mul(mult)
        .ok_or_else(|| format!("invalid period '{s}': too large"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_periods() {
        assert_eq!(parse_period("12h"), Ok(12 * 3_600));
        assert_eq!(parse_period("7d"), Ok(7 * 86_400));
        assert_eq!(parse_period("2w"), Ok(14 * 86_400));
        assert_eq!(parse_period("30d"), Ok(30 * 86_400));
    }

    #[test]
    fn rejects_bad_periods() {
        for bad in ["", "d", "7", "7m", "-1d", "0d", "abc", "7đ", "đ", "9999999999999999d"] {
            assert!(parse_period(bad).is_err(), "{bad} should be rejected");
        }
    }
}
