//! Utility functions

use crate::error::{Error, Result};

/// Get the high-resolution thumbnail URL for a video.
/// Returns the maxresdefault (1280x720) thumbnail URL.
pub fn get_hq_thumbnail_url(video_id: &str) -> String {
    format!("https://i.ytimg.com/vi/{}/maxresdefault.jpg", video_id)
}

/// Extract video ID from various YouTube URL formats
pub fn extract_video_id(input: &str) -> Result<String> {
    // If it's already just an ID (11 characters)
    if input.len() == 11 && !input.contains('/') && !input.contains('?') {
        return Ok(input.to_string());
    }

    // Try to parse as URL
    if let Ok(url) = url::Url::parse(input) {
        // Check for youtube.com/watch?v=VIDEO_ID
        if let Some(query) = url.query() {
            for pair in query.split('&') {
                if let Some(v) = pair.strip_prefix("v=") {
                    return Ok(v.to_string());
                }
            }
        }

        // Check for youtu.be/VIDEO_ID
        if url.host_str() == Some("youtu.be")
            && let Some(mut segments) = url.path_segments()
            && let Some(id) = segments.next()
        {
            return Ok(id.to_string());
        }

        // Check for youtube.com/embed/VIDEO_ID
        if url.path().starts_with("/embed/")
            && let Some(id) = url.path().strip_prefix("/embed/")
        {
            return Ok(id.split('/').next().unwrap_or(id).to_string());
        }
    }

    Err(Error::InvalidVideoId(input.to_string()))
}

/// Check if text contains Asian characters (CJK, Hangul, Thai, etc.)
/// Used for text truncation in UI and relative time parsing
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_id() {
        assert_eq!(extract_video_id("dQw4w9WgXcQ").unwrap(), "dQw4w9WgXcQ");
        assert_eq!(
            extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_video_id("https://youtu.be/dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_video_id("https://www.youtube.com/embed/dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
    }
}
