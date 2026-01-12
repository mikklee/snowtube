//! PeerTube API response models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Convert ISO 8601 timestamp to relative time string
fn iso_to_relative_time(iso: &str) -> Option<String> {
    let dt = DateTime::parse_from_rfc3339(iso).ok()?;
    let now = Utc::now();
    let duration = now.signed_duration_since(dt);

    if duration.num_seconds() < 0 {
        return None; // Future date
    }

    let seconds = duration.num_seconds() as u64;
    Some(common::format_relative_time(seconds))
}

/// Platform name for PeerTube
pub const PLATFORM_NAME: &str = "peertube";

/// Search results response from PeerTube API
#[derive(Debug, Clone, Deserialize)]
pub struct ApiSearchResponse {
    pub total: u32,
    pub data: Vec<ApiVideo>,
}

/// PeerTube video from API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiVideo {
    pub id: u64,
    pub uuid: String,
    pub short_uuid: Option<String>,
    pub name: String,
    pub description: Option<String>,
    /// Duration in seconds
    pub duration: u64,
    pub views: u64,
    pub likes: u64,
    pub dislikes: u64,
    pub thumbnail_path: Option<String>,
    pub preview_path: Option<String>,
    pub published_at: Option<String>,
    pub originally_published_at: Option<String>,
    pub channel: ApiChannel,
    pub account: ApiAccount,
    #[serde(default)]
    pub files: Vec<VideoFile>,
    #[serde(default)]
    pub streaming_playlists: Vec<StreamingPlaylist>,
}

/// PeerTube channel from API (embedded in video responses)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiChannel {
    pub id: u64,
    pub name: String,
    pub display_name: String,
    pub host: String,
    #[serde(default)]
    pub avatars: Vec<Avatar>,
}

/// Full PeerTube channel details from /api/v1/video-channels/{handle}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiVideoChannel {
    pub id: u64,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub host: String,
    pub url: Option<String>,
    #[serde(default)]
    pub followers_count: u64,
    #[serde(default)]
    pub following_count: u64,
    #[serde(default)]
    pub avatars: Vec<Avatar>,
    #[serde(default)]
    pub banners: Vec<Avatar>,
}

/// PeerTube account from API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiAccount {
    pub id: u64,
    pub name: String,
    pub display_name: String,
    pub host: String,
}

/// Avatar/thumbnail image
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Avatar {
    pub path: String,
    pub width: Option<u32>,
}

/// Video file (direct download)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFile {
    pub id: u64,
    pub file_url: String,
    pub file_download_url: Option<String>,
    pub resolution: Resolution,
    pub size: Option<u64>,
    pub fps: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Video resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    pub id: u32,
    pub label: String,
}

/// HLS/DASH streaming playlist
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamingPlaylist {
    pub id: u64,
    #[serde(rename = "type")]
    pub playlist_type: u32, // 1 = HLS
    pub playlist_url: String,
    #[serde(default)]
    pub files: Vec<VideoFile>,
}

impl ApiVideo {
    /// Get the best quality direct video URL
    pub fn best_file_url(&self) -> Option<&str> {
        // First try direct files, sorted by resolution (highest first)
        if let Some(file) = self.files.iter().max_by_key(|f| f.resolution.id) {
            return Some(&file.file_url);
        }

        // Fall back to streaming playlist files
        for playlist in &self.streaming_playlists {
            if let Some(file) = playlist.files.iter().max_by_key(|f| f.resolution.id) {
                return Some(&file.file_url);
            }
        }

        // Fall back to HLS playlist URL
        self.streaming_playlists
            .first()
            .map(|p| p.playlist_url.as_str())
    }

    /// Get the instance URL from the channel's host
    fn instance_url(&self) -> String {
        format!("https://{}", self.channel.host)
    }

    /// Convert to common::Video (derives instance from channel.host)
    /// Used for SepiaSearch results where instance isn't known upfront
    pub fn to_common_video(&self) -> common::Video {
        let instance = self.instance_url();
        self.to_common_video_with_instance(&instance)
    }

    /// Convert to common::Video with explicit instance URL
    /// Used when fetching from a specific instance
    pub fn to_common_video_with_instance(&self, instance: &str) -> common::Video {
        common::Video {
            platform_name: PLATFORM_NAME.to_string(),
            id: self.uuid.clone(),
            title: self.name.clone(),
            description: self.description.clone(),
            duration: Some(self.duration),
            duration_string: Some(common::format_duration(self.duration)),
            view_count: Some(self.views),
            like_count: Some(self.likes),
            published_text: self
                .published_at
                .as_ref()
                .and_then(|ts| iso_to_relative_time(ts)),
            thumbnails: self
                .thumbnail_path
                .as_ref()
                .map(|p| {
                    vec![common::Thumbnail {
                        url: format!("{}{}", instance, p),
                        width: None,
                        height: None,
                    }]
                })
                .unwrap_or_default(),
            channel: Some(common::Channel {
                id: Some(self.channel.name.clone()), // Use channel handle, not numeric ID
                name: self.channel.display_name.clone(),
                url: None,
                thumbnails: self
                    .channel
                    .avatars
                    .iter()
                    .map(|a| common::Thumbnail {
                        url: format!("{}{}", instance, a.path),
                        width: a.width,
                        height: None,
                    })
                    .collect(),
                verified: None,
            }),
            watch_url: format!("{}/w/{}", instance.trim_end_matches('/'), self.uuid),
            instance: Some(instance.to_string()),
            is_premium: Some(false),
            is_short: Some(false),
            badges: Some(vec![]),
        }
    }
}

impl ApiVideoChannel {
    /// Convert to common::ChannelInfo
    pub fn to_channel_info(&self, instance: &str) -> common::ChannelInfo {
        common::ChannelInfo {
            platform_name: PLATFORM_NAME.to_string(),
            id: self.name.clone(),
            name: self.display_name.clone(),
            handle: Some(format!("{}@{}", self.name, self.host)),
            url: self.url.clone(),
            description: self.description.clone(),
            subscriber_count: Some(self.followers_count.to_string()),
            video_count: None,
            verified: None,
            thumbnails: self
                .avatars
                .iter()
                .map(|a| common::Thumbnail {
                    url: format!("{}{}", instance, a.path),
                    width: a.width,
                    height: None,
                })
                .collect(),
            banner: if self.banners.is_empty() {
                None
            } else {
                Some(
                    self.banners
                        .iter()
                        .map(|b| common::Thumbnail {
                            url: format!("{}{}", instance, b.path),
                            width: b.width,
                            height: None,
                        })
                        .collect(),
                )
            },
            instance: Some(instance.to_string()),
        }
    }
}
