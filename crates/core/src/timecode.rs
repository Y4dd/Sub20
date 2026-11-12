//! Parsing and printing of times such as `83.5`, `1:23.5` and `0:01:23.5`.

/// Parses seconds, `MM:SS(.fff)` or `HH:MM:SS(.fff)`.
pub fn parse(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let parts: Vec<&str> = text.split(':').collect();
    if parts.len() > 3 {
        return None;
    }
    let mut total = 0.0;
    for (i, part) in parts.iter().enumerate() {
        let value: f64 = part.trim().parse().ok()?;
        if !value.is_finite() || value < 0.0 {
            return None;
        }
        // Only the last field may carry a fraction or exceed 59.
        if i + 1 < parts.len() && parts.len() > 1 && i > 0 && value >= 60.0 {
            return None;
        }
        total = total * 60.0 + value;
    }
    Some(total)
}

/// Prints seconds as `M:SS.d` or `H:MM:SS.d`.
pub fn format(seconds: f64) -> String {
    let seconds = if seconds.is_finite() && seconds > 0.0 { seconds } else { 0.0 };
    let tenths = (seconds * 10.0).round() as u64;
    let (h, m, s, d) = (tenths / 36_000, (tenths / 600) % 60, (tenths / 10) % 60, tenths % 10);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}.{d}")
    } else {
        format!("{m}:{s:02}.{d}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_forms() {
        assert_eq!(parse("90"), Some(90.0));
        assert_eq!(parse("1:30"), Some(90.0));
        assert_eq!(parse("0:01:30.5"), Some(90.5));
        assert_eq!(parse(" 2:03:04 "), Some(7384.0));
    }

    #[test]
    fn rejects_nonsense() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("abc"), None);
        assert_eq!(parse("1:2:3:4"), None);
        assert_eq!(parse("-4"), None);
        assert_eq!(parse("1:75:00"), None);
    }

    #[test]
    fn formats() {
        assert_eq!(format(0.0), "0:00.0");
        assert_eq!(format(83.46), "1:23.5");
        assert_eq!(format(3725.0), "1:02:05.0");
    }
}
