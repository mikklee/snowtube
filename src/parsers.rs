//! Parsers for YouTube API responses

use crate::error::{Error, Result};
use crate::models::*;
use serde_json::Value;

/// Parse search results from InnerTube API response
pub fn parse_search_results(data: &Value) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();

    let contents = data
        .pointer("/contents/twoColumnSearchResultsRenderer/primaryContents/sectionListRenderer/contents")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::DataNotFound("search contents".to_string()))?;

    for section in contents {
        if let Some(items) = section
            .pointer("/itemSectionRenderer/contents")
            .and_then(|v| v.as_array())
        {
            for item in items {
                if let Some(video) = item.get("videoRenderer") {
                    if let Ok(result) = parse_video_renderer(video) {
                        results.push(result);
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Parse a video renderer object
fn parse_video_renderer(video: &Value) -> Result<SearchResult> {
    let video_id = video
        .pointer("/videoId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let title = extract_text(video.pointer("/title"))
        .unwrap_or_else(|| "Unknown".to_string());

    let description = extract_text(video.pointer("/descriptionSnippet"));

    let channel_name = extract_text(video.pointer("/ownerText"))
        .unwrap_or_else(|| "Unknown".to_string());

    let channel = Some(Channel {
        id: None,
        name: channel_name,
        url: None,
        thumbnail: None,
    });

    let view_count = extract_text(video.pointer("/viewCountText"))
        .and_then(|s| parse_view_count(&s));

    let duration = extract_text(video.pointer("/lengthText"));

    let published_text = extract_text(video.pointer("/publishedTimeText"));

    let thumbnails = parse_thumbnails(video.pointer("/thumbnail"));

    Ok(SearchResult {
        video_id,
        title,
        description,
        channel,
        view_count,
        duration,
        published_text,
        thumbnails,
    })
}

/// Extract text from various YouTube text objects
fn extract_text(value: Option<&Value>) -> Option<String> {
    if let Some(v) = value {
        // Try simpleText first
        if let Some(text) = v.pointer("/simpleText").and_then(|t| t.as_str()) {
            return Some(text.to_string());
        }

        // Try runs array
        if let Some(runs) = v.pointer("/runs").and_then(|r| r.as_array()) {
            let text: String = runs
                .iter()
                .filter_map(|run| run.pointer("/text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("");
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

/// Parse thumbnails from YouTube thumbnail object
fn parse_thumbnails(value: Option<&Value>) -> Vec<Thumbnail> {
    let mut thumbnails = Vec::new();

    if let Some(thumbs) = value
        .and_then(|v| v.pointer("/thumbnails"))
        .and_then(|t| t.as_array())
    {
        for thumb in thumbs {
            if let Some(url) = thumb.pointer("/url").and_then(|u| u.as_str()) {
                thumbnails.push(Thumbnail {
                    url: url.to_string(),
                    width: thumb.pointer("/width").and_then(|w| w.as_u64()).map(|w| w as u32),
                    height: thumb.pointer("/height").and_then(|h| h.as_u64()).map(|h| h as u32),
                });
            }
        }
    }

    thumbnails
}

/// Parse view count string like "1.2M views" to u64
fn parse_view_count(text: &str) -> Option<u64> {
    let text = text.replace(",", "").to_lowercase();
    let parts: Vec<&str> = text.split_whitespace().collect();

    if parts.is_empty() {
        return None;
    }

    let num_str = parts[0];

    if num_str.contains('k') {
        num_str.trim_end_matches('k').parse::<f64>().ok()
            .map(|n| (n * 1_000.0) as u64)
    } else if num_str.contains('m') {
        num_str.trim_end_matches('m').parse::<f64>().ok()
            .map(|n| (n * 1_000_000.0) as u64)
    } else if num_str.contains('b') {
        num_str.trim_end_matches('b').parse::<f64>().ok()
            .map(|n| (n * 1_000_000_000.0) as u64)
    } else {
        num_str.parse::<u64>().ok()
    }
}

/// Parse video info from player response
pub fn parse_video_info(data: &Value) -> Result<VideoInfo> {
    let video_details = data
        .pointer("/videoDetails")
        .ok_or_else(|| Error::DataNotFound("videoDetails".to_string()))?;

    let video_id = video_details
        .pointer("/videoId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::DataNotFound("videoId".to_string()))?
        .to_string();

    let title = video_details
        .pointer("/title")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let description = video_details
        .pointer("/shortDescription")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let channel_name = video_details
        .pointer("/author")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let channel_id = video_details
        .pointer("/channelId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let channel = Channel {
        id: channel_id,
        name: channel_name,
        url: None,
        thumbnail: None,
    };

    let view_count = video_details
        .pointer("/viewCount")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok());

    let like_count = None; // Would need to parse from microformat or engagement panels

    let duration = video_details
        .pointer("/lengthSeconds")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok());

    let thumbnails = parse_thumbnails(video_details.pointer("/thumbnail"));

    let formats = parse_formats(data.pointer("/streamingData/formats"));
    let adaptive_formats = parse_formats(data.pointer("/streamingData/adaptiveFormats"));

    Ok(VideoInfo {
        video_id,
        title,
        description,
        channel,
        view_count,
        like_count,
        duration,
        published_date: None,
        thumbnails,
        formats,
        adaptive_formats,
        captions: None,
    })
}

/// Parse video formats
fn parse_formats(value: Option<&Value>) -> Vec<Format> {
    let mut formats = Vec::new();

    if let Some(fmt_array) = value.and_then(|v| v.as_array()) {
        for fmt in fmt_array {
            if let Some(itag) = fmt.pointer("/itag").and_then(|i| i.as_u64()) {
                formats.push(Format {
                    itag: itag as u32,
                    url: fmt.pointer("/url").and_then(|u| u.as_str()).map(|s| s.to_string()),
                    mime_type: fmt.pointer("/mimeType").and_then(|m| m.as_str()).unwrap_or("").to_string(),
                    bitrate: fmt.pointer("/bitrate").and_then(|b| b.as_u64()),
                    width: fmt.pointer("/width").and_then(|w| w.as_u64()).map(|w| w as u32),
                    height: fmt.pointer("/height").and_then(|h| h.as_u64()).map(|h| h as u32),
                    quality: fmt.pointer("/quality").and_then(|q| q.as_str()).map(|s| s.to_string()),
                    quality_label: fmt.pointer("/qualityLabel").and_then(|q| q.as_str()).map(|s| s.to_string()),
                    fps: fmt.pointer("/fps").and_then(|f| f.as_u64()).map(|f| f as u32),
                    audio_quality: fmt.pointer("/audioQuality").and_then(|a| a.as_str()).map(|s| s.to_string()),
                    audio_sample_rate: fmt.pointer("/audioSampleRate").and_then(|a| a.as_str()).map(|s| s.to_string()),
                    content_length: fmt.pointer("/contentLength").and_then(|c| c.as_str()).map(|s| s.to_string()),
                });
            }
        }
    }

    formats
}
