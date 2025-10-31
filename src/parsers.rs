//! Parsers for YouTube API responses

use crate::error::{Error, Result};
use crate::models::*;
use serde_json::Value;

/// Parse search results from InnerTube API response
pub fn parse_search_results(data: &Value) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();

    let contents = data
        .pointer(
            "/contents/twoColumnSearchResultsRenderer/primaryContents/sectionListRenderer/contents",
        )
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

    let title = extract_text(video.pointer("/title")).unwrap_or_else(|| "Unknown".to_string());

    let description = extract_text(video.pointer("/descriptionSnippet"));

    let channel_name =
        extract_text(video.pointer("/ownerText")).unwrap_or_else(|| "Unknown".to_string());

    let channel_id = video
        .pointer("/ownerText/runs/0/navigationEndpoint/browseEndpoint/browseId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Get the canonical channel URL (e.g. "/@channelhandle")
    let channel_url = video
        .pointer("/ownerText/runs/0/navigationEndpoint/browseEndpoint/canonicalBaseUrl")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let channel_thumbnail =
        parse_thumbnails(video.pointer(
            "/channelThumbnailSupportedRenderers/channelThumbnailWithLinkRenderer/thumbnail",
        ));

    let channel = Some(Channel {
        id: channel_id,
        name: channel_name,
        url: channel_url,
        thumbnail: if !channel_thumbnail.is_empty() {
            Some(channel_thumbnail)
        } else {
            None
        },
    });

    let view_count =
        extract_text(video.pointer("/viewCountText")).and_then(|s| parse_view_count(&s));

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
                    width: thumb
                        .pointer("/width")
                        .and_then(|w| w.as_u64())
                        .map(|w| w as u32),
                    height: thumb
                        .pointer("/height")
                        .and_then(|h| h.as_u64())
                        .map(|h| h as u32),
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
        num_str
            .trim_end_matches('k')
            .parse::<f64>()
            .ok()
            .map(|n| (n * 1_000.0) as u64)
    } else if num_str.contains('m') {
        num_str
            .trim_end_matches('m')
            .parse::<f64>()
            .ok()
            .map(|n| (n * 1_000_000.0) as u64)
    } else if num_str.contains('b') {
        num_str
            .trim_end_matches('b')
            .parse::<f64>()
            .ok()
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
                    url: fmt
                        .pointer("/url")
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string()),
                    mime_type: fmt
                        .pointer("/mimeType")
                        .and_then(|m| m.as_str())
                        .unwrap_or("")
                        .to_string(),
                    bitrate: fmt.pointer("/bitrate").and_then(|b| b.as_u64()),
                    width: fmt
                        .pointer("/width")
                        .and_then(|w| w.as_u64())
                        .map(|w| w as u32),
                    height: fmt
                        .pointer("/height")
                        .and_then(|h| h.as_u64())
                        .map(|h| h as u32),
                    quality: fmt
                        .pointer("/quality")
                        .and_then(|q| q.as_str())
                        .map(|s| s.to_string()),
                    quality_label: fmt
                        .pointer("/qualityLabel")
                        .and_then(|q| q.as_str())
                        .map(|s| s.to_string()),
                    fps: fmt
                        .pointer("/fps")
                        .and_then(|f| f.as_u64())
                        .map(|f| f as u32),
                    audio_quality: fmt
                        .pointer("/audioQuality")
                        .and_then(|a| a.as_str())
                        .map(|s| s.to_string()),
                    audio_sample_rate: fmt
                        .pointer("/audioSampleRate")
                        .and_then(|a| a.as_str())
                        .map(|s| s.to_string()),
                    content_length: fmt
                        .pointer("/contentLength")
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }
    }

    formats
}

