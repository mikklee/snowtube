//! Data models for YouTube API responses

use serde::{Deserialize, Serialize};

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

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub video_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub channel: Option<Channel>,
    pub view_count: Option<u64>,
    pub duration: Option<String>,
    pub published_text: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

/// Channel information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: Option<String>,
    pub name: String,
    pub url: Option<String>,
    pub thumbnail: Option<Vec<Thumbnail>>,
}

/// Thumbnail information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnail {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Video information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub video_id: String,
    pub title: String,
    pub description: Option<String>,
    pub channel: Channel,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    pub duration: Option<u64>,
    pub published_date: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
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
    pub author_thumbnail: Option<Vec<Thumbnail>>,
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
    pub channel: Option<Channel>,
    pub video_count: Option<u64>,
    pub thumbnails: Vec<Thumbnail>,
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
    pub thumbnails: Vec<Thumbnail>,
}
