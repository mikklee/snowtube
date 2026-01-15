//! Time parsing and formatting utilities

use std::time::Duration;

/// Check if text contains Asian characters (CJK, Korean, Thai, etc.)
pub fn contains_asian_characters(text: &str) -> bool {
    text.chars().any(|c| {
        ('\u{3040}'..='\u{309F}').contains(&c) ||  // Hiragana
        ('\u{30A0}'..='\u{30FF}').contains(&c) ||  // Katakana
        ('\u{4E00}'..='\u{9FFF}').contains(&c) ||  // CJK Unified Ideographs
        ('\u{AC00}'..='\u{D7AF}').contains(&c) ||  // Hangul Syllables
        ('\u{1100}'..='\u{11FF}').contains(&c) ||  // Hangul Jamo
        ('\u{0E00}'..='\u{0E7F}').contains(&c) ||  // Thai
        ('\u{3400}'..='\u{4DBF}').contains(&c) ||  // CJK Extension A
        ('\u{F900}'..='\u{FAFF}').contains(&c) // CJK Compatibility
    })
}

/// Time unit multipliers (in seconds)
const MINUTES: u64 = 60;
const HOURS: u64 = 60 * 60;
const DAYS: u64 = 60 * 60 * 24;
const WEEKS: u64 = 60 * 60 * 24 * 7;
const MONTHS: u64 = 60 * 60 * 24 * 30;
const YEARS: u64 = 60 * 60 * 24 * 365;

/// Format duration in seconds to "HH:MM:SS" or "MM:SS" string
pub fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{}:{:02}", minutes, secs)
    }
}

/// Parse a duration string like "10:30" or "1:23:45" to Duration.
/// Returns None if parsing fails.
pub fn parse_duration_string(s: &str) -> Option<Duration> {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        2 => {
            // MM:SS
            let minutes: u64 = parts[0].parse().ok()?;
            let seconds: u64 = parts[1].parse().ok()?;
            Some(Duration::from_secs(minutes * 60 + seconds))
        }
        3 => {
            // HH:MM:SS
            let hours: u64 = parts[0].parse().ok()?;
            let minutes: u64 = parts[1].parse().ok()?;
            let seconds: u64 = parts[2].parse().ok()?;
            Some(Duration::from_secs(hours * 3600 + minutes * 60 + seconds))
        }
        _ => None,
    }
}

/// Format seconds into English relative time string (e.g., "2 hours ago")
pub fn format_relative_time(seconds: u64) -> String {
    let (value, unit) = if seconds < MINUTES {
        (seconds, if seconds == 1 { "second" } else { "seconds" })
    } else if seconds < HOURS {
        let mins = seconds / MINUTES;
        (mins, if mins == 1 { "minute" } else { "minutes" })
    } else if seconds < DAYS {
        let hours = seconds / HOURS;
        (hours, if hours == 1 { "hour" } else { "hours" })
    } else if seconds < WEEKS {
        let days = seconds / DAYS;
        (days, if days == 1 { "day" } else { "days" })
    } else if seconds < MONTHS {
        let weeks = seconds / WEEKS;
        (weeks, if weeks == 1 { "week" } else { "weeks" })
    } else if seconds < YEARS {
        let months = seconds / MONTHS;
        (months, if months == 1 { "month" } else { "months" })
    } else {
        let years = seconds / YEARS;
        (years, if years == 1 { "year" } else { "years" })
    };

    format!("{} {} ago", value, unit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0), "0:00");
        assert_eq!(format_duration(59), "0:59");
        assert_eq!(format_duration(60), "1:00");
        assert_eq!(format_duration(3599), "59:59");
        assert_eq!(format_duration(3600), "1:00:00");
        assert_eq!(format_duration(3661), "1:01:01");
    }

    #[test]
    fn test_parse_duration_string() {
        assert_eq!(
            parse_duration_string("10:30"),
            Some(Duration::from_secs(630))
        );
        assert_eq!(
            parse_duration_string("1:23:45"),
            Some(Duration::from_secs(5025))
        );
        assert_eq!(parse_duration_string("invalid"), None);
    }

    #[test]
    fn test_format_relative_time() {
        assert_eq!(format_relative_time(1), "1 second ago");
        assert_eq!(format_relative_time(30), "30 seconds ago");
        assert_eq!(format_relative_time(60), "1 minute ago");
        assert_eq!(format_relative_time(3600), "1 hour ago");
        assert_eq!(format_relative_time(86400), "1 day ago");
        assert_eq!(format_relative_time(604800), "1 week ago");
        assert_eq!(format_relative_time(2592000), "1 month ago");
        assert_eq!(format_relative_time(31536000), "1 year ago");
    }
}
