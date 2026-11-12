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

/// A length as people say it: "45 seconds", "12 minutes", "1 hour 5 minutes".
/// Rounded to the second under a minute and a half, to the minute above.
pub fn spoken(seconds: f64) -> String {
    let seconds = if seconds.is_finite() && seconds > 0.0 { seconds } else { 0.0 };
    let unit = |n: u64, one: &str| if n == 1 { format!("1 {one}") } else { format!("{n} {one}s") };
    if seconds < 90.0 {
        return unit(seconds.round() as u64, "second");
    }
    let minutes = (seconds / 60.0).round() as u64;
    match (minutes / 60, minutes % 60) {
        (0, m) => unit(m, "minute"),
        (h, 0) => unit(h, "hour"),
        (h, m) => format!("{} {}", unit(h, "hour"), unit(m, "minute")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_are_said_the_way_people_say_them() {
        assert_eq!(spoken(1.0), "1 second");
        assert_eq!(spoken(45.4), "45 seconds");
        assert_eq!(spoken(130.0), "2 minutes");
        assert_eq!(spoken(3600.0), "1 hour");
        assert_eq!(spoken(3900.0), "1 hour 5 minutes");
        assert_eq!(spoken(7200.0), "2 hours");
        assert_eq!(spoken(f64::NAN), "0 seconds");
    }

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
