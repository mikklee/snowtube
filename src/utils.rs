//! Utility functions

use crate::error::{Error, Result};
use whatlang::{Lang, detect};

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

/// Detect language from text and return appropriate (language_code, region_code) tuple
pub fn detect_locale(text: &str) -> (String, String) {
    if let Some(info) = detect(text) {
        let lang = info.lang();

        println!(
            "Detected language: {:?} from text: {:?}",
            lang,
            text.chars().take(50).collect::<String>()
        );

        // Map language to (hl, gl) pairs - language code and most common region
        let locale = match lang {
            // East Asian languages
            Lang::Jpn => ("ja".to_string(), "JP".to_string()),
            Lang::Kor => ("ko".to_string(), "KR".to_string()),
            Lang::Cmn => ("zh-CN".to_string(), "CN".to_string()),

            // European languages
            Lang::Spa => ("es".to_string(), "ES".to_string()),
            Lang::Fra => ("fr".to_string(), "FR".to_string()),
            Lang::Deu => ("de".to_string(), "DE".to_string()),
            Lang::Ita => ("it".to_string(), "IT".to_string()),
            Lang::Por => ("pt".to_string(), "BR".to_string()),
            Lang::Rus => ("ru".to_string(), "RU".to_string()),
            Lang::Pol => ("pl".to_string(), "PL".to_string()),
            Lang::Ukr => ("uk".to_string(), "UA".to_string()),
            Lang::Nld => ("nl".to_string(), "NL".to_string()),
            Lang::Swe => ("sv".to_string(), "SE".to_string()),
            Lang::Dan => ("da".to_string(), "DK".to_string()),
            Lang::Fin => ("fi".to_string(), "FI".to_string()),

            // Middle Eastern languages
            Lang::Ara => ("ar".to_string(), "SA".to_string()),
            Lang::Heb => ("he".to_string(), "IL".to_string()),
            Lang::Tur => ("tr".to_string(), "TR".to_string()),

            // South Asian languages
            Lang::Hin => ("hi".to_string(), "IN".to_string()),
            Lang::Ben => ("bn".to_string(), "BD".to_string()),
            Lang::Tam => ("ta".to_string(), "IN".to_string()),
            Lang::Tel => ("te".to_string(), "IN".to_string()),

            // Southeast Asian languages
            Lang::Tha => ("th".to_string(), "TH".to_string()),
            Lang::Vie => ("vi".to_string(), "VN".to_string()),
            Lang::Ind => ("id".to_string(), "ID".to_string()),

            // Other languages
            Lang::Ces => ("cs".to_string(), "CZ".to_string()),
            Lang::Ron => ("ro".to_string(), "RO".to_string()),
            Lang::Hun => ("hu".to_string(), "HU".to_string()),
            Lang::Ell => ("el".to_string(), "GR".to_string()),

            // Default to English for unhandled or English
            _ => ("en".to_string(), "US".to_string()),
        };

        println!("Applying locale: hl={}, gl={}", locale.0, locale.1);
        locale
    } else {
        // If detection fails, default to English/US
        println!("Language detection failed, applying default locale: hl=en, gl=US");
        ("en".to_string(), "US".to_string())
    }
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
