//! VTT subtitle parsing and timing.

use std::time::Duration;

/// A single subtitle cue with timing and text.
#[derive(Debug, Clone)]
pub struct SubtitleCue {
    pub start: Duration,
    pub end: Duration,
    pub text: String,
}

/// Parsed subtitle track.
#[derive(Debug, Clone, Default)]
pub struct SubtitleTrack {
    pub cues: Vec<SubtitleCue>,
}

impl SubtitleTrack {
    /// Parse a VTT file content into a subtitle track.
    pub fn parse_vtt(content: &str) -> Self {
        let mut cues = Vec::new();
        let mut lines = content.lines().peekable();

        // Skip WEBVTT header
        for line in lines.by_ref() {
            if line.starts_with("WEBVTT") {
                break;
            }
        }

        // Skip any header metadata until first blank line
        while let Some(line) = lines.peek() {
            if line.is_empty() {
                lines.next();
                break;
            }
            lines.next();
        }

        // Parse cues
        while lines.peek().is_some() {
            // Skip blank lines and cue identifiers
            while let Some(line) = lines.peek() {
                if line.contains("-->") {
                    break;
                }
                lines.next();
            }

            // Parse timing line
            let Some(timing_line) = lines.next() else {
                break;
            };

            let Some((start, end)) = parse_timing_line(timing_line) else {
                continue;
            };

            // Collect text lines until blank line
            let mut text_lines = Vec::new();
            while let Some(line) = lines.peek() {
                if line.is_empty() {
                    lines.next();
                    break;
                }
                text_lines.push(lines.next().unwrap());
            }

            if !text_lines.is_empty() {
                let text = text_lines.join("\n");
                // Strip HTML tags
                let text = strip_html_tags(&text);
                cues.push(SubtitleCue { start, end, text });
            }
        }

        SubtitleTrack { cues }
    }

    /// Get the subtitle text for a given position.
    pub fn text_at(&self, position: Duration) -> Option<&str> {
        self.cues
            .iter()
            .find(|cue| position >= cue.start && position < cue.end)
            .map(|cue| cue.text.as_str())
    }
}

/// Parse a VTT timing line like "00:00:01.000 --> 00:00:04.000"
fn parse_timing_line(line: &str) -> Option<(Duration, Duration)> {
    let parts: Vec<&str> = line.split("-->").collect();
    if parts.len() != 2 {
        return None;
    }

    let start = parse_timestamp(parts[0].trim())?;
    // End timestamp might have positioning info after it, take only the time part
    let end_part = parts[1].split_whitespace().next()?;
    let end = parse_timestamp(end_part)?;

    Some((start, end))
}

/// Parse a VTT timestamp like "00:00:01.000" or "00:01.000"
fn parse_timestamp(s: &str) -> Option<Duration> {
    let parts: Vec<&str> = s.split(':').collect();

    match parts.len() {
        // MM:SS.mmm
        2 => {
            let minutes: u64 = parts[0].parse().ok()?;
            let (secs, millis) = parse_seconds_millis(parts[1])?;
            Some(Duration::from_millis(
                minutes * 60 * 1000 + secs * 1000 + millis,
            ))
        }
        // HH:MM:SS.mmm
        3 => {
            let hours: u64 = parts[0].parse().ok()?;
            let minutes: u64 = parts[1].parse().ok()?;
            let (secs, millis) = parse_seconds_millis(parts[2])?;
            Some(Duration::from_millis(
                hours * 3600 * 1000 + minutes * 60 * 1000 + secs * 1000 + millis,
            ))
        }
        _ => None,
    }
}

/// Parse "SS.mmm" into (seconds, milliseconds)
fn parse_seconds_millis(s: &str) -> Option<(u64, u64)> {
    let parts: Vec<&str> = s.split('.').collect();
    let secs: u64 = parts[0].parse().ok()?;
    let millis: u64 = if parts.len() > 1 {
        // Pad or truncate to 3 digits
        let ms_str = parts[1];
        let padded = format!("{:0<3}", ms_str);
        padded[..3].parse().ok()?
    } else {
        0
    };
    Some((secs, millis))
}

/// Strip HTML tags from text.
fn strip_html_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;

    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vtt() {
        let vtt = r#"WEBVTT

1
00:00:01.000 --> 00:00:04.000
Hello world

2
00:00:05.000 --> 00:00:08.000
This is a test
"#;
        let track = SubtitleTrack::parse_vtt(vtt);
        assert_eq!(track.cues.len(), 2);
        assert_eq!(track.cues[0].text, "Hello world");
        assert_eq!(track.cues[0].start, Duration::from_secs(1));
        assert_eq!(track.cues[0].end, Duration::from_secs(4));
    }

    #[test]
    fn test_text_at() {
        let track = SubtitleTrack {
            cues: vec![
                SubtitleCue {
                    start: Duration::from_secs(1),
                    end: Duration::from_secs(4),
                    text: "First".to_string(),
                },
                SubtitleCue {
                    start: Duration::from_secs(5),
                    end: Duration::from_secs(8),
                    text: "Second".to_string(),
                },
            ],
        };

        assert_eq!(track.text_at(Duration::from_secs(0)), None);
        assert_eq!(track.text_at(Duration::from_secs(2)), Some("First"));
        assert_eq!(track.text_at(Duration::from_secs(4)), None);
        assert_eq!(track.text_at(Duration::from_secs(6)), Some("Second"));
    }
}
