//! Data models for YouTube API responses
//!
//! This module contains YouTube-specific types used for parsing API responses.
//! These are converted to common types at the client boundary.

use serde::{Deserialize, Serialize};

/// Platform name for YouTube
pub const PLATFORM_NAME: &str = "youtube";

/// InnerTube context for API requests
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InnerTubeContext {
    pub client: InnerTubeClient,
}

/// InnerTube client information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InnerTubeClient {
    pub client_name: String,
    pub client_version: String,
    pub hl: String,
    pub gl: String,
    pub user_agent: String,
}

/// YouTube search results (internal type, converted to common::SearchResults)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YtSearchResults {
    pub results: Vec<YtSearchResult>,
    pub continuation: Option<String>,
    pub detected_locale: Option<(String, String)>,
}

/// YouTube search result (internal type, converted to common::Video)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YtSearchResult {
    pub video_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub channel: Option<YtChannel>,
    pub view_count: Option<u64>,
    pub duration: Option<String>,
    pub published_text: Option<String>,
    pub thumbnails: Vec<YtThumbnail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_premium: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_short: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badges: Option<Vec<String>>,
}

/// YouTube channel (internal type, converted to common::Channel)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YtChannel {
    pub id: Option<String>,
    pub name: String,
    pub url: Option<String>,
    pub thumbnail: Option<Vec<YtThumbnail>>,
}

/// YouTube channel info (internal type, converted to common::ChannelInfo)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YtChannelInfo {
    pub id: String,
    pub name: String,
    pub handle: Option<String>,
    pub url: Option<String>,
    pub description: Option<String>,
    pub subscriber_count: Option<String>,
    pub video_count: Option<u64>,
    pub verified: Option<bool>,
    pub thumbnails: Vec<YtThumbnail>,
    pub banner: Option<Vec<YtThumbnail>>,
}

/// YouTube channel videos (internal type, converted to common::ChannelVideos)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YtChannelVideos {
    pub videos: Vec<YtSearchResult>,
    pub continuation: Option<String>,
    pub sort_filters: Option<Vec<YtSortFilter>>,
    pub detected_locale: Option<(String, String)>,
}

/// YouTube sort filter (internal type, converted to common::SortFilter)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YtSortFilter {
    pub label: String,
    pub is_selected: bool,
    pub continuation_token: Option<String>,
}

/// YouTube thumbnail
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YtThumbnail {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Video information (full details from video page)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub video_id: String,
    pub title: String,
    pub description: Option<String>,
    pub channel: YtChannel,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    pub duration: Option<u64>,
    pub published_date: Option<String>,
    pub thumbnails: Vec<YtThumbnail>,
    pub formats: Vec<Format>,
    pub adaptive_formats: Vec<Format>,
    pub captions: Option<Vec<Caption>>,
}

/// Video format information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Format {
    pub itag: u32,
    pub url: Option<String>,
    pub mime_type: String,
    pub bitrate: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub quality: Option<String>,
    pub quality_label: Option<String>,
    pub fps: Option<u32>,
    pub audio_quality: Option<String>,
    pub audio_sample_rate: Option<String>,
    pub content_length: Option<String>,
}

/// Caption/subtitle information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Caption {
    pub language_code: String,
    pub language_name: String,
    pub url: String,
    pub is_auto_generated: bool,
}

/// Comment information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: String,
    pub text: String,
    pub author: String,
    pub author_thumbnail: Option<Vec<YtThumbnail>>,
    pub like_count: Option<u64>,
    pub published_text: Option<String>,
    pub is_pinned: bool,
    pub is_hearted: bool,
    pub reply_count: Option<u64>,
}

/// Playlist information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub channel: Option<YtChannel>,
    pub video_count: Option<u64>,
    pub thumbnails: Vec<YtThumbnail>,
    pub videos: Vec<PlaylistVideo>,
}

/// Video in a playlist
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistVideo {
    pub video_id: String,
    pub title: String,
    pub index: Option<u64>,
    pub duration: Option<String>,
    pub thumbnails: Vec<YtThumbnail>,
}

// ============================================================================
// Conversion implementations from YouTube types to common types
// ============================================================================

impl From<YtThumbnail> for common::Thumbnail {
    fn from(t: YtThumbnail) -> Self {
        common::Thumbnail {
            url: t.url,
            width: t.width,
            height: t.height,
        }
    }
}

impl From<&YtThumbnail> for common::Thumbnail {
    fn from(t: &YtThumbnail) -> Self {
        common::Thumbnail {
            url: t.url.clone(),
            width: t.width,
            height: t.height,
        }
    }
}

impl From<YtChannel> for common::Channel {
    fn from(c: YtChannel) -> Self {
        common::Channel {
            id: c.id,
            name: c.name,
            url: c.url,
            thumbnails: c
                .thumbnail
                .map(|ts| ts.into_iter().map(|t| t.into()).collect())
                .unwrap_or_default(),
            verified: None,
        }
    }
}