/// Parse channel info from InnerTube browse response
pub fn parse_channel_info(data: &Value) -> Result<ChannelInfo> {
    // Try to get metadata first (available in both formats)
    let metadata = data
        .pointer("/metadata/channelMetadataRenderer")
        .ok_or_else(|| Error::DataNotFound("channel metadata".to_string()))?;

    let channel_id = metadata
        .pointer("/externalId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::DataNotFound("channelId".to_string()))?
        .to_string();

    let name = metadata
        .pointer("/title")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let description = metadata
        .pointer("/description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let url = metadata
        .pointer("/channelUrl")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let handle = metadata
        .pointer("/vanityChannelUrl")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Avatar from metadata
    let thumbnails = metadata
        .pointer("/avatar/thumbnails")
        .and_then(|v| v.as_array())
        .map(|thumbs| {
            thumbs
                .iter()
                .filter_map(|thumb| {
                    thumb
                        .pointer("/url")
                        .and_then(|u| u.as_str())
                        .map(|url| Thumbnail {
                            url: url.to_string(),
                            width: thumb
                                .pointer("/width")
                                .and_then(|w| w.as_u64())
                                .map(|w| w as u32),
                            height: thumb
                                .pointer("/height")
                                .and_then(|h| h.as_u64())
                                .map(|h| h as u32),
                        })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Try to get additional info from header (may not always be present)
    let subscriber_count = data
        .pointer("/header/c4TabbedHeaderRenderer/subscriberCountText")
        .and_then(|v| extract_text(Some(v)))
        .or_else(|| {
            // Try pageHeaderRenderer format - subscriber count is in the second metadataRow
            data.pointer("/header/pageHeaderRenderer/content/pageHeaderViewModel/metadata/contentMetadataViewModel/metadataRows")
                .and_then(|rows| rows.as_array())
                .and_then(|rows| rows.get(1))  // Second row contains subscriber count and video count
                .and_then(|row| {
                    row.pointer("/metadataParts")
                        .and_then(|parts| parts.as_array())
                        .and_then(|parts| parts.first())  // First part is subscriber count
                        .and_then(|part| part.pointer("/text/content"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
        });

    let video_count = data
        .pointer("/header/c4TabbedHeaderRenderer/videosCountText/runs/0/text")
        .and_then(|v| v.as_str())
        .and_then(|s| s.replace(",", "").parse().ok());

    let verified = data
        .pointer("/header/c4TabbedHeaderRenderer/badges")
        .and_then(|v| v.as_array())
        .map(|badges| {
            badges.iter().any(|badge| {
                badge
                    .pointer("/metadataBadgeRenderer/style")
                    .and_then(|v| v.as_str())
                    .map(|s| s.contains("VERIFIED"))
                    .unwrap_or(false)
            })
        });

    // Try to get banner
    let banner = data
        .pointer("/header/c4TabbedHeaderRenderer/banner")
        .map(|b| parse_thumbnails(Some(b)))
        .filter(|v| !v.is_empty())
        .or_else(|| {
            // Try pageHeaderRenderer format
            data.pointer("/header/pageHeaderRenderer/content/pageHeaderViewModel/banner/imageBannerViewModel/image/sources")
                .and_then(|v| v.as_array())
                .map(|sources| {
                    sources
                        .iter()
                        .filter_map(|src| {
                            src.pointer("/url").and_then(|u| u.as_str()).map(|url| Thumbnail {
                                url: url.to_string(),
                                width: src
                                    .pointer("/width")
                                    .and_then(|w| w.as_u64())
                                    .map(|w| w as u32),
                                height: src
                                    .pointer("/height")
                                    .and_then(|h| h.as_u64())
                                    .map(|h| h as u32),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .filter(|v| !v.is_empty())
        });

    Ok(ChannelInfo {
        id: channel_id,
        name,
        handle,
        url,
        description,
        subscriber_count,
        video_count,
        verified,
        thumbnails,
        banner,
    })
}

/// Parse channel videos from InnerTube browse response
pub fn parse_channel_videos(data: &Value) -> Result<ChannelVideos> {
    let mut videos = Vec::new();
    let mut continuation = None;

    // Try to find the tab contents - could be in different locations depending on whether
    // this is the initial request or a continuation
    let contents = if let Some(c) = data.pointer("/contents/twoColumnBrowseResultsRenderer/tabs") {
        // Initial request - find the videos tab
        let tabs = c
            .as_array()
            .ok_or_else(|| Error::DataNotFound("tabs array".to_string()))?;

        let mut tab_contents = None;
        for tab in tabs {
            if let Some(tab_renderer) = tab.get("tabRenderer") {
                // Check if this is the videos tab (or if it's selected)
                if let Some(contents) = tab_renderer.pointer("/content/richGridRenderer/contents") {
                    tab_contents = Some(contents);
                    break;
                } else if let Some(contents) =
                    tab_renderer.pointer("/content/sectionListRenderer/contents")
                {
                    tab_contents = Some(contents);
                    break;
                }
            }
        }

        tab_contents.ok_or_else(|| Error::DataNotFound("tab contents".to_string()))?
    } else if let Some(c) =
        data.pointer("/onResponseReceivedActions/0/appendContinuationItemsAction/continuationItems")
    {
        // Continuation request
        c
    } else {
        return Err(Error::DataNotFound("channel videos contents".to_string()));
    };

    // Parse videos from the contents
    if let Some(items) = contents.as_array() {
        for item in items {
            // richItemRenderer is used in grid layout
            if let Some(video) = item.pointer("/richItemRenderer/content/videoRenderer") {
                if let Ok(result) = parse_video_renderer(video) {
                    videos.push(result);
                }
            }
            // shortsLockupViewModel is used for Shorts
            else if let Some(short) =
                item.pointer("/richItemRenderer/content/shortsLockupViewModel")
            {
                if let Ok(result) = parse_shorts_lockup(short) {
                    videos.push(result);
                }
            }
            // videoRenderer is used in list layout
            else if let Some(video) = item.get("videoRenderer") {
                if let Ok(result) = parse_video_renderer(video) {
                    videos.push(result);
                }
            }
            // gridVideoRenderer is another possible format
            else if let Some(video) = item.get("gridVideoRenderer") {
                if let Ok(result) = parse_grid_video_renderer(video) {
                    videos.push(result);
                }
            }
            // Check for continuation token
            else if let Some(token) = item
                .pointer("/continuationItemRenderer/continuationEndpoint/continuationCommand/token")
            {
                continuation = token.as_str().map(|s| s.to_string());
            }
        }
    }

    Ok(ChannelVideos {
        videos,
        continuation,
    })
}

/// Parse a grid video renderer (used in channel videos grid layout)
fn parse_grid_video_renderer(video: &Value) -> Result<SearchResult> {
    let video_id = video
        .pointer("/videoId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let title = extract_text(video.pointer("/title")).unwrap_or_else(|| "Unknown".to_string());

    let thumbnails = parse_thumbnails(video.pointer("/thumbnail"));

    let view_count =
        extract_text(video.pointer("/viewCountText")).and_then(|s| parse_view_count(&s));

    let duration =
        extract_text(video.pointer("/thumbnailOverlays/0/thumbnailOverlayTimeStatusRenderer/text"));

    let published_text = extract_text(video.pointer("/publishedTimeText"));

    Ok(SearchResult {
        video_id,
        title,
        description: None,
        channel: None,
        view_count,
        duration,
        published_text,
        thumbnails,
    })
}

/// Parse a shorts lockup (used in Shorts tab)
fn parse_shorts_lockup(short: &Value) -> Result<SearchResult> {
    // Extract video ID from the onTap command URL
    let video_id = short
        .pointer("/onTap/innertubeCommand/commandMetadata/webCommandMetadata/url")
        .and_then(|v| v.as_str())
        .and_then(|url| {
            // URL format is "/shorts/VIDEO_ID"
            url.strip_prefix("/shorts/").map(|s| s.to_string())
        });

    // Try to extract title from overlay or accessibility text
    let title = short
        .pointer("/overlayMetadata/primaryText/content")
        .and_then(|v| v.as_str())
        .or_else(|| {
            // Fallback to accessibility text
            short
                .pointer("/accessibilityText")
                .and_then(|v| v.as_str())
                .and_then(|text| {
                    // Extract title from accessibility text like "Title, 399 thousand views - play Short"
                    text.split(',').next()
                })
        })
        .unwrap_or("Short")
        .to_string();

    // Extract thumbnail - shorts use a different structure
    let thumbnails = short
        .pointer("/thumbnail/sources")
        .and_then(|v| v.as_array())
        .map(|sources| {
            sources
                .iter()
                .filter_map(|src| {
                    src.pointer("/url")
                        .and_then(|u| u.as_str())
                        .map(|url| Thumbnail {
                            url: url.to_string(),
                            width: src
                                .pointer("/width")
                                .and_then(|w| w.as_u64())
                                .map(|w| w as u32),
                            height: src
                                .pointer("/height")
                                .and_then(|h| h.as_u64())
                                .map(|h| h as u32),
                        })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Extract view count from accessibility text if available
    let view_count = short
        .pointer("/accessibilityText")
        .and_then(|v| v.as_str())
        .and_then(|text| {
            // Extract from text like "Title, 399 thousand views - play Short"
            if let Some(views_part) = text.split(',').nth(1) {
                if let Some(views_str) = views_part.trim().strip_suffix(" views - play Short") {
                    parse_view_count(views_str)
                } else {
                    None
                }
            } else {
                None
            }
        });

    Ok(SearchResult {
        video_id,
        title,
        description: None,
        channel: None,
        view_count,
        duration: None, // Shorts don't typically show duration
        published_text: None,
        thumbnails,
    })
}
