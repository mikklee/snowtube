//! Utility functions

use crate::error::{Error, Result};

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
        if url.host_str() == Some("youtu.be") {
            if let Some(mut segments) = url.path_segments() {
                if let Some(id) = segments.next() {
                    return Ok(id.to_string());
                }
            }
        }

        // Check for youtube.com/embed/VIDEO_ID
        if url.path().starts_with("/embed/") {
            if let Some(id) = url.path().strip_prefix("/embed/") {
                return Ok(id.split('/').next().unwrap_or(id).to_string());
            }
        }
    }

    Err(Error::InvalidVideoId(input.to_string()))
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