impl From<&YtChannel> for common::Channel {
    fn from(c: &YtChannel) -> Self {
        common::Channel {
            id: c.id.clone(),
            name: c.name.clone(),
            url: c.url.clone(),
            thumbnails: c
                .thumbnail
                .as_ref()
                .map(|ts| ts.iter().map(|t| t.into()).collect())
                .unwrap_or_default(),
            verified: None,
        }
    }
}

impl From<YtSearchResult> for common::Video {
    fn from(r: YtSearchResult) -> Self {
        let video_id = r.video_id.unwrap_or_default();
        common::Video {
            platform_name: PLATFORM_NAME.to_string(),
            id: video_id.clone(),
            title: r.title,
            description: r.description,
            duration: r
                .duration
                .as_ref()
                .and_then(|d| common::parse_duration_string(d))
                .map(|d| d.as_secs()),
            duration_string: r.duration,
            view_count: r.view_count,
            like_count: None,
            published_text: r.published_text,
            thumbnails: r.thumbnails.into_iter().map(|t| t.into()).collect(),
            channel: r.channel.map(|c| c.into()),
            watch_url: format!("https://www.youtube.com/watch?v={}", video_id),
            instance: None,
            is_premium: r.is_premium,
            is_short: r.is_short,
            badges: r.badges,
        }
    }
}

impl From<&YtSearchResult> for common::Video {
    fn from(r: &YtSearchResult) -> Self {
        let video_id = r.video_id.clone().unwrap_or_default();
        common::Video {
            platform_name: PLATFORM_NAME.to_string(),
            id: video_id.clone(),
            title: r.title.clone(),
            description: r.description.clone(),
            duration: r
                .duration
                .as_ref()
                .and_then(|d| common::parse_duration_string(d))
                .map(|d| d.as_secs()),
            duration_string: r.duration.clone(),
            view_count: r.view_count,
            like_count: None,
            published_text: r.published_text.clone(),
            thumbnails: r.thumbnails.iter().map(|t| t.into()).collect(),
            channel: r.channel.as_ref().map(|c| c.into()),
            watch_url: format!("https://www.youtube.com/watch?v={}", video_id),
            instance: None,
            is_premium: r.is_premium,
            is_short: r.is_short,
            badges: r.badges.clone(),
        }
    }
}

impl From<YtSearchResults> for common::SearchResults {
    fn from(r: YtSearchResults) -> Self {
        common::SearchResults {
            results: r.results.into_iter().map(|v| v.into()).collect(),
            continuations: r
                .continuation
                .map(|token| {
                    vec![common::ContinuationToken {
                        platform_name: PLATFORM_NAME.to_string(),
                        token,
                    }]
                })
                .unwrap_or_default(),
            detected_locale: r.detected_locale,
        }
    }
}

impl From<YtSortFilter> for common::SortFilter {
    fn from(f: YtSortFilter) -> Self {
        common::SortFilter {
            label: f.label,
            is_selected: f.is_selected,
            continuation_token: f.continuation_token,
        }
    }
}

impl From<YtChannelInfo> for common::ChannelInfo {
    fn from(c: YtChannelInfo) -> Self {
        common::ChannelInfo {
            platform_name: PLATFORM_NAME.to_string(),
            id: c.id,
            name: c.name,
            handle: c.handle,
            url: c.url,
            description: c.description,
            subscriber_count: c.subscriber_count,
            video_count: c.video_count,
            verified: c.verified,
            thumbnails: c.thumbnails.into_iter().map(|t| t.into()).collect(),
            banner: c.banner.map(|b| b.into_iter().map(|t| t.into()).collect()),
            instance: None,
        }
    }
}

impl From<YtChannelVideos> for common::ChannelVideos {
    fn from(c: YtChannelVideos) -> Self {
        common::ChannelVideos {
            videos: c.videos.into_iter().map(|v| v.into()).collect(),
            continuation: c.continuation,
            sort_filters: c
                .sort_filters
                .map(|f| f.into_iter().map(|s| s.into()).collect()),
            detected_locale: c.detected_locale,
        }
    }
}

impl From<&VideoInfo> for common::Video {
    fn from(v: &VideoInfo) -> Self {
        common::Video {
            platform_name: PLATFORM_NAME.to_string(),
            id: v.video_id.clone(),
            title: v.title.clone(),
            description: v.description.clone(),
            duration: v.duration,
            duration_string: v.duration.map(common::format_duration),
            view_count: v.view_count,
            like_count: v.like_count,
            published_text: v.published_date.clone(),
            thumbnails: v.thumbnails.iter().map(|t| t.into()).collect(),
            channel: Some((&v.channel).into()),
            watch_url: format!("https://www.youtube.com/watch?v={}", v.video_id),
            instance: None,
            is_premium: None,
            is_short: None,
            badges: None,
        }
    }
}

impl From<VideoInfo> for common::Video {
    fn from(v: VideoInfo) -> Self {
        common::Video::from(&v)
    }
}
